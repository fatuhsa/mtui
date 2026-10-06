use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

/// Auto-scrolling ticker for strings exceeding available display width
pub struct Marquee;

impl Marquee {
    pub fn render(
        text: &str,
        area: Rect,
        step: usize,
        buf: &mut Buffer,
        style: Style,
    ) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let max_w = area.width as usize;
        let char_count = text.chars().count();

        if char_count <= max_w {
            buf.set_string(area.x, area.y, text, style);
            return;
        }

        // Prepare rolling string: "Text   •   Text   •   "
        let sep = "   •   ";
        let loop_text = format!("{}{}{}", text, sep, text);
        let chars: Vec<char> = loop_text.chars().collect();
        let loop_len = char_count + sep.chars().count();

        let offset = if loop_len > 0 { step % loop_len } else { 0 };

        let display_chars: String = chars
            .iter()
            .skip(offset)
            .take(max_w)
            .collect();

        buf.set_string(area.x, area.y, display_chars, style);
    }
}
