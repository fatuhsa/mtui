use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::engine::commands::EngineCommand;
use crate::engine::events::{EngineStateSnapshot, LoopMode, PlaybackStatus};
use crate::ui::cover::CoverArtManager;
use crate::ui::hitmap::{TouchHitMap, UiAction};
use crate::ui::responsive::ScreenProfile;
use crate::ui::theme::Theme;
use crate::ui::widgets::{Marquee, TouchBar, TouchButton};
use crate::util::format_time;

pub struct NowPlayingView;

impl NowPlayingView {
    /// Dynamically computes cover art width and height based on available inner dimensions.
    /// Cell aspect ratio is ~1:2 (1 row height ≈ 2 col width).
    pub fn calculate_art_size(inner_w: u16, inner_h: u16) -> (u16, u16) {
        if inner_w < 10 || inner_h < 8 {
            return (0, 0);
        }

        // Leave enough rows for controls (Title, Artist, Seek, Buttons, Vol, etc.)
        let avail_h = inner_h.saturating_sub(9);
        if avail_h < 3 {
            return (0, 0);
        }

        // Cover height scales flexibly: 3 to 16 rows
        let art_h = avail_h.clamp(3, 16);
        // Desired width is 2 * height for 1:1 square image
        let mut art_w = art_h.saturating_mul(2);

        // Limit width so it fits within inner_w with at least 2 cells padding
        let max_w = inner_w.saturating_sub(2);
        if art_w > max_w {
            art_w = max_w;
            let adjusted_h = (art_w / 2).max(3);
            return (art_w, adjusted_h);
        }

        (art_w, art_h)
    }

    pub fn render(
        area: Rect,
        buf: &mut Buffer,
        state: &EngineStateSnapshot,
        cover_mgr: &mut CoverArtManager,
        theme: &Theme,
        profile: ScreenProfile,
        tick: usize,
        hitmap: &mut TouchHitMap,
    ) -> Option<(u16, u16, String)> {
        if area.width < 10 || area.height < 5 {
            return None;
        }

        let mut pending_graphic = None;

        // Draw outer block
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title("  Now Playing ")
            .title_alignment(Alignment::Center);
        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height == 0 || inner.width == 0 {
            return None;
        }

        let mut cur_y = inner.y;
        let max_y = inner.y + inner.height;

        // 1. Centered Cover Art
        let (art_w, art_h) = Self::calculate_art_size(inner.width, inner.height);
        if art_w > 0 && art_h > 0 && cur_y + art_h <= max_y {
            let art_x = inner.x + (inner.width.saturating_sub(art_w)) / 2;
            let art_rect = Rect::new(art_x, cur_y, art_w, art_h);

            pending_graphic = cover_mgr.render_to_buffer(art_rect, buf);
            cur_y += art_h;
            if max_y.saturating_sub(cur_y) >= 10 {
                cur_y += 1;
            }
        }

        // 2. Centered Animated Equalizer
        let remaining_for_eq = max_y.saturating_sub(cur_y);
        if remaining_for_eq >= 8 {
            let eq_art = match state.status {
                PlaybackStatus::Playing => {
                    let frames = [
                        " ▂ ▄ ▆ █ ▇ ▅ ▃  ▂ ▄ ▆ █ ▇ ▅ ▃ ",
                        "▃ ▅ ▇ █ ▆ ▄ ▂  ▃ ▅ ▇ █ ▆ ▄ ▂ ",
                        "▄ ▆ █ ▇ ▅ ▃ ▂  ▄ ▆ █ ▇ ▅ ▃ ▂ ",
                        "▆ █ ▇ ▅ ▃ ▂  ▂ ▄ ▆ █ ▇ ▅ ▃ ▂ ",
                    ];
                    frames[tick % frames.len()]
                }
                PlaybackStatus::Paused => "─ ─ [ PAUSED ] ─ ─",
                PlaybackStatus::Stopped => "─ ─ [ STOPPED ] ─ ─",
            };

            let eq_style = match state.status {
                PlaybackStatus::Playing => Style::default().fg(theme.playing).add_modifier(Modifier::BOLD),
                PlaybackStatus::Paused => Style::default().fg(theme.paused),
                PlaybackStatus::Stopped => Style::default().fg(theme.stopped),
            };

            Paragraph::new(eq_art)
                .alignment(Alignment::Center)
                .style(eq_style)
                .render(Rect::new(inner.x, cur_y, inner.width, 1), buf);

            cur_y += 1;
            if remaining_for_eq >= 11 {
                cur_y += 1;
            }
        }

        // 3. Track Title & Artist (Centered Marquee)
        let title_str = state
            .current_track
            .as_ref()
            .map(|t| t.title.as_str())
            .unwrap_or("No Track Loaded");

        let artist_str = state
            .current_track
            .as_ref()
            .and_then(|t| t.artist.as_deref())
            .unwrap_or("Select a track from Files (Tap Files tab)");

        if cur_y < max_y {
            let title_rect = Rect::new(inner.x, cur_y, inner.width, 1);
            let title_style = Style::default().fg(theme.text).add_modifier(Modifier::BOLD);
            Marquee::render_centered(title_str, title_rect, tick, buf, title_style);
            cur_y += 1;
        }

        if cur_y < max_y {
            let artist_rect = Rect::new(inner.x, cur_y, inner.width, 1);
            let artist_style = Style::default().fg(theme.primary);
            Marquee::render_centered(artist_str, artist_rect, tick, buf, artist_style);
            cur_y += 1;
            if max_y.saturating_sub(cur_y) >= 7 {
                cur_y += 1;
            }
        }

        // 4. Centered Touch Seek Bar
        if cur_y < max_y {
            let fraction = if state.duration_secs > 0.0 {
                state.position_secs / state.duration_secs
            } else {
                0.0
            };

            let pos_str = format_time(state.position_secs);
            let dur_str = format_time(state.duration_secs);

            let bar_w = inner.width.min(64).max(18);
            let bar_x = inner.x + (inner.width.saturating_sub(bar_w)) / 2;
            let bar_rect = Rect::new(bar_x, cur_y, bar_w, 1);

            TouchBar::progress(fraction, &pos_str, &dur_str)
                .filled_style(theme.progress_filled_style())
                .empty_style(theme.progress_empty_style())
                .label_style(Style::default().fg(theme.muted))
                .render_and_register(bar_rect, buf, hitmap);

            cur_y += 1;
            if max_y.saturating_sub(cur_y) >= 5 {
                cur_y += 1;
            }
        }

        // 5. Large Touch Playback Controls (Centered)
        if cur_y < max_y {
            let play_label = match state.status {
                PlaybackStatus::Playing => " Pause",
                PlaybackStatus::Paused | PlaybackStatus::Stopped => " Play",
            };

            let prev_btn = TouchButton::new(" Prev", UiAction::Engine(EngineCommand::Prev))
                .style(theme.button_style());
            let play_btn = TouchButton::new(play_label, UiAction::Engine(EngineCommand::TogglePlay))
                .style(Style::default().fg(Color::Black).bg(theme.primary).add_modifier(Modifier::BOLD));
            let next_btn = TouchButton::new(" Next", UiAction::Engine(EngineCommand::Next))
                .style(theme.button_style());

            let (btn_w, spacing) = if profile.is_compact() {
                (9u16, 1u16)
            } else {
                (12u16, 2u16)
            };

            let total_w = btn_w * 3 + spacing * 2;
            let start_x = inner.x + (inner.width.saturating_sub(total_w)) / 2;

            prev_btn.render_and_register(Rect::new(start_x, cur_y, btn_w, 1), buf, hitmap);
            play_btn.render_and_register(Rect::new(start_x + btn_w + spacing, cur_y, btn_w, 1), buf, hitmap);
            next_btn.render_and_register(Rect::new(start_x + (btn_w + spacing) * 2, cur_y, btn_w, 1), buf, hitmap);

            cur_y += 1;
            if max_y.saturating_sub(cur_y) >= 3 {
                cur_y += 1;
            }
        }

        // 6. Secondary Controls: Loop Mode, Shuffle, and Cover Protocol Switcher (Centered)
        if cur_y < max_y {
            let loop_label = match state.loop_mode {
                LoopMode::Off => " Off",
                LoopMode::Track => " 1",
                LoopMode::All => " All",
            };

            let shuf_label = if state.shuffle_enabled { " ON" } else { " OFF" };
            let art_label = format!(" {}", cover_mgr.protocol.display_name());

            let loop_btn = TouchButton::new(loop_label, UiAction::Engine(EngineCommand::CycleLoopMode))
                .style(if state.loop_mode != LoopMode::Off {
                    Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.muted)
                });

            let shuf_btn = TouchButton::new(shuf_label, UiAction::Engine(EngineCommand::ToggleShuffle))
                .style(if state.shuffle_enabled {
                    Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.muted)
                });

            let art_btn = TouchButton::new(&art_label, UiAction::CycleCoverProtocol)
                .style(Style::default().fg(theme.primary));

            let loop_w = 9u16;
            let shuf_w = 9u16;
            let art_w = (art_label.len() as u16) + 4;

            let (show_shuf, show_art) = if inner.width >= 34 {
                (true, true)
            } else if inner.width >= 22 {
                (true, false)
            } else {
                (false, false)
            };

            let total_sec_w = loop_w
                + if show_shuf { shuf_w + 1 } else { 0 }
                + if show_art { art_w + 1 } else { 0 };

            let mut bx = inner.x + (inner.width.saturating_sub(total_sec_w)) / 2;
            loop_btn.render_and_register(Rect::new(bx, cur_y, loop_w, 1), buf, hitmap);
            bx += loop_w + 1;

            if show_shuf {
                shuf_btn.render_and_register(Rect::new(bx, cur_y, shuf_w, 1), buf, hitmap);
                bx += shuf_w + 1;
            }

            if show_art {
                art_btn.render_and_register(Rect::new(bx, cur_y, art_w, 1), buf, hitmap);
            }

            cur_y += 1;
            if max_y.saturating_sub(cur_y) >= 2 {
                cur_y += 1;
            }
        }

        // 7. Interactive Volume Bar with [-] [+] touch buttons (Centered)
        if cur_y < max_y && inner.width > 20 {
            let vol_str = format!("Vol: {:3}%", state.volume);
            let vol_frac = (state.volume as f64) / 100.0;

            let minus_btn = TouchButton::new("-", UiAction::Engine(EngineCommand::AdjustVolume(-5)))
                .style(Style::default().fg(theme.muted));
            let plus_btn = TouchButton::new("+", UiAction::Engine(EngineCommand::AdjustVolume(5)))
                .style(Style::default().fg(theme.muted));

            let total_vol_w = inner.width.min(50).max(20);
            let start_vol_x = inner.x + (inner.width.saturating_sub(total_vol_w)) / 2;
            let minus_w = 4u16;
            let plus_w = 4u16;
            let bar_w = total_vol_w.saturating_sub(minus_w + plus_w + 2);
            let bar_x = start_vol_x + minus_w + 1;

            minus_btn.render_and_register(Rect::new(start_vol_x, cur_y, minus_w, 1), buf, hitmap);
            TouchBar::volume(vol_frac, &vol_str, "")
                .filled_style(Style::default().fg(theme.secondary))
                .empty_style(theme.progress_empty_style())
                .label_style(Style::default().fg(theme.muted))
                .render_and_register(Rect::new(bar_x, cur_y, bar_w, 1), buf, hitmap);
            plus_btn.render_and_register(Rect::new(start_vol_x + total_vol_w - plus_w, cur_y, plus_w, 1), buf, hitmap);
        }

        pending_graphic
    }
}
