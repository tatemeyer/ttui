//! Compact single-line trend indicator: one column per value, no
//! axes, auto-scaled to its own data.

use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;
use crossterm::style::Color;

const LEVELS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// A single-row sparkline — one column per value (a trailing window
/// if there are more values than `area.width`), auto-scaled to the
/// slice's own min/max.
pub struct Sparkline<'a> {
    values: &'a [f32],
    color: Color,
}

impl<'a> Sparkline<'a> {
    /// Creates a sparkline over `values`.
    pub fn new(values: &'a [f32], color: Color) -> Self {
        Sparkline { values, color }
    }

    /// Renders the trailing `area.width` values as one row of
    /// height-coded block glyphs at `area.y` — always exactly one
    /// row, regardless of `area.height`. When there are fewer values
    /// than `area.width`, they render left-anchored starting at
    /// `area.x`, leaving the remaining columns empty until enough
    /// values accumulate.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 || self.values.is_empty() {
            return;
        }
        let window_len = (area.width as usize).min(self.values.len());
        let window = &self.values[self.values.len() - window_len..];
        let min = window.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = window.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        for (i, &value) in window.iter().enumerate() {
            let level = if max > min {
                (((value - min) / (max - min)) * (LEVELS.len() - 1) as f32).round() as usize
            } else {
                LEVELS.len() / 2
            };
            let symbol = LEVELS[level.min(LEVELS.len() - 1)];
            buf.set(
                area.x + i as u16,
                area.y,
                Cell {
                    symbol,
                    fg: self.color,
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
    use crate::buffer::{Buffer, Cell};

    fn area(width: u16, height: u16) -> Rect {
        Rect {
            x: 0,
            y: 0,
            width,
            height,
        }
    }

    #[test]
    fn higher_value_renders_a_taller_glyph_than_a_lower_one() {
        let values = [0.0, 100.0];
        let mut buf = Buffer::new(2, 1);
        Sparkline::new(&values, Color::Reset).render(area(2, 1), &mut buf);
        assert_eq!(buf.get(0, 0).symbol, LEVELS[0]);
        assert_eq!(buf.get(1, 0).symbol, LEVELS[7]);
    }

    #[test]
    fn flat_data_renders_the_middle_level_glyph_without_panic() {
        let values = [42.0, 42.0, 42.0];
        let mut buf = Buffer::new(3, 1);
        Sparkline::new(&values, Color::Reset).render(area(3, 1), &mut buf);
        for x in 0..3 {
            assert_eq!(buf.get(x, 0).symbol, LEVELS[4]);
        }
    }

    #[test]
    fn more_values_than_area_width_shows_only_the_trailing_window() {
        let values = [0.0, 100.0, 50.0];
        let mut buf = Buffer::new(2, 1);
        Sparkline::new(&values, Color::Reset).render(area(2, 1), &mut buf);
        // trailing window = last 2 values [100.0, 50.0]; min=50, max=100
        assert_eq!(buf.get(0, 0).symbol, LEVELS[7]);
        assert_eq!(buf.get(1, 0).symbol, LEVELS[0]);
    }

    #[test]
    fn fewer_values_than_area_width_renders_only_that_many_columns() {
        let values = [10.0, 20.0];
        let mut buf = Buffer::new(5, 1);
        Sparkline::new(&values, Color::Reset).render(area(5, 1), &mut buf);
        assert_eq!(buf.get(0, 0).symbol, LEVELS[0]);
        assert_eq!(buf.get(1, 0).symbol, LEVELS[7]);
        assert_eq!(buf.get(2, 0).symbol, ' ');
    }

    #[test]
    fn an_intermediate_value_renders_the_correctly_scaled_middle_level() {
        let values = [0.0, 75.0, 100.0];
        let mut buf = Buffer::new(3, 1);
        Sparkline::new(&values, Color::Reset).render(area(3, 1), &mut buf);
        // (75-0)/(100-0) * 7 = 5.25 -> rounds to level 5, not level 6.
        assert_eq!(buf.get(1, 0).symbol, LEVELS[5]);
    }

    #[test]
    fn empty_values_renders_nothing_without_panic() {
        let values: [f32; 0] = [];
        let mut buf = Buffer::new(5, 1);
        Sparkline::new(&values, Color::Reset).render(area(5, 1), &mut buf);
        assert_eq!(*buf.get(0, 0), Cell::default());
    }

    #[test]
    fn renders_on_exactly_one_row_regardless_of_area_height() {
        let values = [10.0, 20.0];
        let mut buf = Buffer::new(5, 3);
        Sparkline::new(&values, Color::Reset).render(area(5, 3), &mut buf);
        assert_ne!(buf.get(0, 0).symbol, ' ');
        assert_eq!(buf.get(0, 1).symbol, ' ');
        assert_eq!(buf.get(0, 2).symbol, ' ');
    }

    #[test]
    fn zero_width_or_zero_height_area_renders_nothing_without_panic() {
        let values = [10.0, 20.0];
        let mut buf = Buffer::new(5, 5);
        Sparkline::new(&values, Color::Reset).render(
            Rect {
                x: 0,
                y: 0,
                width: 0,
                height: 3,
            },
            &mut buf,
        );
        Sparkline::new(&values, Color::Reset).render(
            Rect {
                x: 0,
                y: 0,
                width: 3,
                height: 0,
            },
            &mut buf,
        );
        assert_eq!(*buf.get(0, 0), Cell::default());
    }
}
