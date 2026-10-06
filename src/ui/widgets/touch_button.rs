use crate::ui::hitmap::{TouchHitMap, UiAction};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

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

        let mut style = self.style;
        if self.highlight {
            style = style.add_modifier(ratatui::style::Modifier::REVERSED);
        }

        let formatted = format!("[ {} ]", self.label);
        let form_w = crate::util::display_width(&formatted);
        let max_w = area.width as usize;

        if form_w <= max_w {
            let offset_x = area.x + ((area.width.saturating_sub(form_w as u16)) / 2);
            buf.set_string(offset_x, area.y, &formatted, style);
        } else {
            // Try compact format without inner padding "[label]"
            let compact = format!("[{}]", self.label);
            let comp_w = crate::util::display_width(&compact);
            if comp_w <= max_w {
                let offset_x = area.x + ((area.width.saturating_sub(comp_w as u16)) / 2);
                buf.set_string(offset_x, area.y, &compact, style);
            } else {
                // If still too small, safely truncate to width without char splitting or overflow
                let truncated = crate::util::truncate_to_width(&compact, max_w);
                buf.set_string(area.x, area.y, truncated, style);
            }
        }
    }
}
