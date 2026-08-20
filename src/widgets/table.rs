//! Header-row-plus-data-rows table with fixed-width columns.

use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;
use crate::theme::Theme;
use crate::widgets::selection::selection_colors;
use crossterm::style::Color;
use unicode_width::UnicodeWidthChar;

/// A table with a header row and selectable data rows, fixed-width
/// columns.
pub struct Table<'a> {
    headers: &'a [String],
    rows: &'a [Vec<String>],
    selected: usize,
    col_width: u16,
    theme: Option<&'a Theme>,
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
            theme: None,
        }
    }

    /// Renders the selected row with `theme`'s `accent` on
    /// `background`, unselected rows in `primary`, and the header in
    /// `secondary`. Without it, the pre-2.0 fixed black-on-white
    /// selection highlight is used and the header renders in
    /// `Color::Reset`.
    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
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
        let header_fg = self.theme.map_or(Color::Reset, |t| t.secondary);
        self.render_row(area, area.y, self.headers, header_fg, Color::Reset, buf);
        for (row_idx, row) in self
            .rows
            .iter()
            .take(area.height.saturating_sub(1) as usize)
            .enumerate()
        {
            let (fg, bg) = selection_colors(self.theme, row_idx == self.selected);
            self.render_row(area, area.y + 1 + row_idx as u16, row, fg, bg, buf);
        }
    }
}

/// Fits `cell` into `width` display cells, appending U+2026 when it is
/// cut. Measures display width rather than `char` count, so wide glyphs
/// (CJK) do not misalign the columns after them, and never splits a
/// wide glyph across the boundary.
// Not yet called from `render_row` -- Task 12 wires it in. Kept private
// (no undesigned API surface) with `dead_code` allowed until then.
#[allow(dead_code)]
fn fit(cell: &str, width: u16) -> String {
    let width = width as usize;
    if width == 0 {
        return String::new();
    }

    let total: usize = cell.chars().map(|c| c.width().unwrap_or(0)).sum();
    if total <= width {
        return cell.to_string();
    }

    // Try reserving one cell for the marker. Only meaningful when
    // width > 1 -- at width 1 a lone marker carries no information, so
    // that case always spends the cell on content instead (below).
    if width > 1 {
        let budget = width - 1;
        let mut out = String::new();
        let mut used = 0usize;
        for c in cell.chars() {
            let w = c.width().unwrap_or(0);
            if used + w > budget {
                break;
            }
            out.push(c);
            used += w;
        }
        if !out.is_empty() {
            out.push('\u{2026}');
            return out;
        }
        // Reserving a cell for the marker left no room for even the
        // first glyph (it's wider than the budget) -- fall through and
        // fill the full width with content only, no marker, rather
        // than emit a marker with nothing in front of it.
    }

    let mut out = String::new();
    let mut used = 0usize;
    for c in cell.chars() {
        let w = c.width().unwrap_or(0);
        if used + w > width {
            break;
        }
        out.push(c);
        used += w;
    }
    out
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

    #[test]
    fn themed_table_colours_header_selected_and_unselected_rows() {
        use crate::widgets::test_support::test_theme;

        let headers = vec!["Name".to_string()];
        let rows = vec![vec!["svc-a".to_string()], vec!["svc-b".to_string()]];
        let mut buf = Buffer::new(10, 3);
        let area = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 3,
        };
        let theme = test_theme();

        Table::new(&headers, &rows, 0, 5)
            .theme(&theme)
            .render(area, &mut buf);

        assert_eq!(buf.get(0, 0).fg, theme.secondary); // header
        assert_eq!(buf.get(0, 1).fg, theme.accent); // selected
        assert_eq!(buf.get(0, 1).bg, theme.background);
        assert_eq!(buf.get(0, 2).fg, theme.primary); // unselected
    }

    #[test]
    fn fit_returns_short_content_untouched() {
        assert_eq!(fit("ok", 5), "ok");
    }

    #[test]
    fn fit_truncates_with_an_ellipsis_when_content_overflows() {
        // 11 display cells of content into 6 -> 5 kept plus the marker.
        assert_eq!(fit("tardis-idle", 6), "tardi…");
    }

    #[test]
    fn fit_into_one_cell_truncates_without_a_lone_marker() {
        // A bare "…" carries no information; prefer the first character.
        assert_eq!(fit("tardis", 1), "t");
    }

    #[test]
    fn fit_measures_display_width_not_char_count() {
        // "東京" is 2 chars but occupies 4 cells; into 3 cells only the
        // first wide glyph plus the marker fit.
        assert_eq!(fit("東京", 3), "東…");
    }

    #[test]
    fn fit_never_splits_a_wide_glyph_across_the_boundary() {
        // Into 2 cells, "東" alone exactly fills it; the marker would
        // overflow, so it is dropped rather than cutting the glyph.
        assert_eq!(fit("東京", 2), "東");
    }

    #[test]
    fn fit_to_zero_width_is_empty() {
        assert_eq!(fit("anything", 0), "");
    }

    #[test]
    fn unthemed_table_keeps_the_pre_2_0_colours() {
        // Characterisation test: no `.theme()` must render exactly as 1.x
        // did. Worthless if written after the change.
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

        assert_eq!(buf.get(0, 0).fg, Color::Reset); // header
        assert_eq!(buf.get(0, 1).fg, Color::Black); // selected
        assert_eq!(buf.get(0, 1).bg, Color::White);
        assert_eq!(buf.get(0, 2).fg, Color::Reset); // unselected
    }
}
