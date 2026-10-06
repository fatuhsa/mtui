use ratatui::style::{Color, Modifier, Style};

/// Color palette and styles tailored for high readability on mobile screens in Termux
#[derive(Debug, Clone)]
pub struct Theme {
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
    pub playing: Color,
    pub paused: Color,
    pub stopped: Color,
    pub text: Color,
    pub muted: Color,
    pub border: Color,
    pub active_tab: Color,
    pub inactive_tab: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::neon()
    }
}

impl Theme {
    /// Modern Neon theme: vivid accents and sharp contrast
    pub fn neon() -> Self {
        Self {
            primary: Color::Cyan,
            secondary: Color::Magenta,
            accent: Color::Yellow,
            playing: Color::Green,
            paused: Color::Yellow,
            stopped: Color::DarkGray,
            text: Color::White,
            muted: Color::DarkGray,
            border: Color::Cyan,
            active_tab: Color::Cyan,
            inactive_tab: Color::DarkGray,
        }
    }

    /// AMOLED theme: high contrast monochrome with lime accents
    pub fn amoled() -> Self {
        Self {
            primary: Color::White,
            secondary: Color::LightGreen,
            accent: Color::LightYellow,
            playing: Color::LightGreen,
            paused: Color::Yellow,
            stopped: Color::DarkGray,
            text: Color::White,
            muted: Color::DarkGray,
            border: Color::White,
            active_tab: Color::LightGreen,
            inactive_tab: Color::DarkGray,
        }
    }

    pub fn title_style(&self) -> Style {
        Style::default()
            .fg(self.primary)
            .add_modifier(Modifier::BOLD)
    }

    pub fn active_tab_style(&self) -> Style {
        Style::default()
            .fg(Color::Black)
            .bg(self.active_tab)
            .add_modifier(Modifier::BOLD)
    }

    pub fn inactive_tab_style(&self) -> Style {
        Style::default().fg(self.inactive_tab)
    }

    pub fn button_style(&self) -> Style {
        Style::default().fg(self.primary)
    }

    pub fn button_highlight_style(&self) -> Style {
        Style::default()
            .fg(Color::Black)
            .bg(self.secondary)
            .add_modifier(Modifier::BOLD)
    }

    pub fn progress_filled_style(&self) -> Style {
        Style::default().fg(self.primary)
    }

    pub fn progress_empty_style(&self) -> Style {
        Style::default().fg(self.muted)
    }

    pub fn status_style(&self, status: crate::engine::events::PlaybackStatus) -> Style {
        match status {
            crate::engine::events::PlaybackStatus::Playing => Style::default()
                .fg(self.playing)
                .add_modifier(Modifier::BOLD),
            crate::engine::events::PlaybackStatus::Paused => Style::default()
                .fg(self.paused)
                .add_modifier(Modifier::BOLD),
            crate::engine::events::PlaybackStatus::Stopped => Style::default().fg(self.stopped),
        }
    }
}
