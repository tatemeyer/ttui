//! Fixture binary for `pty_roundtrip` integration tests: an app whose
//! opening draw keeps animating for a while and only then holds still —
//! the shape of every real TTUI example's boot sequence, and the case a
//! poll-counting stability criterion cannot handle.
//!
//! It draws a fixed text layout immediately, then repaints **the identical
//! glyphs at the identical positions** in a new foreground colour every
//! `TICK` for `ANIMATION`, then stops and never writes again. `TICK`
//! deliberately matches `omnitrix`'s 33ms `tick_rate()`, because that is
//! the number the old "unchanged for one poll" rule aliased against:
//! `POLL_INTERVAL` is shorter than a tick, so a consecutive poll pair
//! frequently lands entirely inside one tick gap and reads the screen as
//! "unchanged" no matter how continuously it is animating. Quiescence has
//! to hold out for a window longer than a frame period to tell a finished
//! draw from a mid-animation gap, which is what `STABLE_WINDOW` is.
//!
//! The animation is colour-only for the same reason `color_only_redraw`
//! is: it mirrors the `omnitrix` boot fade (#139), where the glyphs never
//! move and only their foreground brightens.
//!
//! After the animation the fixture blocks on `event::read()` and exits on
//! any key, so it produces no further output for quiescence to see and
//! teardown stays immediate.
use crossterm::event::{self, Event};
use crossterm::terminal;
use std::io::Write;
use std::time::{Duration, Instant};

/// Matches `omnitrix`'s `tick_rate()`. See the module comment.
const TICK: Duration = Duration::from_millis(33);

/// Long enough that a criterion which settles on a single quiet poll pair
/// is overwhelmingly likely to do so somewhere inside it (each poll pair
/// has a real chance of falling in a tick gap, and this holds ~18 of
/// them), so the test asserting we outlast the animation is reliably red
/// against that rule rather than luckily green.
const ANIMATION: Duration = Duration::from_millis(600);

/// Plain ASCII only — every glyph here is in `glyph::glyph_for`'s map.
const LAYOUT: [&str; 3] = ["BOOT SEQUENCE", "STAGE 2 OF 4", "PLEASE WAIT"];

/// Repaints `LAYOUT` from the home position in 256-colour index `idx`.
/// The text written is byte-for-byte identical every time; only the
/// colour differs.
fn paint(out: &mut impl Write, idx: u8) -> std::io::Result<()> {
    write!(out, "\x1b[H")?;
    for line in LAYOUT {
        write!(out, "\x1b[38;5;{idx}m{line}\x1b[0m\r\n")?;
    }
    out.flush()
}

fn main() -> std::io::Result<()> {
    terminal::enable_raw_mode()?;
    let mut out = std::io::stdout();

    // The 232..=255 grayscale ramp: consecutive indices are 10 apart in
    // every channel, so no two frames of this fade can collapse to the
    // same rendered pixel.
    let mut shade: u8 = 232;
    paint(&mut out, shade)?;

    let started = Instant::now();
    while started.elapsed() < ANIMATION {
        std::thread::sleep(TICK);
        shade = if shade >= 255 { 232 } else { shade + 1 };
        paint(&mut out, shade)?;
    }

    // Silent from here on.
    loop {
        if let Event::Key(_) = event::read()? {
            break;
        }
    }
    terminal::disable_raw_mode()?;
    Ok(())
}
