//! Raw-mode terminal setup/teardown: entering the alternate screen,
//! diff-based redraws, input polling, and panic-safe cleanup.

use std::io::{stdout, BufWriter, Stdout, Write};
use std::time::Duration;

use crossterm::event::{self, Event};
use crossterm::style::{
    Attribute, Color, Print, SetAttribute, SetBackgroundColor, SetForegroundColor,
};
use crossterm::{cursor, execute, queue, terminal};

use crate::buffer::CellDiff;

/// A raw-mode, alternate-screen terminal handle. Restores normal
/// terminal state automatically on drop.
pub struct Terminal {
    out: BufWriter<Stdout>,
}

impl Terminal {
    /// Enables raw mode, enters the alternate screen, and hides the
    /// cursor.
    pub fn new() -> std::io::Result<Self> {
        terminal::enable_raw_mode()?;
        let mut out = BufWriter::new(stdout());
        execute!(out, terminal::EnterAlternateScreen, cursor::Hide)?;
        Ok(Terminal { out })
    }

    /// Current terminal size in cells, `(width, height)`.
    pub fn size(&self) -> std::io::Result<(u16, u16)> {
        terminal::size()
    }

    /// Writes only the given changed cells to the terminal, then
    /// flushes once. Thin wrapper over [`render_diff`].
    pub fn draw_diff(&mut self, diffs: &[CellDiff]) -> std::io::Result<()> {
        render_diff(&mut self.out, diffs)?;
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

/// Encodes `diffs` as terminal control sequences into `writer`,
/// coalescing redundant cursor moves and SGR changes; does not flush.
/// The lower-level primitive [`Terminal::draw_diff`] wraps with a
/// buffered stdout and a single flush.
pub fn render_diff(writer: &mut impl Write, diffs: &[CellDiff]) -> std::io::Result<()> {
    let mut last_pos: Option<(u16, u16)> = None;
    let mut last_fg: Option<Color> = None;
    let mut last_bg: Option<Color> = None;
    let mut last_bold: Option<bool> = None;

    for d in diffs {
        // Move only when this cell isn't the previous cell's right
        // neighbor — after Print the cursor already sits there. A run
        // can't cross a row end, so autowrap is never relied upon.
        let contiguous =
            matches!(last_pos, Some((px, py)) if py == d.y && d.x.checked_sub(1) == Some(px));
        if !contiguous {
            queue!(writer, cursor::MoveTo(d.x, d.y))?;
        }

        // NormalIntensity (not a full SGR reset) clears bold without
        // touching color, so fg/bg can be tracked independently.
        let bold = d.cell.style.bold;
        if last_bold != Some(bold) {
            let attr = if bold {
                Attribute::Bold
            } else {
                Attribute::NormalIntensity
            };
            queue!(writer, SetAttribute(attr))?;
            last_bold = Some(bold);
        }
        if last_fg != Some(d.cell.fg) {
            queue!(writer, SetForegroundColor(d.cell.fg))?;
            last_fg = Some(d.cell.fg);
        }
        if last_bg != Some(d.cell.bg) {
            queue!(writer, SetBackgroundColor(d.cell.bg))?;
            last_bg = Some(d.cell.bg);
        }
        queue!(writer, Print(d.cell.symbol))?;
        last_pos = Some((d.x, d.y));
    }
    Ok(())
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
mod render_diff_tests {
    use super::*;
    use crate::buffer::{Cell, CellDiff, CellStyle};
    use crossterm::style::Color;

    fn d(x: u16, y: u16, symbol: char, fg: Color, bg: Color, bold: bool) -> CellDiff {
        CellDiff {
            x,
            y,
            cell: Cell {
                symbol,
                fg,
                bg,
                style: CellStyle { bold },
            },
        }
    }

    fn render(diffs: &[CellDiff]) -> Vec<u8> {
        let mut buf = Vec::new();
        render_diff(&mut buf, diffs).unwrap();
        buf
    }

    /// Bytes crossterm emits for a single command — lets assertions
    /// count real control sequences without hard-coding ANSI codes.
    fn encode<C: crossterm::Command>(cmd: C) -> Vec<u8> {
        let mut v = Vec::new();
        crossterm::queue!(&mut v, cmd).unwrap();
        v
    }

    fn count(hay: &[u8], needle: &[u8]) -> usize {
        if needle.is_empty() || needle.len() > hay.len() {
            return 0;
        }
        hay.windows(needle.len()).filter(|w| *w == needle).count()
    }

    // Number of MoveTo commands == number of CSI cursor-position
    // terminators ('H'); test glyphs are never 'H'.
    fn move_count(out: &[u8]) -> usize {
        count(out, b"H")
    }

    #[test]
    fn empty_diffs_produce_no_output() {
        assert!(render(&[]).is_empty());
    }

    #[test]
    fn single_diff_emits_move_colors_intensity_and_glyph() {
        let out = render(&[d(3, 2, 'A', Color::Reset, Color::Reset, false)]);
        assert_eq!(move_count(&out), 1);
        assert!(out.contains(&b'A'));
        assert_eq!(count(&out, &encode(SetForegroundColor(Color::Reset))), 1);
        assert_eq!(count(&out, &encode(SetBackgroundColor(Color::Reset))), 1);
        assert_eq!(
            count(&out, &encode(SetAttribute(Attribute::NormalIntensity))),
            1
        );
    }

    #[test]
    fn contiguous_same_styled_run_moves_once_and_sets_style_once() {
        let out = render(&[
            d(3, 2, 'A', Color::Reset, Color::Reset, false),
            d(4, 2, 'B', Color::Reset, Color::Reset, false),
        ]);
        assert_eq!(move_count(&out), 1, "contiguous run needs one MoveTo");
        assert_eq!(count(&out, &encode(SetForegroundColor(Color::Reset))), 1);
        assert_eq!(count(&out, &encode(SetBackgroundColor(Color::Reset))), 1);
        assert_eq!(
            count(&out, &encode(SetAttribute(Attribute::NormalIntensity))),
            1
        );
        assert!(out.contains(&b'A') && out.contains(&b'B'));
    }

    #[test]
    fn positional_gap_forces_a_second_move() {
        let out = render(&[
            d(3, 2, 'A', Color::Reset, Color::Reset, false),
            d(6, 2, 'B', Color::Reset, Color::Reset, false),
        ]);
        assert_eq!(move_count(&out), 2);
    }

    #[test]
    fn new_row_forces_a_second_move() {
        let out = render(&[
            d(3, 2, 'A', Color::Reset, Color::Reset, false),
            d(4, 3, 'B', Color::Reset, Color::Reset, false),
        ]);
        assert_eq!(move_count(&out), 2);
    }

    #[test]
    fn color_change_mid_run_re_emits_that_color_only() {
        let out = render(&[
            d(3, 2, 'A', Color::Reset, Color::Reset, false),
            d(4, 2, 'B', Color::Red, Color::Reset, false),
        ]);
        assert_eq!(move_count(&out), 1, "still contiguous");
        assert_eq!(count(&out, &encode(SetForegroundColor(Color::Reset))), 1);
        assert_eq!(count(&out, &encode(SetForegroundColor(Color::Red))), 1);
        assert_eq!(
            count(&out, &encode(SetBackgroundColor(Color::Reset))),
            1,
            "bg unchanged, emitted once"
        );
    }

    #[test]
    fn bold_toggle_emits_intensity_transitions() {
        let out = render(&[
            d(3, 2, 'A', Color::Reset, Color::Reset, false),
            d(4, 2, 'B', Color::Reset, Color::Reset, true),
            d(5, 2, 'C', Color::Reset, Color::Reset, false),
        ]);
        assert_eq!(count(&out, &encode(SetAttribute(Attribute::Bold))), 1);
        assert_eq!(
            count(&out, &encode(SetAttribute(Attribute::NormalIntensity))),
            2,
            "first cell + bold-off transition"
        );
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
