//! Raw-mode terminal setup/teardown: entering the alternate screen,
//! diff-based redraws, input polling, and panic-safe cleanup.

use std::io::{stdout, Stdout, Write};
use std::time::Duration;

use crossterm::event::{self, Event};
use crossterm::style::{Attribute, Print, SetAttribute, SetBackgroundColor, SetForegroundColor};
use crossterm::{cursor, execute, terminal};

use crate::buffer::CellDiff;

/// A raw-mode, alternate-screen terminal handle. Restores normal
/// terminal state automatically on drop.
pub struct Terminal {
    out: Stdout,
}

impl Terminal {
    /// Enables raw mode, enters the alternate screen, and hides the
    /// cursor.
    pub fn new() -> std::io::Result<Self> {
        terminal::enable_raw_mode()?;
        let mut out = stdout();
        execute!(out, terminal::EnterAlternateScreen, cursor::Hide)?;
        Ok(Terminal { out })
    }

    /// Current terminal size in cells, `(width, height)`.
    pub fn size(&self) -> std::io::Result<(u16, u16)> {
        terminal::size()
    }

    /// Writes only the given changed cells to the terminal.
    pub fn draw_diff(&mut self, diffs: &[CellDiff]) -> std::io::Result<()> {
        for d in diffs {
            let attr = if d.cell.style.bold {
                Attribute::Bold
            } else {
                Attribute::Reset
            };
            execute!(
                self.out,
                cursor::MoveTo(d.x, d.y),
                SetAttribute(Attribute::Reset),
                SetAttribute(attr),
                SetForegroundColor(d.cell.fg),
                SetBackgroundColor(d.cell.bg),
                Print(d.cell.symbol),
            )?;
        }
        self.out.flush()
    }

    /// Polls for one input event, up to `timeout`; `None` on timeout.
    pub fn next_event(&self, timeout: Duration) -> std::io::Result<Option<Event>> {
        if event::poll(timeout)? {
            Ok(Some(event::read()?))
        } else {
            Ok(None)
        }
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = execute!(self.out, SetAttribute(Attribute::Reset));
        let _ = terminal::disable_raw_mode();
        let _ = execute!(self.out, terminal::LeaveAlternateScreen, cursor::Show);
    }
}

/// Wraps the default panic hook so a panic mid-raw-mode still restores
/// normal terminal state before printing the panic message.
pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(stdout(), SetAttribute(Attribute::Reset));
        let _ = terminal::disable_raw_mode();
        let _ = execute!(stdout(), terminal::LeaveAlternateScreen, cursor::Show);
        default_hook(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::terminal;

    #[test]
    #[ignore = "requires a real terminal (TTY); run with `cargo test -- --ignored`"]
    fn enter_and_drop_restores_raw_mode() {
        assert!(!terminal::is_raw_mode_enabled().unwrap());
        {
            let _term = Terminal::new().unwrap();
            assert!(terminal::is_raw_mode_enabled().unwrap());
        }
        assert!(!terminal::is_raw_mode_enabled().unwrap());
    }

    #[test]
    #[ignore = "requires a real terminal (TTY); run with `cargo test -- --ignored`"]
    fn panic_hook_disables_raw_mode_before_unwinding() {
        install_panic_hook();
        terminal::enable_raw_mode().unwrap();
        assert!(terminal::is_raw_mode_enabled().unwrap());

        let result = std::panic::catch_unwind(|| {
            panic!("simulated crash");
        });

        assert!(result.is_err());
        assert!(!terminal::is_raw_mode_enabled().unwrap());
    }
}
