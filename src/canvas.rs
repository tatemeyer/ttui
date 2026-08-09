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
            CanvasMode::Braille => { /* added in Task 3 */ }
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
}
