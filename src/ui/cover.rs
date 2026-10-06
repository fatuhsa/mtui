use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use crate::util::get_or_extract_cover_art;

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

/// Asynchronous cover art manager preventing any UI blocking during extraction and rendering
pub struct CoverArtManager {
    pub protocol: CoverProtocol,
    current_audio_path: Option<PathBuf>,
    current_rendered: RenderedCover,
    req_tx: Sender<(PathBuf, CoverProtocol, u16, u16)>,
    resp_rx: Receiver<(PathBuf, CoverProtocol, RenderedCover)>,
}

impl CoverArtManager {
    pub fn new() -> Self {
        let (req_tx, req_rx) = mpsc::channel::<(PathBuf, CoverProtocol, u16, u16)>();
        let (resp_tx, resp_rx) = mpsc::channel::<(PathBuf, CoverProtocol, RenderedCover)>();

        // Background worker thread for extracting and converting images
        thread::spawn(move || {
            while let Ok((audio_path, proto, width, height)) = req_rx.recv() {
                if proto == CoverProtocol::Off {
                    let _ = resp_tx.send((audio_path, proto, RenderedCover::VinylArt));
                    continue;
                }

                if let Some(cover_path) = get_or_extract_cover_art(&audio_path) {
                    let rendered = render_image_with_chafa(&cover_path, proto, width, height);
                    let _ = resp_tx.send((audio_path, proto, rendered));
                } else {
                    let _ = resp_tx.send((audio_path, proto, RenderedCover::VinylArt));
                }
            }
        });

        Self {
            protocol: CoverProtocol::Sixel,
            current_audio_path: None,
            current_rendered: RenderedCover::VinylArt,
            req_tx,
            resp_rx,
        }
    }

    /// Checks if a new cover art was processed in the background
    pub fn update(&mut self) {
        while let Ok((path, proto, rendered)) = self.resp_rx.try_recv() {
            if self.current_audio_path.as_ref() == Some(&path) && self.protocol == proto {
                self.current_rendered = rendered;
            }
        }
    }

    /// Requests rendering for the specified track path
    pub fn set_track(&mut self, audio_path: Option<&Path>, width: u16, height: u16) {
        let path_buf = audio_path.map(|p| p.to_path_buf());
        if self.current_audio_path != path_buf {
            self.current_audio_path = path_buf.clone();
            self.current_rendered = RenderedCover::VinylArt;

            if let Some(path) = path_buf {
                let _ = self.req_tx.send((path, self.protocol, width, height));
            }
        }
    }

    /// Cycle to next graphics protocol
    pub fn cycle_protocol(&mut self, width: u16, height: u16) {
        self.protocol = self.protocol.next();
        if let Some(path) = &self.current_audio_path {
            let _ = self.req_tx.send((path.clone(), self.protocol, width, height));
        }
    }

    /// Renders cover art into area.
    /// Returns Some((x, y, escape_str)) if Sixel/iTerm2 needs to be written directly to stdout.
    pub fn render_to_buffer(
        &self,
        area: Rect,
        buf: &mut Buffer,
    ) -> Option<(u16, u16, String)> {
        if area.width < 8 || area.height < 4 {
            return None;
        }

        match &self.current_rendered {
            RenderedCover::EscapeSequence(seq) => {
                // Clear the box in buffer so no characters interfere with the image
                for y in area.y..area.y + area.height {
                    for x in area.x..area.x + area.width {
                        if let Some(cell) = buf.cell_mut((x, y)) {
                            cell.set_symbol(" ");
                        }
                    }
                }
                // Return coordinates for terminal stdout write
                Some((area.x, area.y, seq.clone()))
            }
            RenderedCover::TextLines(lines) => {
                for (i, line) in lines.iter().enumerate() {
                    let y = area.y + (i as u16);
                    if y >= area.y + area.height {
                        break;
                    }
                    buf.set_string(area.x, y, line, Style::default());
                }
                None
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
        let start_x = area.x + (area.width.saturating_sub(line.chars().count() as u16)) / 2;
        buf.set_string(
            start_x,
            y,
            *line,
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        );
    }
}
