//! Sub-cell rendering primitive (half-block + braille) — SPIKE
//! PROTOTYPE for the rendering-fidelity spike
//! (docs/design/specs/core/2026-08-08-rendering-fidelity-spike-design.md).
//! Not a committed, stable API: expect this to be rewritten once the
//! spike's recommendations are acted on.

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
/// `Cell`s via `blit`. See module docs — spike prototype, not a
/// committed API.
pub struct Canvas {
    width: u16,  // in cells
    height: u16, // in cells
    mode: CanvasMode,
    subpixels_x: u16,
    subpixels_y: u16,
    grid: Vec<Option<Color>>, // len = grid_width() * grid_height()
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
            self.grid[idx] = Some(color);
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
                let top = self.grid[self.index(cx, cy * 2)];
                let bottom = self.grid[self.index(cx, cy * 2 + 1)];
                let cell = match (top, bottom) {
                    (None, None) => continue, // transparent: leave buf untouched
                    (Some(t), None) => Cell {
                        symbol: '▀',
                        fg: t,
                        bg: Color::Reset,
                        style: CellStyle::default(),
                    },
                    (None, Some(b)) => Cell {
                        symbol: '▄',
                        fg: b,
                        bg: Color::Reset,
                        style: CellStyle::default(),
                    },
                    (Some(t), Some(b)) if t == b => Cell {
                        symbol: '█',
                        fg: t,
                        bg: t,
                        style: CellStyle::default(),
                    },
                    (Some(t), Some(b)) => Cell {
                        symbol: '▀',
                        fg: t,
                        bg: b,
                        style: CellStyle::default(),
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
                let mut color: Option<Color> = None;
                for row in 0..4u16 {
                    for col in 0..2u16 {
                        let px = cx * 2 + col;
                        let py = cy * 4 + row;
                        if let Some(c) = self.grid[self.index(px, py)] {
                            mask |= DOT_BITS[row as usize][col as usize];
                            color = Some(c); // last-write-wins per cell
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
                            fg: color.unwrap(),
                            bg: Color::Reset,
                            style: CellStyle::default(),
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
        self.line(x, y, x + w - 1, y, color);
        self.line(x, y + h - 1, x + w - 1, y + h - 1, color);
        self.line(x, y, x, y + h - 1, color);
        self.line(x + w - 1, y, x + w - 1, y + h - 1, color);
    }

    /// Fills a solid rectangle with top-left at `(x, y)` (subpixel
    /// coordinates).
    pub fn fill_rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: Color) {
        for row in y..y + h {
            for col in x..x + w {
                self.set_pixel(col, row, color);
            }
        }
    }
}
