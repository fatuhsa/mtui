use crate::engine::events::LoopMode;
use crate::engine::track::Track;

/// Commands that any UI frontend or external controller can send to the engine
#[derive(Debug, Clone, PartialEq)]
pub enum EngineCommand {
    /// Play currently loaded track
    Play,
    /// Pause playback
    Pause,
    /// Toggle between Play and Pause
    TogglePlay,
    /// Stop playback completely
    Stop,
    /// Skip to next track in queue
    Next,
    /// Skip to previous track in queue
    Prev,
    /// Seek to an absolute timestamp in seconds
    Seek(f64),
    /// Seek to a percentage of duration (0.0 to 1.0) - ideal for touch progress bar
    SeekPercent(f64),
    /// Seek relative offset in seconds (e.g. +5.0 or -5.0)
    SeekRelative(f64),
    /// Set volume directly (0 to 100)
    SetVolume(u32),
    /// Adjust volume relatively (e.g. +5 or -5)
    AdjustVolume(i32),
    /// Append a single track to the end of the queue
    AddTrack(Track),
    /// Append multiple tracks to the end of the queue
    AddTracks(Vec<Track>),
    /// Play a track immediately (prepends or inserts and starts)
    PlayTrackNow(Track),
    /// Jump directly to a track at the specified queue index
    PlayIndex(usize),
    /// Remove track at the specified queue index
    RemoveIndex(usize),
    /// Clear the entire queue and stop playback
    ClearQueue,
    /// Set a specific loop mode
    SetLoopMode(LoopMode),
    /// Cycle through Loop modes (Off -> All -> Track -> Off)
    CycleLoopMode,
    /// Toggle shuffle mode on/off
    ToggleShuffle,
    /// Request engine shutdown
    Quit,
}
