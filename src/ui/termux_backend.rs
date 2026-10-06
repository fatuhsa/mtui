use std::io::{self, Write};
use ratatui::backend::{Backend, ClearType, CrosstermBackend, WindowSize};
use ratatui::buffer::Cell;
use ratatui::layout::{Position, Size};

/// A robust crossterm backend wrapper tailored for Termux / mobile terminal environments.
///
/// In standard `CrosstermBackend`, `get_cursor_position()` sends `\x1B[6n` (Device Status Report / CPR)
/// to stdout and polls stdin for 2000ms. In Termux on Android, pinch-to-zoom and touch gestures trigger
/// rapid SIGWINCH resizes and mouse event bursts, which frequently drop or delay the CPR response.
/// When Crossterm times out after 2000ms, it raises an `io::ErrorKind::Other` error:
/// `"The cursor position could not be read within a normal duration"`, causing the app to crash.
///
/// `TermuxBackend` maintains an in-memory tracked cursor position so `get_cursor_position()`
/// resolves immediately in 0ms without sending any CPR escape sequences or reading stdin.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct TermuxBackend<W: Write> {
    inner: CrosstermBackend<W>,
    cursor_pos: Position,
}

impl<W: Write> TermuxBackend<W> {
    pub fn new(writer: W) -> Self {
        Self {
            inner: CrosstermBackend::new(writer),
            cursor_pos: Position::default(),
        }
    }

    pub fn inner(&self) -> &CrosstermBackend<W> {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut CrosstermBackend<W> {
        &mut self.inner
    }
}

impl<W: Write> Write for TermuxBackend<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        std::io::Write::flush(&mut self.inner)
    }
}

impl<W: Write> Backend for TermuxBackend<W> {
    type Error = io::Error;

    fn draw<'a, I>(&mut self, content: I) -> Result<(), Self::Error>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        self.inner.draw(content)
    }

    fn append_lines(&mut self, n: u16) -> Result<(), Self::Error> {
        self.inner.append_lines(n)
    }

    fn hide_cursor(&mut self) -> Result<(), Self::Error> {
        self.inner.hide_cursor()
    }

    fn show_cursor(&mut self) -> Result<(), Self::Error> {
        self.inner.show_cursor()
    }

    fn get_cursor_position(&mut self) -> Result<Position, Self::Error> {
        // Never query the terminal device over stdin (\x1b[6n) in Termux/Android!
        // During pinch-zoom or rapid touch/resize events, CPR queries can time out or hang.
        // Returning the tracked cursor position is instant and 100% reliable.
        Ok(self.cursor_pos)
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> Result<(), Self::Error> {
        let pos = position.into();
        self.cursor_pos = pos;
        self.inner.set_cursor_position(pos)
    }

    fn clear(&mut self) -> Result<(), Self::Error> {
        self.inner.clear()
    }

    fn clear_region(&mut self, clear_type: ClearType) -> Result<(), Self::Error> {
        self.inner.clear_region(clear_type)
    }

    fn size(&self) -> Result<Size, Self::Error> {
        self.inner.size()
    }

    fn window_size(&mut self) -> Result<WindowSize, Self::Error> {
        self.inner.window_size()
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Backend::flush(&mut self.inner)
    }
}
