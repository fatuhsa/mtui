use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Generic Audio Backend Trait allowing complete separation from underlying audio player
pub trait AudioBackend: Send + 'static {
    fn load_file(&mut self, path: &Path) -> Result<(), String>;
    fn play(&mut self) -> Result<(), String>;
    fn pause(&mut self) -> Result<(), String>;
    fn seek(&mut self, seconds: f64) -> Result<(), String>;
    fn set_volume(&mut self, volume: u32) -> Result<(), String>;
    fn get_position(&mut self) -> Result<Option<f64>, String>;
    fn get_duration(&mut self) -> Result<Option<f64>, String>;
    fn is_idle(&mut self) -> Result<bool, String>;
    fn stop_playback(&mut self) -> Result<(), String> {
        self.stop()
    }
    fn stop(&mut self) -> Result<(), String>;
    fn shutdown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// MPV backend communicating over Unix Domain Socket JSON IPC
pub struct MpvBackend {
    child: Option<Child>,
    socket_path: PathBuf,
    stream: Option<UnixStream>,
    reader: Option<BufReader<UnixStream>>,
    request_counter: AtomicU64,
}

static BACKEND_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

impl MpvBackend {
    pub fn new() -> Result<Self, String> {
        let temp_dir = std::env::temp_dir();
        let pid = std::process::id();
        let seq = BACKEND_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
        let socket_path = temp_dir.join(format!("mtui_mpv_{}_{}.sock", pid, seq));

        // Clean up stale socket if present
        let _ = std::fs::remove_file(&socket_path);

        // Spawn mpv process
        // In Termux, mpv works seamlessly with --idle and Android audio drivers
        let child = Command::new("mpv")
            .arg("--idle=yes")
            .arg("--no-video")
            .arg("--no-terminal")
            .arg("--really-quiet")
            .arg(format!("--input-ipc-server={}", socket_path.display()))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to spawn mpv (is mpv installed?): {}", e))?;

        let mut backend = Self {
            child: Some(child),
            socket_path,
            stream: None,
            reader: None,
            request_counter: AtomicU64::new(1),
        };

        // Try connecting to IPC socket with retries (up to 1.5s)
        let start = Instant::now();
        let mut connected = false;
        while start.elapsed() < Duration::from_millis(1500) {
            if let Some(child) = backend.child.as_mut() {
                if let Ok(Some(status)) = child.try_wait() {
                    let _ = backend.shutdown();
                    return Err(format!(
                        "mpv process exited prematurely with status: {}",
                        status
                    ));
                }
            }
            if backend.socket_path.exists() {
                if let Ok(stream) = UnixStream::connect(&backend.socket_path) {
                    stream
                        .set_read_timeout(Some(Duration::from_millis(200)))
                        .ok();
                    stream
                        .set_write_timeout(Some(Duration::from_millis(200)))
                        .ok();
                    let reader = BufReader::new(
                        stream
                            .try_clone()
                            .map_err(|e| format!("Failed to clone stream: {}", e))?,
                    );
                    backend.stream = Some(stream);
                    backend.reader = Some(reader);
                    connected = true;
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(50));
        }

        if !connected {
            let _ = backend.shutdown();
            return Err("Timed out connecting to mpv IPC socket".to_string());
        }

        // Set default volume
        backend.set_volume(80).ok();

        Ok(backend)
    }

    fn send_ipc_command(
        &mut self,
        command: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let req_id = self.request_counter.fetch_add(1, Ordering::SeqCst);
        let payload = serde_json::json!({
            "command": command,
            "request_id": req_id
        });

        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| "MPV IPC stream not connected".to_string())?;

        let mut json_str =
            serde_json::to_string(&payload).map_err(|e| format!("Serialization error: {}", e))?;
        json_str.push('\n');

        stream
            .write_all(json_str.as_bytes())
            .map_err(|e| format!("Socket write error: {}", e))?;
        stream
            .flush()
            .map_err(|e| format!("Socket flush error: {}", e))?;

        // Read response
        let reader = self
            .reader
            .as_mut()
            .ok_or_else(|| "MPV IPC reader not available".to_string())?;

        let mut line = String::new();
        let read_start = Instant::now();

        // Loop reading lines until matching request_id or timeout
        while read_start.elapsed() < Duration::from_millis(400) {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => return Err("IPC socket closed".to_string()),
                Ok(_) => {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                        if let Some(id) = val.get("request_id").and_then(|v| v.as_u64()) {
                            if id == req_id {
                                if let Some(err) = val.get("error").and_then(|v| v.as_str()) {
                                    if err == "success" {
                                        return Ok(val
                                            .get("data")
                                            .cloned()
                                            .unwrap_or(serde_json::Value::Null));
                                    } else {
                                        return Err(format!("mpv error: {}", err));
                                    }
                                }
                                return Ok(val);
                            }
                        }
                    }
                }
                Err(ref e)
                    if e.kind() == std::io::ErrorKind::TimedOut
                        || e.kind() == std::io::ErrorKind::WouldBlock =>
                {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) => return Err(format!("Socket read error: {}", e)),
            }
        }

        Err("MPV IPC request timed out".to_string())
    }

    fn get_property<T: serde::de::DeserializeOwned>(
        &mut self,
        property: &str,
    ) -> Result<Option<T>, String> {
        let val = match self.send_ipc_command(vec![
            serde_json::json!("get_property"),
            serde_json::json!(property),
        ]) {
            Ok(v) => v,
            Err(e) if e.contains("property unavailable") => return Ok(None),
            Err(e) => return Err(e),
        };

        if val.is_null() {
            Ok(None)
        } else {
            match serde_json::from_value::<T>(val) {
                Ok(parsed) => Ok(Some(parsed)),
                Err(_) => Ok(None),
            }
        }
    }
}

impl AudioBackend for MpvBackend {
    fn load_file(&mut self, path: &Path) -> Result<(), String> {
        let path_str = path
            .to_str()
            .ok_or_else(|| "Invalid UTF-8 in path".to_string())?;
        self.send_ipc_command(vec![
            serde_json::json!("loadfile"),
            serde_json::json!(path_str),
            serde_json::json!("replace"),
        ])?;
        Ok(())
    }

    fn play(&mut self) -> Result<(), String> {
        self.send_ipc_command(vec![
            serde_json::json!("set_property"),
            serde_json::json!("pause"),
            serde_json::json!(false),
        ])?;
        Ok(())
    }

    fn pause(&mut self) -> Result<(), String> {
        self.send_ipc_command(vec![
            serde_json::json!("set_property"),
            serde_json::json!("pause"),
            serde_json::json!(true),
        ])?;
        Ok(())
    }

    fn seek(&mut self, seconds: f64) -> Result<(), String> {
        self.send_ipc_command(vec![
            serde_json::json!("seek"),
            serde_json::json!(seconds),
            serde_json::json!("absolute"),
        ])?;
        Ok(())
    }

    fn set_volume(&mut self, volume: u32) -> Result<(), String> {
        let clamped = volume.min(100);
        self.send_ipc_command(vec![
            serde_json::json!("set_property"),
            serde_json::json!("volume"),
            serde_json::json!(clamped),
        ])?;
        Ok(())
    }

    fn get_position(&mut self) -> Result<Option<f64>, String> {
        self.get_property::<f64>("time-pos")
    }

    fn get_duration(&mut self) -> Result<Option<f64>, String> {
        self.get_property::<f64>("duration")
    }

    fn is_idle(&mut self) -> Result<bool, String> {
        let idle = self.get_property::<bool>("idle-active")?.unwrap_or(true);
        Ok(idle)
    }

    fn stop(&mut self) -> Result<(), String> {
        self.send_ipc_command(vec![serde_json::json!("stop")])
            .map(|_| ())
    }

    fn shutdown(&mut self) -> Result<(), String> {
        let _ = self.send_ipc_command(vec![serde_json::json!("stop")]);
        let _ = self.send_ipc_command(vec![serde_json::json!("quit")]);
        self.stream = None;
        self.reader = None;
        if let Some(mut child) = self.child.take() {
            let mut exited = false;
            for _ in 0..10 {
                if let Ok(Some(_)) = child.try_wait() {
                    exited = true;
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            if !exited {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        let _ = std::fs::remove_file(&self.socket_path);
        Ok(())
    }
}

impl Drop for MpvBackend {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

/// In-memory Mock Backend for automated testing without audio drivers or mpv
#[derive(Debug)]
pub struct MockBackend {
    pub loaded_path: Option<PathBuf>,
    pub is_playing: bool,
    pub position: f64,
    pub duration: f64,
    pub volume: u32,
    pub is_idle_state: bool,
}

impl Default for MockBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl MockBackend {
    pub fn new() -> Self {
        Self {
            loaded_path: None,
            is_playing: false,
            position: 0.0,
            duration: 180.0,
            volume: 80,
            is_idle_state: true,
        }
    }
}

impl AudioBackend for MockBackend {
    fn load_file(&mut self, path: &Path) -> Result<(), String> {
        self.loaded_path = Some(path.to_path_buf());
        self.position = 0.0;
        self.is_playing = true;
        self.is_idle_state = false;
        Ok(())
    }

    fn play(&mut self) -> Result<(), String> {
        self.is_playing = true;
        Ok(())
    }

    fn pause(&mut self) -> Result<(), String> {
        self.is_playing = false;
        Ok(())
    }

    fn seek(&mut self, seconds: f64) -> Result<(), String> {
        self.position = seconds.clamp(0.0, self.duration);
        Ok(())
    }

    fn set_volume(&mut self, volume: u32) -> Result<(), String> {
        self.volume = volume.min(100);
        Ok(())
    }

    fn get_position(&mut self) -> Result<Option<f64>, String> {
        Ok(Some(self.position))
    }

    fn get_duration(&mut self) -> Result<Option<f64>, String> {
        Ok(Some(self.duration))
    }

    fn is_idle(&mut self) -> Result<bool, String> {
        Ok(self.is_idle_state)
    }

    fn stop(&mut self) -> Result<(), String> {
        self.is_playing = false;
        self.is_idle_state = true;
        self.position = 0.0;
        Ok(())
    }
}
