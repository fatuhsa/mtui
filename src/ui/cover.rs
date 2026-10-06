use crate::util::get_or_extract_cover_art;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

/// Supported terminal image protocols
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverProtocol {
    Sixel,
    Iterm2,
    HalfBlock,
    Off,
}

impl CoverProtocol {
    pub fn next(&self) -> Self {
        match self {
            CoverProtocol::Sixel => CoverProtocol::Iterm2,
            CoverProtocol::Iterm2 => CoverProtocol::HalfBlock,
            CoverProtocol::HalfBlock => CoverProtocol::Off,
            CoverProtocol::Off => CoverProtocol::Sixel,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            CoverProtocol::Sixel => "Sixel",
            CoverProtocol::Iterm2 => "iTerm2",
            CoverProtocol::HalfBlock => "Blocks",
            CoverProtocol::Off => "Off",
        }
    }
}

/// Rendered cover art payload
#[derive(Debug, Clone)]
pub enum RenderedCover {
    /// Escape sequence string for Sixel / iTerm2 to be written at (x, y)
    EscapeSequence(String),
    /// ANSI text lines for HalfBlock symbols
    TextLines(Vec<String>),
    /// Fallback vinyl disk art
    VinylArt,
}

/// Asynchronous cover art manager with size awareness and dirty state tracking
pub struct CoverArtManager {
    pub protocol: CoverProtocol,
    current_audio_path: Option<PathBuf>,
    current_size: (u16, u16),
    current_rendered: RenderedCover,
    pub is_dirty: bool,
    req_tx: Sender<(PathBuf, CoverProtocol, u16, u16)>,
    resp_rx: Receiver<(PathBuf, CoverProtocol, u16, u16, RenderedCover)>,
}

impl Default for CoverArtManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CoverArtManager {
    pub fn new() -> Self {
        let (req_tx, req_rx) = mpsc::channel::<(PathBuf, CoverProtocol, u16, u16)>();
        let (resp_tx, resp_rx) =
            mpsc::channel::<(PathBuf, CoverProtocol, u16, u16, RenderedCover)>();

        // Background worker thread for extracting and converting images
        thread::spawn(move || {
            while let Ok(mut latest_req) = req_rx.recv() {
                // Drain any backlog in queue so rapid resizing / protocol cycling
                // always processes only the most recent request (latest request wins)
                while let Ok(newer_req) = req_rx.try_recv() {
                    latest_req = newer_req;
                }

                let (audio_path, proto, width, height) = latest_req;
                if proto == CoverProtocol::Off {
                    let _ =
                        resp_tx.send((audio_path, proto, width, height, RenderedCover::VinylArt));
                    continue;
                }

                if let Some(cover_path) = get_or_extract_cover_art(&audio_path) {
                    let rendered = render_image_with_chafa(&cover_path, proto, width, height);
                    let _ = resp_tx.send((audio_path, proto, width, height, rendered));
                } else {
                    let _ =
                        resp_tx.send((audio_path, proto, width, height, RenderedCover::VinylArt));
                }
            }
        });

        Self {
            protocol: CoverProtocol::Sixel,
            current_audio_path: None,
            current_size: (0, 0),
            current_rendered: RenderedCover::VinylArt,
            is_dirty: true,
            req_tx,
            resp_rx,
        }
    }

    /// Checks if a new cover art was processed in the background
    pub fn update(&mut self) {
        while let Ok((path, proto, w, h, rendered)) = self.resp_rx.try_recv() {
            if self.current_audio_path.as_ref() == Some(&path)
                && self.protocol == proto
                && self.current_size == (w, h)
            {
                self.current_rendered = rendered;
                self.is_dirty = true;
            }
        }
    }

    /// Mark graphics as needing redraw (e.g. after screen clear or returning to NowPlaying)
    pub fn mark_dirty(&mut self) {
        self.is_dirty = true;
    }

    /// Requests rendering for the specified track path with flexible dimensions
    pub fn set_track(&mut self, audio_path: Option<&Path>, width: u16, height: u16) {
        let path_buf = audio_path.map(|p| p.to_path_buf());
        let size_changed = self.current_size != (width, height);
        let track_changed = self.current_audio_path != path_buf;

        if track_changed || size_changed {
            self.current_audio_path = path_buf.clone();
            self.current_size = (width, height);
            self.is_dirty = true;

            if track_changed {
                self.current_rendered = RenderedCover::VinylArt;
            }

            if let Some(path) = path_buf {
                if width >= 4 && height >= 2 {
                    let _ = self.req_tx.send((path, self.protocol, width, height));
                }
            } else {
                self.current_rendered = RenderedCover::VinylArt;
            }
        }
    }

    /// Cycle to next graphics protocol
    pub fn cycle_protocol(&mut self) {
        self.protocol = self.protocol.next();
        self.is_dirty = true;
        if let Some(path) = &self.current_audio_path {
            let (w, h) = self.current_size;
            if w > 0 && h > 0 {
                let _ = self.req_tx.send((path.clone(), self.protocol, w, h));
            }
        }
    }

    /// Renders cover art into area.
    /// Returns Some((x, y, escape_str)) ONLY when graphics need to be output to stdout.
    /// Once output, is_dirty is cleared so we do not flood the terminal with escape codes.
    pub fn render_to_buffer(&mut self, area: Rect, buf: &mut Buffer) -> Option<(u16, u16, String)> {
        if area.width < 4 || area.height < 2 {
            return None;
        }

        match &self.current_rendered {
            RenderedCover::EscapeSequence(seq) => {
                // Clear the box in buffer so character cells don't conflict with graphics
                for y in area.y..area.y + area.height {
                    for x in area.x..area.x + area.width {
                        if let Some(cell) = buf.cell_mut((x, y)) {
                            cell.set_symbol(" ");
                            cell.set_style(Style::default());
                        }
                    }
                }

                // Only return graphic escape sequence when dirty
                if self.is_dirty {
                    self.is_dirty = false;
                    Some((area.x, area.y, seq.clone()))
                } else {
                    None
                }
            }
            RenderedCover::TextLines(lines) => {
                // Clear the box in buffer
                for y in area.y..area.y + area.height {
                    for x in area.x..area.x + area.width {
                        if let Some(cell) = buf.cell_mut((x, y)) {
                            cell.set_symbol(" ");
                            cell.set_style(Style::default());
                        }
                    }
                }

                if self.is_dirty {
                    self.is_dirty = false;
                    let mut combined = String::new();
                    for (i, line) in lines.iter().enumerate() {
                        let y = area.y + (i as u16);
                        if y >= area.y + area.height {
                            break;
                        }
                        if i > 0 {
                            combined.push_str(&format!("\x1b[{};{}H", y + 1, area.x + 1));
                        }
                        combined.push_str(line);
                    }
                    Some((area.x, area.y, combined))
                } else {
                    None
                }
            }
            RenderedCover::VinylArt => {
                render_vinyl_art(area, buf);
                None
            }
        }
    }
}

fn render_image_with_chafa(
    cover_path: &Path,
    protocol: CoverProtocol,
    width: u16,
    height: u16,
) -> RenderedCover {
    let fmt_arg = match protocol {
        CoverProtocol::Sixel => "sixel",
        CoverProtocol::Iterm2 => "iterm",
        CoverProtocol::HalfBlock => "symbols",
        CoverProtocol::Off => return RenderedCover::VinylArt,
    };

    let size_arg = format!("{}x{}", width, height);

    let output = Command::new("chafa")
        .arg("-f")
        .arg(fmt_arg)
        .arg("--size")
        .arg(size_arg)
        .arg(cover_path)
        .stdin(Stdio::null())
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).to_string();
            match protocol {
                CoverProtocol::Sixel | CoverProtocol::Iterm2 => {
                    return RenderedCover::EscapeSequence(s);
                }
                CoverProtocol::HalfBlock => {
                    let lines: Vec<String> = s.lines().map(|l| l.to_string()).collect();
                    return RenderedCover::TextLines(lines);
                }
                CoverProtocol::Off => {}
            }
        }
    }

    RenderedCover::VinylArt
}

fn render_vinyl_art(area: Rect, buf: &mut Buffer) {
    let art = [
        " ╭──────────────╮ ",
        " │    󰎆  󰎈    │ ",
        " │   ( ◉  ◉ )   │ ",
        " │  mtui player │ ",
        " ╰──────────────╯ ",
    ];

    let start_y = area.y + (area.height.saturating_sub(art.len() as u16)) / 2;
    for (i, line) in art.iter().enumerate() {
        let y = start_y + (i as u16);
        if y >= area.y + area.height {
            break;
        }
        let line_len = line.chars().count() as u16;
        let start_x = area.x + (area.width.saturating_sub(line_len)) / 2;
        buf.set_string(
            start_x,
            y,
            *line,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    }
}
