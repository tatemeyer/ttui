//! Segmented circular progress ring.

use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;
use crossterm::style::Color;

/// A horizontal segmented progress bar filled to `percent` in
/// `color`.
pub struct EnergyCore {
    percent: u16,
    color: Color,
}

impl EnergyCore {
    /// Creates a bar filled to `percent` (0-100) in `color`.
    pub fn new(percent: u16, color: Color) -> Self {
        EnergyCore { percent, color }
    }

    /// Renders the filled/empty segments across `area`'s width.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let filled_width = (area.width as u32 * self.percent.min(100) as u32 / 100) as u16;
        for x in 0..area.width {
            let filled = x < filled_width;
            let spark = self.percent >= 100 && filled && x % 4 == 0;
            let (symbol, fg) = if spark {
                ('✦', Color::White)
            } else if filled {
                ('▓', self.color)
            } else {
                ('░', self.color)
            };
            buf.set(
                area.x + x,
                area.y,
                Cell {
                    symbol,
                    fg,
                    bg: Color::Reset,
                    ..Default::default()
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area10x1() -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 1,
        }
    }

    #[test]
    fn zero_percent_renders_all_empty_track() {
        let mut buf = Buffer::new(10, 1);
        EnergyCore::new(0, Color::Green).render(area10x1(), &mut buf);

        for x in 0..10 {
            assert_eq!(buf.get(x, 0).symbol, '░');
            assert_eq!(buf.get(x, 0).fg, Color::Green);
        }
    }

    #[test]
    fn fifty_percent_fills_half() {
        let mut buf = Buffer::new(10, 1);
        EnergyCore::new(50, Color::Green).render(area10x1(), &mut buf);

        for x in 0..5 {
            assert_eq!(buf.get(x, 0).symbol, '▓');
        }
        for x in 5..10 {
            assert_eq!(buf.get(x, 0).symbol, '░');
        }
    }

    #[test]
    fn full_percent_sparks_every_fourth_cell() {
        let mut buf = Buffer::new(8, 1);
        let area = Rect {
            x: 0,
            y: 0,
            width: 8,
            height: 1,
        };
        EnergyCore::new(100, Color::Green).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, '✦');
        assert_eq!(buf.get(0, 0).fg, Color::White);
        assert_eq!(buf.get(1, 0).symbol, '▓');
        assert_eq!(buf.get(4, 0).symbol, '✦');
        assert_eq!(buf.get(5, 0).symbol, '▓');
    }

    #[test]
    fn zero_width_area_does_not_panic() {
        let mut buf = Buffer::new(1, 1);
        let area = Rect {
            x: 0,
            y: 0,
            width: 0,
            height: 1,
        };
        EnergyCore::new(50, Color::Green).render(area, &mut buf);
    }
}
