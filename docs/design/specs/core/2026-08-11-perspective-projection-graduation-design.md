# Perspective Projection Graduation — Design

**Status:** draft, pending your review before we move to planning.
**Date:** 2026-08-11
**Relationship to prior specs:** graduates the depth & perspective
projection spike
(`2026-08-10-depth-perspective-projection-spike-design.md`, PR #99)
into real, committed, TDD-covered core API, per that spec's
"Recommendations (post-spike)" section — its graduation recommendation
plus the concrete conditions the final review's `tools/visual-snapshot`
capture surfaced (a real `Canvas` color-selection bug, a legibility-
at-distance finding, two robustness landmines). Builds on `Canvas`
(`2026-08-08-rendering-primitives-graduation-design.md`) and
`easing`/`Transition` (unchanged). This Arc's own findings feed the
paused Falcon cockpit-view redesign, which resumes once this lands.

**Dependency:** this Arc assumes PR #99 has already merged to `main` —
it builds directly on that PR's `examples/depth_spike.rs` prototype and
its recommendations write-up. (It has: PR #99 merged before this spec
was written.)

## Context / Motivation

The spike deliberately shipped prototype-quality code exempt from TDD,
living entirely in one example file, explicitly not committed API. Its
recommendations concluded the central hypothesis validated (parallax
genuinely reads as convincing, falling out of the shared projection
math with zero per-object tuning) and recommended graduating to a real
module — conditioned on resolving a real `Canvas` bug the visual
capture found (3 of a test cube's 12 wireframe edges were invisible,
swallowed by fill color) and two documented robustness gaps
(`fill_polygon`'s unbounded scanline loop, `project_line`'s saturating-
not-clipping behavior). This spec is that graduation.

## Scope

**Tag: `coding`.** Full TDD applies, no exceptions — this is committed
core, not a spike.

### 1. `src/perspective.rs` — new module, the projection core

Named to avoid colliding with the existing `src/camera.rs` (2D buffer
dimming, unrelated to 3D projection):

```rust
/// A point in camera-relative 3D space.
#[derive(Clone, Copy, Debug)]
pub struct Point3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// A line segment in camera-relative 3D space.
#[derive(Clone, Copy, Debug)]
pub struct Line3 {
    pub start: Point3,
    pub end: Point3,
}

/// A flat-shaded polygon in camera-relative 3D space — 3+ vertices,
/// in order around the perimeter (not necessarily convex).
#[derive(Clone, Debug)]
pub struct Polygon3 {
    pub vertices: Vec<Point3>,
}

/// A fixed-forward pinhole camera: positioned at the origin, looking
/// down `+Z`. No position/orientation fields — general camera
/// movement stays out of scope (see Non-goals), same as the spike.
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    /// Points at or nearer than this depth are clipped — projection
    /// is undefined at `z = 0` and inverted for `z < 0`.
    pub near: f32,
    /// Controls field of view: larger = more zoomed in/narrower FOV.
    pub focal_length: f32,
}

/// Corrects for terminal cells being roughly twice as tall as wide —
/// same compensation the `Dial` widget already applies to its ring
/// radius. Not a `Camera` field: this is about terminal cell geometry,
/// not the camera itself.
pub const ASPECT_COMPENSATION: f32 = 2.0;

impl Camera {
    /// Projects `p` to `(screen_x, screen_y, scale)` in cell
    /// coordinates relative to `(center_x, center_y)`, where `scale`
    /// grows as the point gets nearer (drives size/brightness
    /// falloff). Returns `None` if `p.z <= self.near`.
    pub fn project(&self, p: Point3, center_x: f32, center_y: f32) -> Option<(f32, f32, f32)> {
        if p.z <= self.near {
            return None;
        }
        let ndc_x = p.x / p.z;
        let ndc_y = p.y / p.z;
        let scale = self.focal_length / p.z;
        let screen_x = center_x + ndc_x * self.focal_length * ASPECT_COMPENSATION;
        let screen_y = center_y - ndc_y * self.focal_length;
        Some((screen_x, screen_y, scale))
    }
}
```

Same formula as the spike, now a method on `Camera` instead of a free
function reading module constants — lets a caller configure `near`/
`focal_length` per scene instead of the spike's hardcoded values.

### 2. Near-plane and LOD rules — unchanged from the spike, made explicit

- **Near-plane clipping stays the spike's simplified rule**: for
  `Line3`/`Polygon3`, if *any* vertex fails `project` (at/behind the
  near plane), the whole shape is skipped — no true 3D segment
  clipping at the near plane. Confirmed out of scope (see Non-goals);
  nothing in this Arc or the paused Falcon redesign needs it.
- **New: a caller-configurable minimum-scale cutoff.** The spike's
  visual capture found a projected cube degenerates into a featureless
  blob once it gets too small/far — not a bug, but a real legibility
  finding. `project_line`/`fill_polygon` (below) each take a `min_scale:
  f32` parameter; if every vertex's `scale` (from `Camera::project`)
  falls below it, the function skips drawing and returns/does nothing,
  same as if the shape were behind the near plane. Callers with no
  opinion pass `0.0` (never skips) — this is opt-in, not a forced
  default.

### 3. `Camera::project_line` — real screen-edge clipping

```rust
impl Camera {
    /// Projects `line` to `Canvas` subpixel coordinates for a canvas
    /// whose subpixel grid is `subpixels_x`/`subpixels_y` per cell and
    /// `screen_w`/`screen_h` cells wide/tall. Returns `None` if either
    /// endpoint is at/behind the near plane, if every vertex's `scale`
    /// is below `min_scale`, or if the projected segment falls
    /// entirely outside the visible `[0, screen_w] x [0, screen_h]`
    /// rectangle. A segment partially outside is clipped to the
    /// visible rectangle (Cohen-Sutherland) before being converted to
    /// subpixel coordinates — no more saturating an off-screen
    /// endpoint to `(0, 0)` and drawing a spurious edge-hugging
    /// segment, the bug the spike's final review found.
    pub fn project_line(
        &self,
        line: Line3,
        center_x: f32,
        center_y: f32,
        screen_w: f32,
        screen_h: f32,
        subpixels_x: f32,
        subpixels_y: f32,
        min_scale: f32,
    ) -> Option<(u16, u16, u16, u16)> {
        let (sx0, sy0, scale0) = self.project(line.start, center_x, center_y)?;
        let (sx1, sy1, scale1) = self.project(line.end, center_x, center_y)?;
        if scale0.max(scale1) < min_scale {
            return None;
        }
        let (cx0, cy0, cx1, cy1) = clip_to_screen(sx0, sy0, sx1, sy1, screen_w, screen_h)?;
        Some((
            (cx0 * subpixels_x).round().clamp(0.0, u16::MAX as f32) as u16,
            (cy0 * subpixels_y).round().clamp(0.0, u16::MAX as f32) as u16,
            (cx1 * subpixels_x).round().clamp(0.0, u16::MAX as f32) as u16,
            (cy1 * subpixels_y).round().clamp(0.0, u16::MAX as f32) as u16,
        ))
    }
}

const INSIDE: u8 = 0;
const LEFT: u8 = 1;
const RIGHT: u8 = 2;
const TOP: u8 = 4;
const BOTTOM: u8 = 8;

fn outcode(x: f32, y: f32, xmax: f32, ymax: f32) -> u8 {
    let mut code = INSIDE;
    if x < 0.0 {
        code |= LEFT;
    } else if x > xmax {
        code |= RIGHT;
    }
    if y < 0.0 {
        code |= TOP;
    } else if y > ymax {
        code |= BOTTOM;
    }
    code
}

/// Clips a line segment to the visible `[0, xmax] x [0, ymax]`
/// rectangle via Cohen-Sutherland. Returns `None` if the segment lies
/// entirely outside.
fn clip_to_screen(
    mut x0: f32,
    mut y0: f32,
    mut x1: f32,
    mut y1: f32,
    xmax: f32,
    ymax: f32,
) -> Option<(f32, f32, f32, f32)> {
    let mut code0 = outcode(x0, y0, xmax, ymax);
    let mut code1 = outcode(x1, y1, xmax, ymax);
    loop {
        if code0 | code1 == 0 {
            return Some((x0, y0, x1, y1));
        }
        if code0 & code1 != 0 {
            return None;
        }
        let out = if code0 != 0 { code0 } else { code1 };
        let (x, y) = if out & TOP != 0 {
            (x0 + (x1 - x0) * (0.0 - y0) / (y1 - y0), 0.0)
        } else if out & BOTTOM != 0 {
            (x0 + (x1 - x0) * (ymax - y0) / (y1 - y0), ymax)
        } else if out & RIGHT != 0 {
            (xmax, y0 + (y1 - y0) * (xmax - x0) / (x1 - x0))
        } else {
            (0.0, y0 + (y1 - y0) * (0.0 - x0) / (x1 - x0))
        };
        if out == code0 {
            x0 = x;
            y0 = y;
            code0 = outcode(x0, y0, xmax, ymax);
        } else {
            x1 = x;
            y1 = y;
            code1 = outcode(x1, y1, xmax, ymax);
        }
    }
}
```

### 4. `Canvas::fill_polygon` — new committed method

Graduates the spike's scanline fill into `Canvas` itself, alongside
`set_pixel`/`line`/`rect`/`fill_rect` — general sub-cell rasterization
capability, not specific to 3D projection, matching where `line`/`rect`
already live. Two fixes over the spike's version: the scanline loop is
clamped to the canvas's own bounds (the spike's unbounded-loop
landmine — a near-camera vertex could previously produce a `max_y` in
the hundreds of thousands and stall a frame), and it takes projected 2D
points directly rather than doing its own projection — the projection
module and `Canvas` stay decoupled, with `Camera::project_polygon`
(below) as the glue.

```rust
impl Canvas {
    /// Fills the polygon described by `points` (subpixel coordinates,
    /// 3+ points, in perimeter order) via an even-odd scanline fill.
    /// Does nothing if fewer than 3 points are given. The scanline
    /// loop is clamped to the canvas's own valid row range regardless
    /// of the input points' range, so a point far outside the canvas
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
            xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let mut i = 0;
            while i + 1 < xs.len() {
                let x_start = xs[i].round().max(0.0) as u16;
                let x_end = xs[i + 1].round().max(0.0) as u16;
                for x in x_start..=x_end {
                    self.set_pixel(x, y, color);
                }
                i += 2;
            }
        }
    }
}
```

`Camera` gains the projection-side glue, mirroring `project_line`'s
shape:

```rust
impl Camera {
    /// Projects `polygon`'s vertices to `Canvas` subpixel coordinates,
    /// ready for `Canvas::fill_polygon`. Returns `None` under the same
    /// conditions as `project_line` (any vertex behind the near plane,
    /// every vertex's scale below `min_scale`) — no screen-edge
    /// clipping for polygons this Arc (see Non-goals), only the
    /// existing near-plane/LOD skip rules.
    pub fn project_polygon(
        &self,
        polygon: &Polygon3,
        center_x: f32,
        center_y: f32,
        subpixels_x: f32,
        subpixels_y: f32,
        min_scale: f32,
    ) -> Option<Vec<(f32, f32)>> {
        let mut points = Vec::with_capacity(polygon.vertices.len());
        let mut max_scale = 0.0f32;
        for &v in &polygon.vertices {
            let (sx, sy, scale) = self.project(v, center_x, center_y)?;
            max_scale = max_scale.max(scale);
            points.push((sx * subpixels_x, sy * subpixels_y));
        }
        if max_scale < min_scale {
            return None;
        }
        Some(points)
    }
}
```

### 5. `Canvas`'s Braille color selection — bug fix, per-cell write-order tracking

The spike's final review found 3 of a test cube's 12 wireframe edges
invisible: `blit_braille` picks a cell's color from whichever subpixel
the scan loop (`for row in 0..4 { for col in 0..2 { ... } }`) happens
to visit *last*, not whichever was *written* last — despite an
existing comment on that line claiming "last-write-wins per cell." A
wireframe edge landing inside an already-filled region loses its color
to the fill unless it happens to land on the specific subpixel the scan
order visits last.

Fix: track a write-sequence number per subpixel, and have `blit_braille`
pick the highest-sequence (most recently written) color among a cell's
set subpixels, not whichever the scan happens to touch last.

```rust
pub struct Canvas {
    width: u16,
    height: u16,
    mode: CanvasMode,
    subpixels_x: u16,
    subpixels_y: u16,
    grid: Vec<Option<(Color, u64)>>, // was: Vec<Option<Color>>
    next_seq: u64,                   // new
}
```

`new()` additionally initializes `next_seq: 0`. `set_pixel` becomes:

```rust
pub fn set_pixel(&mut self, x: u16, y: u16, color: Color) {
    if x < self.grid_width() && y < self.grid_height() {
        let idx = self.index(x, y);
        let seq = self.next_seq;
        self.next_seq += 1;
        self.grid[idx] = Some((color, seq));
    }
}
```

`clear_pixel` is unchanged (still sets the slot to `None`). `blit_half_block`
extracts just the color from each subpixel tuple (`.map(|(c, _)| c)`) —
write order doesn't matter there, since half-block mode always shows
both subpixels distinctly rather than picking one winner. `blit_braille`'s
inner loop changes from unconditionally overwriting `color` to comparing
sequence numbers:

```rust
let mut winner: Option<(Color, u64)> = None;
for row in 0..4u16 {
    for col in 0..2u16 {
        let px = cx * 2 + col;
        let py = cy * 4 + row;
        if let Some((c, seq)) = self.grid[self.index(px, py)] {
            mask |= DOT_BITS[row as usize][col as usize];
            if winner.map(|(_, best)| seq > best).unwrap_or(true) {
                winner = Some((c, seq));
            }
        }
    }
}
```

with the cell's final color read from `winner.unwrap().0` instead of
the old `color.unwrap()`.

**This is a real bug fix to already-committed, tested code** — the
existing `braille_last_written_dot_wins_the_cells_color` test
(`src/canvas.rs`) does not actually catch this class of bug: its two
`set_pixel` calls happen to already agree on write order and scan
order, so it can't distinguish "last written" from "last scanned." A
new regression test is required (see Testing).

## Non-goals

- **General camera movement/rotation.** `Camera` stays fixed-forward
  (position/orientation are not fields) — confirmed unchanged from the
  spike; nothing in this Arc or the paused Falcon redesign needs it.
- **True near-plane segment/polygon clipping.** The simplified
  "any vertex behind the near plane skips the whole shape" rule stays,
  confirmed unchanged from the spike.
- **Screen-edge clipping for polygons.** `project_polygon` gets the
  same near-plane/LOD skip as `project_line`, but not screen-edge
  clipping — `Canvas::fill_polygon`'s scanline loop is already clamped
  to the canvas's own bounds (item 4 above), which is sufficient to
  prevent the unbounded-loop landmine; a polygon straddling the screen
  edge simply fills whatever portion of it lands on-canvas, same as
  `fill_rect` already does today for an out-of-bounds rectangle. Only
  `project_line` gets true edge clipping, since a spurious edge-hugging
  *line* segment (the bug found) is visually obvious in a way a
  partially-off-canvas *fill* is not.
- **Falcon integration.** Building the actual cockpit-view redesign
  with this module is the next Arc, not this one.
- **The other three brainstormed directions** (audio, advanced input,
  data-viz widgets) — unrelated, separate future Arcs.

## Testing

Per `.claude/rules/development-conventions.md`: `coding`-tagged, TDD
mandatory, no exceptions.

- **`Camera::project`** — the spike's own hand-traced test vectors
  (four points at `z` in `{2.0, 5.0, 10.0, 0.2}` at fixed `x`/`y`,
  confirming monotonically-decreasing offset-from-center and
  monotonically-decreasing `scale` as `z` grows, and `None` for the
  behind-near-plane point) become real `#[test]` cases instead of a
  reviewer's hand-trace.
- **`clip_to_screen`** — a line fully inside (unchanged), fully outside
  in each of the four directions (`None`), and a line crossing exactly
  one edge (clipped endpoint lands exactly on the boundary) and exactly
  two edges (a corner-crossing case) — standard Cohen-Sutherland
  coverage.
- **`Camera::project_line`** — combines clipping with the near-plane/
  `min_scale` skip rules: both endpoints visible (unchanged), one
  endpoint behind the near plane (`None`), both endpoints' scale below
  `min_scale` (`None`), a line partially off-screen (clipped, not
  saturated to `(0,0)` — the spike's own regression case, reproduced
  as a real test).
- **`Canvas::fill_polygon`** — the spike's own hand-traced scanline
  rows (from the final task review's verification) become real test
  cases; plus a case confirming the scanline loop never iterates past
  `self.grid_height()` even when given points with `y` far outside
  the canvas (the robustness fix, not just a correctness check).
- **`Camera::project_polygon`** — near-plane/`min_scale` skip rules,
  mirroring `project_line`'s tests.
- **`Canvas` Braille color fix** — the existing
  `braille_last_written_dot_wins_the_cells_color` test stays (still
  correct, just not sufficient on its own); a new test writes to a
  scan-order-*last* subpixel first and a scan-order-*first* subpixel
  second, asserting the *second* (chronologically later) color wins —
  the actual regression test for the bug the spike's visual capture
  found, since the existing test's two writes happen to agree on scan
  order and write order and so cannot distinguish the two rules.
- **`Canvas::blit_half_block`** — confirm it still behaves identically
  after the `grid` representation change (existing tests should pass
  unmodified once the tuple-unwrapping change lands; no new test
  needed here beyond confirming the existing suite is unaffected).

## Critical files

- `src/perspective.rs` — new: `Point3`/`Line3`/`Polygon3`/`Camera`,
  `project`/`project_line`/`project_polygon`, `clip_to_screen`.
- `src/lib.rs` — `pub mod perspective;`.
- `src/canvas.rs` — `fill_polygon` (new method), `grid`'s
  representation change (`Option<Color>` → `Option<(Color, u64)>`),
  `next_seq` field, `set_pixel`/`blit_half_block`/`blit_braille`
  updates, new regression test.

## Verification

- `cargo test` — full suite green, including all new tests above.
- `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` —
  clean (hard gates — no spike-style exemption applies to this Arc).
- `cargo build --examples` — `examples/depth_spike.rs` is **not**
  migrated to consume the new module in this Arc (it stays as
  prototype-quality reference code demonstrating the math that's now
  graduated — rewriting it to import `src/perspective.rs` instead of
  its own copy is optional cleanup, not required); confirm it still
  builds unmodified.
- `cargo run -p visual-snapshot -- --example depth_spike ...` is **not**
  required for this Arc (no example's `view()`/`on_tick()` changes) —
  this Arc is a pure `src/` addition plus one `Canvas` bug fix, and the
  bug fix's correctness is established by the new regression test
  above, not a fresh visual capture. The next Arc (Falcon redesign,
  once it actually renders something with this module) is where a
  visual-snapshot check becomes mandatory again per
  `.claude/rules/development-conventions.md`.
