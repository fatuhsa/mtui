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
        let max_len = (area.width as usize).min(formatted.len());
        let slice = &formatted[..max_len];

        buf.set_string(
            area.x,
            area.y,
            slice,
            self.style,
        );
    }
}
