use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Hash audio path and file metadata into a hex string for caching
pub fn hash_audio_source(audio_path: &Path) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    audio_path.hash(&mut hasher);
    if let Ok(meta) = audio_path.metadata() {
        meta.len().hash(&mut hasher);
        if let Ok(mtime) = meta.modified() {
            mtime.hash(&mut hasher);
        }
    }
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

    let cache_file = temp_dir.join(format!("{}.jpg", hash_audio_source(audio_path)));
    if cache_file.is_file() {
        if let Ok(cache_meta) = cache_file.metadata() {
            if cache_meta.len() > 100 {
                let is_fresh = if let (Ok(audio_meta), Ok(cache_mtime)) = (audio_path.metadata(), cache_meta.modified()) {
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

    // 3. Extract embedded cover art via ffmpeg
    // Explicitly transcode first video frame to JPEG format via -vframes 1 -f image2 -c:v mjpeg
    // so embedded PNG, WebP, or other artwork formats cleanly produce valid JPEG cache files
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
