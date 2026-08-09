//! Alternating two-color text row with a trailing cursor glyph, for a
//! console/typewriter feel.

use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;
use crossterm::style::Color;

/// A text row whose characters alternate between two colors, with a
/// trailing `▌` cursor.
pub struct DNAConsole<'a> {
    content: &'a str,
    primary: Color,
    secondary: Color,
}

impl<'a> DNAConsole<'a> {
    /// Creates a console row over `content`, alternating `primary`/
    /// `secondary` per character.
    pub fn new(content: &'a str, primary: Color, secondary: Color) -> Self {
        DNAConsole {
            content,
            primary,
            secondary,
        }
    }

    /// Renders the alternating-color text plus trailing cursor.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 {
            return;
        }
        let max_content = area.width.saturating_sub(1) as usize;
        let mut count: u16 = 0;
        for (i, ch) in self.content.chars().take(max_content).enumerate() {
            let color = if i % 2 == 0 {
                self.primary
            } else {
                self.secondary
            };
            buf.set(
                area.x + i as u16,
                area.y,
                Cell {
                    symbol: ch,
                    fg: color,
                    bg: Color::Reset,
                    alpha: 1.0,
                    ..Default::default()
                },
            );
            count = i as u16 + 1;
        }
        if count < area.width {
            buf.set(
                area.x + count,
                area.y,
                Cell {
                    symbol: '▌',
                    fg: self.primary,
                    bg: Color::Reset,
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

    #[test]
    fn alternates_colors_per_character_with_a_trailing_cursor() {
        let mut buf = Buffer::new(10, 1);
        let area = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 1,
        };

        DNAConsole::new("AB", Color::Red, Color::Blue).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, 'A');
        assert_eq!(buf.get(0, 0).fg, Color::Red);
        assert_eq!(buf.get(1, 0).symbol, 'B');
        assert_eq!(buf.get(1, 0).fg, Color::Blue);
        assert_eq!(buf.get(2, 0).symbol, '▌');
        assert_eq!(buf.get(2, 0).fg, Color::Red);
    }

    #[test]
    fn zero_width_area_renders_nothing_without_panic() {
        let mut buf = Buffer::new(1, 1);
        let area = Rect {
            x: 0,
            y: 0,
            width: 0,
            height: 1,
        };
        DNAConsole::new("A", Color::Red, Color::Blue).render(area, &mut buf);
    }

    #[test]
    fn one_wide_area_renders_only_the_cursor() {
        let mut buf = Buffer::new(1, 1);
        let area = Rect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        };

        DNAConsole::new("AB", Color::Red, Color::Blue).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, '▌');
        assert_eq!(buf.get(0, 0).fg, Color::Red);
    }
}
