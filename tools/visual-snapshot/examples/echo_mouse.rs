//! Minimal fixture binary for click-scripting integration tests:
//! echoes the debug representation of each mouse event it receives,
//! exits on Esc key.
use crossterm::event::{self, Event};
use crossterm::terminal;
use std::io::Write;

fn main() -> std::io::Result<()> {
    terminal::enable_raw_mode()?;
    crossterm::execute!(std::io::stdout(), event::EnableMouseCapture)?;
    let mut out = std::io::stdout();
    loop {
        match event::read()? {
            Event::Mouse(m) => {
                write!(out, "{:?}", m.kind)?;
                out.flush()?;
            }
            Event::Key(key) if key.code == event::KeyCode::Esc => break,
            _ => {}
        }
    }
    crossterm::execute!(std::io::stdout(), event::DisableMouseCapture)?;
    terminal::disable_raw_mode()?;
    Ok(())
}
