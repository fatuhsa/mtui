use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Measures the terminal display width of a string in columns
pub fn display_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

/// Truncates a string to fit within `max_width` display columns.
/// Guarantees truncation on a valid UTF-8 char boundary without panic.
pub fn truncate_to_width(s: &str, max_width: usize) -> &str {
    let mut cur_w = 0;
    for (byte_idx, ch) in s.char_indices() {
        let ch_w = UnicodeWidthChar::width(ch).unwrap_or(0);
        if cur_w + ch_w > max_width {
            return &s[..byte_idx];
        }
        cur_w += ch_w;
    }
    s
}

/// Truncates a string to fit within `max_width` display columns, appending `ellipsis`
/// if truncated. Never panics on multibyte characters (CJK, emojis, Cyrillic, etc.).
pub fn truncate_with_ellipsis(s: &str, max_width: usize, ellipsis: &str) -> String {
    let total_w = display_width(s);
    if total_w <= max_width {
        return s.to_string();
    }
    let ell_w = display_width(ellipsis);
    if max_width <= ell_w {
        return truncate_to_width(ellipsis, max_width).to_string();
    }
    let target_w = max_width - ell_w;
    let prefix = truncate_to_width(s, target_w);
    format!("{}{}", prefix, ellipsis)
}

/// Truncates a string from the left to fit within `max_width` display columns,
/// prepending `ellipsis` if truncated (useful for long directory paths).
pub fn truncate_left_with_ellipsis(s: &str, max_width: usize, ellipsis: &str) -> String {
    let total_w = display_width(s);
    if total_w <= max_width {
        return s.to_string();
    }
    let ell_w = display_width(ellipsis);
    if max_width <= ell_w {
        return truncate_to_width(ellipsis, max_width).to_string();
    }
    let target_w = max_width - ell_w;
    let mut cur_w = 0;
    let mut start_byte = s.len();
    for (byte_idx, ch) in s.char_indices().rev() {
        let ch_w = UnicodeWidthChar::width(ch).unwrap_or(0);
        if cur_w + ch_w > target_w {
            break;
        }
        cur_w += ch_w;
        start_byte = byte_idx;
    }
    format!("{}{}", ellipsis, &s[start_byte..])
}
