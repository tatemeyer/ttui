//! Scrollable selectable list, one item per row.

use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;
use crate::theme::Theme;
use crate::widgets::selection::selection_colors;

/// A vertical list of items with one highlighted selection.
pub struct List<'a> {
    items: &'a [String],
    selected: usize,
    theme: Option<&'a Theme>,
}

impl<'a> List<'a> {
    /// Creates a list over `items`, highlighting the one at `selected`.
    pub fn new(items: &'a [String], selected: usize) -> Self {
        List {
            items,
            selected,
            theme: None,
        }
    }

    /// Renders selection with `theme`'s `accent` on `background`, and
    /// unselected rows in `primary`. Without it, the pre-2.0 fixed
    /// black-on-white highlight is used.
    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Renders up to `area.height` items as rows, top to bottom.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        for (row, item) in self.items.iter().take(area.height as usize).enumerate() {
            let (fg, bg) = selection_colors(self.theme, row == self.selected);
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
    use crate::widgets::test_support::test_theme;
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

    #[test]
    fn themed_list_uses_accent_on_background_for_the_selected_row() {
        use crate::theme::Theme;

        let items = vec!["alpha".to_string(), "beta".to_string()];
        let mut buf = Buffer::new(10, 2);
        let area = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 2,
        };
        let theme = Theme {
            accent: Color::Rgb { r: 255, g: 0, b: 0 },
            background: Color::Rgb { r: 0, g: 0, b: 32 },
            primary: Color::Rgb { r: 0, g: 255, b: 0 },
            ..test_theme()
        };

        List::new(&items, 1).theme(&theme).render(area, &mut buf);

        // selected row (index 1) -> accent on background
        assert_eq!(buf.get(0, 1).fg, Color::Rgb { r: 255, g: 0, b: 0 });
        assert_eq!(buf.get(0, 1).bg, Color::Rgb { r: 0, g: 0, b: 32 });
        // unselected row -> primary
        assert_eq!(buf.get(0, 0).fg, Color::Rgb { r: 0, g: 255, b: 0 });
    }

    #[test]
    fn unthemed_list_keeps_the_pre_2_0_colours() {
        // Characterisation test: no `.theme()` must render exactly as 1.x
        // did. Worthless if written after the change.
        let items = vec!["alpha".to_string(), "beta".to_string()];
        let mut buf = Buffer::new(10, 2);
        let area = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 2,
        };

        List::new(&items, 1).render(area, &mut buf);

        assert_eq!(buf.get(0, 1).fg, Color::Black);
        assert_eq!(buf.get(0, 1).bg, Color::White);
        assert_eq!(buf.get(0, 0).fg, Color::Reset);
        assert_eq!(buf.get(0, 0).bg, Color::Reset);
    }
}
