use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Widget};

use crate::ui::hitmap::TouchHitMap;
use crate::ui::responsive::ScreenProfile;
use crate::ui::theme::Theme;

pub struct HelpView;

impl HelpView {
    pub fn render(
        area: Rect,
        buf: &mut Buffer,
        theme: &Theme,
        _profile: ScreenProfile,
        _hitmap: &mut TouchHitMap,
    ) {
        if area.width < 10 || area.height < 5 {
            return;
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title("  Touch & Controls Guide ")
            .title_alignment(Alignment::Center);
        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        let lines = vec![
            (" TOUCH CONTROLS (Termux)", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
            ("  • Tap Tabs at the top to change screens", Style::default().fg(theme.text)),
            ("  • Tap [Play/Pause], [Prev], [Next] to control music", Style::default().fg(theme.text)),
            ("  • Tap any point on Progress Bar to seek position", Style::default().fg(theme.text)),
            ("  • Tap Volume Bar or [-]/[+] to adjust volume", Style::default().fg(theme.text)),
            ("  • Tap folder in Files to enter, tap [.. Up] to exit", Style::default().fg(theme.text)),
            ("  • Tap song in Files to play immediately", Style::default().fg(theme.text)),
            ("  • Tap [] beside song to append it to Queue", Style::default().fg(theme.text)),
            ("  • Tap song in Queue to jump to it, tap [] to remove", Style::default().fg(theme.text)),
            ("  • Tap [] on top right to minimize (type 'fg' to restore)", Style::default().fg(theme.text)),
            ("  • Tap [ Sixel/iTerm2/Blocks] to toggle cover art mode", Style::default().fg(theme.text)),
            ("  • Tap [] on top right or press 'q' to quit", Style::default().fg(theme.text)),
            ("  • Tap ▲/▼ or swipe/scroll to navigate lists", Style::default().fg(theme.text)),
            ("", Style::default()),
            (" KEYBOARD SHORTCUTS (Optional)", Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD)),
            ("  • Space       : Toggle Play / Pause", Style::default().fg(theme.text)),
            ("  • n / p       : Next / Previous Track", Style::default().fg(theme.text)),
            ("  • Left / Right: Seek -5s / +5s", Style::default().fg(theme.text)),
            ("  • + / -       : Volume Up / Down", Style::default().fg(theme.text)),
            ("  • m           : Minimize to background (fg to restore)", Style::default().fg(theme.text)),
            ("  • c           : Cycle Cover Art (Sixel/iTerm2/Blocks/Off)", Style::default().fg(theme.text)),
            ("  • Tab         : Switch between Tabs", Style::default().fg(theme.text)),
            ("  • s           : Toggle Shuffle", Style::default().fg(theme.text)),
            ("  • l           : Cycle Loop Mode (Off/All/Track)", Style::default().fg(theme.text)),
            ("  • q           : Exit player cleanly", Style::default().fg(theme.text)),
        ];

        let mut y = inner.y;
        for (text, style) in lines {
            if y >= inner.y + inner.height {
                break;
            }
            buf.set_string(inner.x + 1, y, text, style);
            y += 1;
        }
    }
}
