use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use crate::engine::track::Track;

/// Discovers common music directories on Android / Termux
pub fn detect_default_music_dirs() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    // 1. Android standard SDCard music
    let sdcard_music = PathBuf::from("/sdcard/Music");
    if sdcard_music.is_dir() {
        candidates.push(sdcard_music);
    }

    // 2. Termux ~/storage/music symlink
    if let Ok(home) = std::env::var("HOME") {
        let termux_music = PathBuf::from(&home).join("storage").join("music");
        if termux_music.is_dir() && !candidates.contains(&termux_music) {
            candidates.push(termux_music);
        }
    }

    // 3. /storage/emulated/0/Music
    let emulated_music = PathBuf::from("/storage/emulated/0/Music");
    if emulated_music.is_dir() && !candidates.contains(&emulated_music) {
        candidates.push(emulated_music);
    }

    // 4. Current working directory fallback
    if let Ok(cwd) = std::env::current_dir() {
        if !candidates.contains(&cwd) {
            candidates.push(cwd);
        }
    }

    candidates
}

/// Recursively scans a directory for supported audio tracks
pub fn scan_audio_files<P: AsRef<Path>>(dir: P) -> Vec<Track> {
    let mut tracks = Vec::new();

    for entry in WalkDir::new(dir)
        .follow_links(true)
        .max_depth(5)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() && Track::is_audio_file(path) {
            tracks.push(Track::from_path(path));
        }
    }

    // Sort tracks alphabetically by display name
    tracks.sort_by(|a, b| a.display_name().to_lowercase().cmp(&b.display_name().to_lowercase()));
    tracks
}

/// Formats seconds into "MM:SS" or "HH:MM:SS"
pub fn format_time(seconds: f64) -> String {
    if seconds.is_nan() || seconds < 0.0 {
        return "00:00".to_string();
    }
    let total_secs = seconds.round() as u64;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let secs = total_secs % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, secs)
    } else {
        format!("{:02}:{:02}", minutes, secs)
    }
}
