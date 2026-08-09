//! Header-row-plus-data-rows table with fixed-width columns.

use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;
use crossterm::style::Color;

/// A table with a header row and selectable data rows, fixed-width
/// columns.
pub struct Table<'a> {
    headers: &'a [String],
    rows: &'a [Vec<String>],
    selected: usize,
    col_width: u16,
}

impl<'a> Table<'a> {
    /// Creates a table over `headers`/`rows`, highlighting the data
    /// row at `selected`, each column `col_width` cells wide.
    pub fn new(
        headers: &'a [String],
        rows: &'a [Vec<String>],
        selected: usize,
        col_width: u16,
    ) -> Self {
        Table {
            headers,
            rows,
            selected,
            col_width,
        }
    }

    fn render_row(
        &self,
        area: Rect,
        y: u16,
        cells: &[String],
        fg: Color,
        bg: Color,
        buf: &mut Buffer,
    ) {
        let mut x = area.x;
        for cell in cells {
            for i in 0..self.col_width {
                if x + i >= area.x + area.width {
                    break;
                }
                let ch = cell.chars().nth(i as usize).unwrap_or(' ');
                buf.set(
                    x + i,
                    y,
                    Cell {
                        symbol: ch,
                        fg,
                        bg,
                        alpha: 1.0,
                        ..Default::default()
                    },
                );
            }
            x += self.col_width;
        }
    }

    /// Renders the header row followed by data rows, top to bottom.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 {
            return;
        }
        self.render_row(area, area.y, self.headers, Color::Reset, Color::Reset, buf);
        for (row_idx, row) in self
            .rows
            .iter()
            .take(area.height.saturating_sub(1) as usize)
            .enumerate()
        {
            let (fg, bg) = if row_idx == self.selected {
                (Color::Black, Color::White)
            } else {
                (Color::Reset, Color::Reset)
            };
            self.render_row(area, area.y + 1 + row_idx as u16, row, fg, bg, buf);
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
    fn renders_header_row_then_data_rows() {
        let headers = vec!["Name".to_string()];
        let rows = vec![vec!["svc-a".to_string()], vec!["svc-b".to_string()]];
        let mut buf = Buffer::new(10, 3);
        let area = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 3,
        };

        Table::new(&headers, &rows, 0, 5).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, 'N'); // header row
        assert_eq!(buf.get(0, 1).symbol, 's'); // first data row
    }

    #[test]
    fn selected_data_row_is_highlighted_not_the_header() {
        let headers = vec!["Name".to_string()];
        let rows = vec![vec!["svc-a".to_string()], vec!["svc-b".to_string()]];
        let mut buf = Buffer::new(10, 3);
        let area = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 3,
        };

        Table::new(&headers, &rows, 1, 5).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).bg, Color::Reset);
        assert_eq!(buf.get(0, 1).bg, Color::Reset);
        assert_eq!(buf.get(0, 2).bg, Color::White);
    }

    #[test]
    fn does_not_panic_on_zero_height_rect() {
        let headers = vec!["Name".to_string()];
        let rows = vec![vec!["svc-a".to_string()], vec!["svc-b".to_string()]];
        let mut buf = Buffer::new(10, 3);
        let area = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 0,
        };

        Table::new(&headers, &rows, 0, 5).render(area, &mut buf);
        // Should return without panicking; buffer is untouched
    }
}
