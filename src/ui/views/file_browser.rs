use std::fs;
use std::path::{Path, PathBuf};
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::engine::track::Track;
use crate::ui::hitmap::{TouchHitMap, UiAction};
use crate::ui::responsive::ScreenProfile;
use crate::ui::theme::Theme;
use crate::ui::widgets::TouchButton;

#[derive(Debug, Clone)]
pub struct BrowserItem {
    pub path: PathBuf,
    pub is_dir: bool,
    pub name: String,
}

pub struct FileBrowserView;

impl FileBrowserView {
    pub fn list_directory(dir: &Path) -> Vec<BrowserItem> {
        let mut items = Vec::new();

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();

                // Skip hidden files/folders (.thumbnails, etc.)
                if name.starts_with('.') {
                    continue;
                }

                if path.is_dir() {
                    items.push(BrowserItem {
                        path,
                        is_dir: true,
                        name,
                    });
                } else if path.is_file() && Track::is_audio_file(&path) {
                    items.push(BrowserItem {
                        path,
                        is_dir: false,
                        name,
                    });
                }
            }
        }

        // Sort: directories first (alphabetical), then files (alphabetical)
        items.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });

        items
    }

    pub fn render(
        area: Rect,
        buf: &mut Buffer,
        current_dir: &Path,
        items: &[BrowserItem],
        scroll_offset: usize,
        selected_index: Option<usize>,
        theme: &Theme,
        _profile: ScreenProfile,
        hitmap: &mut TouchHitMap,
    ) {
        if area.width < 10 || area.height < 5 {
            return;
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title("  File Browser ")
            .title_alignment(Alignment::Center);
        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        let mut cur_y = inner.y;
        let max_y = inner.y + inner.height;

        // 1. Current Path Display & Action Buttons
        let path_str = current_dir.to_string_lossy();
        let display_path = crate::util::truncate_left_with_ellipsis(&path_str, inner.width as usize, "...");

        let path_p = Paragraph::new(display_path).style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD));
        path_p.render(Rect::new(inner.x, cur_y, inner.width, 1), buf);
        cur_y += 1;

        // Button row: [  .. Up ] [  Add All ] [ ▲ ] [ ▼ ]
        let up_btn = TouchButton::new(" .. Up", UiAction::BrowseParent)
            .style(theme.button_style());
        let add_all_btn = TouchButton::new(" Add All", UiAction::BrowseAddAll(current_dir.to_path_buf()))
            .style(Style::default().fg(theme.secondary));

        let up_w = 11u16.min(inner.width);
        up_btn.render_and_register(Rect::new(inner.x, cur_y, up_w, 1), buf, hitmap);

        if inner.width > up_w + 14 {
            add_all_btn.render_and_register(Rect::new(inner.x + up_w + 1, cur_y, 13, 1), buf, hitmap);
        }

        // Scroll touch buttons on the right edge
        if inner.width > 35 {
            let scroll_up = TouchButton::new("▲", UiAction::ScrollUp).style(Style::default().fg(theme.muted));
            let scroll_dn = TouchButton::new("▼", UiAction::ScrollDown).style(Style::default().fg(theme.muted));
            scroll_up.render_and_register(Rect::new(inner.x + inner.width - 10, cur_y, 5, 1), buf, hitmap);
            scroll_dn.render_and_register(Rect::new(inner.x + inner.width - 5, cur_y, 5, 1), buf, hitmap);
        }

        cur_y += 2;

        // 2. File and Directory List
        let available_rows = (max_y.saturating_sub(cur_y)) as usize;
        if available_rows == 0 {
            return;
        }

        if items.is_empty() {
            let empty_p = Paragraph::new("No audio files or folders found here.")
                .style(Style::default().fg(theme.muted))
                .alignment(Alignment::Center);
            empty_p.render(Rect::new(inner.x, cur_y, inner.width, 1), buf);
            return;
        }

        let visible_items = items.iter().skip(scroll_offset).take(available_rows);

        for (rel_idx, item) in visible_items.enumerate() {
            let item_y = cur_y + (rel_idx as u16);
            if item_y >= max_y {
                break;
            }

            let absolute_idx = scroll_offset + rel_idx;
            let is_selected = selected_index == Some(absolute_idx);

            let row_rect = Rect::new(inner.x, item_y, inner.width, 1);

            let action = if item.is_dir {
                UiAction::BrowseEnter(item.path.clone())
            } else {
                UiAction::BrowsePlayTrack(item.path.clone())
            };

            // Register row in hit-map for touch tap
            hitmap.register_list_row(row_rect, absolute_idx, action);

            // Row styling
            let (prefix, icon_style) = if item.is_dir {
                (" ", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
            } else {
                (" ", Style::default().fg(theme.text))
            };

            let row_style = if is_selected {
                Style::default().fg(Color::Black).bg(theme.primary).add_modifier(Modifier::BOLD)
            } else {
                icon_style
            };

            let max_name_len = inner.width.saturating_sub(10) as usize;
            let truncated_name = crate::util::truncate_with_ellipsis(&item.name, max_name_len, "...");
            let display_name = format!("{}{}", prefix, truncated_name);

            buf.set_string(inner.x, item_y, &display_name, row_style);

            // For audio files: add a dedicated [  ] touch button on the right edge to queue without interrupting
            if !item.is_dir && inner.width > 25 {
                let add_btn = TouchButton::new("", UiAction::BrowseQueueTrack(item.path.clone()))
                    .style(Style::default().fg(theme.secondary));
                let add_rect = Rect::new(inner.x + inner.width - 5, item_y, 5, 1);
                add_btn.render_and_register(add_rect, buf, hitmap);
            }
        }
    }
}
