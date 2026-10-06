use crate::ui::hitmap::TouchHitMap;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

pub enum BarKind {
    Progress,
    Volume,
}

/// A touch-interactive progress/seek or volume slider bar
pub struct TouchBar<'a> {
    kind: BarKind,
    current_fraction: f64,
    left_label: &'a str,
    right_label: &'a str,
    filled_style: Style,
    empty_style: Style,
    label_style: Style,
}

impl<'a> TouchBar<'a> {
    pub fn progress(fraction: f64, left_label: &'a str, right_label: &'a str) -> Self {
        Self {
            kind: BarKind::Progress,
            current_fraction: fraction.clamp(0.0, 1.0),
            left_label,
            right_label,
            filled_style: Style::default(),
            empty_style: Style::default(),
            label_style: Style::default(),
        }
    }

    pub fn volume(fraction: f64, left_label: &'a str, right_label: &'a str) -> Self {
        Self {
            kind: BarKind::Volume,
            current_fraction: fraction.clamp(0.0, 1.0),
            left_label,
            right_label,
            filled_style: Style::default(),
            empty_style: Style::default(),
            label_style: Style::default(),
        }
    }

    pub fn filled_style(mut self, style: Style) -> Self {
        self.filled_style = style;
        self
    }

    pub fn empty_style(mut self, style: Style) -> Self {
        self.empty_style = style;
        self
    }

    pub fn label_style(mut self, style: Style) -> Self {
        self.label_style = style;
        self
    }

    pub fn render_and_register(self, area: Rect, buf: &mut Buffer, hitmap: &mut TouchHitMap) {
        if area.width < 10 || area.height == 0 {
            return;
        }

        let left_w = crate::util::display_width(self.left_label) as u16;
        let right_w = crate::util::display_width(self.right_label) as u16;

        // Render left label
        if left_w > 0 && area.width > left_w {
            buf.set_string(area.x, area.y, self.left_label, self.label_style);
        }

        // Render right label
        if right_w > 0 && area.width > left_w + right_w + 3 {
            let rx = area.x + area.width - right_w;
            buf.set_string(rx, area.y, self.right_label, self.label_style);
        }

        // Inner slider bar area
        let bar_start_x = area.x + left_w + (if left_w > 0 { 1 } else { 0 });
        let bar_end_x = if right_w > 0 {
            area.x + area.width - right_w - 1
        } else {
            area.x + area.width
        };

        if bar_end_x <= bar_start_x + 2 {
            return;
        }

        let bar_width = bar_end_x - bar_start_x;
        let bar_rect = Rect::new(bar_start_x, area.y, bar_width, 1);

        // Register in touch hit-map so any tap inside bar_rect triggers instant seek/vol
        match self.kind {
            BarKind::Progress => hitmap.register_progress_bar(bar_rect),
            BarKind::Volume => hitmap.register_volume_bar(bar_rect),
        }

        // Draw track slider: filled chars, knob, empty chars
        let filled_chars =
            ((self.current_fraction * (bar_width as f64)).round() as u16).min(bar_width);

        for i in 0..bar_width {
            let cur_x = bar_start_x + i;
            if i < filled_chars {
                if i + 1 == filled_chars {
                    // Knob
                    buf.set_string(cur_x, area.y, "●", self.filled_style);
                } else {
                    buf.set_string(cur_x, area.y, "━", self.filled_style);
                }
            } else {
                buf.set_string(cur_x, area.y, "─", self.empty_style);
            }
        }
    }
}
