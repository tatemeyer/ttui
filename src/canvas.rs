//! Sub-cell rendering primitive: `HalfBlock` mode gives 2x vertical
//! resolution with full 2-color fidelity per cell; `Braille` mode
//! gives 4x resolution with one fg color per cell. Graduated from the
//! rendering-fidelity spike
//! (docs/design/specs/core/2026-08-08-rendering-fidelity-spike-design.md)
//! per
//! docs/design/specs/core/2026-08-08-rendering-primitives-graduation-design.md.

use crate::buffer::{Buffer, Cell, CellStyle};
use crossterm::style::Color;

/// Which sub-cell rasterization technique a `Canvas` uses.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CanvasMode {
    /// 1x2 subpixels per cell, full 2-color fidelity (▀/▄/█).
    HalfBlock,
    /// 2x4 subpixels per cell, one fg color per cell (braille glyphs).
    Braille,
}

/// A higher-resolution drawing surface that rasterizes into ordinary
/// `Cell`s via `blit`. See module docs for `HalfBlock` vs `Braille`
/// mode details.
pub struct Canvas {
    width: u16,  // in cells
    height: u16, // in cells
    mode: CanvasMode,
    subpixels_x: u16,
    subpixels_y: u16,
    grid: Vec<Option<(Color, u64)>>, // (color, write-sequence number)
    next_seq: u64,
}

impl Canvas {
    /// Creates a blank `width`x`height`-cell canvas in `mode`.
    pub fn new(width: u16, height: u16, mode: CanvasMode) -> Self {
        let (subpixels_x, subpixels_y) = match mode {
            CanvasMode::HalfBlock => (1, 2),
            CanvasMode::Braille => (2, 4),
        };
        let grid_w = width as usize * subpixels_x as usize;
        let grid_h = height as usize * subpixels_y as usize;
        Canvas {
            width,
            height,
            mode,
            subpixels_x,
            subpixels_y,
            grid: vec![None; grid_w * grid_h],
            next_seq: 0,
        }
    }

    fn grid_width(&self) -> u16 {
        self.width * self.subpixels_x
    }

    fn grid_height(&self) -> u16 {
        self.height * self.subpixels_y
    }

    fn index(&self, x: u16, y: u16) -> usize {
        y as usize * self.grid_width() as usize + x as usize
    }

    /// Sets the subpixel at `(x, y)` (subpixel coordinates) to `color`.
    /// Out-of-bounds coordinates are silently ignored.
    pub fn set_pixel(&mut self, x: u16, y: u16, color: Color) {
        if x < self.grid_width() && y < self.grid_height() {
            let idx = self.index(x, y);
            let seq = self.next_seq;
            self.next_seq += 1;
            self.grid[idx] = Some((color, seq));
        }
    }

    /// Clears the subpixel at `(x, y)` back to transparent.
    pub fn clear_pixel(&mut self, x: u16, y: u16) {
        if x < self.grid_width() && y < self.grid_height() {
            let idx = self.index(x, y);
            self.grid[idx] = None;
        }
    }

    /// Rasterizes this canvas into `buf` at cell offset `(x, y)`.
    /// Cells with no set subpixels are left untouched (transparent).
    pub fn blit(&self, buf: &mut Buffer, x: u16, y: u16) {
        match self.mode {
            CanvasMode::HalfBlock => self.blit_half_block(buf, x, y),
            CanvasMode::Braille => self.blit_braille(buf, x, y),
        }
    }

    fn blit_half_block(&self, buf: &mut Buffer, ox: u16, oy: u16) {
        for cy in 0..self.height {
            for cx in 0..self.width {
                let top = self.grid[self.index(cx, cy * 2)].map(|(c, _)| c);
                let bottom = self.grid[self.index(cx, cy * 2 + 1)].map(|(c, _)| c);
                let cell = match (top, bottom) {
                    (None, None) => continue, // transparent: leave buf untouched
                    (Some(t), None) => Cell {
                        symbol: '▀',
                        fg: t,
                        bg: Color::Reset,
                        style: CellStyle::default(),
                        alpha: 1.0,
                    },
                    (None, Some(b)) => Cell {
                        symbol: '▄',
                        fg: b,
                        bg: Color::Reset,
                        style: CellStyle::default(),
                        alpha: 1.0,
                    },
                    (Some(t), Some(b)) if t == b => Cell {
                        symbol: '█',
                        fg: t,
                        bg: t,
                        style: CellStyle::default(),
                        alpha: 1.0,
                    },
                    (Some(t), Some(b)) => Cell {
                        symbol: '▀',
                        fg: t,
                        bg: b,
                        style: CellStyle::default(),
                        alpha: 1.0,
                    },
                };
                let bx = ox + cx;
                let by = oy + cy;
                if bx < buf.width && by < buf.height {
                    buf.set(bx, by, cell);
                }
            }
        }
    }

    fn blit_braille(&self, buf: &mut Buffer, ox: u16, oy: u16) {
        // Braille dot bit layout (Unicode "Braille Patterns" block,
        // U+2800): bit0/bit3 = row0 col0/col1, bit1/bit4 = row1,
        // bit2/bit5 = row2, bit6/bit7 = row3.
        const DOT_BITS: [[u8; 2]; 4] = [[0x01, 0x08], [0x02, 0x10], [0x04, 0x20], [0x40, 0x80]];
        for cy in 0..self.height {
            for cx in 0..self.width {
                let mut mask: u8 = 0;
                let mut winner: Option<(Color, u64)> = None;
                for row in 0..4u16 {
                    for col in 0..2u16 {
                        let px = cx * 2 + col;
                        let py = cy * 4 + row;
                        if let Some((c, seq)) = self.grid[self.index(px, py)] {
                            mask |= DOT_BITS[row as usize][col as usize];
                            if winner.map(|(_, best)| seq > best).unwrap_or(true) {
                                winner = Some((c, seq)); // genuinely last-write-wins now
                            }
                        }
                    }
                }
                if mask == 0 {
                    continue; // transparent
                }
                let symbol = char::from_u32(0x2800 + mask as u32).unwrap();
                let bx = ox + cx;
                let by = oy + cy;
                if bx < buf.width && by < buf.height {
                    buf.set(
                        bx,
                        by,
                        Cell {
                            symbol,
                            fg: winner.unwrap().0,
                            bg: Color::Reset,
                            style: CellStyle::default(),
                            alpha: 1.0,
                        },
                    );
                }
            }
        }
    }

    /// Draws a straight line between two subpixel points (Bresenham).
    pub fn line(&mut self, x0: u16, y0: u16, x1: u16, y1: u16, color: Color) {
        let (mut x0, mut y0) = (x0 as i32, y0 as i32);
        let (x1, y1) = (x1 as i32, y1 as i32);
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            if x0 >= 0 && y0 >= 0 {
                self.set_pixel(x0 as u16, y0 as u16, color);
            }
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    /// Draws a rectangle outline with top-left at `(x, y)` (subpixel
    /// coordinates).
    pub fn rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: Color) {
        if w == 0 || h == 0 {
            return;
        }
        let x1 = x.saturating_add(w).saturating_sub(1);
        let y1 = y.saturating_add(h).saturating_sub(1);
        self.line(x, y, x1, y, color);
        self.line(x, y1, x1, y1, color);
        self.line(x, y, x, y1, color);
        self.line(x1, y, x1, y1, color);
    }

    /// Fills a solid rectangle with top-left at `(x, y)` (subpixel
    /// coordinates).
    pub fn fill_rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: Color) {
        for row in y..y.saturating_add(h) {
            for col in x..x.saturating_add(w) {
                self.set_pixel(col, row, color);
            }
        }
    }

    /// Fills the polygon described by `points` (subpixel coordinates,
    /// 3+ points, in perimeter order) via an even-odd scanline fill.
    /// Does nothing if fewer than 3 points are given. Both the row
    /// range and each row's column range are clamped to the canvas's
    /// own bounds, so a point far outside the canvas in either axis
    /// cannot cause an oversized per-frame scan.
    pub fn fill_polygon(&mut self, points: &[(f32, f32)], color: Color) {
        if points.len() < 3 {
            return;
        }
        let min_y = points
            .iter()
            .map(|p| p.1)
            .fold(f32::INFINITY, f32::min)
            .floor()
            .max(0.0) as u16;
        let max_y = points
            .iter()
            .map(|p| p.1)
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .min(self.grid_height().saturating_sub(1) as f32) as u16;
        for y in min_y..=max_y {
            let yf = y as f32 + 0.5;
            let mut xs: Vec<f32> = Vec::new();
            for i in 0..points.len() {
                let (x0, y0) = points[i];
                let (x1, y1) = points[(i + 1) % points.len()];
                if (y0 <= yf && y1 > yf) || (y1 <= yf && y0 > yf) {
                    let t = (yf - y0) / (y1 - y0);
                    xs.push(x0 + t * (x1 - x0));
                }
            }
            xs.sort_by(f32::total_cmp);
            let mut i = 0;
            while i + 1 < xs.len() {
                let x_start = xs[i].round().max(0.0) as u16;
                let x_end = xs[i + 1]
                    .round()
                    .max(0.0)
                    .min(self.grid_width().saturating_sub(1) as f32)
                    as u16;
                for x in x_start..=x_end {
                    self.set_pixel(x, y, color);
                }
                i += 2;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::Cell;

    fn red() -> Color {
        Color::Rgb { r: 255, g: 0, b: 0 }
    }
    fn blue() -> Color {
        Color::Rgb { r: 0, g: 0, b: 255 }
    }

    #[test]
    fn half_block_top_only_produces_upper_half_block_with_reset_bg() {
        let mut c = Canvas::new(1, 1, CanvasMode::HalfBlock);
        c.set_pixel(0, 0, red());
        let mut buf = Buffer::new(1, 1);
        c.blit(&mut buf, 0, 0);
        assert_eq!(
            *buf.get(0, 0),
            Cell {
                symbol: '▀',
                fg: red(),
                bg: Color::Reset,
                alpha: 1.0,
                ..Default::default()
            }
        );
    }

    #[test]
    fn half_block_bottom_only_produces_lower_half_block_with_reset_bg() {
        let mut c = Canvas::new(1, 1, CanvasMode::HalfBlock);
        c.set_pixel(0, 1, blue());
        let mut buf = Buffer::new(1, 1);
        c.blit(&mut buf, 0, 0);
        assert_eq!(
            *buf.get(0, 0),
            Cell {
                symbol: '▄',
                fg: blue(),
                bg: Color::Reset,
                alpha: 1.0,
                ..Default::default()
            }
        );
    }

    #[test]
    fn half_block_both_equal_produces_solid_block() {
        let mut c = Canvas::new(1, 1, CanvasMode::HalfBlock);
        c.set_pixel(0, 0, red());
        c.set_pixel(0, 1, red());
        let mut buf = Buffer::new(1, 1);
        c.blit(&mut buf, 0, 0);
        assert_eq!(
            *buf.get(0, 0),
            Cell {
                symbol: '█',
                fg: red(),
                bg: red(),
                alpha: 1.0,
                ..Default::default()
            }
        );
    }

    #[test]
    fn half_block_both_different_splits_fg_and_bg() {
        let mut c = Canvas::new(1, 1, CanvasMode::HalfBlock);
        c.set_pixel(0, 0, red());
        c.set_pixel(0, 1, blue());
        let mut buf = Buffer::new(1, 1);
        c.blit(&mut buf, 0, 0);
        assert_eq!(
            *buf.get(0, 0),
            Cell {
                symbol: '▀',
                fg: red(),
                bg: blue(),
                alpha: 1.0,
                ..Default::default()
            }
        );
    }

    #[test]
    fn half_block_unset_cell_leaves_target_buffer_untouched() {
        let c = Canvas::new(1, 1, CanvasMode::HalfBlock);
        let mut buf = Buffer::new(1, 1);
        let sentinel = Cell {
            symbol: 'X',
            fg: Color::Green,
            bg: Color::Yellow,
            alpha: 1.0,
            ..Default::default()
        };
        buf.set(0, 0, sentinel.clone());
        c.blit(&mut buf, 0, 0);
        assert_eq!(*buf.get(0, 0), sentinel);
    }

    #[test]
    fn set_pixel_out_of_bounds_is_silently_ignored() {
        let mut c = Canvas::new(1, 1, CanvasMode::HalfBlock);
        c.set_pixel(99, 99, red()); // grid is only 1x2 for a 1x1 half-block canvas
        let mut buf = Buffer::new(1, 1);
        c.blit(&mut buf, 0, 0);
        assert_eq!(*buf.get(0, 0), Cell::default());
    }

    #[test]
    fn clear_pixel_removes_a_previously_set_pixel() {
        let mut c = Canvas::new(1, 1, CanvasMode::HalfBlock);
        c.set_pixel(0, 0, red());
        c.clear_pixel(0, 0);
        let mut buf = Buffer::new(1, 1);
        c.blit(&mut buf, 0, 0);
        assert_eq!(*buf.get(0, 0), Cell::default());
    }

    #[test]
    fn braille_single_dot_produces_the_correct_glyph_and_color() {
        let mut c = Canvas::new(1, 1, CanvasMode::Braille);
        c.set_pixel(0, 0, red()); // top-left dot: bit 0x01
        let mut buf = Buffer::new(1, 1);
        c.blit(&mut buf, 0, 0);
        let expected_symbol = char::from_u32(0x2800 + 0x01).unwrap();
        assert_eq!(
            *buf.get(0, 0),
            Cell {
                symbol: expected_symbol,
                fg: red(),
                bg: Color::Reset,
                alpha: 1.0,
                ..Default::default()
            }
        );
    }

    #[test]
    fn braille_two_dots_combine_their_bits_into_one_glyph() {
        let mut c = Canvas::new(1, 1, CanvasMode::Braille);
        c.set_pixel(0, 0, red()); // bit 0x01
        c.set_pixel(1, 0, red()); // bit 0x08
        let mut buf = Buffer::new(1, 1);
        c.blit(&mut buf, 0, 0);
        let expected_symbol = char::from_u32(0x2800 + 0x01 + 0x08).unwrap();
        assert_eq!(buf.get(0, 0).symbol, expected_symbol);
    }

    #[test]
    fn braille_last_written_dot_wins_the_cells_color() {
        let mut c = Canvas::new(1, 1, CanvasMode::Braille);
        c.set_pixel(0, 0, red());
        c.set_pixel(1, 0, blue());
        let mut buf = Buffer::new(1, 1);
        c.blit(&mut buf, 0, 0);
        assert_eq!(buf.get(0, 0).fg, blue());
    }

    #[test]
    fn braille_last_written_wins_even_when_earlier_in_scan_order() {
        // The scan visits (row, col) in order (0,0),(0,1),(1,0),(1,1),
        // (2,0),(2,1),(3,0),(3,1) — so subpixel (1,3) [row=3,col=1] is
        // LAST in scan order, and (0,0) [row=0,col=0] is FIRST. Here
        // they're written in the OPPOSITE order: the scan-order-last
        // subpixel is written FIRST (chronologically), and the
        // scan-order-first subpixel is written SECOND (chronologically
        // more recent). A scan-order-based (buggy) rule would report
        // `red` (whichever the row/col loop touches last); a true
        // last-write-wins rule reports `blue` (written later in real
        // call order) — this is exactly the distinction the existing
        // `braille_last_written_dot_wins_the_cells_color` test above
        // cannot catch, since its two `set_pixel` calls happen to
        // already agree on scan order and write order.
        let mut c = Canvas::new(1, 1, CanvasMode::Braille);
        c.set_pixel(1, 3, red());
        c.set_pixel(0, 0, blue());
        let mut buf = Buffer::new(1, 1);
        c.blit(&mut buf, 0, 0);
        assert_eq!(buf.get(0, 0).fg, blue());
    }

    #[test]
    fn braille_unset_cell_leaves_target_buffer_untouched() {
        let c = Canvas::new(1, 1, CanvasMode::Braille);
        let mut buf = Buffer::new(1, 1);
        let sentinel = Cell {
            symbol: 'X',
            fg: Color::Green,
            bg: Color::Yellow,
            alpha: 1.0,
            ..Default::default()
        };
        buf.set(0, 0, sentinel.clone());
        c.blit(&mut buf, 0, 0);
        assert_eq!(*buf.get(0, 0), sentinel);
    }

    #[test]
    fn line_draws_a_horizontal_run_of_pixels() {
        let mut c = Canvas::new(3, 1, CanvasMode::HalfBlock);
        c.line(0, 0, 2, 0, red()); // top row of a 3-wide, 1-tall (2-subpixel-tall) canvas
        let mut buf = Buffer::new(3, 1);
        c.blit(&mut buf, 0, 0);
        for x in 0..3 {
            assert_eq!(
                buf.get(x, 0).symbol,
                '▀',
                "cell {x} should show the top-only half-block"
            );
            assert_eq!(buf.get(x, 0).fg, red());
        }
    }

    #[test]
    fn line_draws_a_diagonal_run_via_the_bresenham_error_term_branches() {
        // 3x3-cell HalfBlock canvas -> 3x6 subpixel grid. A 45-degree
        // diagonal (dx == -dy) exercises both the `e2 >= dy` and
        // `e2 <= dx` branches every step, unlike the dy == 0 horizontal
        // case covered above.
        //
        // Hand-traced Bresenham for line(0,0,2,2): dx=2, dy=-2, err=0.
        //   step 0: plot (0,0); e2=0 >= -2 -> x=1; e2=0 <= 2 -> y=1
        //   step 1: plot (1,1); e2=0 >= -2 -> x=2; e2=0 <= 2 -> y=2
        //   step 2: plot (2,2); x==x1 && y==y1 -> stop
        // Subpixel (0,0) -> cell (0,0) top row -> '▀'.
        // Subpixel (1,1) -> cell (1,0) bottom row -> '▄'.
        // Subpixel (2,2) -> cell (2,1) top row -> '▀'.
        let mut c = Canvas::new(3, 3, CanvasMode::HalfBlock);
        c.line(0, 0, 2, 2, red());
        let mut buf = Buffer::new(3, 3);
        c.blit(&mut buf, 0, 0);

        assert_eq!(buf.get(0, 0).symbol, '▀');
        assert_eq!(buf.get(0, 0).fg, red());
        assert_eq!(buf.get(1, 0).symbol, '▄');
        assert_eq!(buf.get(1, 0).fg, red());
        assert_eq!(buf.get(2, 1).symbol, '▀');
        assert_eq!(buf.get(2, 1).fg, red());

        // Everything off the diagonal stays untouched.
        assert_eq!(*buf.get(1, 1), Cell::default());
        assert_eq!(*buf.get(2, 0), Cell::default());
        assert_eq!(*buf.get(0, 1), Cell::default());
        assert_eq!(*buf.get(0, 2), Cell::default());
        assert_eq!(*buf.get(1, 2), Cell::default());
    }

    #[test]
    fn rect_draws_all_four_edges_of_the_outline() {
        let mut c = Canvas::new(3, 3, CanvasMode::HalfBlock); // grid 3x6
        c.rect(0, 0, 3, 6, red());
        let mut buf = Buffer::new(3, 3);
        c.blit(&mut buf, 0, 0);
        // Every perimeter cell should have picked up red on at least one subpixel;
        // the center cell (1,1) should remain untouched (default).
        assert_ne!(*buf.get(0, 0), Cell::default());
        assert_ne!(*buf.get(1, 0), Cell::default());
        assert_ne!(*buf.get(2, 0), Cell::default());
        assert_ne!(*buf.get(0, 1), Cell::default());
        assert_ne!(*buf.get(2, 1), Cell::default());
        assert_ne!(*buf.get(0, 2), Cell::default());
        assert_ne!(*buf.get(1, 2), Cell::default());
        assert_ne!(*buf.get(2, 2), Cell::default());
        assert_eq!(*buf.get(1, 1), Cell::default());
    }

    #[test]
    fn fill_rect_fills_every_pixel_in_the_region() {
        let mut c = Canvas::new(2, 1, CanvasMode::HalfBlock); // grid 2x2
        c.fill_rect(0, 0, 2, 2, red());
        let mut buf = Buffer::new(2, 1);
        c.blit(&mut buf, 0, 0);
        assert_eq!(buf.get(0, 0).symbol, '█');
        assert_eq!(buf.get(1, 0).symbol, '█');
    }

    #[test]
    fn blit_clips_to_the_target_buffers_bounds_without_panicking() {
        let mut c = Canvas::new(3, 3, CanvasMode::HalfBlock);
        c.fill_rect(0, 0, 3, 6, red());
        let mut buf = Buffer::new(2, 2); // smaller than the canvas
        c.blit(&mut buf, 0, 0); // must not panic
        assert_eq!(buf.get(0, 0).symbol, '█');
        assert_eq!(buf.get(1, 1).symbol, '█');
    }

    #[test]
    fn rect_and_fill_rect_near_u16_max_do_not_panic() {
        let mut c = Canvas::new(4, 4, CanvasMode::HalfBlock);
        c.rect(u16::MAX - 2, u16::MAX - 2, 10, 10, red()); // must not panic on overflow
        c.fill_rect(u16::MAX - 2, u16::MAX - 2, 10, 10, red()); // must not panic on overflow
    }

    #[test]
    fn fill_polygon_does_nothing_for_fewer_than_three_points() {
        let mut c = Canvas::new(2, 2, CanvasMode::HalfBlock);
        c.fill_polygon(&[(0.0, 0.0), (3.0, 3.0)], red());
        let mut buf = Buffer::new(2, 2);
        c.blit(&mut buf, 0, 0);
        assert_eq!(*buf.get(0, 0), Cell::default());
        assert_eq!(*buf.get(1, 1), Cell::default());
    }

    #[test]
    fn fill_polygon_fills_a_rectangle_with_correct_even_odd_boundaries() {
        // HalfBlock canvas, 4 cells wide x 3 cells tall (subpixel grid 4x6).
        // Rectangle vertices at subpixel (1,1)-(1,5)-(3,5)-(3,1) fill
        // subpixel columns 1-3 (inclusive both boundary columns, per
        // this scan's existing crossing-pair convention) across
        // subpixel rows 1-4 — row 5 sits exactly on the bottom edge
        // and is correctly left unfilled by the per-row crossing test,
        // even though it's within the outer loop's scanned range.
        // Hand-traced against the actual algorithm, not assumed.
        let mut c = Canvas::new(4, 3, CanvasMode::HalfBlock);
        c.fill_polygon(&[(1.0, 1.0), (1.0, 5.0), (3.0, 5.0), (3.0, 1.0)], red());
        let mut buf = Buffer::new(4, 3);
        c.blit(&mut buf, 0, 0);

        // Column 0 (outside the rectangle): every row stays default.
        assert_eq!(*buf.get(0, 0), Cell::default());
        assert_eq!(*buf.get(0, 1), Cell::default());
        assert_eq!(*buf.get(0, 2), Cell::default());

        // Columns 1-3: bottom-only at row 0 (subpixel y=1 filled, y=0
        // not), solid at row 1 (subpixel y=2 and y=3 both filled),
        // top-only at row 2 (subpixel y=4 filled, y=5 not).
        for cx in 1..=3 {
            assert_eq!(buf.get(cx, 0).symbol, '▄', "col {cx} row 0");
            assert_eq!(buf.get(cx, 0).fg, red());
            assert_eq!(buf.get(cx, 1).symbol, '█', "col {cx} row 1");
            assert_eq!(buf.get(cx, 2).symbol, '▀', "col {cx} row 2");
            assert_eq!(buf.get(cx, 2).fg, red());
        }
    }

    #[test]
    fn fill_polygon_handles_wildly_out_of_range_coordinates_without_panicking() {
        // Vertices far outside the canvas on BOTH axes (simulating
        // what a near-camera projected point could produce, since
        // `project_polygon` deliberately does no screen-edge clipping)
        // must not panic, and the portion of the polygon that's
        // actually on-canvas must still fill correctly. This does NOT
        // verify the row/column clamps' iteration-count optimization
        // itself: the `as u16` casts already saturate out-of-range
        // floats, and `set_pixel`'s own bounds check makes any write
        // beyond the canvas a silent no-op regardless of whether the
        // clamps are present — so this black-box pixel-output test
        // can't distinguish "loop scans ~4 rows/cols" from "loop scans
        // 65536 rows/cols". The clamps are still worth keeping (avoids
        // that wasted scan) but aren't observable from here; see the
        // `fill_polygon` doc comment for the perf rationale instead.
        let mut c = Canvas::new(2, 2, CanvasMode::HalfBlock); // grid 2x4
        c.fill_polygon(
            &[
                (0.0, 0.0),
                (0.0, 1_000_000.0),
                (1_000_000.0, 1_000_000.0),
                (1_000_000.0, 0.0),
            ],
            red(),
        );
        let mut buf = Buffer::new(2, 2);
        c.blit(&mut buf, 0, 0); // must not panic
        for cy in 0..2 {
            for cx in 0..2 {
                assert_eq!(buf.get(cx, cy).symbol, '█', "cell ({cx},{cy})");
            }
        }
    }

    #[test]
    fn project_polygon_then_fill_polygon_produce_the_expected_triangle() {
        // Integration test across the Camera::project_polygon ->
        // Canvas::fill_polygon seam: every other test in this file or
        // in perspective.rs exercises exactly one of these two
        // functions, hand-typing stand-in values for whatever the
        // other one would have produced. This test instead projects a
        // real 3-vertex Polygon3 with a real Camera and feeds the
        // resulting Vec<(f32, f32)> directly into fill_polygon, so a
        // coordinate-convention mismatch between the two (a y-flip, a
        // cell-vs-subpixel unit mismatch) would actually be caught.
        // Using 3 distinct vertices here (rather than perspective.rs's
        // existing single-vertex project_polygon test) also proves
        // multiple vertices project correctly, in order.
        use crate::perspective::{Camera, Point3, Polygon3};

        let cam = Camera {
            near: 0.5,
            focal_length: 8.0,
        };
        // All three vertices at z=4.0; x/y chosen so every ndc value
        // is an exact power-of-two fraction (no epsilon needed).
        // Hand-verified projection (center=(2.0,2.5), subpixels=(1.0,2.0)):
        //   A(x=-0.25,y=1.0,z=4): ndc=(-0.0625,0.25)  -> screen=(1.0,0.5) -> subpixel (1.0,1.0)
        //   B(x=-0.25,y=0.0,z=4): ndc=(-0.0625,0.0)   -> screen=(1.0,2.5) -> subpixel (1.0,5.0)
        //   C(x=0.25, y=0.5,z=4): ndc=(0.0625,0.125)  -> screen=(3.0,1.5) -> subpixel (3.0,3.0)
        let triangle = Polygon3 {
            vertices: vec![
                Point3 {
                    x: -0.25,
                    y: 1.0,
                    z: 4.0,
                },
                Point3 {
                    x: -0.25,
                    y: 0.0,
                    z: 4.0,
                },
                Point3 {
                    x: 0.25,
                    y: 0.5,
                    z: 4.0,
                },
            ],
        };
        let points = cam
            .project_polygon(&triangle, 2.0, 2.5, 1.0, 2.0, 0.0)
            .expect("all three vertices are in front of the near plane");
        assert_eq!(points, vec![(1.0, 1.0), (1.0, 5.0), (3.0, 3.0)]);

        let mut c = Canvas::new(4, 3, CanvasMode::HalfBlock); // grid 4x6
        c.fill_polygon(&points, red());
        let mut buf = Buffer::new(4, 3);
        c.blit(&mut buf, 0, 0);

        // Hand-traced even-odd scanline fill of the projected triangle
        // (1,1)-(1,5)-(3,3): subpixel row 1 fills cols 1-2, rows 2-3
        // fill cols 1-3, row 4 fills cols 1-2, rows 0 and 5 fill
        // nothing (row 0 is above the topmost vertex; row 5 sits
        // exactly on vertex B and the per-row crossing test correctly
        // excludes it, same convention as the rectangle test above).
        assert_eq!(buf.get(1, 0).symbol, '▄', "cell (1,0): bottom-only");
        assert_eq!(buf.get(1, 0).fg, red());
        assert_eq!(buf.get(2, 0).symbol, '▄', "cell (2,0): bottom-only");
        assert_eq!(*buf.get(0, 0), Cell::default(), "left of the triangle");
        assert_eq!(*buf.get(3, 0), Cell::default(), "right of the triangle");

        for cx in 1..=3 {
            assert_eq!(buf.get(cx, 1).symbol, '█', "cell ({cx},1): solid row");
            assert_eq!(buf.get(cx, 1).fg, red());
        }
        assert_eq!(*buf.get(0, 1), Cell::default());

        assert_eq!(buf.get(1, 2).symbol, '▀', "cell (1,2): top-only");
        assert_eq!(buf.get(1, 2).fg, red());
        assert_eq!(buf.get(2, 2).symbol, '▀', "cell (2,2): top-only");
        assert_eq!(*buf.get(0, 2), Cell::default());
        assert_eq!(*buf.get(3, 2), Cell::default());
    }
}
