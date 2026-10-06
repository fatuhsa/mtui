use ratatui::layout::Rect;

/// Screen size profile for responsive mobile rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenProfile {
    /// Narrow phone in portrait mode (< 55 cols)
    Compact,
    /// Standard landscape / phone horizontal (55 - 90 cols)
    Normal,
    /// Wide display or tablet (>= 90 cols)
    Wide,
}

impl ScreenProfile {
    pub fn from_rect(area: Rect) -> Self {
        if area.width < 55 {
            ScreenProfile::Compact
        } else if area.width < 90 {
            ScreenProfile::Normal
        } else {
            ScreenProfile::Wide
        }
    }

    pub fn is_compact(&self) -> bool {
        matches!(self, ScreenProfile::Compact)
    }
}
