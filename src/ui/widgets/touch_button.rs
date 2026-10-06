use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use crate::ui::hitmap::{TouchHitMap, UiAction};

/// A touch-friendly button widget with explicit hit-target registration
pub struct TouchButton<'a> {
    label: &'a str,
    action: UiAction,
    style: Style,
    highlight: bool,
}

impl<'a> TouchButton<'a> {
    pub fn new(label: &'a str, action: UiAction) -> Self {
        Self {
            label,
            action,
            style: Style::default(),
            highlight: false,
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn highlight(mut self, highlight: bool) -> Self {
        self.highlight = highlight;
        self
    }

    /// Renders button and registers its click zone in the hit-map
    pub fn render_and_register(self, area: Rect, buf: &mut Buffer, hitmap: &mut TouchHitMap) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        hitmap.register_button(area, self.action);

        let formatted = format!("[ {} ]", self.label);
        let char_count = formatted.chars().count();
        let max_w = area.width as usize;

        if char_count <= max_w {
            let offset_x = area.x + ((area.width.saturating_sub(char_count as u16)) / 2);
            buf.set_string(offset_x, area.y, &formatted, self.style);
        } else {
            // Try compact format without inner padding "[label]"
            let compact = format!("[{}]", self.label);
            let compact_count = compact.chars().count();
            if compact_count <= max_w {
                let offset_x = area.x + ((area.width.saturating_sub(compact_count as u16)) / 2);
                buf.set_string(offset_x, area.y, &compact, self.style);
            } else {
                // If still too small, safely take full characters
                let chars: String = compact.chars().take(max_w).collect();
                buf.set_string(area.x, area.y, &chars, self.style);
            }
        }
    }
}
