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
    pub fn render(
        area: Rect,
        buf: &mut Buffer,
        state: &EngineStateSnapshot,
        cover_mgr: &CoverArtManager,
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

        // 1. Cover Art & Equalizer Section
        if inner.height >= 14 {
            let art_w = 20u16.min(inner.width.saturating_sub(4));
            let art_h = 7u16.min(inner.height.saturating_sub(10));

            if profile.is_compact() {
                // Centered Cover Art box
                let art_x = inner.x + (inner.width.saturating_sub(art_w)) / 2;
                let art_rect = Rect::new(art_x, cur_y, art_w, art_h);

                pending_graphic = cover_mgr.render_to_buffer(art_rect, buf);
                cur_y += art_h + 1;
            } else {
                // Wide split: Cover Art on Left, Animated Equalizer & Details on Right
                let art_rect = Rect::new(inner.x + 1, cur_y, art_w, art_h);
                pending_graphic = cover_mgr.render_to_buffer(art_rect, buf);

                // Right side: Animated Equalizer
                let right_x = inner.x + art_w + 3;
                let right_w = inner.width.saturating_sub(art_w + 4);

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
                    .render(Rect::new(right_x, cur_y + art_h / 2, right_w, 1), buf);

                cur_y += art_h + 1;
            }
        } else if inner.height >= 10 {
            // Small screen: fallback to compact 1-line equalizer
            let eq_art = match state.status {
                PlaybackStatus::Playing => " ▂ ▄ ▆ █ ▇ ▅ ▃ ▂ ▄ ▆ █ ▇ ▅ ▃ ",
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
            cur_y += 2;
        }

        // 2. Track Title & Artist
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
            Marquee::render(title_str, title_rect, tick, buf, title_style);
            cur_y += 1;
        }

        if cur_y < max_y {
            let artist_rect = Rect::new(inner.x, cur_y, inner.width, 1);
            let artist_style = Style::default().fg(theme.primary);
            Marquee::render(artist_str, artist_rect, tick, buf, artist_style);
            cur_y += 2;
        }

        // 3. Touch Seek Bar
        if cur_y < max_y {
            let fraction = if state.duration_secs > 0.0 {
                state.position_secs / state.duration_secs
            } else {
                0.0
            };

            let pos_str = format_time(state.position_secs);
            let dur_str = format_time(state.duration_secs);

            let bar_rect = Rect::new(inner.x, cur_y, inner.width, 1);
            TouchBar::progress(fraction, &pos_str, &dur_str)
                .filled_style(theme.progress_filled_style())
                .empty_style(theme.progress_empty_style())
                .label_style(Style::default().fg(theme.muted))
                .render_and_register(bar_rect, buf, hitmap);

            cur_y += 2;
        }

        // 4. Large Touch Playback Controls
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

            if profile.is_compact() {
                let row_w = 32u16.min(inner.width);
                let start_x = inner.x + (inner.width.saturating_sub(row_w)) / 2;

                prev_btn.render_and_register(Rect::new(start_x, cur_y, 9, 1), buf, hitmap);
                play_btn.render_and_register(Rect::new(start_x + 10, cur_y, 11, 1), buf, hitmap);
                next_btn.render_and_register(Rect::new(start_x + 22, cur_y, 9, 1), buf, hitmap);
            } else {
                let btn_w = 12u16;
                let total_w = btn_w * 3 + 4;
                let start_x = inner.x + (inner.width.saturating_sub(total_w)) / 2;

                prev_btn.render_and_register(Rect::new(start_x, cur_y, btn_w, 1), buf, hitmap);
                play_btn.render_and_register(Rect::new(start_x + btn_w + 2, cur_y, btn_w, 1), buf, hitmap);
                next_btn.render_and_register(Rect::new(start_x + (btn_w + 2) * 2, cur_y, btn_w, 1), buf, hitmap);
            }

            cur_y += 2;
        }

        // 5. Secondary Controls: Loop Mode, Shuffle, and Cover Protocol Switcher
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

            let mut bx = inner.x;
            loop_btn.render_and_register(Rect::new(bx, cur_y, 9, 1), buf, hitmap);
            bx += 10;

            if inner.width > 22 {
                shuf_btn.render_and_register(Rect::new(bx, cur_y, 9, 1), buf, hitmap);
                bx += 10;
            }

            if inner.width > 34 {
                let art_w = (art_label.len() as u16) + 4;
                art_btn.render_and_register(Rect::new(bx, cur_y, art_w, 1), buf, hitmap);
            }

            cur_y += 2;
        }

        // 6. Interactive Volume Bar with [-] [+] touch buttons
        if cur_y < max_y {
            let vol_str = format!("Vol: {:3}%", state.volume);
            let vol_frac = (state.volume as f64) / 100.0;

            let minus_btn = TouchButton::new("-", UiAction::Engine(EngineCommand::AdjustVolume(-5)))
                .style(Style::default().fg(theme.muted));
            let plus_btn = TouchButton::new("+", UiAction::Engine(EngineCommand::AdjustVolume(5)))
                .style(Style::default().fg(theme.muted));

            let minus_w = 5u16;
            let plus_w = 5u16;

            if inner.width > 25 {
                let bar_x = inner.x + minus_w + 1;
                let bar_w = inner.width.saturating_sub(minus_w + plus_w + 2);

                minus_btn.render_and_register(Rect::new(inner.x, cur_y, minus_w, 1), buf, hitmap);
                TouchBar::volume(vol_frac, &vol_str, "")
                    .filled_style(Style::default().fg(theme.secondary))
                    .empty_style(theme.progress_empty_style())
                    .label_style(Style::default().fg(theme.muted))
                    .render_and_register(Rect::new(bar_x, cur_y, bar_w, 1), buf, hitmap);
                plus_btn.render_and_register(Rect::new(inner.x + inner.width - plus_w, cur_y, plus_w, 1), buf, hitmap);
            }
        }

        pending_graphic
    }
}
