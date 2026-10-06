use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Deterministic 64-bit FNV-1a hash ensuring persistent cache keys across process runs
pub fn fnv1a_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Hash audio path into a deterministic hex string for persistent caching
pub fn hash_audio_source(audio_path: &Path) -> String {
    let hash = fnv1a_hash(audio_path.to_string_lossy().as_bytes());
    format!("{:016x}", hash)
}

/// Discovers or extracts cover art for an audio track
pub fn get_or_extract_cover_art(audio_path: &Path) -> Option<PathBuf> {
    let parent = audio_path.parent()?;

    // 1. Check for standard image files in same directory
    let common_names = [
        "cover.jpg",
        "cover.png",
        "folder.jpg",
        "front.jpg",
        "album.jpg",
    ];
    for name in &common_names {
        let candidate = parent.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    // 2. Prepare cache directory in temp
    let temp_dir = std::env::temp_dir().join("mtui_covers");
    let _ = std::fs::create_dir_all(&temp_dir);

    let hash_str = hash_audio_source(audio_path);
    let cache_file = temp_dir.join(format!("{}.jpg", hash_str));
    if cache_file.is_file() {
        if let Ok(cache_meta) = cache_file.metadata() {
            if cache_meta.len() > 100 {
                let is_fresh = if let (Ok(audio_meta), Ok(cache_mtime)) =
                    (audio_path.metadata(), cache_meta.modified())
                {
                    if let Ok(audio_mtime) = audio_meta.modified() {
                        cache_mtime >= audio_mtime
                    } else {
                        true
                    }
                } else {
                    true
                };
                if is_fresh {
                    return Some(cache_file);
                }
            }
        }
    }

    // 3. Extract embedded cover art via ffmpeg to temporary file first (atomic write)
    let temp_file = temp_dir.join(format!("{}_{}.tmp", hash_str, std::process::id()));
    let status = Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(audio_path)
        .arg("-an")
        .arg("-vframes")
        .arg("1")
        .arg("-f")
        .arg("image2")
        .arg("-c:v")
        .arg("mjpeg")
        .arg(&temp_file)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if let Ok(st) = status {
        if st.success() && temp_file.is_file() {
            if let Ok(meta) = temp_file.metadata() {
                if meta.len() > 100 && std::fs::rename(&temp_file, &cache_file).is_ok() {
                    return Some(cache_file);
                }
            }
        }
    }

    // Clean up partial or failed temp output
    let _ = std::fs::remove_file(&temp_file);
    None
}
