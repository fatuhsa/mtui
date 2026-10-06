use crate::engine::track::Track;

/// Current status of audio playback
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PlaybackStatus {
    Stopped,
    Playing,
    Paused,
}

/// Loop / repeat modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LoopMode {
    Off,
    Track,
    All,
}

impl LoopMode {
    pub fn next(&self) -> Self {
        match self {
            LoopMode::Off => LoopMode::All,
            LoopMode::All => LoopMode::Track,
            LoopMode::Track => LoopMode::Off,
        }
    }

    pub fn display_label(&self) -> &'static str {
        match self {
            LoopMode::Off => "Off",
            LoopMode::Track => "Track",
            LoopMode::All => "All",
        }
    }
}

/// Complete snapshot of the music engine state, emitted to any UI frontend
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EngineStateSnapshot {
    pub current_track: Option<Track>,
    pub status: PlaybackStatus,
    pub position_secs: f64,
    pub duration_secs: f64,
    pub volume: u32,
    pub queue: Vec<Track>,
    pub queue_index: Option<usize>,
    pub loop_mode: LoopMode,
    pub shuffle_enabled: bool,
    pub error_message: Option<String>,
}

impl Default for EngineStateSnapshot {
    fn default() -> Self {
        Self {
            current_track: None,
            status: PlaybackStatus::Stopped,
            position_secs: 0.0,
            duration_secs: 0.0,
            volume: 100,
            queue: Vec::new(),
            queue_index: None,
            loop_mode: LoopMode::Off,
            shuffle_enabled: false,
            error_message: None,
        }
    }
}

/// Events emitted asynchronously by the audio engine
#[derive(Debug, Clone)]
pub enum EngineEvent {
    StateUpdated(Box<EngineStateSnapshot>),
    TrackStarted(Track),
    TrackEnded,
    Error(String),
}
