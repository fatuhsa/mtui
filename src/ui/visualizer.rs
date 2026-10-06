use std::hash::{Hash, Hasher};
use std::time::Instant;

/// High-performance rhythmic audio visualizer with beat tracking and gravity physics
#[derive(Debug, Clone)]
pub struct Visualizer {
    /// Current bar amplitude levels (0.0 to 1.0)
    bar_levels: Vec<f64>,
    /// Last playback position reported by the engine
    last_pos: f64,
    /// Instant when last_pos was updated
    last_instant: Instant,
    /// Whether audio is currently playing
    is_playing: bool,
    /// Track-specific hash for unique song groove and tempo
    track_hash: u64,
}

impl Visualizer {
    pub fn new() -> Self {
        Self {
            bar_levels: Vec::new(),
            last_pos: 0.0,
            last_instant: Instant::now(),
            is_playing: false,
            track_hash: 1337,
        }
    }

    /// Updates playback state and synchronizes time
    pub fn update_state(&mut self, pos: f64, is_playing: bool, track_path: Option<&str>) {
        if let Some(path) = track_path {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            path.hash(&mut hasher);
            self.track_hash = hasher.finish();
        }

        // Detect seek or normal playback tick
        if (pos - self.last_pos).abs() > 0.05 || is_playing != self.is_playing {
            self.last_pos = pos;
            self.last_instant = Instant::now();
        }

        self.is_playing = is_playing;
    }

    /// High-precision monotonically interpolated time in seconds
    pub fn current_time(&self) -> f64 {
        if self.is_playing {
            let elapsed = self.last_instant.elapsed().as_secs_f64();
            self.last_pos + elapsed.min(0.5)
        } else {
            self.last_pos
        }
    }

    /// Generates beat-synchronized spectrum bar characters for the current frame
    pub fn render_bars(&mut self, num_bars: usize) -> String {
        if self.bar_levels.len() != num_bars {
            self.bar_levels.resize(num_bars, 0.0);
        }

        if !self.is_playing {
            // Decay bars smoothly when paused
            for b in self.bar_levels.iter_mut() {
                *b = (*b * 0.82).max(0.0);
            }
        }

        let time = self.current_time();

        // Musical BPM: derived from track signature (range 118.0 .. 138.0 BPM)
        let bpm = 118.0 + ((self.track_hash % 21) as f64);
        let beat_freq = bpm / 60.0;
        let beat_time = time * beat_freq;
        let beat_phase = beat_time.fract();

        // 1. Kick drum pulse on beats 1, 2, 3, 4 (exponential attack & rapid decay)
        let kick = (1.0 - beat_phase).powf(3.2);

        // 2. Sub-bass resonance (808 pulse)
        let sub = (beat_time * std::f64::consts::PI).sin().abs().powf(2.0);

        // 3. Snare backbeat on beats 2 and 4
        let snare_phase = ((beat_time + 1.0) % 2.0).fract();
        let snare = (1.0 - snare_phase).powf(4.0) * 0.9;

        // 4. Hi-hat rhythm at 16th notes (4x tempo)
        let hat_phase = (beat_time * 4.0).fract();
        let hat = (1.0 - hat_phase).powf(2.5) * 0.75;

        // 5. Off-beat 8th note groove
        let off_beat = ((beat_time * 2.0).fract() - 0.5).abs() * 2.0;

        let glyphs = [' ', ' ', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
        let mut output = String::with_capacity(num_bars * 4);

        for (i, current_level) in self.bar_levels.iter_mut().enumerate() {
            let norm_idx = (i as f64) / (num_bars as f64);

            let target = if self.is_playing {
                if norm_idx < 0.25 {
                    // Low frequencies / Sub-bass
                    let bass_factor = (1.0 - norm_idx * 3.2).max(0.0);
                    let pulse = kick * 0.75 + sub * 0.35;
                    let wobble = (time * 6.0 + (i as f64) * 0.8).sin() * 0.15;
                    (pulse * bass_factor + wobble + 0.12).clamp(0.0, 1.0)
                } else if norm_idx < 0.50 {
                    // Low-mids (snare, bassline, rhythm guitars)
                    let mid_factor = 1.0 - (norm_idx - 0.35).abs() * 3.0;
                    let wave1 = (time * 8.5 + (i as f64) * 0.6).sin() * 0.25;
                    let wave2 = (beat_time * 2.0 + (i as f64) * 0.4).cos().abs() * 0.35;
                    let pulse = snare * 0.55 + off_beat * 0.3;
                    (pulse * mid_factor + wave1 + wave2 + 0.15).clamp(0.0, 1.0)
                } else if norm_idx < 0.75 {
                    // High-mids (lead melody, vocals)
                    let sweep = (time * 5.2 + (i as f64) * 0.9).sin() * 0.35 + 0.4;
                    let accent = if (beat_time % 1.0) < 0.25 { 0.25 } else { 0.0 };
                    (sweep + accent + off_beat * 0.2).clamp(0.0, 1.0)
                } else {
                    // Treble / Highs (hi-hats, cymbals, air)
                    let treble_factor = (norm_idx - 0.70) * 3.3;
                    let jitter = ((time * 19.3 + (i as f64) * 7.1).sin() * 0.5 + 0.5) * 0.3;
                    let pulse = hat * 0.65 + jitter;
                    (pulse * treble_factor + 0.08).clamp(0.0, 1.0)
                }
            } else {
                0.0
            };

            // Physics engine:
            // Fast attack: jump up instantly when target is higher
            // Exponential gravity decay: smoothly drop when target is lower
            let decay = 0.84;
            if target > *current_level {
                *current_level = target;
            } else {
                *current_level = (*current_level * decay).max(target);
            }

            let level_idx = ((*current_level * 8.0).round() as usize).min(8);
            output.push(glyphs[level_idx]);
            output.push(' ');
        }

        output
    }
}
