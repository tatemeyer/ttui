//! Two-position analog toggle switch, rendered as a bracketed slash.

use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;

/// A two-position switch, on (`[ / ]`) or off (`[ \ ]`).
pub struct AnalogToggle {
    on: bool,
}

impl AnalogToggle {
    /// Creates a toggle in the given state.
    pub fn new(on: bool) -> Self {
        AnalogToggle { on }
    }

    /// Renders the toggle glyphs starting at `area`'s top-left.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let text = if self.on { "[ / ]" } else { "[ \\ ]" };
        for (i, ch) in text.chars().take(area.width as usize).enumerate() {
            buf.set(
                area.x + i as u16,
                area.y,
                Cell {
                    symbol: ch,
                    alpha: 1.0,
                    ..Default::default()
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area5x1() -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: 5,
            height: 1,
        }
    }

    #[test]
    fn off_renders_a_backslash_lever() {
        let mut buf = Buffer::new(5, 1);
        AnalogToggle::new(false).render(area5x1(), &mut buf);

        let expected = ['[', ' ', '\\', ' ', ']'];
        for (i, ch) in expected.iter().enumerate() {
            assert_eq!(buf.get(i as u16, 0).symbol, *ch);
        }
    }

    #[test]
    fn on_renders_a_forward_slash_lever() {
        let mut buf = Buffer::new(5, 1);
        AnalogToggle::new(true).render(area5x1(), &mut buf);

        let expected = ['[', ' ', '/', ' ', ']'];
        for (i, ch) in expected.iter().enumerate() {
            assert_eq!(buf.get(i as u16, 0).symbol, *ch);
        }
    }

    #[test]
    fn narrower_than_five_clips_without_panic() {
        let mut buf = Buffer::new(2, 1);
        let area = Rect {
            x: 0,
            y: 0,
            width: 2,
            height: 1,
        };

        AnalogToggle::new(false).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, '[');
        assert_eq!(buf.get(1, 0).symbol, ' ');
    }
}
