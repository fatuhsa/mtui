use crate::engine::commands::EngineCommand;
use ratatui::layout::Rect;
use std::path::PathBuf;

/// Identifies all possible actions triggered by UI interactions (Touch or Keyboard)
#[derive(Debug, Clone, PartialEq)]
pub enum UiAction {
    /// Switch to a top-level tab
    SwitchTab(usize),
    /// Dispatch an audio command directly to the engine
    Engine(EngineCommand),
    /// File browser: Enter directory
    BrowseEnter(PathBuf),
    /// File browser: Go up to parent directory
    BrowseParent,
    /// File browser: Add current directory tracks to queue
    BrowseAddAll(PathBuf),
    /// File browser: Tap on an audio track
    BrowsePlayTrack(PathBuf),
    /// File browser: Append track to queue
    BrowseQueueTrack(PathBuf),
    /// List navigation: select item index
    SelectListItem(usize),
    /// Scroll list view up
    ScrollUp,
    /// Scroll list view down
    ScrollDown,
    /// Minimize player to background (suspend to shell while audio keeps playing)
    Minimize,
    /// Cycle cover art protocol (Sixel -> iTerm2 -> Blocks -> Off)
    CycleCoverProtocol,
    /// Request exit
    Quit,
}

/// Target hit-box registered during UI rendering
#[derive(Debug, Clone)]
enum HitTarget {
    Button {
        rect: Rect,
        action: UiAction,
    },
    ProgressBar {
        rect: Rect,
    },
    VolumeBar {
        rect: Rect,
    },
    ListRow {
        rect: Rect,
        _item_index: usize,
        action: UiAction,
    },
}

/// Dynamic touch hit-map rebuilt every render frame for precise touch handling
#[derive(Debug, Default)]
pub struct TouchHitMap {
    targets: Vec<HitTarget>,
}

impl TouchHitMap {
    pub fn new() -> Self {
        Self {
            targets: Vec::with_capacity(64),
        }
    }

    pub fn clear(&mut self) {
        self.targets.clear();
    }

    /// Registers a tappable button
    pub fn register_button(&mut self, rect: Rect, action: UiAction) {
        self.targets.push(HitTarget::Button { rect, action });
    }

    /// Registers an interactive progress/seek bar
    pub fn register_progress_bar(&mut self, rect: Rect) {
        self.targets.push(HitTarget::ProgressBar { rect });
    }

    /// Registers an interactive volume slider
    pub fn register_volume_bar(&mut self, rect: Rect) {
        self.targets.push(HitTarget::VolumeBar { rect });
    }

    /// Registers a clickable row within a list
    pub fn register_list_row(&mut self, rect: Rect, item_index: usize, action: UiAction) {
        self.targets.push(HitTarget::ListRow {
            rect,
            _item_index: item_index,
            action,
        });
    }

    /// Resolves a touch tap coordinate (x, y) into a concrete UiAction
    pub fn resolve_tap(&self, col: u16, row: u16) -> Option<UiAction> {
        for target in self.targets.iter().rev() {
            match target {
                HitTarget::Button { rect, action } => {
                    if contains(*rect, col, row) {
                        return Some(action.clone());
                    }
                }
                HitTarget::ProgressBar { rect } => {
                    if contains(*rect, col, row) && rect.width > 0 {
                        let rel_x = (col.saturating_sub(rect.x)) as f64;
                        let pct = (rel_x / (rect.width as f64)).clamp(0.0, 1.0);
                        return Some(UiAction::Engine(EngineCommand::SeekPercent(pct)));
                    }
                }
                HitTarget::VolumeBar { rect } => {
                    if contains(*rect, col, row) && rect.width > 0 {
                        let rel_x = (col.saturating_sub(rect.x)) as f64;
                        let pct = (rel_x / (rect.width as f64)).clamp(0.0, 1.0);
                        let vol = (pct * 100.0).round() as u32;
                        return Some(UiAction::Engine(EngineCommand::SetVolume(vol)));
                    }
                }
                HitTarget::ListRow { rect, action, .. } => {
                    if contains(*rect, col, row) {
                        return Some(action.clone());
                    }
                }
            }
        }
        None
    }
}

fn contains(rect: Rect, col: u16, row: u16) -> bool {
    col >= rect.x
        && col < rect.x.saturating_add(rect.width)
        && row >= rect.y
        && row < rect.y.saturating_add(rect.height)
}
