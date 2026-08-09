//! Scrollable selectable list, one item per row.

use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;
use crossterm::style::Color;

/// A vertical list of items with one highlighted selection.
pub struct List<'a> {
    items: &'a [String],
    selected: usize,
}

impl<'a> List<'a> {
    /// Creates a list over `items`, highlighting the one at `selected`.
    pub fn new(items: &'a [String], selected: usize) -> Self {
        List { items, selected }
    }

    /// Renders up to `area.height` items as rows, top to bottom.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        for (row, item) in self.items.iter().take(area.height as usize).enumerate() {
            let (fg, bg) = if row == self.selected {
                (Color::Black, Color::White)
            } else {
                (Color::Reset, Color::Reset)
            };
            for x in 0..area.width {
                buf.set(
                    area.x + x,
                    area.y + row as u16,
                    Cell {
                        symbol: ' ',
                        fg,
                        bg,
                        alpha: 1.0,
                        ..Default::default()
                    },
                );
            }
            for (i, ch) in item.chars().take(area.width as usize).enumerate() {
                buf.set(
                    area.x + i as u16,
                    area.y + row as u16,
                    Cell {
                        symbol: ch,
                        fg,
                        bg,
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
    use crate::buffer::Buffer;
    use crate::layout::Rect;
    use crossterm::style::Color;

    #[test]
    fn renders_each_item_on_its_own_row() {
        let items = vec!["one".to_string(), "two".to_string()];
        let mut buf = Buffer::new(5, 2);
        let area = Rect {
            x: 0,
            y: 0,
            width: 5,
            height: 2,
        };

        List::new(&items, 0).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, 'o');
        assert_eq!(buf.get(0, 1).symbol, 't');
    }

    #[test]
    fn selected_row_is_highlighted() {
        let items = vec!["one".to_string(), "two".to_string()];
        let mut buf = Buffer::new(5, 2);
        let area = Rect {
            x: 0,
            y: 0,
            width: 5,
            height: 2,
        };

        List::new(&items, 1).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).bg, Color::Reset);
        assert_eq!(buf.get(0, 1).bg, Color::White);
    }
}
