//! Fixture binary for `pty_roundtrip` integration tests: the case the old
//! plain-text quiescence signal was structurally unable to see.
//!
//! It draws a fixed text layout at startup and then, `REDRAW_DELAY` after
//! a key arrives, repaints **the identical glyphs at the identical
//! positions** with a different background colour. Nothing
//! `vt100::Screen::contents()` reports ever changes after the first draw,
//! so a quiescence signal comparing rendered text observes *no* reaction
//! to the key at all and rides the full `MAX_SETTLE_WAIT` before giving
//! up — while `render_screen`, which reads the full cell state, rasterizes
//! a visibly different frame. That gap is #139/#131 in miniature: the
//! `omnitrix` boot fade changes only `fg`, and `#131`'s sprites change
//! only `bg`.
//!
//! The delay is deliberately longer than two `POLL_INTERVAL` ticks so the
//! reaction cannot be picked up accidentally by two polls that merely
//! happened to straddle it. Exits immediately on Esc, without the delay,
//! so teardown isn't slowed down.
use crossterm::event::{self, Event};
use crossterm::terminal;
use std::io::Write;
use std::time::Duration;

const REDRAW_DELAY: Duration = Duration::from_millis(150);

/// The layout drawn at startup and repainted, unchanged, on the redraw.
/// Plain ASCII only — every glyph here is in `glyph::glyph_for`'s map.
const LAYOUT: [&str; 3] = ["STATUS: NOMINAL", "SIGNAL: 42", "MODE:   IDLE"];

/// Repaints `LAYOUT` from the home position, prefixing each line with
/// `sgr` (an SGR escape body, e.g. `44` for a blue background). The text
/// written is byte-for-byte identical every time; only `sgr` differs.
fn paint(out: &mut impl Write, sgr: &str) -> std::io::Result<()> {
    write!(out, "\x1b[H")?;
    for line in LAYOUT {
        write!(out, "\x1b[{sgr}m{line}\x1b[0m\r\n")?;
    }
    out.flush()
}

fn main() -> std::io::Result<()> {
    terminal::enable_raw_mode()?;
    let mut out = std::io::stdout();
    // `0` (reset) leaves every cell on the terminal's default background,
    // exactly as if no SGR had been written at all.
    paint(&mut out, "0")?;
    loop {
        if let Event::Key(key) = event::read()? {
            if key.code == event::KeyCode::Esc {
                break;
            }
            std::thread::sleep(REDRAW_DELAY);
            // Same glyphs, same positions, blue background.
            paint(&mut out, "44")?;
        }
    }
    terminal::disable_raw_mode()?;
    Ok(())
}
