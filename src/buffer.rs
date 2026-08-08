//! `Cell`/`Buffer`/`LayerStack` — the framework's core render target.
//! Widgets write `Cell`s into a `Buffer`; apps composite multiple
//! layers into one via `LayerStack` before the terminal diff-flush.

use crossterm::style::Color;

/// Text styling flags for a single `Cell`, beyond color.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct CellStyle {
    /// Whether the cell renders bold.
    pub bold: bool,
}

/// One terminal character cell: glyph, foreground/background color,
/// and style.
#[derive(Clone, PartialEq, Debug)]
pub struct Cell {
    /// The glyph to render.
    pub symbol: char,
    /// Foreground (text) color.
    pub fg: Color,
    /// Background color.
    pub bg: Color,
    /// Bold/etc. styling.
    pub style: CellStyle,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            symbol: ' ',
            fg: Color::Reset,
            bg: Color::Reset,
            style: CellStyle::default(),
        }
    }
}

/// A flat grid of `Cell`s — one render target's worth of terminal
/// content.
#[derive(Clone, Debug)]
pub struct Buffer {
    /// Grid width in cells.
    pub width: u16,
    /// Grid height in cells.
    pub height: u16,
    cells: Vec<Cell>,
}

impl Buffer {
    /// Creates a `width`x`height` buffer filled with default
    /// (blank) cells.
    pub fn new(width: u16, height: u16) -> Self {
        Buffer {
            width,
            height,
            cells: vec![Cell::default(); width as usize * height as usize],
        }
    }

    /// Returns the cell at `(x, y)`. Panics if out of bounds.
    pub fn get(&self, x: u16, y: u16) -> &Cell {
        &self.cells[self.index(x, y)]
    }

    /// Overwrites the cell at `(x, y)`. Panics if out of bounds.
    pub fn set(&mut self, x: u16, y: u16, cell: Cell) {
        let idx = self.index(x, y);
        self.cells[idx] = cell;
    }

    fn index(&self, x: u16, y: u16) -> usize {
        y as usize * self.width as usize + x as usize
    }
}

/// A single changed cell, as produced by `diff`.
#[derive(Debug, PartialEq)]
pub struct CellDiff {
    /// Column of the changed cell.
    pub x: u16,
    /// Row of the changed cell.
    pub y: u16,
    /// The cell's new value.
    pub cell: Cell,
}

/// Returns every cell in `next` that differs from `prev` at the same
/// position — used to redraw only what actually changed.
pub fn diff(prev: &Buffer, next: &Buffer) -> Vec<CellDiff> {
    let mut out = Vec::new();
    for y in 0..next.height {
        for x in 0..next.width {
            let n = next.get(x, y);
            if n != prev.get(x, y) {
                out.push(CellDiff {
                    x,
                    y,
                    cell: n.clone(),
                });
            }
        }
    }
    out
}

// Transparency rule: a cell is "transparent" (lets a lower layer show
// through during compositing) iff it equals `Cell::default()`. An overlay
// layer painting a plain space with default fg/bg/style does NOT occlude
// what's beneath it — it must set a non-default fg, bg, or style to
// actually cover the layer below (e.g. a bolded blank cell is non-default
// and DOES occlude, even though it renders identically to a blank).
/// An ordered stack of same-sized `Buffer`s, composited top-to-bottom
/// with `Cell::default()` cells treated as transparent (see the
/// transparency-rule comment above).
#[derive(Clone, Debug)]
pub struct LayerStack {
    // Invariant: always has length >= 1; layers[0] is the base layer. This
    // is what keeps Deref's `&self.layers[0]` from ever panicking.
    layers: Vec<Buffer>,
}

impl LayerStack {
    /// Creates a stack with a single `width`x`height` base layer.
    pub fn new(width: u16, height: u16) -> Self {
        LayerStack {
            layers: vec![Buffer::new(width, height)],
        }
    }

    /// Adds a new blank layer on top and returns it for writing.
    pub fn push_layer(&mut self) -> &mut Buffer {
        let width = self.layers[0].width;
        let height = self.layers[0].height;
        self.layers.push(Buffer::new(width, height));
        self.layers.last_mut().unwrap()
    }

    // `index` must already exist via a prior `push_layer()` call — there is
    // no auto-grow; an out-of-range index panics (standard Vec indexing
    // panic).
    /// Mutable access to the layer at `index` (0 = base). Panics if
    /// `index` hasn't been created via `push_layer`.
    pub fn layer_mut(&mut self, index: usize) -> &mut Buffer {
        &mut self.layers[index]
    }

    // Read-only counterpart to `layer_mut` — same out-of-range panic
    // behavior (standard Vec indexing panic).
    /// Read-only access to the layer at `index` (0 = base). Panics if
    /// `index` hasn't been created via `push_layer`.
    pub fn layer(&self, index: usize) -> &Buffer {
        &self.layers[index]
    }

    // Number of layers currently in the stack (always >= 1).
    /// Number of layers currently in the stack (always >= 1).
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    // Depth-1 fast path: returns a clone of the base layer with no scan.
    // For depth > 1: top-to-bottom scan, stopping at the first (topmost)
    // non-default cell at each position (see transparency rule on
    // `LayerStack`).
    /// Flattens every layer into one `Buffer`, topmost non-default
    /// cell winning at each position.
    pub fn composite(&self) -> Buffer {
        if self.layers.len() == 1 {
            return self.layers[0].clone();
        }
        let width = self.layers[0].width;
        let height = self.layers[0].height;
        let mut out = Buffer::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let mut cell = Cell::default();
                for layer in self.layers.iter().rev() {
                    let c = layer.get(x, y);
                    if *c != Cell::default() {
                        cell = c.clone();
                        break;
                    }
                }
                out.set(x, y, cell);
            }
        }
        out
    }
}

impl std::ops::Deref for LayerStack {
    type Target = Buffer;
    fn deref(&self) -> &Buffer {
        &self.layers[0]
    }
}

impl std::ops::DerefMut for LayerStack {
    fn deref_mut(&mut self) -> &mut Buffer {
        &mut self.layers[0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_style_default_bold_is_false() {
        assert!(!CellStyle::default().bold);
    }

    #[test]
    fn cell_default_style_equals_cell_style_default() {
        assert_eq!(Cell::default().style, CellStyle::default());
    }

    #[test]
    fn cells_identical_except_bold_are_unequal() {
        let cell1 = Cell::default();
        let mut cell2 = Cell::default();
        cell2.style.bold = true;
        assert_ne!(cell1, cell2);
    }

    #[test]
    fn new_buffer_is_filled_with_default_cells() {
        let buf = Buffer::new(3, 2);
        assert_eq!(buf.width, 3);
        assert_eq!(buf.height, 2);
        assert_eq!(*buf.get(0, 0), Cell::default());
        assert_eq!(*buf.get(2, 1), Cell::default());
    }

    #[test]
    fn set_then_get_returns_the_cell() {
        let mut buf = Buffer::new(3, 2);
        let cell = Cell {
            symbol: 'x',
            fg: crossterm::style::Color::Red,
            bg: crossterm::style::Color::Reset,
            ..Default::default()
        };
        buf.set(1, 1, cell.clone());
        assert_eq!(*buf.get(1, 1), cell);
    }

    #[test]
    fn diff_returns_only_changed_cells() {
        let prev = Buffer::new(2, 1);
        let mut next = Buffer::new(2, 1);
        let cell = Cell {
            symbol: 'x',
            fg: Color::Reset,
            bg: Color::Reset,
            ..Default::default()
        };
        next.set(1, 0, cell.clone());

        let diffs = diff(&prev, &next);

        assert_eq!(diffs, vec![CellDiff { x: 1, y: 0, cell }]);
    }

    #[test]
    fn diff_of_identical_buffers_is_empty() {
        let a = Buffer::new(2, 2);
        let b = Buffer::new(2, 2);
        assert!(diff(&a, &b).is_empty());
    }

    #[test]
    fn new_layer_stack_has_one_default_filled_base_layer() {
        let mut stack = LayerStack::new(3, 2);
        assert_eq!(*stack.layer_mut(0).get(0, 0), Cell::default());
        assert_eq!(*stack.layer_mut(0).get(2, 1), Cell::default());
    }

    #[test]
    fn push_layer_appends_a_same_dimension_default_filled_layer() {
        let mut stack = LayerStack::new(3, 2);
        let cell = Cell {
            symbol: 'x',
            fg: Color::Red,
            bg: Color::Reset,
            ..Default::default()
        };
        stack.push_layer().set(1, 1, cell.clone());

        assert_eq!(*stack.layer_mut(1).get(1, 1), cell);
        assert_eq!(*stack.layer_mut(0).get(1, 1), Cell::default());
    }

    #[test]
    fn layer_stack_derefs_to_the_base_layer() {
        let mut stack = LayerStack::new(3, 2);
        let cell = Cell {
            symbol: 'y',
            fg: Color::Reset,
            bg: Color::Red,
            ..Default::default()
        };
        stack.set(0, 1, cell.clone()); // DerefMut -> base layer, no layer_mut(0) needed

        assert_eq!(*stack.get(0, 1), cell); // Deref -> base layer
        assert_eq!(*stack.layer_mut(0).get(0, 1), cell); // same cell via explicit index
    }

    #[test]
    fn composite_of_a_single_layer_stack_matches_that_layer() {
        let mut stack = LayerStack::new(2, 2);
        let cell = Cell {
            symbol: 'z',
            fg: Color::Green,
            bg: Color::Reset,
            ..Default::default()
        };
        stack.set(1, 0, cell.clone());

        let out = stack.composite();

        assert_eq!(*out.get(1, 0), cell);
        assert_eq!(*out.get(0, 0), Cell::default());
        assert_eq!(*out.get(0, 1), Cell::default());
        assert_eq!(*out.get(1, 1), Cell::default());
    }

    #[test]
    fn composite_of_a_three_layer_stack_lets_topmost_non_default_cell_win() {
        let mut stack = LayerStack::new(3, 1);
        let a = Cell {
            symbol: 'a',
            fg: Color::Reset,
            bg: Color::Reset,
            ..Default::default()
        };
        let b = Cell {
            symbol: 'b',
            fg: Color::Reset,
            bg: Color::Reset,
            ..Default::default()
        };
        let c = Cell {
            symbol: 'c',
            fg: Color::Reset,
            bg: Color::Reset,
            ..Default::default()
        };

        stack.set(0, 0, a.clone()); // base layer: 'a' at x=0
        stack.push_layer().set(1, 0, b.clone()); // layer 1: 'b' at x=1
        stack.push_layer().set(0, 0, c.clone()); // layer 2 (top): 'c' at x=0

        let out = stack.composite();

        assert_eq!(*out.get(0, 0), c); // layer 2's 'c' overwrites layer 0's 'a'
        assert_eq!(*out.get(1, 0), b); // layer 1's 'b' survives (layer 2 is default here)
        assert_eq!(*out.get(2, 0), Cell::default()); // every layer default here
    }

    #[test]
    fn cloning_a_layer_stack_preserves_all_layers_not_just_the_base() {
        let mut stack = LayerStack::new(2, 1);
        let base_cell = Cell {
            symbol: 'a',
            fg: Color::Reset,
            bg: Color::Reset,
            ..Default::default()
        };
        let top_cell = Cell {
            symbol: 'b',
            fg: Color::Blue,
            bg: Color::Reset,
            ..Default::default()
        };
        stack.set(0, 0, base_cell.clone()); // base layer via DerefMut
        stack.push_layer().set(1, 0, top_cell.clone()); // layer 1 (top)
        stack.push_layer(); // layer 2, left default so composite still needs layer 1

        let mut cloned = stack.clone();

        // If `LayerStack::clone` had autoderef-resolved to `Buffer::clone`
        // (missing derive), `cloned` would be a `Buffer` and this wouldn't
        // compile as a `LayerStack` method call; asserting on `layer_mut`
        // and the layer count below only typechecks against a real
        // `LayerStack` clone.
        assert_eq!(cloned.layers.len(), 3);
        assert_eq!(*cloned.layer_mut(0).get(0, 0), base_cell);
        assert_eq!(*cloned.layer_mut(1).get(1, 0), top_cell);

        // Composite must still see the top layer's cell, proving the clone
        // retained every pushed layer rather than collapsing to the base.
        let out = cloned.composite();
        assert_eq!(*out.get(1, 0), top_cell);
        assert_eq!(*out.get(0, 0), base_cell);
    }

    #[test]
    fn layer_count_reflects_pushed_layers() {
        let mut stack = LayerStack::new(2, 2);
        assert_eq!(stack.layer_count(), 1);
        stack.push_layer();
        stack.push_layer();
        assert_eq!(stack.layer_count(), 3);
    }

    #[test]
    fn layer_gives_read_only_access_to_a_pushed_layer() {
        let mut stack = LayerStack::new(2, 2);
        let cell = Cell {
            symbol: 'q',
            fg: Color::Reset,
            bg: Color::Reset,
            ..Default::default()
        };
        stack.push_layer().set(0, 0, cell.clone());

        assert_eq!(*stack.layer(1).get(0, 0), cell);
    }
}
