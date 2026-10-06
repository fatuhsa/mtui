use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Hash a path into a hex string for caching
fn hash_path(path: &Path) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Discovers or extracts cover art for an audio track
pub fn get_or_extract_cover_art(audio_path: &Path) -> Option<PathBuf> {
    let parent = audio_path.parent()?;

    // 1. Check for standard image files in same directory
    let common_names = ["cover.jpg", "cover.png", "folder.jpg", "front.jpg", "album.jpg"];
    for name in &common_names {
        let candidate = parent.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    // 2. Prepare cache directory in temp
    let temp_dir = std::env::temp_dir().join("mtui_covers");
    let _ = std::fs::create_dir_all(&temp_dir);

    let cache_file = temp_dir.join(format!("{}.jpg", hash_path(audio_path)));
    if cache_file.is_file() {
        if let Ok(meta) = cache_file.metadata() {
            if meta.len() > 100 {
                return Some(cache_file);
            }
        }
    }

    // 3. Extract embedded cover art via ffmpeg
    let status = Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(audio_path)
        .arg("-an")
        .arg("-vcodec")
        .arg("copy")
        .arg(&cache_file)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if let Ok(st) = status {
        if st.success() && cache_file.is_file() {
            if let Ok(meta) = cache_file.metadata() {
                if meta.len() > 100 {
                    return Some(cache_file);
                }
            }
        }
    }

    None
}
