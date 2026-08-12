//! Horizontal bar chart: one labeled row per item, bar length scaled
//! to a shared maximum, all bars left-aligned at a common column.

use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;
use crossterm::style::Color;

/// A horizontal bar chart — one row per `(label, value)` pair, bar
/// length scaled against `max`.
pub struct BarChart<'a> {
    items: &'a [(&'a str, f32)],
    max: f32,
    color: Color,
}

impl<'a> BarChart<'a> {
    /// Creates a chart over `items`, with bar lengths scaled against
    /// `max` (a value exceeding `max` draws a full-width bar — not
    /// clamped as an error, just visually capped).
    pub fn new(items: &'a [(&'a str, f32)], max: f32, color: Color) -> Self {
        BarChart { items, max, color }
    }

    /// Renders one row per item (truncated to `area.height` rows).
    /// Each label is drawn as-is (not padded) and clipped at the
    /// render area's right edge; every bar's starting column is fixed
    /// at the longest label's character-width plus one space,
    /// regardless of that row's own label length, then filled with
    /// `█` cells proportional to `value / max`.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let label_width = self
            .items
            .iter()
            .map(|(l, _)| l.chars().count())
            .max()
            .unwrap_or(0) as u16;
        for (row, (label, value)) in self.items.iter().take(area.height as usize).enumerate() {
            let y = area.y + row as u16;
            let mut x = area.x;
            for (i, ch) in label.chars().enumerate() {
                // `i as u16 >= label_width` is a defensive guard: label_width is the max
                // char-count across all items, so no single label can exceed it in practice.
                if i as u16 >= label_width || x >= area.x + area.width {
                    break;
                }
                buf.set(
                    x,
                    y,
                    Cell {
                        symbol: ch,
                        fg: self.color,
                        bg: Color::Reset,
                        alpha: 1.0,
                        ..Default::default()
                    },
                );
                x += 1;
            }
            x = (area.x + label_width + 1).min(area.x + area.width);
            let bar_space = (area.x + area.width).saturating_sub(x);
            let fraction = if self.max > 0.0 {
                (value / self.max).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let filled = (fraction * bar_space as f32).round() as u16;
            for i in 0..filled.min(bar_space) {
                buf.set(
                    x + i,
                    y,
                    Cell {
                        symbol: '█',
                        fg: self.color,
                        bg: Color::Reset,
                        alpha: 1.0,
                        ..Default::default()
                    },
                );
            }
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
    fn bar_length_is_proportional_to_value_over_max() {
        let items = [("A", 50.0)];
        let mut buf = Buffer::new(20, 1);
        BarChart::new(&items, 100.0, Color::Reset).render(area(20, 1), &mut buf);
        // label "A" (1 char) + 1 space -> bar starts at x=2, bar_space=18
        // fraction 0.5 -> filled = round(0.5*18) = 9
        for i in 0..9 {
            assert_eq!(
                buf.get(2 + i, 0).symbol,
                '█',
                "expected filled at offset {i}"
            );
        }
        assert_ne!(buf.get(2 + 9, 0).symbol, '█');
    }

    #[test]
    fn value_equal_to_max_fills_full_bar_width() {
        let items = [("X", 100.0)];
        let mut buf = Buffer::new(10, 1);
        BarChart::new(&items, 100.0, Color::Reset).render(area(10, 1), &mut buf);
        // label "X" (1) + 1 space -> bar starts at x=2, bar_space=8
        for i in 0..8 {
            assert_eq!(buf.get(2 + i, 0).symbol, '█');
        }
    }

    #[test]
    fn value_exceeding_max_still_fills_only_the_full_bar_width() {
        let items = [("X", 999.0)];
        let mut buf = Buffer::new(10, 1);
        BarChart::new(&items, 100.0, Color::Reset).render(area(10, 1), &mut buf);
        for i in 0..8 {
            assert_eq!(buf.get(2 + i, 0).symbol, '█');
        }
    }

    #[test]
    fn bars_align_to_the_longest_label_across_items() {
        let items = [("A", 50.0), ("Longer", 50.0)];
        let mut buf = Buffer::new(30, 2);
        BarChart::new(&items, 100.0, Color::Reset).render(area(30, 2), &mut buf);
        // "Longer" is 6 chars -> label_width=6 -> both bars start at x=7
        assert_eq!(buf.get(0, 0).symbol, 'A');
        assert_eq!(buf.get(7, 0).symbol, '█');
        assert_eq!(buf.get(7, 1).symbol, '█');
        assert_ne!(
            buf.get(6, 1).symbol,
            '█',
            "column 6 is the separator space, not part of the bar"
        );
    }

    #[test]
    fn more_items_than_area_height_truncates_without_panic() {
        let items = [("A", 10.0), ("B", 10.0), ("C", 10.0)];
        let mut buf = Buffer::new(10, 2);
        BarChart::new(&items, 100.0, Color::Reset).render(area(10, 2), &mut buf);
        assert_eq!(buf.get(0, 0).symbol, 'A');
        assert_eq!(buf.get(0, 1).symbol, 'B');
    }

    #[test]
    fn zero_width_or_zero_height_area_renders_nothing_without_panic() {
        let items = [("A", 50.0)];
        let mut buf = Buffer::new(10, 10);
        BarChart::new(&items, 100.0, Color::Reset).render(
            Rect {
                x: 0,
                y: 0,
                width: 0,
                height: 5,
            },
            &mut buf,
        );
        BarChart::new(&items, 100.0, Color::Reset).render(
            Rect {
                x: 0,
                y: 0,
                width: 5,
                height: 0,
            },
            &mut buf,
        );
        assert_eq!(*buf.get(0, 0), Cell::default());
    }

    #[test]
    fn non_exact_fractions_round_to_the_nearest_whole_cell() {
        // label "X" (1 char) + 1 space -> bar starts at x=2, bar_space=10.
        //
        // Note: a value landing exactly on a .5 boundary (e.g. 3.5) can't
        // distinguish `.round()` from `.ceil()`, since Rust's f32::round()
        // rounds half-away-from-zero: round(3.5) == ceil(3.5) == 4. So this
        // uses two off-boundary fractions, one rounding down and one
        // rounding up, to pin the rounding mode against both `.ceil()` and
        // `.floor()`.

        // round-down case: 0.33*10 = 3.3 -> round = 3; ceil would give 4.
        let items_down = [("X", 3.3)];
        let mut buf_down = Buffer::new(12, 1);
        BarChart::new(&items_down, 10.0, Color::Reset).render(area(12, 1), &mut buf_down);
        for i in 0..3 {
            assert_eq!(
                buf_down.get(2 + i, 0).symbol,
                '█',
                "expected filled at offset {i}"
            );
        }
        assert_ne!(
            buf_down.get(2 + 3, 0).symbol,
            '█',
            "round(3.3)=3, not 4 (ceil)"
        );

        // round-up case: 0.37*10 = 3.7 -> round = 4; floor would give 3.
        let items_up = [("X", 3.7)];
        let mut buf_up = Buffer::new(12, 1);
        BarChart::new(&items_up, 10.0, Color::Reset).render(area(12, 1), &mut buf_up);
        for i in 0..4 {
            assert_eq!(
                buf_up.get(2 + i, 0).symbol,
                '█',
                "expected filled at offset {i}"
            );
        }
        assert_ne!(
            buf_up.get(2 + 4, 0).symbol,
            '█',
            "round(3.7)=4, not 3 (floor)"
        );
    }

    #[test]
    fn max_of_zero_renders_empty_bars_without_panic() {
        let items = [("A", 50.0)];
        let mut buf = Buffer::new(10, 1);
        BarChart::new(&items, 0.0, Color::Reset).render(area(10, 1), &mut buf);
        for x in 2..10 {
            assert_ne!(buf.get(x, 0).symbol, '█');
        }
    }
}
