//! Header-row-plus-data-rows table. Column geometry is delegated to
//! [`Layout`] rather than reimplemented here: [`Table::widths`] takes
//! the same [`Constraint`] vocabulary `Layout` splits areas with, so a
//! `Fill(1)` column adapts to terminal width instead of needing one
//! fixed number that works for every column.

use crate::buffer::{Buffer, Cell};
use crate::layout::{Constraint, Direction, Layout, Rect};
use crate::theme::Theme;
use crate::widgets::selection::selection_colors;
use crossterm::style::Color;
use unicode_width::UnicodeWidthChar;

/// A table with a header row and selectable data rows. Column widths
/// follow [`Constraint`]s given via [`Table::widths`], splitting evenly
/// by default.
pub struct Table<'a> {
    headers: &'a [String],
    rows: &'a [Vec<String>],
    selected: usize,
    widths: Option<&'a [Constraint]>,
    spacing: u16,
    theme: Option<&'a Theme>,
}

impl<'a> Table<'a> {
    /// Creates a table over `headers`/`rows`, highlighting the data row
    /// at `selected`. Columns split equally unless [`Table::widths`] is
    /// given.
    pub fn new(headers: &'a [String], rows: &'a [Vec<String>], selected: usize) -> Self {
        Table {
            headers,
            rows,
            selected,
            widths: None,
            spacing: 0,
            theme: None,
        }
    }

    /// Sizes each column by a [`Constraint`], the same vocabulary
    /// [`Layout`] uses -- so `Fill(1)` gives a column whatever space the
    /// fixed ones leave. `headers.len()` defines the column count:
    /// columns beyond it are ignored, and headers beyond `widths.len()`
    /// are not rendered.
    pub fn widths(mut self, widths: &'a [Constraint]) -> Self {
        self.widths = Some(widths);
        self
    }

    /// Inserts `gap` blank cells between adjacent columns.
    pub fn spacing(mut self, gap: u16) -> Self {
        self.spacing = gap;
        self
    }

    /// One `Rect` per rendered column, computed by handing this
    /// table's constraints to [`Layout`]. `headers.len()` defines the
    /// column count; supplying fewer widths renders fewer columns, and
    /// supplying more than `headers.len()` renders only
    /// `headers.len()`.
    fn column_rects(&self, area: Rect) -> Vec<Rect> {
        let n = match self.widths {
            Some(w) => w.len().min(self.headers.len()),
            None => self.headers.len(),
        };
        let constraints: Vec<Constraint> = match self.widths {
            Some(w) => w[..n].to_vec(),
            None => vec![Constraint::Fill(1); n],
        };
        Layout::new(Direction::Horizontal, constraints)
            .spacing(self.spacing)
            .split(area)
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
        rects: &[Rect],
        area: Rect,
        y: u16,
        cells: &[String],
        colors: (Color, Color),
        buf: &mut Buffer,
    ) {
        let (fg, bg) = colors;
        for (rect, cell) in rects.iter().zip(cells) {
            let text = fit(cell, rect.width);
            let mut x = rect.x;
            for ch in text.chars() {
                // Explicit clip: Layout::split does not clamp, and
                // Buffer::set wraps an out-of-range x onto a later row.
                if x >= area.x + area.width || x >= rect.x + rect.width {
                    break;
                }
                buf.set(
                    x,
                    y,
                    Cell {
                        symbol: ch,
                        fg,
                        bg,
                        alpha: 1.0,
                        ..Default::default()
                    },
                );
                x += ch.width().unwrap_or(1) as u16;
            }
            // Pad the rest of the column with blanks carrying the same
            // fg/bg: a cell narrower than its column must still paint
            // the whole column's background, or a selection highlight
            // breaks into separate islands instead of one contiguous
            // bar. Same clip as above.
            while x < rect.x + rect.width && x < area.x + area.width {
                buf.set(
                    x,
                    y,
                    Cell {
                        symbol: ' ',
                        fg,
                        bg,
                        alpha: 1.0,
                        ..Default::default()
                    },
                );
                x += 1;
            }
        }
    }

    /// Renders the header row followed by data rows, top to bottom.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 {
            return;
        }
        let rects = self.column_rects(area);
        let header_fg = self.theme.map_or(Color::Reset, |t| t.secondary);
        self.render_row(
            &rects,
            area,
            area.y,
            self.headers,
            (header_fg, Color::Reset),
            buf,
        );
        for (row_idx, row) in self
            .rows
            .iter()
            .take(area.height.saturating_sub(1) as usize)
            .enumerate()
        {
            let colors = selection_colors(self.theme, row_idx == self.selected);
            self.render_row(&rects, area, area.y + 1 + row_idx as u16, row, colors, buf);
        }
    }
}

/// Fits `cell` into `width` display cells, appending U+2026 when it is
/// cut. Measures display width rather than `char` count, so wide glyphs
/// (CJK) do not misalign the columns after them, and never splits a
/// wide glyph across the boundary.
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

        Table::new(&headers, &rows, 0)
            .widths(&[Constraint::Fixed(5)])
            .render(area, &mut buf);

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

        Table::new(&headers, &rows, 1)
            .widths(&[Constraint::Fixed(5)])
            .render(area, &mut buf);

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

        Table::new(&headers, &rows, 0)
            .widths(&[Constraint::Fixed(5)])
            .render(area, &mut buf);
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

        Table::new(&headers, &rows, 0)
            .widths(&[Constraint::Fixed(5)])
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
    fn column_rects_gives_fixed_columns_their_width_and_fill_the_rest() {
        // #170's exact shape: narrow columns plus one that takes the rest.
        let headers = vec!["a".into(), "b".into(), "c".into()];
        let rows: Vec<Vec<String>> = vec![];
        let widths = [
            Constraint::Fixed(4),
            Constraint::Fixed(6),
            Constraint::Fill(1),
        ];
        let area = Rect {
            x: 0,
            y: 0,
            width: 30,
            height: 2,
        };

        let t = Table::new(&headers, &rows, 0).widths(&widths);
        let rects = t.column_rects(area);

        assert_eq!(rects.len(), 3);
        assert_eq!((rects[0].x, rects[0].width), (0, 4));
        assert_eq!((rects[1].x, rects[1].width), (4, 6));
        assert_eq!((rects[2].x, rects[2].width), (10, 20)); // the rest
    }

    #[test]
    fn spacing_inserts_a_gap_between_columns() {
        let headers = vec!["a".into(), "b".into()];
        let rows: Vec<Vec<String>> = vec![];
        let widths = [Constraint::Fixed(4), Constraint::Fixed(4)];
        let area = Rect {
            x: 0,
            y: 0,
            width: 20,
            height: 2,
        };

        let rects = Table::new(&headers, &rows, 0)
            .widths(&widths)
            .spacing(1)
            .column_rects(area);

        assert_eq!(rects[0].x, 0);
        assert_eq!(rects[1].x, 5); // 4 wide + 1 gap
    }

    #[test]
    fn without_widths_columns_split_equally() {
        // Characterisation of the pre-2.0 default.
        let headers = vec!["a".into(), "b".into(), "c".into()];
        let rows: Vec<Vec<String>> = vec![];
        let area = Rect {
            x: 0,
            y: 0,
            width: 30,
            height: 2,
        };

        let rects = Table::new(&headers, &rows, 0).column_rects(area);

        assert_eq!(rects.len(), 3);
        for r in &rects {
            assert_eq!(r.width, 10);
        }
    }

    #[test]
    fn more_widths_than_headers_renders_only_the_headers_columns() {
        let headers = vec!["a".into(), "b".into()];
        let rows: Vec<Vec<String>> = vec![];
        let widths = [Constraint::Fixed(3); 5];
        let area = Rect {
            x: 0,
            y: 0,
            width: 30,
            height: 2,
        };

        let rects = Table::new(&headers, &rows, 0)
            .widths(&widths)
            .column_rects(area);

        assert_eq!(rects.len(), 2);
    }

    #[test]
    fn fewer_widths_than_headers_renders_only_the_supplied_columns() {
        let headers = vec!["a".into(), "b".into(), "c".into()];
        let rows: Vec<Vec<String>> = vec![];
        let widths = [Constraint::Fixed(3)];
        let area = Rect {
            x: 0,
            y: 0,
            width: 30,
            height: 2,
        };

        let rects = Table::new(&headers, &rows, 0)
            .widths(&widths)
            .column_rects(area);

        assert_eq!(rects.len(), 1);
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

        Table::new(&headers, &rows, 0)
            .widths(&[Constraint::Fixed(5)])
            .render(area, &mut buf);

        assert_eq!(buf.get(0, 0).fg, Color::Reset); // header
        assert_eq!(buf.get(0, 1).fg, Color::Black); // selected
        assert_eq!(buf.get(0, 1).bg, Color::White);
        assert_eq!(buf.get(0, 2).fg, Color::Reset); // unselected
    }

    #[test]
    fn renders_each_cell_inside_its_own_column_rect() {
        let headers = vec!["a".into(), "b".into()];
        let rows = vec![vec!["xx".into(), "yy".into()]];
        let widths = [Constraint::Fixed(4), Constraint::Fixed(4)];
        let mut buf = Buffer::new(20, 2);
        let area = Rect {
            x: 0,
            y: 0,
            width: 20,
            height: 2,
        };

        Table::new(&headers, &rows, 0)
            .widths(&widths)
            .render(area, &mut buf);

        assert_eq!(buf.get(0, 1).symbol, 'x');
        assert_eq!(buf.get(4, 1).symbol, 'y'); // second column starts at 4
    }

    #[test]
    fn a_column_wider_than_the_area_does_not_wrap_onto_the_next_row() {
        // Layout::split does not clamp, and Buffer::set wraps an
        // out-of-range x onto a later row (#161). Table must clip itself.
        let headers = vec!["a".into()];
        let rows = vec![vec!["abcdefghij".into()]];
        let widths = [Constraint::Fixed(50)]; // far wider than the area
        let mut buf = Buffer::new(4, 3);
        let area = Rect {
            x: 0,
            y: 0,
            width: 4,
            height: 3,
        };

        Table::new(&headers, &rows, 0)
            .widths(&widths)
            .render(area, &mut buf);

        // Row 2 was never written to.
        for x in 0..4 {
            assert_eq!(buf.get(x, 2).symbol, ' ');
        }
    }

    #[test]
    fn a_wide_glyph_cell_leaves_the_next_column_aligned() {
        let headers = vec!["a".into(), "b".into()];
        let rows = vec![vec!["東京".into(), "ok".into()]];
        let widths = [Constraint::Fixed(4), Constraint::Fixed(4)];
        let mut buf = Buffer::new(20, 2);
        let area = Rect {
            x: 0,
            y: 0,
            width: 20,
            height: 2,
        };

        Table::new(&headers, &rows, 0)
            .widths(&widths)
            .render(area, &mut buf);

        // Sanity check: within column 1, "京" starts at x=2 (after "東"'s
        // two display cells) rather than x=1 -- if `x` advanced by 1 per
        // `char` instead of by display width, this would land at x=1 and
        // leave x=2 blank. The column-2 assertion below would pass either
        // way (each column resets `x` to its own rect.x), so it alone
        // wouldn't catch that regression.
        assert_eq!(buf.get(2, 1).symbol, '京');
        assert_eq!(buf.get(4, 1).symbol, 'o'); // still starts at 4
    }

    #[test]
    fn selected_row_highlight_spans_the_full_column_width() {
        // A cell narrower than its column must still paint the whole
        // column's background, or the selection highlight breaks into
        // separate islands instead of one contiguous bar.
        let headers = vec!["a".into(), "b".into()];
        let rows = vec![vec!["x".into(), "y".into()]];
        let widths = [Constraint::Fixed(4), Constraint::Fixed(4)];
        let mut buf = Buffer::new(20, 2);
        let area = Rect {
            x: 0,
            y: 0,
            width: 20,
            height: 2,
        };

        Table::new(&headers, &rows, 0)
            .widths(&widths)
            .render(area, &mut buf);

        // "x" is one char wide; x=1..4 is past the text but still
        // inside the 4-wide column and must carry the selected bg.
        assert_eq!(buf.get(1, 1).symbol, ' ');
        assert_eq!(buf.get(1, 1).bg, Color::White);
        assert_eq!(buf.get(3, 1).bg, Color::White);
    }
}
