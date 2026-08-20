//! `Cell`/`Buffer`/`LayerStack` — the framework's core render target.
//! Widgets write `Cell`s into a `Buffer`; apps composite multiple
//! layers into one via `LayerStack` before the terminal diff-flush.

use crossterm::style::Color;

/// Text intensity — a single SGR axis; a cell is bold, dim, or
/// neither, never more than one at once.
///
/// `#[non_exhaustive]`: new variants may be added in a minor release,
/// so downstream `match`es need a wildcard arm.
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Intensity {
    /// No intensity styling.
    #[default]
    Normal,
    /// Bold (increased intensity).
    Bold,
    /// Dim (decreased intensity).
    Dim,
}

/// Text styling flags for a single `Cell`, beyond color.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct CellStyle {
    /// Bold/dim/neither — mutually exclusive by construction.
    pub intensity: Intensity,
    /// Whether the cell renders underlined.
    pub underline: bool,
    /// Whether the cell renders italic.
    pub italic: bool,
    /// Whether fg/bg render swapped.
    pub reverse: bool,
    /// Whether the cell renders with a strikethrough.
    pub strikethrough: bool,
}

/// One terminal character cell: glyph, foreground/background color,
/// style, and coverage.
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
    /// How much this cell covers whatever is beneath it during
    /// `LayerStack::composite()` — `0.0` fully transparent, `1.0`
    /// fully opaque. Meaningless once a `Buffer` has been composited
    /// and is headed for `diff`/the terminal; every cell leaving
    /// `composite()` is either untouched (`0.0`, stays default) or
    /// real content (`1.0`).
    pub alpha: f32,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            symbol: ' ',
            fg: Color::Reset,
            bg: Color::Reset,
            style: CellStyle::default(),
            alpha: 0.0,
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

    /// Returns the cell at `(x, y)`.
    ///
    /// # Panics
    /// In debug builds, if `x >= width` or `y >= height`. Release builds
    /// do not check; an out-of-range `x` reads a later row (see #161 —
    /// a real `assert!` here measured a 22%/12% (full-paint/single-cell)
    /// regression on `benches/set.rs`, so the check stays debug-only).
    pub fn get(&self, x: u16, y: u16) -> &Cell {
        &self.cells[self.index(x, y)]
    }

    /// Overwrites the cell at `(x, y)`.
    ///
    /// # Panics
    /// In debug builds, if `x >= width` or `y >= height`. Release builds
    /// do not check; an out-of-range `x` writes into a later row (see
    /// #161 — a real `assert!` here measured a 22%/12%
    /// (full-paint/single-cell) regression on `benches/set.rs`, so the
    /// check stays debug-only).
    pub fn set(&mut self, x: u16, y: u16, cell: Cell) {
        let idx = self.index(x, y);
        self.cells[idx] = cell;
    }

    /// Draws this buffer into `dest` with its top-left corner at
    /// `(x, y)`, copying every cell — a plain overwrite, not an
    /// alpha composite (that is `LayerStack::composite`).
    ///
    /// The argument order mirrors `Canvas::blit(&self, buf, x, y)` —
    /// "draw me into that" — so the two read the same way round rather
    /// than as opposites.
    ///
    /// Anything falling outside `dest` is clipped and nothing panics,
    /// including an `x` or `y` past the far edge. The clipping is
    /// explicit rather than left to `set`: in release builds `set`
    /// doesn't check `x` against `width` at all (see #161 and its
    /// `debug_assert!` in `index`), so an out-of-range `x` would
    /// silently land on a later row instead of panicking, and a blit
    /// that inherited that would smear its overhang across the rows
    /// below.
    pub fn blit(&self, dest: &mut Buffer, x: u16, y: u16) {
        for row in 0..self.height {
            let Some(dy) = y.checked_add(row) else { break };
            if dy >= dest.height {
                break;
            }
            for col in 0..self.width {
                let Some(dx) = x.checked_add(col) else { break };
                if dx >= dest.width {
                    break;
                }
                dest.set(dx, dy, self.get(col, row).clone());
            }
        }
    }

    fn index(&self, x: u16, y: u16) -> usize {
        debug_assert!(
            x < self.width && y < self.height,
            "index ({x}, {y}) out of bounds for a {}x{} buffer",
            self.width,
            self.height
        );
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

// Transparency rule: a cell is "transparent" (lets lower layers show
// through during compositing) iff its alpha is 0.0. The symbol, fg, bg,
// and style are irrelevant to transparency — a blank cell with alpha: 1.0
// DOES occlude what's beneath it, and a fully-styled cell with alpha: 0.0
// does NOT occlude. This is the foundation of the Porter-Duff "over"
// compositing algorithm used by `composite_cell`.
/// An ordered stack of same-sized `Buffer`s, composited top-to-bottom
/// via Porter-Duff "over" accumulation: each layer's contribution
/// scales with its alpha, colors blend via weighted lerp, and the
/// first layer to reach >= 0.5 contribution wins the glyph/style.
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
    // For depth > 1: top-to-bottom Porter-Duff "over" accumulation at each
    // position (see transparency rule on `LayerStack`). The first layer
    // whose contribution >= 0.5 wins the glyph/style; if none reach that
    // threshold, the topmost non-transparent contributor wins. Colors blend
    // via incremental weighted lerp across all contributing layers.
    /// Flattens every layer into one `Buffer` via Porter-Duff "over"
    /// compositing: alpha-weighted top-to-bottom accumulation.
    pub fn composite(&self) -> Buffer {
        if self.layers.len() == 1 {
            let mut out = self.layers[0].clone();
            let width = out.width;
            let height = out.height;
            for y in 0..height {
                for x in 0..width {
                    out.set(x, y, normalize_alpha(out.get(x, y).clone()));
                }
            }
            return out;
        }
        let width = self.layers[0].width;
        let height = self.layers[0].height;
        let mut out = Buffer::new(width, height);
        for y in 0..height {
            for x in 0..width {
                out.set(x, y, composite_cell(&self.layers, x, y));
            }
        }
        out
    }
}

// Single-layer fast path helper: applies the same "alpha decides the
// output, not the raw value" rule composite_cell enforces for multi-layer
// stacks, without the accumulation loop a lone layer never needs. Keeps
// the `Cell.alpha` doc contract (every cell leaving `composite()` carries
// alpha 0.0 or 1.0) true for single-layer stacks too.
fn normalize_alpha(cell: Cell) -> Cell {
    if cell.alpha <= 0.0 {
        Cell::default()
    } else {
        Cell { alpha: 1.0, ..cell }
    }
}

// Top-to-bottom Porter-Duff "over" accumulation. `remaining` tracks how
// much of this pixel is still undecided; each layer claims
// `alpha * remaining` of it, and `remaining` shrinks by `(1 - alpha)`.
// When every cell involved has alpha 1.0 (true for every existing app
// post-migration), the first non-transparent layer claims 100% on
// contact and the loop breaks immediately — byte-identical to the old
// "topmost non-default cell wins" scan, for the same reason (early exit
// on full coverage).
fn composite_cell(layers: &[Buffer], x: u16, y: u16) -> Cell {
    let mut remaining = 1.0_f32;
    let mut acc_weight = 0.0_f32;
    let mut acc_fg = Color::Reset;
    let mut acc_bg = Color::Reset;
    let mut winner: Option<(char, CellStyle)> = None;
    let mut first: Option<(char, CellStyle)> = None;

    for layer in layers.iter().rev() {
        if remaining <= 0.0 {
            break;
        }
        let cell = layer.get(x, y);
        if cell.alpha <= 0.0 {
            continue;
        }
        let contribution = cell.alpha * remaining;

        if first.is_none() {
            first = Some((cell.symbol, cell.style));
        }
        if winner.is_none() && contribution >= 0.5 {
            winner = Some((cell.symbol, cell.style));
        }

        acc_fg = if acc_weight <= 0.0 {
            cell.fg
        } else {
            crate::easing::lerp_color(acc_fg, cell.fg, contribution / (acc_weight + contribution))
        };
        acc_bg = if acc_weight <= 0.0 {
            cell.bg
        } else {
            crate::easing::lerp_color(acc_bg, cell.bg, contribution / (acc_weight + contribution))
        };
        acc_weight += contribution;

        remaining *= 1.0 - cell.alpha;
    }

    match winner.or(first) {
        None => Cell::default(),
        Some((symbol, style)) => Cell {
            symbol,
            fg: acc_fg,
            bg: acc_bg,
            style,
            alpha: 1.0,
        },
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
    fn intensity_default_is_normal() {
        assert_eq!(Intensity::default(), Intensity::Normal);
    }

    #[test]
    fn cell_style_default_intensity_is_normal() {
        assert_eq!(CellStyle::default().intensity, Intensity::Normal);
    }

    #[test]
    fn cell_default_style_equals_cell_style_default() {
        assert_eq!(Cell::default().style, CellStyle::default());
    }

    #[test]
    fn cells_identical_except_intensity_are_unequal() {
        let cell1 = Cell::default();
        let mut cell2 = Cell::default();
        cell2.style.intensity = Intensity::Bold;
        assert_ne!(cell1, cell2);
    }

    #[test]
    fn bold_dim_and_normal_are_pairwise_distinct() {
        assert_ne!(Intensity::Normal, Intensity::Bold);
        assert_ne!(Intensity::Bold, Intensity::Dim);
        assert_ne!(Intensity::Normal, Intensity::Dim);
    }

    #[test]
    fn cell_default_alpha_is_zero() {
        assert_eq!(Cell::default().alpha, 0.0);
    }

    #[test]
    fn composite_blends_partial_alpha_between_two_layers() {
        let mut stack = LayerStack::new(1, 1);
        let base = Cell {
            symbol: 'a',
            fg: Color::Rgb { r: 0, g: 0, b: 0 },
            bg: Color::Reset,
            alpha: 1.0,
            ..Default::default()
        };
        stack.set(0, 0, base);
        let top = Cell {
            symbol: 'b',
            fg: Color::Rgb {
                r: 200,
                g: 100,
                b: 50,
            },
            bg: Color::Reset,
            alpha: 0.5,
            ..Default::default()
        };
        stack.push_layer().set(0, 0, top);

        let out = stack.composite();

        // top's contribution = 0.5 * remaining(1.0) = 0.5; base's is the
        // other 0.5 (remaining after top). Exact midpoint.
        assert_eq!(
            out.get(0, 0).fg,
            Color::Rgb {
                r: 100,
                g: 50,
                b: 25
            }
        );
        assert_eq!(out.get(0, 0).symbol, 'b'); // top's contribution (0.5) meets the >= 0.5 threshold
        assert_eq!(out.get(0, 0).alpha, 1.0);
    }

    #[test]
    fn composite_blends_bg_partial_alpha_between_two_layers() {
        // Mirrors composite_blends_partial_alpha_between_two_layers but
        // exercises the bg accumulation branch (structurally identical to
        // fg's) with real Rgb values — previously only ever tested with
        // bg: Color::Reset.
        let mut stack = LayerStack::new(1, 1);
        let base = Cell {
            symbol: 'a',
            fg: Color::Reset,
            bg: Color::Rgb { r: 0, g: 0, b: 0 },
            alpha: 1.0,
            ..Default::default()
        };
        stack.set(0, 0, base);
        let top = Cell {
            symbol: 'b',
            fg: Color::Reset,
            bg: Color::Rgb {
                r: 200,
                g: 100,
                b: 50,
            },
            alpha: 0.5,
            ..Default::default()
        };
        stack.push_layer().set(0, 0, top);

        let out = stack.composite();

        // top's contribution = 0.5 * remaining(1.0) = 0.5; base's is the
        // other 0.5 (remaining after top). Exact midpoint.
        assert_eq!(
            out.get(0, 0).bg,
            Color::Rgb {
                r: 100,
                g: 50,
                b: 25
            }
        );
        assert_eq!(out.get(0, 0).symbol, 'b'); // top's contribution (0.5) meets the >= 0.5 threshold
        assert_eq!(out.get(0, 0).alpha, 1.0);
    }

    #[test]
    fn composite_accumulates_correctly_across_three_partially_transparent_layers() {
        let mut stack = LayerStack::new(1, 1);
        let bottom_fg = Color::Rgb { r: 0, g: 0, b: 0 };
        let mid_fg = Color::Rgb {
            r: 100,
            g: 100,
            b: 100,
        };
        let top_fg = Color::Rgb {
            r: 200,
            g: 200,
            b: 200,
        };
        stack.set(
            0,
            0,
            Cell {
                symbol: 'a',
                fg: bottom_fg,
                bg: Color::Reset,
                alpha: 1.0,
                ..Default::default()
            },
        );
        stack.push_layer().set(
            0,
            0,
            Cell {
                symbol: 'b',
                fg: mid_fg,
                bg: Color::Reset,
                alpha: 0.5,
                ..Default::default()
            },
        );
        stack.push_layer().set(
            0,
            0,
            Cell {
                symbol: 'c',
                fg: top_fg,
                bg: Color::Reset,
                alpha: 0.5,
                ..Default::default()
            },
        );

        let out = stack.composite();

        // Hand-verified accumulation, top to bottom: top ('c') contributes
        // 0.5*1.0=0.5 (remaining -> 0.5); mid ('b') then contributes
        // 0.5*0.5=0.25 (remaining -> 0.25); bottom ('a', fully opaque)
        // claims the last 0.25. Expected fg is computed via the exact same
        // incremental pairwise-lerp steps the implementation performs (not
        // a closed-form average — each step truncates to u8 independently,
        // same as the real algorithm), so this is the algorithm's own
        // formula used as its test oracle, not an independently-derived
        // magic number.
        let expected_fg = {
            let after_mid = crate::easing::lerp_color(top_fg, mid_fg, 0.25 / 0.75); // mid's contribution / total-so-far
            crate::easing::lerp_color(after_mid, bottom_fg, 0.25 / 1.0) // bottom's contribution / total-so-far
        };
        assert_eq!(out.get(0, 0).fg, expected_fg);
        assert_eq!(out.get(0, 0).symbol, 'c'); // topmost to cross the 0.5 threshold
    }

    #[test]
    fn a_fully_opaque_layer_occludes_everything_beneath_it() {
        let mut stack = LayerStack::new(1, 1);
        // Bottom layer's color would show up in the result if (incorrectly) blended in.
        stack.set(
            0,
            0,
            Cell {
                symbol: 'z',
                fg: Color::Rgb { r: 255, g: 0, b: 0 },
                bg: Color::Reset,
                alpha: 1.0,
                ..Default::default()
            },
        );
        stack.push_layer().set(
            0,
            0,
            Cell {
                symbol: 'y',
                fg: Color::Rgb { r: 0, g: 255, b: 0 },
                bg: Color::Reset,
                alpha: 1.0,
                ..Default::default()
            },
        );

        let out = stack.composite();

        assert_eq!(out.get(0, 0).symbol, 'y');
        assert_eq!(out.get(0, 0).fg, Color::Rgb { r: 0, g: 255, b: 0 });
    }

    #[test]
    fn non_rgb_colors_fall_back_to_the_lerp_color_target_not_a_true_blend() {
        let mut stack = LayerStack::new(1, 1);
        stack.set(
            0,
            0,
            Cell {
                symbol: 'a',
                fg: Color::Green,
                bg: Color::Reset,
                alpha: 1.0,
                ..Default::default()
            },
        );
        stack.push_layer().set(
            0,
            0,
            Cell {
                symbol: 'b',
                fg: Color::Yellow,
                bg: Color::Reset,
                alpha: 0.5,
                ..Default::default()
            },
        );

        let out = stack.composite();

        // Neither Green nor Yellow is Color::Rgb. The accumulator seeds to
        // Yellow (top layer), then blends against Green (bottom) via
        // lerp_color, which falls back to returning its `to` argument
        // outright for non-Rgb pairs (per easing.rs) — so the result is
        // Green exactly, not a true yellow/green mix. This is a known,
        // pre-existing lerp_color limitation this spec does not attempt to
        // fix (see the design doc's Non-goals) — this test documents it,
        // not hides it.
        assert_eq!(out.get(0, 0).fg, Color::Green);
    }

    #[test]
    fn glyph_selection_uses_the_first_layer_to_reach_half_contribution() {
        let mut stack = LayerStack::new(1, 1);
        stack.set(
            0,
            0,
            Cell {
                symbol: 'a',
                fg: Color::Reset,
                bg: Color::Reset,
                alpha: 1.0,
                ..Default::default()
            },
        );
        stack.push_layer().set(
            0,
            0,
            Cell {
                symbol: 'b',
                fg: Color::Reset,
                bg: Color::Reset,
                alpha: 0.5,
                ..Default::default()
            },
        );

        let out = stack.composite();
        // top layer's contribution is exactly 0.5 * 1.0 = 0.5, which meets
        // (not just exceeds) the >= 0.5 threshold.
        assert_eq!(out.get(0, 0).symbol, 'b');
    }

    #[test]
    fn glyph_selection_falls_back_to_the_topmost_contributor_when_none_reach_half() {
        let mut stack = LayerStack::new(1, 1);
        stack.set(
            0,
            0,
            Cell {
                symbol: 'a',
                fg: Color::Reset,
                bg: Color::Reset,
                alpha: 0.3,
                ..Default::default()
            },
        );
        stack.push_layer().set(
            0,
            0,
            Cell {
                symbol: 'b',
                fg: Color::Reset,
                bg: Color::Reset,
                alpha: 0.3,
                ..Default::default()
            },
        );

        let out = stack.composite();
        // top ('b') contributes 0.3*1.0=0.3; bottom ('a') then contributes
        // 0.3*0.7=0.21 — neither reaches the 0.5 threshold individually, so
        // the rule falls back to "topmost non-transparent contributor", 'b'.
        assert_eq!(out.get(0, 0).symbol, 'b');
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
            alpha: 1.0,
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
            alpha: 1.0,
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
            alpha: 1.0,
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
            alpha: 1.0,
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
            alpha: 1.0,
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
    fn composite_of_a_single_layer_stack_normalizes_zero_alpha_to_default() {
        let mut stack = LayerStack::new(1, 1);
        // A cell that isn't Cell::default() but carries alpha: 0.0 — per
        // the Cell.alpha doc contract, composite() must still output it as
        // untouched (Cell::default()), even on the single-layer fast path.
        stack.set(
            0,
            0,
            Cell {
                symbol: 'x',
                fg: Color::Rgb { r: 1, g: 2, b: 3 },
                bg: Color::Reset,
                alpha: 0.0,
                ..Default::default()
            },
        );

        let out = stack.composite();

        assert_eq!(*out.get(0, 0), Cell::default());
    }

    #[test]
    fn composite_of_a_single_layer_stack_forces_fractional_alpha_to_one() {
        let mut stack = LayerStack::new(1, 1);
        let cell = Cell {
            symbol: 'x',
            fg: Color::Rgb {
                r: 10,
                g: 20,
                b: 30,
            },
            bg: Color::Reset,
            alpha: 0.3,
            ..Default::default()
        };
        stack.set(0, 0, cell.clone());

        let out = stack.composite();

        // Per the Cell.alpha doc contract, every cell leaving composite()
        // carries alpha 0.0 or 1.0 — the raw 0.3 must not pass through
        // unchanged, even though this is the single-layer fast path.
        assert_eq!(out.get(0, 0).symbol, cell.symbol);
        assert_eq!(out.get(0, 0).fg, cell.fg);
        assert_eq!(out.get(0, 0).bg, cell.bg);
        assert_eq!(out.get(0, 0).style, cell.style);
        assert_eq!(out.get(0, 0).alpha, 1.0);
    }

    #[test]
    fn a_visually_blank_but_alpha_1_cell_occludes_what_is_beneath_it() {
        // Post-migration: alpha (not "looks like Cell::default()") determines
        // transparency. A painted blank space is real content, not an
        // untouched position — see docs/design/specs/core/2026-08-09-cell-alpha-compositing-design.md's
        // addendum for the concrete regression this documents.
        let mut stack = LayerStack::new(1, 1);
        stack.set(
            0,
            0,
            Cell {
                symbol: 'x',
                fg: Color::Rgb {
                    r: 10,
                    g: 20,
                    b: 30,
                },
                bg: Color::Rgb {
                    r: 40,
                    g: 50,
                    b: 60,
                },
                alpha: 1.0,
                ..Default::default()
            },
        );
        stack.push_layer().set(
            0,
            0,
            Cell {
                symbol: ' ',
                fg: Color::Reset,
                bg: Color::Reset,
                alpha: 1.0,
                ..Default::default()
            },
        );

        let out = stack.composite();

        assert_eq!(out.get(0, 0).symbol, ' ');
        assert_eq!(out.get(0, 0).fg, Color::Reset);
        assert_eq!(out.get(0, 0).bg, Color::Reset);
    }

    #[test]
    fn composite_early_exits_once_a_layer_reaches_full_opacity() {
        let mut stack = LayerStack::new(3, 1);
        let a = Cell {
            symbol: 'a',
            fg: Color::Reset,
            bg: Color::Reset,
            alpha: 1.0,
            ..Default::default()
        };
        let b = Cell {
            symbol: 'b',
            fg: Color::Reset,
            bg: Color::Reset,
            alpha: 1.0,
            ..Default::default()
        };
        let c = Cell {
            symbol: 'c',
            fg: Color::Reset,
            bg: Color::Reset,
            alpha: 1.0,
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
            alpha: 1.0,
            ..Default::default()
        };
        let top_cell = Cell {
            symbol: 'b',
            fg: Color::Blue,
            bg: Color::Reset,
            alpha: 1.0,
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
            alpha: 1.0,
            ..Default::default()
        };
        stack.push_layer().set(0, 0, cell.clone());

        assert_eq!(*stack.layer(1).get(0, 0), cell);
    }

    fn marked(symbol: char) -> Cell {
        Cell {
            symbol,
            fg: Color::Reset,
            bg: Color::Reset,
            alpha: 1.0,
            ..Default::default()
        }
    }

    fn filled(width: u16, height: u16, symbol: char) -> Buffer {
        let mut buf = Buffer::new(width, height);
        for y in 0..height {
            for x in 0..width {
                buf.set(x, y, marked(symbol));
            }
        }
        buf
    }

    #[test]
    fn blit_at_the_origin_copies_every_cell() {
        let src = filled(2, 2, 'x');
        let mut dest = Buffer::new(2, 2);

        src.blit(&mut dest, 0, 0);

        for y in 0..2 {
            for x in 0..2 {
                assert_eq!(*dest.get(x, y), marked('x'), "({x}, {y})");
            }
        }
    }

    #[test]
    fn blit_places_the_sources_top_left_at_the_offset() {
        let src = filled(2, 1, 'x');
        let mut dest = Buffer::new(4, 3);

        src.blit(&mut dest, 1, 2);

        assert_eq!(*dest.get(1, 2), marked('x'));
        assert_eq!(*dest.get(2, 2), marked('x'));
        // Everything else, including the cell left of the offset and the
        // row above it, stays default.
        assert_eq!(*dest.get(0, 2), Cell::default());
        assert_eq!(*dest.get(3, 2), Cell::default());
        assert_eq!(*dest.get(1, 1), Cell::default());
    }

    #[test]
    fn blit_writes_only_what_fits_when_the_source_overhangs() {
        let src = filled(4, 4, 'x');
        let mut dest = Buffer::new(3, 2);

        src.blit(&mut dest, 1, 1); // must not panic

        assert_eq!(*dest.get(1, 1), marked('x'));
        assert_eq!(*dest.get(2, 1), marked('x'));
        // The row the source overhangs into does not exist; the row it
        // did not reach stays default.
        assert_eq!(*dest.get(0, 0), Cell::default());
        assert_eq!(*dest.get(0, 1), Cell::default());
    }

    /// #161's shape: in a release build, `Buffer::set` only checks the
    /// flat index, so on a 4x3 buffer `set(5, 0, ..)` indexes `0 * 4 + 5`
    /// and silently writes to `(1, 1)` instead of panicking (debug builds
    /// catch it via `index`'s `debug_assert!`). `blit` clips on `x`
    /// explicitly rather than inheriting that, so an `x` past the right
    /// edge writes nothing and leaves the next row alone, in both build
    /// profiles.
    #[test]
    fn blit_past_the_right_edge_does_not_wrap_onto_the_next_row() {
        let src = filled(2, 1, 'x');
        let mut dest = Buffer::new(4, 3);

        src.blit(&mut dest, 4, 0); // wholly past the right edge

        for y in 0..3 {
            for x in 0..4 {
                assert_eq!(
                    *dest.get(x, y),
                    Cell::default(),
                    "({x}, {y}) was written by an out-of-bounds blit"
                );
            }
        }
    }

    #[test]
    fn blit_partly_past_the_right_edge_clips_the_overhanging_columns() {
        let src = filled(3, 1, 'x');
        let mut dest = Buffer::new(4, 3);

        src.blit(&mut dest, 3, 0);

        assert_eq!(*dest.get(3, 0), marked('x'));
        // The two overhanging columns must not appear anywhere in row 1.
        for x in 0..4 {
            assert_eq!(*dest.get(x, 1), Cell::default(), "(({x}, 1))");
        }
    }

    #[test]
    fn blit_past_the_bottom_edge_writes_nothing() {
        let src = filled(2, 2, 'x');
        let mut dest = Buffer::new(4, 2);

        src.blit(&mut dest, 0, 2); // must not panic

        for y in 0..2 {
            for x in 0..4 {
                assert_eq!(*dest.get(x, y), Cell::default(), "({x}, {y})");
            }
        }
    }

    /// An offset near `u16::MAX` must clip, not overflow the `x + column`
    /// addition (a debug-build panic).
    #[test]
    fn blit_near_u16_max_does_not_overflow_the_offset_arithmetic() {
        let src = filled(4, 4, 'x');
        let mut dest = Buffer::new(4, 4);

        src.blit(&mut dest, u16::MAX - 1, u16::MAX - 1); // must not panic

        assert_eq!(*dest.get(0, 0), Cell::default());
    }

    // debug_assert!-gated: benches/set.rs measured a real `assert!` here at
    // +22.6%/+11.9% (full_paint/single_cell) versus Task 3's baseline, both
    // p < 0.05 ("Performance has regressed") — see #161. The check (and
    // these two tests, which exercise it) exists only in debug builds.
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "out of bounds")]
    fn set_with_an_x_past_the_width_panics_instead_of_wrapping() {
        // Documented as a panic since 1.0, but `index` only checked the
        // flat offset: on a 4x3 buffer, set(5, 0, ..) landed on (1, 1).
        let mut buf = Buffer::new(4, 3);
        let mut cell = buf.get(0, 0).clone();
        cell.symbol = 'X';
        buf.set(5, 0, cell);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "out of bounds")]
    fn get_with_an_x_past_the_width_panics_instead_of_reading_a_neighbour() {
        let buf = Buffer::new(4, 3);
        let _ = buf.get(5, 0);
    }

    #[test]
    fn set_at_the_last_valid_cell_still_works() {
        // Guards against an off-by-one in the new check.
        let mut buf = Buffer::new(4, 3);
        let mut cell = buf.get(0, 0).clone();
        cell.symbol = 'Z';
        buf.set(3, 2, cell);
        assert_eq!(buf.get(3, 2).symbol, 'Z');
    }
}
