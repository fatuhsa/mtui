use std::path::{Path, PathBuf};

/// Audio file extensions supported by the engine
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "m4a", "opus", "ogg", "wav", "aac", "alac", "wma", "webm",
];

/// Represents an audio track in the player
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Track {
    pub path: PathBuf,
    pub title: String,
    pub artist: Option<String>,
    pub duration_secs: Option<f64>,
}

impl Track {
    /// Creates a Track from a file path, intelligently deriving title and artist from filename
    pub fn from_path<P: AsRef<Path>>(path: P) -> Self {
        let p = path.as_ref().to_path_buf();
        let file_stem = p
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown Track");

        // Try to parse "Artist - Title" or "Title"
        let (artist, title) = if let Some((left, right)) = file_stem.split_once(" - ") {
            (Some(left.trim().to_string()), right.trim().to_string())
        } else {
            (None, file_stem.trim().to_string())
        };

        Self {
            path: p,
            title,
            artist,
            duration_secs: None,
        }
    }

    /// Checks if a given path has a supported audio extension
    pub fn is_audio_file<P: AsRef<Path>>(path: P) -> bool {
        path.as_ref()
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                let ext_lower = ext.to_lowercase();
                SUPPORTED_EXTENSIONS.contains(&ext_lower.as_str())
            })
            .unwrap_or(false)
    }

    /// Display string: "Artist - Title" or just "Title"
    pub fn display_name(&self) -> String {
        if let Some(artist) = &self.artist {
            format!("{} - {}", artist, self.title)
        } else {
            self.title.clone()
        }
    }
}
