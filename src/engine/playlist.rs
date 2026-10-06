use std::time::{SystemTime, UNIX_EPOCH};
use crate::engine::events::LoopMode;
use crate::engine::track::Track;

/// Fast, deterministic PRNG (XorShift64) seeded from system time, eliminating external rand crate dependencies
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0xCAFE_BABE_DEAD_BEEF);
        let state = if nanos == 0 { 0x1234_5678_9ABC_DEF0 } else { nanos };
        Self { state }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn next_usize(&mut self, upper: usize) -> usize {
        if upper == 0 {
            0
        } else {
            (self.next_u64() % (upper as u64)) as usize
        }
    }
}

/// Playlist management system handling track order, shuffling, repeating, and queue mutations
#[derive(Debug, Clone)]
pub struct Playlist {
    tracks: Vec<Track>,
    current_index: Option<usize>,
    loop_mode: LoopMode,
    shuffle_enabled: bool,
    shuffled_order: Vec<usize>,
    shuffle_pos: usize,
}

impl Playlist {
    pub fn new() -> Self {
        Self {
            tracks: Vec::new(),
            current_index: None,
            loop_mode: LoopMode::Off,
            shuffle_enabled: false,
            shuffled_order: Vec::new(),
            shuffle_pos: 0,
        }
    }

    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    pub fn current_index(&self) -> Option<usize> {
        self.current_index
    }

    pub fn current_track(&self) -> Option<&Track> {
        self.current_index.and_then(|idx| self.tracks.get(idx))
    }

    pub fn loop_mode(&self) -> LoopMode {
        self.loop_mode
    }

    pub fn set_loop_mode(&mut self, mode: LoopMode) {
        self.loop_mode = mode;
    }

    pub fn cycle_loop_mode(&mut self) -> LoopMode {
        self.loop_mode = self.loop_mode.next();
        self.loop_mode
    }

    pub fn is_shuffle(&self) -> bool {
        self.shuffle_enabled
    }

    pub fn set_shuffle(&mut self, enabled: bool) {
        self.shuffle_enabled = enabled;
        if enabled {
            self.rebuild_shuffle();
        }
    }

    pub fn toggle_shuffle(&mut self) -> bool {
        self.set_shuffle(!self.shuffle_enabled);
        self.shuffle_enabled
    }

    pub fn add_track(&mut self, track: Track) {
        self.tracks.push(track);
        if self.shuffle_enabled {
            self.rebuild_shuffle();
        }
    }

    pub fn add_tracks(&mut self, new_tracks: Vec<Track>) {
        if new_tracks.is_empty() {
            return;
        }
        self.tracks.extend(new_tracks);
        if self.shuffle_enabled {
            self.rebuild_shuffle();
        }
    }

    pub fn insert_and_select(&mut self, track: Track) -> usize {
        let insert_idx = match self.current_index {
            Some(idx) => (idx + 1).min(self.tracks.len()),
            None => self.tracks.len(),
        };
        self.tracks.insert(insert_idx, track);
        self.current_index = Some(insert_idx);
        if self.shuffle_enabled {
            self.rebuild_shuffle();
        }
        insert_idx
    }

    pub fn select_index(&mut self, idx: usize) -> Option<&Track> {
        if idx < self.tracks.len() {
            self.current_index = Some(idx);
            if self.shuffle_enabled {
                if let Some(pos) = self.shuffled_order.iter().position(|&i| i == idx) {
                    self.shuffle_pos = pos;
                }
            }
            self.tracks.get(idx)
        } else {
            None
        }
    }

    pub fn remove_index(&mut self, idx: usize) -> Option<Track> {
        if idx >= self.tracks.len() {
            return None;
        }

        let removed = self.tracks.remove(idx);

        // Adjust current_index
        if let Some(cur) = self.current_index {
            if cur == idx {
                if self.tracks.is_empty() {
                    self.current_index = None;
                } else if cur >= self.tracks.len() {
                    self.current_index = Some(self.tracks.len() - 1);
                }
            } else if cur > idx {
                self.current_index = Some(cur - 1);
            }
        }

        if self.shuffle_enabled {
            self.rebuild_shuffle();
        }

        Some(removed)
    }

    pub fn clear(&mut self) {
        self.tracks.clear();
        self.current_index = None;
        self.shuffled_order.clear();
        self.shuffle_pos = 0;
    }

    /// Calculates the next track index taking into account loop modes and shuffle
    pub fn next(&mut self) -> Option<usize> {
        if self.tracks.is_empty() {
            return None;
        }

        if self.loop_mode == LoopMode::Track {
            return self.current_index;
        }

        if self.shuffle_enabled {
            if self.shuffled_order.is_empty() {
                self.rebuild_shuffle();
            }
            if self.shuffle_pos + 1 < self.shuffled_order.len() {
                self.shuffle_pos += 1;
                let next_idx = self.shuffled_order[self.shuffle_pos];
                self.current_index = Some(next_idx);
                return Some(next_idx);
            } else if self.loop_mode == LoopMode::All {
                self.shuffle_pos = 0;
                let next_idx = self.shuffled_order[0];
                self.current_index = Some(next_idx);
                return Some(next_idx);
            } else {
                return None;
            }
        }

        match self.current_index {
            Some(idx) => {
                if idx + 1 < self.tracks.len() {
                    let next_idx = idx + 1;
                    self.current_index = Some(next_idx);
                    Some(next_idx)
                } else if self.loop_mode == LoopMode::All {
                    self.current_index = Some(0);
                    Some(0)
                } else {
                    None
                }
            }
            None => {
                self.current_index = Some(0);
                Some(0)
            }
        }
    }

    /// Calculates the previous track index
    pub fn prev(&mut self) -> Option<usize> {
        if self.tracks.is_empty() {
            return None;
        }

        if self.loop_mode == LoopMode::Track {
            return self.current_index;
        }

        if self.shuffle_enabled {
            if self.shuffle_pos > 0 {
                self.shuffle_pos -= 1;
                let prev_idx = self.shuffled_order[self.shuffle_pos];
                self.current_index = Some(prev_idx);
                return Some(prev_idx);
            } else if self.loop_mode == LoopMode::All && !self.shuffled_order.is_empty() {
                self.shuffle_pos = self.shuffled_order.len() - 1;
                let prev_idx = self.shuffled_order[self.shuffle_pos];
                self.current_index = Some(prev_idx);
                return Some(prev_idx);
            } else {
                return self.current_index;
            }
        }

        match self.current_index {
            Some(idx) => {
                if idx > 0 {
                    let prev_idx = idx - 1;
                    self.current_index = Some(prev_idx);
                    Some(prev_idx)
                } else if self.loop_mode == LoopMode::All {
                    let last_idx = self.tracks.len() - 1;
                    self.current_index = Some(last_idx);
                    Some(last_idx)
                } else {
                    Some(0)
                }
            }
            None => {
                self.current_index = Some(0);
                Some(0)
            }
        }
    }

    fn rebuild_shuffle(&mut self) {
        let count = self.tracks.len();
        if count == 0 {
            self.shuffled_order.clear();
            self.shuffle_pos = 0;
            return;
        }

        let mut order: Vec<usize> = (0..count).collect();
        let mut rng = SimpleRng::new();

        // Fisher-Yates shuffle
        for i in (1..count).rev() {
            let j = rng.next_usize(i + 1);
            order.swap(i, j);
        }

        // Put current playing track at current pos if active
        if let Some(cur) = self.current_index {
            if let Some(pos) = order.iter().position(|&x| x == cur) {
                order.swap(0, pos);
                self.shuffle_pos = 0;
            }
        }

        self.shuffled_order = order;
    }
}
