use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::engine::commands::EngineCommand;
use crate::engine::events::{EngineStateSnapshot, LoopMode, PlaybackStatus};
use crate::ui::hitmap::{TouchHitMap, UiAction};
use crate::ui::responsive::ScreenProfile;
use crate::ui::theme::Theme;
use crate::ui::widgets::TouchButton;

pub struct QueueView;

impl QueueView {
    pub fn render(
        area: Rect,
        buf: &mut Buffer,
        state: &EngineStateSnapshot,
        scroll_offset: usize,
        selected_index: Option<usize>,
        theme: &Theme,
        _profile: ScreenProfile,
        hitmap: &mut TouchHitMap,
    ) {
        if area.width < 10 || area.height < 5 {
            return;
        }

        let title = format!("  Queue ({}) ", state.queue.len());
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(title)
            .title_alignment(Alignment::Center);
        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        let mut cur_y = inner.y;
        let max_y = inner.y + inner.height;

        // 1. Controls Row: [  Clear ] [  Shuffle ] [  Loop ]
        let clear_btn = TouchButton::new(" Clear", UiAction::Engine(EngineCommand::ClearQueue))
            .style(Style::default().fg(theme.muted));

        let loop_label = match state.loop_mode {
            LoopMode::Off => " Off",
            LoopMode::Track => " 1",
            LoopMode::All => " All",
        };
        let loop_btn = TouchButton::new(loop_label, UiAction::Engine(EngineCommand::CycleLoopMode))
            .style(if state.loop_mode != LoopMode::Off {
                Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.muted)
            });

        let shuf_label = if state.shuffle_enabled { " ON" } else { " OFF" };
        let shuf_btn = TouchButton::new(shuf_label, UiAction::Engine(EngineCommand::ToggleShuffle))
            .style(if state.shuffle_enabled {
                Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.muted)
            });

        let mut btn_x = inner.x;
        clear_btn.render_and_register(Rect::new(btn_x, cur_y, 11, 1), buf, hitmap);
        btn_x += 12;

        if inner.width > 25 {
            loop_btn.render_and_register(Rect::new(btn_x, cur_y, 13, 1), buf, hitmap);
            btn_x += 14;
        }

        if inner.width > 40 {
            shuf_btn.render_and_register(Rect::new(btn_x, cur_y, 12, 1), buf, hitmap);
        }

        // Scroll touch buttons on the right edge
        if inner.width > 35 {
            let scroll_up = TouchButton::new("▲", UiAction::ScrollUp).style(Style::default().fg(theme.muted));
            let scroll_dn = TouchButton::new("▼", UiAction::ScrollDown).style(Style::default().fg(theme.muted));
            scroll_up.render_and_register(Rect::new(inner.x + inner.width - 10, cur_y, 5, 1), buf, hitmap);
            scroll_dn.render_and_register(Rect::new(inner.x + inner.width - 5, cur_y, 5, 1), buf, hitmap);
        }

        cur_y += 2;

        // 2. Queue Track List
        let available_rows = (max_y.saturating_sub(cur_y)) as usize;
        if available_rows == 0 {
            return;
        }

        if state.queue.is_empty() {
            let empty_p = Paragraph::new("Queue is empty.\nTap 'Files' tab and add songs!")
                .style(Style::default().fg(theme.muted))
                .alignment(Alignment::Center);
            empty_p.render(Rect::new(inner.x, cur_y, inner.width, 2), buf);
            return;
        }

        let visible_items = state.queue.iter().enumerate().skip(scroll_offset).take(available_rows);

        for (rel_idx, (idx, track)) in visible_items.enumerate() {
            let item_y = cur_y + (rel_idx as u16);
            if item_y >= max_y {
                break;
            }

            let is_current = state.queue_index == Some(idx);
            let is_selected = selected_index == Some(idx);

            let row_rect = Rect::new(inner.x, item_y, inner.width, 1);

            // Tap row to play track immediately!
            hitmap.register_list_row(row_rect, idx, UiAction::Engine(EngineCommand::PlayIndex(idx)));

            let (marker, marker_style) = if is_current {
                match state.status {
                    PlaybackStatus::Playing => (" ", Style::default().fg(theme.playing).add_modifier(Modifier::BOLD)),
                    PlaybackStatus::Paused => (" ", Style::default().fg(theme.paused).add_modifier(Modifier::BOLD)),
                    PlaybackStatus::Stopped => (" ", Style::default().fg(theme.stopped)),
                }
            } else {
                ("  ", Style::default().fg(theme.muted))
            };

            let row_style = if is_selected {
                Style::default().fg(Color::Black).bg(theme.primary).add_modifier(Modifier::BOLD)
            } else if is_current {
                marker_style
            } else {
                Style::default().fg(theme.text)
            };

            let max_name_len = inner.width.saturating_sub(12) as usize;
            let display_name = track.display_name();
            let truncated_name = crate::util::truncate_with_ellipsis(&display_name, max_name_len, "...");

            let text = format!("{}{}. {}", marker, idx + 1, truncated_name);
            buf.set_string(inner.x, item_y, &text, row_style);

            // Touch remove button [  ] on right edge
            if inner.width > 25 {
                let remove_btn = TouchButton::new("", UiAction::Engine(EngineCommand::RemoveIndex(idx)))
                    .style(Style::default().fg(Color::Red));
                let remove_rect = Rect::new(inner.x + inner.width - 5, item_y, 5, 1);
                remove_btn.render_and_register(remove_rect, buf, hitmap);
            }
        }
    }
}
