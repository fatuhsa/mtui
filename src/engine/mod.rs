pub mod backend;
pub mod commands;
pub mod events;
pub mod playlist;
pub mod track;

use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub use backend::{AudioBackend, MockBackend, MpvBackend};
pub use commands::EngineCommand;
pub use events::{EngineEvent, EngineStateSnapshot, LoopMode, PlaybackStatus};
pub use playlist::Playlist;
pub use track::Track;

/// Main Audio Engine orchestrator, running fully independent of any UI or terminal framework
pub struct AudioEngine {
    command_tx: Sender<EngineCommand>,
    _worker_thread: JoinHandle<()>,
}

impl AudioEngine {
    /// Launches the audio engine with a specific backend (e.g. MpvBackend or MockBackend)
    /// Returns the AudioEngine controller and a Receiver for state snapshots
    pub fn start<B: AudioBackend>(
        mut backend: B,
    ) -> (Self, Receiver<EngineStateSnapshot>) {
        let (command_tx, command_rx) = mpsc::channel::<EngineCommand>();
        let (state_tx, state_rx) = mpsc::channel::<EngineStateSnapshot>();

        let worker_thread = thread::spawn(move || {
            let mut playlist = Playlist::new();
            let mut status = PlaybackStatus::Stopped;
            let mut volume: u32 = 80;
            let mut position_secs: f64 = 0.0;
            let mut duration_secs: f64 = 0.0;
            let mut error_msg: Option<String> = None;
            let mut last_state_broadcast = Instant::now();

            let emit_state = |playlist: &Playlist,
                              status: PlaybackStatus,
                              pos: f64,
                              dur: f64,
                              vol: u32,
                              err: &Option<String>,
                              tx: &Sender<EngineStateSnapshot>| {
                let snapshot = EngineStateSnapshot {
                    current_track: playlist.current_track().cloned(),
                    status,
                    position_secs: pos,
                    duration_secs: dur,
                    volume: vol,
                    queue: playlist.tracks().to_vec(),
                    queue_index: playlist.current_index(),
                    loop_mode: playlist.loop_mode(),
                    shuffle_enabled: playlist.is_shuffle(),
                    error_message: err.clone(),
                };
                let _ = tx.send(snapshot);
            };

            // Emit initial state
            emit_state(
                &playlist,
                status,
                position_secs,
                duration_secs,
                volume,
                &error_msg,
                &state_tx,
            );

            loop {
                // Poll command with timeout
                let timeout = if status == PlaybackStatus::Playing {
                    Duration::from_millis(150)
                } else {
                    Duration::from_millis(300)
                };

                let cmd_res = command_rx.recv_timeout(timeout);
                let mut state_changed = false;

                if let Ok(cmd) = cmd_res {
                    match cmd {
                        EngineCommand::Quit => {
                            let _ = backend.shutdown();
                            break;
                        }
                        EngineCommand::Play => {
                            if let Some(track) = playlist.current_track() {
                                if status == PlaybackStatus::Paused {
                                    if let Err(e) = backend.play() {
                                        error_msg = Some(e);
                                    } else {
                                        status = PlaybackStatus::Playing;
                                        error_msg = None;
                                    }
                                } else {
                                    if let Err(e) = backend.load_file(&track.path) {
                                        error_msg = Some(e);
                                    } else if let Err(e) = backend.play() {
                                        error_msg = Some(e);
                                    } else {
                                        status = PlaybackStatus::Playing;
                                        position_secs = 0.0;
                                        error_msg = None;
                                    }
                                }
                                state_changed = true;
                            }
                        }
                        EngineCommand::Pause => {
                            if status == PlaybackStatus::Playing {
                                if let Err(e) = backend.pause() {
                                    error_msg = Some(e);
                                } else {
                                    status = PlaybackStatus::Paused;
                                    error_msg = None;
                                }
                                state_changed = true;
                            }
                        }
                        EngineCommand::TogglePlay => {
                            match status {
                                PlaybackStatus::Playing => {
                                    if let Err(e) = backend.pause() {
                                        error_msg = Some(e);
                                    } else {
                                        status = PlaybackStatus::Paused;
                                        error_msg = None;
                                    }
                                }
                                PlaybackStatus::Paused => {
                                    if let Err(e) = backend.play() {
                                        error_msg = Some(e);
                                    } else {
                                        status = PlaybackStatus::Playing;
                                        error_msg = None;
                                    }
                                }
                                PlaybackStatus::Stopped => {
                                    if playlist.current_track().is_none() && !playlist.tracks().is_empty() {
                                        playlist.select_index(0);
                                    }
                                    if let Some(track) = playlist.current_track() {
                                        if let Err(e) = backend.load_file(&track.path) {
                                            error_msg = Some(e);
                                        } else if let Err(e) = backend.play() {
                                            error_msg = Some(e);
                                        } else {
                                            status = PlaybackStatus::Playing;
                                            position_secs = 0.0;
                                            error_msg = None;
                                        }
                                    }
                                }
                            }
                            state_changed = true;
                        }
                        EngineCommand::Stop => {
                            let _ = backend.stop();
                            status = PlaybackStatus::Stopped;
                            position_secs = 0.0;
                            state_changed = true;
                        }
                        EngineCommand::Next => {
                            if let Some(_) = playlist.next() {
                                if let Some(track) = playlist.current_track() {
                                    if let Err(e) = backend.load_file(&track.path) {
                                        error_msg = Some(e);
                                    } else if let Err(e) = backend.play() {
                                        error_msg = Some(e);
                                    } else {
                                        status = PlaybackStatus::Playing;
                                        position_secs = 0.0;
                                        duration_secs = 0.0;
                                        error_msg = None;
                                    }
                                }
                            } else {
                                let _ = backend.stop();
                                status = PlaybackStatus::Stopped;
                            }
                            state_changed = true;
                        }
                        EngineCommand::Prev => {
                            // If current track has played more than 3 seconds, replay from start
                            if position_secs > 3.0 {
                                if let Err(e) = backend.seek(0.0) {
                                    error_msg = Some(e);
                                } else {
                                    position_secs = 0.0;
                                }
                            } else if let Some(_) = playlist.prev() {
                                if let Some(track) = playlist.current_track() {
                                    if let Err(e) = backend.load_file(&track.path) {
                                        error_msg = Some(e);
                                    } else if let Err(e) = backend.play() {
                                        error_msg = Some(e);
                                    } else {
                                        status = PlaybackStatus::Playing;
                                        position_secs = 0.0;
                                        duration_secs = 0.0;
                                        error_msg = None;
                                    }
                                }
                            }
                            state_changed = true;
                        }
                        EngineCommand::Seek(secs) => {
                            let target = secs.clamp(0.0, duration_secs.max(1.0));
                            if let Err(e) = backend.seek(target) {
                                error_msg = Some(e);
                            } else {
                                position_secs = target;
                            }
                            state_changed = true;
                        }
                        EngineCommand::SeekPercent(pct) => {
                            if duration_secs > 0.0 {
                                let target = (pct.clamp(0.0, 1.0) * duration_secs).clamp(0.0, duration_secs);
                                if let Err(e) = backend.seek(target) {
                                    error_msg = Some(e);
                                } else {
                                    position_secs = target;
                                }
                                state_changed = true;
                            }
                        }
                        EngineCommand::SeekRelative(offset) => {
                            let target = (position_secs + offset).clamp(0.0, duration_secs.max(1.0));
                            if let Err(e) = backend.seek(target) {
                                error_msg = Some(e);
                            } else {
                                position_secs = target;
                            }
                            state_changed = true;
                        }
                        EngineCommand::SetVolume(vol) => {
                            let target_vol = vol.min(100);
                            if let Err(e) = backend.set_volume(target_vol) {
                                error_msg = Some(e);
                            } else {
                                volume = target_vol;
                            }
                            state_changed = true;
                        }
                        EngineCommand::AdjustVolume(delta) => {
                            let target_vol = ((volume as i32) + delta).clamp(0, 100) as u32;
                            if let Err(e) = backend.set_volume(target_vol) {
                                error_msg = Some(e);
                            } else {
                                volume = target_vol;
                            }
                            state_changed = true;
                        }
                        EngineCommand::AddTrack(track) => {
                            let was_empty = playlist.tracks().is_empty();
                            playlist.add_track(track);
                            if was_empty {
                                playlist.select_index(0);
                            }
                            state_changed = true;
                        }
                        EngineCommand::AddTracks(tracks) => {
                            let was_empty = playlist.tracks().is_empty();
                            playlist.add_tracks(tracks);
                            if was_empty && !playlist.tracks().is_empty() {
                                playlist.select_index(0);
                            }
                            state_changed = true;
                        }
                        EngineCommand::PlayTrackNow(track) => {
                            playlist.insert_and_select(track);
                            if let Some(cur) = playlist.current_track() {
                                if let Err(e) = backend.load_file(&cur.path) {
                                    error_msg = Some(e);
                                } else if let Err(e) = backend.play() {
                                    error_msg = Some(e);
                                } else {
                                    status = PlaybackStatus::Playing;
                                    position_secs = 0.0;
                                    duration_secs = 0.0;
                                    error_msg = None;
                                }
                            }
                            state_changed = true;
                        }
                        EngineCommand::PlayIndex(idx) => {
                            if let Some(track) = playlist.select_index(idx) {
                                if let Err(e) = backend.load_file(&track.path) {
                                    error_msg = Some(e);
                                } else if let Err(e) = backend.play() {
                                    error_msg = Some(e);
                                } else {
                                    status = PlaybackStatus::Playing;
                                    position_secs = 0.0;
                                    duration_secs = 0.0;
                                    error_msg = None;
                                }
                                state_changed = true;
                            }
                        }
                        EngineCommand::RemoveIndex(idx) => {
                            playlist.remove_index(idx);
                            state_changed = true;
                        }
                        EngineCommand::ClearQueue => {
                            let _ = backend.stop();
                            playlist.clear();
                            status = PlaybackStatus::Stopped;
                            position_secs = 0.0;
                            duration_secs = 0.0;
                            state_changed = true;
                        }
                        EngineCommand::SetLoopMode(mode) => {
                            playlist.set_loop_mode(mode);
                            state_changed = true;
                        }
                        EngineCommand::CycleLoopMode => {
                            playlist.cycle_loop_mode();
                            state_changed = true;
                        }
                        EngineCommand::ToggleShuffle => {
                            playlist.toggle_shuffle();
                            state_changed = true;
                        }
                    }
                }

                // Periodic playback query when playing
                if status == PlaybackStatus::Playing {
                    if let Ok(Some(pos)) = backend.get_position() {
                        position_secs = pos;
                    }
                    if let Ok(Some(dur)) = backend.get_duration() {
                        if dur > 0.0 {
                            duration_secs = dur;
                        }
                    }

                    // Check if track ended
                    if let Ok(idle) = backend.is_idle() {
                        if idle && position_secs > 0.0 {
                            // Track has finished playing!
                            if let Some(_) = playlist.next() {
                                if let Some(track) = playlist.current_track() {
                                    if let Err(e) = backend.load_file(&track.path) {
                                        error_msg = Some(e);
                                        status = PlaybackStatus::Stopped;
                                    } else if let Err(e) = backend.play() {
                                        error_msg = Some(e);
                                        status = PlaybackStatus::Stopped;
                                    } else {
                                        position_secs = 0.0;
                                        duration_secs = 0.0;
                                    }
                                }
                            } else {
                                status = PlaybackStatus::Stopped;
                                position_secs = 0.0;
                            }
                            state_changed = true;
                        }
                    }

                    // Force broadcast during playback for smooth progress updates
                    if last_state_broadcast.elapsed() >= Duration::from_millis(150) {
                        state_changed = true;
                    }
                }

                if state_changed {
                    emit_state(
                        &playlist,
                        status,
                        position_secs,
                        duration_secs,
                        volume,
                        &error_msg,
                        &state_tx,
                    );
                    last_state_broadcast = Instant::now();
                }
            }
        });

        (
            Self {
                command_tx,
                _worker_thread: worker_thread,
            },
            state_rx,
        )
    }

    /// Dispatches a command asynchronously to the engine
    pub fn send(&self, command: EngineCommand) {
        let _ = self.command_tx.send(command);
    }

    /// Clones the sender channel so external components can also send commands
    pub fn command_sender(&self) -> Sender<EngineCommand> {
        self.command_tx.clone()
    }
}
