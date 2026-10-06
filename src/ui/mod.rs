pub mod cover;
pub mod hitmap;
pub mod responsive;
pub mod theme;
pub mod views;
pub mod widgets;

use std::path::{Path, PathBuf};
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Widget};
use ratatui::Frame;

pub use cover::{CoverArtManager, CoverProtocol};
pub use hitmap::{TouchHitMap, UiAction};
pub use responsive::ScreenProfile;
pub use theme::Theme;
pub use views::*;

use crate::engine::commands::EngineCommand;
use crate::engine::events::{EngineStateSnapshot, PlaybackStatus};
use crate::engine::track::Track;
use crate::ui::widgets::TouchButton;
use crate::util::{detect_default_music_dirs, scan_audio_files};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppTab {
    NowPlaying = 0,
    Files = 1,
    Queue = 2,
    Help = 3,
}

impl AppTab {
    pub fn from_index(idx: usize) -> Self {
        match idx {
            0 => AppTab::NowPlaying,
            1 => AppTab::Files,
            2 => AppTab::Queue,
            _ => AppTab::Help,
        }
    }

    pub fn to_index(self) -> usize {
        self as usize
    }

    pub fn next(self) -> Self {
        match self {
            AppTab::NowPlaying => AppTab::Files,
            AppTab::Files => AppTab::Queue,
            AppTab::Queue => AppTab::Help,
            AppTab::Help => AppTab::NowPlaying,
        }
    }
}

/// The UI Controller coordinating presentation and dispatching events to the AudioEngine
pub struct AppUi {
    pub current_tab: AppTab,
    pub previous_tab: AppTab,
    pub needs_terminal_clear: bool,
    pub theme: Theme,
    pub hitmap: TouchHitMap,
    pub tick: usize,
    pub cover_mgr: CoverArtManager,
    pub pending_graphic: Option<(u16, u16, String)>,

    // File browser state
    pub browser_dir: PathBuf,
    pub browser_items: Vec<BrowserItem>,
    pub browser_scroll: usize,
    pub browser_selected: Option<usize>,

    // Queue view state
    pub queue_scroll: usize,
    pub queue_selected: Option<usize>,
}

impl AppUi {
    pub fn new() -> Self {
        let default_dir = detect_default_music_dirs()
            .into_iter()
            .next()
            .unwrap_or_else(|| PathBuf::from("."));

        let items = FileBrowserView::list_directory(&default_dir);

        Self {
            current_tab: AppTab::NowPlaying,
            previous_tab: AppTab::NowPlaying,
            needs_terminal_clear: false,
            theme: Theme::neon(),
            hitmap: TouchHitMap::new(),
            tick: 0,
            cover_mgr: CoverArtManager::new(),
            pending_graphic: None,
            browser_dir: default_dir,
            browser_items: items,
            browser_scroll: 0,
            browser_selected: None,
            queue_scroll: 0,
            queue_selected: None,
        }
    }

    pub fn on_tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    pub fn navigate_browser<P: AsRef<Path>>(&mut self, path: P) {
        let p = path.as_ref().to_path_buf();
        if p.is_dir() {
            self.browser_items = FileBrowserView::list_directory(&p);
            self.browser_dir = p;
            self.browser_scroll = 0;
            self.browser_selected = None;
        }
    }

    pub fn browser_up(&mut self) {
        if let Some(parent) = self.browser_dir.parent() {
            self.navigate_browser(parent.to_path_buf());
        }
    }

    /// Handles touch and mouse clicks
    pub fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Option<UiAction> {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.hitmap.resolve_tap(mouse.column, mouse.row)
            }
            MouseEventKind::ScrollUp => Some(UiAction::ScrollUp),
            MouseEventKind::ScrollDown => Some(UiAction::ScrollDown),
            _ => None,
        }
    }

    /// Handles keyboard input
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Option<UiAction> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => Some(UiAction::Quit),
            KeyCode::Char('m') | KeyCode::Char('M') => Some(UiAction::Minimize),
            KeyCode::Char('c') | KeyCode::Char('C') => Some(UiAction::CycleCoverProtocol),
            KeyCode::Char(' ') => Some(UiAction::Engine(EngineCommand::TogglePlay)),
            KeyCode::Char('n') => Some(UiAction::Engine(EngineCommand::Next)),
            KeyCode::Char('p') => Some(UiAction::Engine(EngineCommand::Prev)),
            KeyCode::Char('s') => Some(UiAction::Engine(EngineCommand::ToggleShuffle)),
            KeyCode::Char('l') => Some(UiAction::Engine(EngineCommand::CycleLoopMode)),
            KeyCode::Left => Some(UiAction::Engine(EngineCommand::SeekRelative(-5.0))),
            KeyCode::Right => Some(UiAction::Engine(EngineCommand::SeekRelative(5.0))),
            KeyCode::Char('+') | KeyCode::Char('=') => Some(UiAction::Engine(EngineCommand::AdjustVolume(5))),
            KeyCode::Char('-') => Some(UiAction::Engine(EngineCommand::AdjustVolume(-5))),
            KeyCode::Tab => {
                let next_tab = self.current_tab.next();
                Some(UiAction::SwitchTab(next_tab.to_index()))
            }
            KeyCode::Up => Some(UiAction::ScrollUp),
            KeyCode::Down => Some(UiAction::ScrollDown),
            _ => None,
        }
    }

    /// Processes high-level UI actions (like navigation and list scrolling)
    /// Returns true if an EngineCommand needs to be dispatched to AudioEngine
    pub fn process_action(
        &mut self,
        action: UiAction,
        engine_tx: &std::sync::mpsc::Sender<EngineCommand>,
    ) -> bool {
        match action {
            UiAction::SwitchTab(idx) => {
                let target = AppTab::from_index(idx);
                if self.current_tab != target {
                    self.previous_tab = self.current_tab;
                    self.current_tab = target;
                    self.needs_terminal_clear = true;
                    if target == AppTab::NowPlaying {
                        self.cover_mgr.mark_dirty();
                    }
                }
                false
            }
            UiAction::CycleCoverProtocol => {
                self.cover_mgr.cycle_protocol();
                self.needs_terminal_clear = true;
                false
            }
            UiAction::Engine(cmd) => {
                let _ = engine_tx.send(cmd);
                false
            }
            UiAction::BrowseEnter(path) => {
                self.navigate_browser(path);
                false
            }
            UiAction::BrowseParent => {
                self.browser_up();
                false
            }
            UiAction::BrowseAddAll(dir) => {
                let tracks = scan_audio_files(dir);
                if !tracks.is_empty() {
                    let _ = engine_tx.send(EngineCommand::AddTracks(tracks));
                }
                false
            }
            UiAction::BrowsePlayTrack(path) => {
                let track = Track::from_path(path);
                let _ = engine_tx.send(EngineCommand::PlayTrackNow(track));
                if self.current_tab != AppTab::NowPlaying {
                    self.previous_tab = self.current_tab;
                    self.current_tab = AppTab::NowPlaying;
                    self.needs_terminal_clear = true;
                }
                self.cover_mgr.mark_dirty();
                false
            }
            UiAction::BrowseQueueTrack(path) => {
                let track = Track::from_path(path);
                let _ = engine_tx.send(EngineCommand::AddTrack(track));
                false
            }
            UiAction::SelectListItem(idx) => {
                match self.current_tab {
                    AppTab::Files => self.browser_selected = Some(idx),
                    AppTab::Queue => self.queue_selected = Some(idx),
                    _ => {}
                }
                false
            }
            UiAction::ScrollUp => {
                match self.current_tab {
                    AppTab::Files => self.browser_scroll = self.browser_scroll.saturating_sub(1),
                    AppTab::Queue => self.queue_scroll = self.queue_scroll.saturating_sub(1),
                    _ => {}
                }
                false
            }
            UiAction::ScrollDown => {
                match self.current_tab {
                    AppTab::Files => {
                        if self.browser_scroll + 1 < self.browser_items.len() {
                            self.browser_scroll += 1;
                        }
                    }
                    AppTab::Queue => {
                        self.queue_scroll += 1;
                    }
                    _ => {}
                }
                false
            }
            UiAction::Minimize => false,
            UiAction::Quit => true,
        }
    }

    /// Renders the complete TUI frame
    pub fn draw(&mut self, frame: &mut Frame, state: &EngineStateSnapshot) {
        let area = frame.area();
        self.hitmap.clear();
        self.pending_graphic = None;

        if area.width < 10 || area.height < 5 {
            return;
        }

        let profile = ScreenProfile::from_rect(area);

        // Vertical layout: [Top Tab Bar] -> [Active View] -> [Bottom Mini Bar]
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tab bar
                Constraint::Min(3),    // Main view
                Constraint::Length(1), // Status bar
            ])
            .split(area);

        let tab_area = chunks[0];
        let main_area = chunks[1];
        let status_area = chunks[2];

        // Compute flexible cover art dimensions from main_area inner box
        let inner_w = main_area.width.saturating_sub(2);
        let inner_h = main_area.height.saturating_sub(2);
        let (art_w, art_h) = NowPlayingView::calculate_art_size(inner_w, inner_h);

        // Update cover art background loader with dynamic dimensions
        self.cover_mgr.update();
        let current_path = state.current_track.as_ref().map(|t| t.path.as_path());
        self.cover_mgr.set_track(current_path, art_w, art_h);

        // 1. Draw Top Tab Bar with Touch Targets (including Minimize and Quit)
        self.draw_tab_bar(tab_area, frame.buffer_mut(), profile);

        // 2. Draw Active View
        match self.current_tab {
            AppTab::NowPlaying => {
                self.pending_graphic = NowPlayingView::render(
                    main_area,
                    frame.buffer_mut(),
                    state,
                    &mut self.cover_mgr,
                    &self.theme,
                    profile,
                    self.tick,
                    &mut self.hitmap,
                );
            }
            AppTab::Files => {
                FileBrowserView::render(
                    main_area,
                    frame.buffer_mut(),
                    &self.browser_dir,
                    &self.browser_items,
                    self.browser_scroll,
                    self.browser_selected,
                    &self.theme,
                    profile,
                    &mut self.hitmap,
                );
            }
            AppTab::Queue => {
                QueueView::render(
                    main_area,
                    frame.buffer_mut(),
                    state,
                    self.queue_scroll,
                    self.queue_selected,
                    &self.theme,
                    profile,
                    &mut self.hitmap,
                );
            }
            AppTab::Help => {
                HelpView::render(
                    main_area,
                    frame.buffer_mut(),
                    &self.theme,
                    profile,
                    &mut self.hitmap,
                );
            }
        }

        // 3. Draw Bottom Status Bar
        self.draw_status_bar(status_area, frame.buffer_mut(), state);
    }

    fn draw_tab_bar(&mut self, area: Rect, buf: &mut Buffer, _profile: ScreenProfile) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.border));
        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        // Touch Minimize [  ] and Quit [  ] on right edge
        let right_reserved = if inner.width > 24 {
            let min_btn = TouchButton::new("", UiAction::Minimize)
                .style(Style::default().fg(self.theme.secondary));
            let min_rect = Rect::new(inner.x + inner.width.saturating_sub(12), inner.y, 5, 1);
            min_btn.render_and_register(min_rect, buf, &mut self.hitmap);

            let quit_btn = TouchButton::new("", UiAction::Quit)
                .style(Style::default().fg(Color::Red));
            let quit_rect = Rect::new(inner.x + inner.width.saturating_sub(6), inner.y, 5, 1);
            quit_btn.render_and_register(quit_rect, buf, &mut self.hitmap);
            14u16
        } else if inner.width > 12 {
            let quit_btn = TouchButton::new("", UiAction::Quit)
                .style(Style::default().fg(Color::Red));
            let quit_rect = Rect::new(inner.x + inner.width.saturating_sub(6), inner.y, 5, 1);
            quit_btn.render_and_register(quit_rect, buf, &mut self.hitmap);
            8u16
        } else {
            0u16
        };

        let avail_tab_w = inner.width.saturating_sub(right_reserved);

        let tabs = [
            ("", "Play", AppTab::NowPlaying),
            ("", "File", AppTab::Files),
            ("", "Queue", AppTab::Queue),
            ("", "Help", AppTab::Help),
        ];

        // Total width if all tabs show full "[ icon name ]":
        // 10 + 1 + 10 + 1 + 11 + 1 + 10 = 44 cols
        let use_full = avail_tab_w >= 44;

        let mut x = inner.x;
        for (icon, name, tab) in tabs {
            let is_active = self.current_tab == tab;
            let style = if is_active {
                self.theme.active_tab_style()
            } else {
                self.theme.inactive_tab_style()
            };

            let label = if use_full || is_active {
                format!("{} {}", icon, name)
            } else {
                icon.to_string()
            };

            let btn_w = (label.chars().count() as u16) + 4;
            if x + btn_w <= inner.x + avail_tab_w {
                let btn_rect = Rect::new(x, inner.y, btn_w, 1);
                TouchButton::new(&label, UiAction::SwitchTab(tab.to_index()))
                    .style(style)
                    .render_and_register(btn_rect, buf, &mut self.hitmap);
                x += btn_w + 1;
            }
        }
    }

    fn draw_status_bar(&mut self, area: Rect, buf: &mut Buffer, state: &EngineStateSnapshot) {
        let status_text = match state.status {
            PlaybackStatus::Playing => " PLAYING",
            PlaybackStatus::Paused => " PAUSED",
            PlaybackStatus::Stopped => " STOPPED",
        };

        let status_style = self.theme.status_style(state.status);

        let current_song = state
            .current_track
            .as_ref()
            .map(|t| t.display_name())
            .unwrap_or_else(|| "No track loaded".to_string());

        let bar_text = format!(" {} │ {} ", status_text, current_song);
        let max_len = area.width as usize;
        let slice = if bar_text.len() > max_len {
            &bar_text[..max_len]
        } else {
            &bar_text
        };

        buf.set_string(area.x, area.y, slice, status_style);
    }
}
