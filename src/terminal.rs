use std::io::{stdout, Stdout, Write};

use crossterm::style::{Print, SetBackgroundColor, SetForegroundColor};
use crossterm::{cursor, execute, terminal};

use crate::buffer::CellDiff;

pub struct Terminal {
    out: Stdout,
}

impl Terminal {
    pub fn new() -> std::io::Result<Self> {
        terminal::enable_raw_mode()?;
        let mut out = stdout();
        execute!(out, terminal::EnterAlternateScreen, cursor::Hide)?;
        Ok(Terminal { out })
    }

    pub fn size(&self) -> std::io::Result<(u16, u16)> {
        terminal::size()
    }

    pub fn draw_diff(&mut self, diffs: &[CellDiff]) -> std::io::Result<()> {
        for d in diffs {
            execute!(
                self.out,
                cursor::MoveTo(d.x, d.y),
                SetForegroundColor(d.cell.fg),
                SetBackgroundColor(d.cell.bg),
                Print(d.cell.symbol),
            )?;
        }
        self.out.flush()
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(self.out, terminal::LeaveAlternateScreen, cursor::Show);
    }
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
}
