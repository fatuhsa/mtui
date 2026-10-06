use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Auto-scrolling ticker for strings exceeding available display width
pub struct Marquee;

impl Marquee {
    pub fn render(text: &str, area: Rect, step: usize, buf: &mut Buffer, style: Style) {
        Self::render_aligned(text, area, step, buf, style, false);
    }

    pub fn render_centered(text: &str, area: Rect, step: usize, buf: &mut Buffer, style: Style) {
        Self::render_aligned(text, area, step, buf, style, true);
    }

    fn render_aligned(
        text: &str,
        area: Rect,
        step: usize,
        buf: &mut Buffer,
        style: Style,
        centered: bool,
    ) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let max_w = area.width as usize;
        let total_w = UnicodeWidthStr::width(text);

        if total_w <= max_w {
            let start_x = if centered {
                area.x + ((area.width.saturating_sub(total_w as u16)) / 2)
            } else {
                area.x
            };
            buf.set_string(start_x, area.y, text, style);
            return;
        }

        // Prepare rolling string: "Text   •   Text   •   "
        let sep = "   •   ";
        let loop_text = format!("{}{}{}", text, sep, text);
        let chars: Vec<char> = loop_text.chars().collect();
        let loop_len = text.chars().count() + sep.chars().count();

        let offset = if loop_len > 0 { step % loop_len } else { 0 };

        let mut display_chars = String::new();
        let mut cur_w = 0;
        for &ch in chars.iter().skip(offset) {
            let ch_w = UnicodeWidthChar::width(ch).unwrap_or(0);
            if cur_w + ch_w > max_w {
                break;
            }
            display_chars.push(ch);
            cur_w += ch_w;
        }

        buf.set_string(area.x, area.y, display_chars, style);
    }
}
