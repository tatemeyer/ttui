# Perspective Projection Graduation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Graduate the depth & perspective projection spike into a real, TDD-covered `src/perspective.rs` module plus a `Canvas::fill_polygon` method, resolving the spike's own documented conditions: real screen-edge clipping for lines, a caller-configurable minimum-scale LOD cutoff, a bounds-clamped scanline fill, and a fix for the `Canvas` Braille last-write-wins bug the spike's visual capture found.

**Architecture:** One new module (`src/perspective.rs`: `Point3`/`Line3`/`Polygon3`/`Camera`) plus two changes to the already-committed `src/canvas.rs` (`fill_polygon`, and a `grid` representation change that fixes the Braille color bug). No `Cell`/`LayerStack`/`Buffer` changes. `examples/depth_spike.rs` is untouched — it stays as prototype reference code.

**Tech Stack:** Rust, existing `ttui` core (`buffer`, `canvas`).

## Global Constraints

- **Tag: `coding`. Full TDD applies to every task — no exceptions.** This is committed core, not a spike.
- `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` are hard gates on every task.
- No new dependency.
- One worktree for this whole plan, created via `superpowers:using-git-worktrees` before Task 1, per `.claude/rules/git-github-standards.md`.
- `coding`-tagged → **Gated** autonomy tier: ships as a PR to `main` with all four required checks green, squash-merged at the end.
- `tools/visual-snapshot` is **not required** for this plan — no example's `view()`/`on_tick()` changes (per `.claude/rules/development-conventions.md`'s "Visual review" section, the mandate applies to rendering-affecting `src/` code or an example's render loop; this plan touches `src/canvas.rs` and adds `src/perspective.rs`, both covered by real unit tests instead — the next Arc, which actually renders something with this module, is where a visual capture becomes mandatory again).
- Spec being implemented: `docs/design/specs/core/2026-08-11-perspective-projection-graduation-design.md`.

---

### Task 1: `src/perspective.rs` — `Point3`/`Line3`/`Polygon3`/`Camera::project`

**Files:**
- Create: `src/perspective.rs`
- Modify: `src/lib.rs` (register the module)

**Interfaces:**
- Consumes: nothing new (pure math, no dependency on `buffer`/`canvas`).
- Produces: `Point3`, `Line3`, `Polygon3`, `Camera { near, focal_length }`, `Camera::project`, `ASPECT_COMPENSATION` — Tasks 2-3 add methods to the same `Camera` type in this same file.

- [ ] **Step 1: Write the failing tests**

Create `src/perspective.rs` with just the doc comment, types, and test module first:

```rust
//! Fixed-forward pinhole-camera projection — points, lines, and
//! polygons in camera-relative 3D space project to 2D screen
//! coordinates plus a depth-derived scale. Graduated from the depth &
//! perspective projection spike
//! (docs/design/specs/core/2026-08-10-depth-perspective-projection-spike-design.md)
//! per
//! docs/design/specs/core/2026-08-11-perspective-projection-graduation-design.md.
//! No camera position/orientation — see that spec's Non-goals.

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
/// movement is out of scope.
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    /// Points at or nearer than this depth are clipped.
    pub near: f32,
    /// Controls field of view: larger = more zoomed in/narrower FOV.
    pub focal_length: f32,
}

/// Corrects for terminal cells being roughly twice as tall as wide —
/// same compensation the `Dial` widget already applies to its ring
/// radius.
pub const ASPECT_COMPENSATION: f32 = 2.0;

#[cfg(test)]
mod tests {
    use super::*;

    fn camera() -> Camera {
        Camera {
            near: 0.5,
            focal_length: 8.0,
        }
    }

    #[test]
    fn point_behind_the_near_plane_returns_none() {
        let cam = camera();
        let p = Point3 {
            x: 4.0,
            y: 0.0,
            z: 0.2,
        };
        assert_eq!(cam.project(p, 10.0, 5.0), None);
    }

    #[test]
    fn point_exactly_at_the_near_plane_returns_none() {
        let cam = camera();
        let p = Point3 {
            x: 4.0,
            y: 0.0,
            z: 0.5,
        };
        assert_eq!(cam.project(p, 10.0, 5.0), None);
    }

    #[test]
    fn nearer_points_project_farther_from_center_and_with_larger_scale() {
        let cam = camera();
        let center_x = 10.0;
        let center_y = 5.0;

        let (x2, y2, s2) = cam
            .project(
                Point3 {
                    x: 4.0,
                    y: 0.0,
                    z: 2.0,
                },
                center_x,
                center_y,
            )
            .unwrap();
        let (x4, y4, s4) = cam
            .project(
                Point3 {
                    x: 4.0,
                    y: 0.0,
                    z: 4.0,
                },
                center_x,
                center_y,
            )
            .unwrap();
        let (x8, y8, s8) = cam
            .project(
                Point3 {
                    x: 4.0,
                    y: 0.0,
                    z: 8.0,
                },
                center_x,
                center_y,
            )
            .unwrap();

        // Hand-verified (all inputs are powers of 2, so every
        // intermediate value is exactly representable in binary
        // floating point — no epsilon needed):
        // z=2: ndc_x=2.0, scale=4.0, screen_x=10+2.0*8*2=42.0
        // z=4: ndc_x=1.0, scale=2.0, screen_x=10+1.0*8*2=26.0
        // z=8: ndc_x=0.5, scale=1.0, screen_x=10+0.5*8*2=18.0
        assert_eq!((x2, s2), (42.0, 4.0));
        assert_eq!((x4, s4), (26.0, 2.0));
        assert_eq!((x8, s8), (18.0, 1.0));
        assert_eq!(y2, 5.0);
        assert_eq!(y4, 5.0);
        assert_eq!(y8, 5.0);

        // Farther points land closer to center_x (convergence toward
        // the vanishing point) and have smaller scale (dimmer/smaller).
        assert!((x8 - center_x).abs() < (x4 - center_x).abs());
        assert!((x4 - center_x).abs() < (x2 - center_x).abs());
        assert!(s8 < s4 && s4 < s2);
    }

    #[test]
    fn positive_world_y_maps_to_a_smaller_screen_y() {
        let cam = camera();
        // z=4.0, y=2.0: ndc_y=0.5, screen_y=5.0-0.5*8.0=1.0 (exact).
        let (x, y, _) = cam
            .project(
                Point3 {
                    x: 0.0,
                    y: 2.0,
                    z: 4.0,
                },
                10.0,
                5.0,
            )
            .unwrap();
        assert_eq!(x, 10.0); // ndc_x = 0 -> no horizontal offset
        assert_eq!(y, 1.0);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib perspective::tests`
Expected: FAIL to compile — `Camera::project` doesn't exist yet, and the module isn't registered in `src/lib.rs` yet either.

- [ ] **Step 3: Implement `Camera::project`**

Add to `src/perspective.rs`, after the `ASPECT_COMPENSATION` constant (before the test module):

```rust
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

- [ ] **Step 4: Register the module**

In `src/lib.rs`, add in alphabetical order (after `pub mod particles;`, before `pub mod terminal;` — check the actual current ordering and insert correctly):

```rust
/// Fixed-forward pinhole-camera projection: points, lines, polygons.
pub mod perspective;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --lib perspective::tests`
Expected: PASS — all 4 tests.

- [ ] **Step 6: Run clippy and fmt**

Run: `cargo clippy --lib -- -D warnings` and `cargo fmt --check -- src/perspective.rs src/lib.rs`
Expected: both clean.

- [ ] **Step 7: Commit**

```bash
git add src/perspective.rs src/lib.rs
git commit -m "feat(core): add Point3/Line3/Polygon3/Camera::project

Graduates the depth & perspective projection spike's core math into
a real, TDD-covered module — a fixed-forward pinhole camera projecting
3D points to 2D screen position plus a depth-derived scale. No camera
position/orientation; general camera movement stays out of scope."
```

---

### Task 2: `Camera::project_line` — real screen-edge clipping

**Files:**
- Modify: `src/perspective.rs`

**Interfaces:**
- Consumes: `Camera::project` (Task 1).
- Produces: `Camera::project_line`, `clip_to_screen` (private) — Task 3's `project_polygon` follows the same near-plane/`min_scale` pattern (but not screen-edge clipping, per the spec's Non-goals).

- [ ] **Step 1: Write the failing tests**

Add to `src/perspective.rs`'s `#[cfg(test)] mod tests`, after the existing tests:

```rust
    fn assert_close(actual: f32, expected: f32, label: &str) {
        assert!(
            (actual - expected).abs() < 0.001,
            "{label}: expected {expected}, got {actual}"
        );
    }

    #[test]
    fn clip_to_screen_leaves_a_fully_inside_line_unchanged() {
        let result = clip_to_screen(2.0, 2.0, 8.0, 8.0, 10.0, 10.0);
        assert_eq!(result, Some((2.0, 2.0, 8.0, 8.0)));
    }

    #[test]
    fn clip_to_screen_rejects_a_line_fully_outside_on_each_side() {
        assert_eq!(clip_to_screen(-5.0, 3.0, -1.0, 3.0, 10.0, 10.0), None); // left
        assert_eq!(clip_to_screen(15.0, 3.0, 20.0, 3.0, 10.0, 10.0), None); // right
        assert_eq!(clip_to_screen(3.0, -5.0, 3.0, -1.0, 10.0, 10.0), None); // top
        assert_eq!(clip_to_screen(3.0, 15.0, 3.0, 20.0, 10.0, 10.0), None); // bottom
    }

    #[test]
    fn clip_to_screen_clips_a_single_edge_crossing() {
        // (5,5) is inside; (15,5) is outside past the right edge.
        let result = clip_to_screen(5.0, 5.0, 15.0, 5.0, 10.0, 10.0);
        assert_eq!(result, Some((5.0, 5.0, 10.0, 5.0)));
    }

    #[test]
    fn clip_to_screen_resolves_a_two_edge_corner_crossing() {
        // (20,15) is outside past both the right and bottom edges;
        // Cohen-Sutherland must resolve this through two clip
        // iterations (an intermediate point against one edge that's
        // still outside via the other) before landing inside.
        let result = clip_to_screen(20.0, 15.0, 5.0, 5.0, 10.0, 10.0);
        let (cx0, cy0, cx1, cy1) = result.expect("line intersects the box");
        assert_close(cx0, 10.0, "clipped start.x (right edge)");
        assert!(
            (0.0..=10.0).contains(&cy0),
            "clipped start.y should be within the box, got {cy0}"
        );
        assert_eq!(cx1, 5.0);
        assert_eq!(cy1, 5.0);
    }

    #[test]
    fn project_line_returns_none_when_either_endpoint_is_behind_the_near_plane() {
        let cam = camera();
        let line = Line3 {
            start: Point3 { x: -1.0, y: 0.0, z: 4.0 },
            end: Point3 { x: 1.0, y: 0.0, z: 0.2 },
        };
        assert_eq!(
            cam.project_line(line, 5.0, 5.0, 10.0, 10.0, 2.0, 4.0, 0.0),
            None
        );
    }

    #[test]
    fn project_line_returns_none_when_every_vertexs_scale_is_below_min_scale() {
        let cam = camera();
        // Both points at z=100: scale = 8.0/100.0 = 0.08.
        let line = Line3 {
            start: Point3 { x: -1.0, y: 0.0, z: 100.0 },
            end: Point3 { x: 1.0, y: 0.0, z: 100.0 },
        };
        assert_eq!(
            cam.project_line(line, 5.0, 5.0, 10.0, 10.0, 2.0, 4.0, 0.1),
            None
        );
    }

    #[test]
    fn project_line_projects_a_fully_visible_line_to_subpixel_coordinates() {
        let cam = camera();
        // Both points at z=4.0: x=-1 -> ndc_x=-0.25 -> screen_x=5+(-0.25*8*2)=1.0
        //                        x=1  -> ndc_x=0.25  -> screen_x=5+(0.25*8*2)=9.0
        // Both within [0,10]x[0,10] -> no clipping. Subpixel factors (2.0,4.0):
        // (1.0*2, 5.0*4, 9.0*2, 5.0*4) = (2, 20, 18, 20).
        let line = Line3 {
            start: Point3 { x: -1.0, y: 0.0, z: 4.0 },
            end: Point3 { x: 1.0, y: 0.0, z: 4.0 },
        };
        let result = cam.project_line(line, 5.0, 5.0, 10.0, 10.0, 2.0, 4.0, 0.0);
        assert_eq!(result, Some((2, 20, 18, 20)));
    }

    #[test]
    fn project_line_clips_instead_of_saturating_an_off_screen_endpoint() {
        let cam = camera();
        // x=20 at z=4.0: ndc_x=5.0, screen_x=5+5.0*8*2=85.0 (far off-screen).
        // x=0 at z=4.0: screen_x=5.0 (on-screen, at y=5.0 both).
        // Real clipping lands the off-screen endpoint at screen_x=10.0
        // (the right edge), not saturated to 0 (the spike's bug).
        let line = Line3 {
            start: Point3 { x: 20.0, y: 0.0, z: 4.0 },
            end: Point3 { x: 0.0, y: 0.0, z: 4.0 },
        };
        let result = cam
            .project_line(line, 5.0, 5.0, 10.0, 10.0, 2.0, 4.0, 0.0)
            .expect("line crosses into the visible screen");
        // Clipped start lands at screen (10.0, 5.0) -> subpixel (20, 20).
        assert_eq!(result, (20, 20, 10, 20));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib perspective::tests`
Expected: FAIL to compile — `clip_to_screen`/`project_line` don't exist yet.

- [ ] **Step 3: Implement `clip_to_screen` and `Camera::project_line`**

Add to `src/perspective.rs`, after the `Camera::project` `impl` block (before the test module):

```rust
const OUTCODE_INSIDE: u8 = 0;
const OUTCODE_LEFT: u8 = 1;
const OUTCODE_RIGHT: u8 = 2;
const OUTCODE_TOP: u8 = 4;
const OUTCODE_BOTTOM: u8 = 8;

fn outcode(x: f32, y: f32, xmax: f32, ymax: f32) -> u8 {
    let mut code = OUTCODE_INSIDE;
    if x < 0.0 {
        code |= OUTCODE_LEFT;
    } else if x > xmax {
        code |= OUTCODE_RIGHT;
    }
    if y < 0.0 {
        code |= OUTCODE_TOP;
    } else if y > ymax {
        code |= OUTCODE_BOTTOM;
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
        let (x, y) = if out & OUTCODE_TOP != 0 {
            (x0 + (x1 - x0) * (0.0 - y0) / (y1 - y0), 0.0)
        } else if out & OUTCODE_BOTTOM != 0 {
            (x0 + (x1 - x0) * (ymax - y0) / (y1 - y0), ymax)
        } else if out & OUTCODE_RIGHT != 0 {
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

impl Camera {
    /// Projects `line` to `Canvas` subpixel coordinates for a canvas
    /// whose subpixel grid is `subpixels_x`/`subpixels_y` per cell and
    /// `screen_w`/`screen_h` cells wide/tall. Returns `None` if either
    /// endpoint is at/behind the near plane, if every vertex's `scale`
    /// is below `min_scale`, or if the projected segment falls
    /// entirely outside the visible screen. A segment partially
    /// outside is clipped to the visible rectangle before being
    /// converted to subpixel coordinates.
    #[allow(clippy::too_many_arguments)]
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib perspective::tests`
Expected: PASS — all tests, including the 8 new ones from this task.

- [ ] **Step 5: Run clippy and fmt**

Run: `cargo clippy --lib -- -D warnings` and `cargo fmt --check -- src/perspective.rs`
Expected: both clean.

- [ ] **Step 6: Commit**

```bash
git add src/perspective.rs
git commit -m "feat(core): add Camera::project_line with real screen-edge clipping

Cohen-Sutherland clipping against the visible screen rectangle,
replacing the spike's saturating-off-screen-to-(0,0) shortcut, which
could draw a spurious edge-hugging line segment. Also adds the
caller-configurable min_scale legibility cutoff."
```

---

### Task 3: `Canvas::fill_polygon` + `Camera::project_polygon`

**Files:**
- Modify: `src/canvas.rs`
- Modify: `src/perspective.rs`

**Interfaces:**
- Consumes: `Camera::project` (Task 1), `Canvas::set_pixel`/`grid_height` (existing).
- Produces: `Canvas::fill_polygon(&mut self, points: &[(f32,f32)], color: Color)`, `Camera::project_polygon` — no later task depends on these beyond final verification.

- [ ] **Step 1: Write the failing tests for `Canvas::fill_polygon`**

Add to `src/canvas.rs`'s `#[cfg(test)] mod tests`, after the existing tests:

```rust
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
    fn fill_polygon_scanline_loop_stays_bounded_despite_huge_input_coordinates() {
        // A vertex with y far outside the canvas (simulating what a
        // near-camera projected point could produce) must not cause
        // the scanline loop to iterate beyond the canvas's own rows —
        // the spike's own unbounded-loop landmine, fixed at the Canvas
        // level regardless of what a caller's projection produces.
        let mut c = Canvas::new(2, 2, CanvasMode::HalfBlock); // grid 2x4
        c.fill_polygon(
            &[(0.0, 0.0), (0.0, 1_000_000.0), (2.0, 1_000_000.0), (2.0, 0.0)],
            red(),
        );
        let mut buf = Buffer::new(2, 2);
        c.blit(&mut buf, 0, 0); // must return promptly (bounded loop), not stall
        for cy in 0..2 {
            for cx in 0..2 {
                assert_eq!(buf.get(cx, cy).symbol, '█', "cell ({cx},{cy})");
            }
        }
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib canvas::tests`
Expected: FAIL to compile — `fill_polygon` doesn't exist yet.

- [ ] **Step 3: Implement `Canvas::fill_polygon`**

Add to `src/canvas.rs`'s `impl Canvas` block, after `fill_rect`:

```rust
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
```

`grid_height()` is an existing private method on `Canvas` (already used elsewhere in this file) — no new accessor needed.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib canvas::tests`
Expected: PASS — all tests, including the 3 new `fill_polygon` ones.

- [ ] **Step 5: Write the failing tests for `Camera::project_polygon`**

Add to `src/perspective.rs`'s `#[cfg(test)] mod tests`, after the `project_line` tests:

```rust
    #[test]
    fn project_polygon_returns_none_when_any_vertex_is_behind_the_near_plane() {
        let cam = camera();
        let poly = Polygon3 {
            vertices: vec![
                Point3 { x: -1.0, y: -1.0, z: 4.0 },
                Point3 { x: -1.0, y: 1.0, z: 4.0 },
                Point3 { x: 1.0, y: 0.0, z: 0.2 },
            ],
        };
        assert_eq!(cam.project_polygon(&poly, 5.0, 5.0, 2.0, 4.0, 0.0), None);
    }

    #[test]
    fn project_polygon_returns_none_when_every_vertexs_scale_is_below_min_scale() {
        let cam = camera();
        let poly = Polygon3 {
            vertices: vec![
                Point3 { x: -1.0, y: -1.0, z: 100.0 },
                Point3 { x: -1.0, y: 1.0, z: 100.0 },
                Point3 { x: 1.0, y: 0.0, z: 100.0 },
            ],
        };
        assert_eq!(cam.project_polygon(&poly, 5.0, 5.0, 2.0, 4.0, 0.1), None);
    }

    #[test]
    fn project_polygon_projects_every_vertex_when_visible() {
        let cam = camera();
        // z=4.0, x=0.0, y=0.0 -> ndc=(0,0) -> screen=(5.0,5.0) -> subpixel (10.0,20.0).
        let poly = Polygon3 {
            vertices: vec![Point3 { x: 0.0, y: 0.0, z: 4.0 }],
        };
        let result = cam
            .project_polygon(&poly, 5.0, 5.0, 2.0, 4.0, 0.0)
            .expect("single visible vertex projects");
        assert_eq!(result, vec![(10.0, 20.0)]);
    }
```

- [ ] **Step 6: Run tests to verify they fail**

Run: `cargo test --lib perspective::tests`
Expected: FAIL to compile — `project_polygon` doesn't exist yet.

- [ ] **Step 7: Implement `Camera::project_polygon`**

Add to `src/perspective.rs`'s `impl Camera` block (after `project_line`):

```rust
    /// Projects `polygon`'s vertices to `Canvas` subpixel coordinates,
    /// ready for `Canvas::fill_polygon`. Returns `None` if any vertex
    /// is at/behind the near plane, or if every vertex's `scale` is
    /// below `min_scale` — no screen-edge clipping for polygons (see
    /// this Arc's design spec Non-goals).
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
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `cargo test --lib perspective::tests`
Expected: PASS — all tests, including the 3 new `project_polygon` ones.

- [ ] **Step 9: Run clippy and fmt**

Run: `cargo clippy --lib -- -D warnings` and `cargo fmt --check -- src/canvas.rs src/perspective.rs`
Expected: both clean.

- [ ] **Step 10: Commit**

```bash
git add src/canvas.rs src/perspective.rs
git commit -m "feat(core): add Canvas::fill_polygon and Camera::project_polygon

Graduates the spike's scanline fill into Canvas alongside
set_pixel/line/rect/fill_rect, with its scanline loop now clamped to
the canvas's own bounds (the spike's unbounded-loop landmine). The
projection module stays decoupled from Canvas: project_polygon hands
back plain 2D points, Canvas::fill_polygon consumes them."
```

---

### Task 4: `Canvas`'s Braille color selection — last-write-wins bug fix

**Files:**
- Modify: `src/canvas.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces: no new public API — `set_pixel`'s and `blit`'s existing signatures are unchanged; only `Canvas`'s internal `grid` representation and `blit_braille`'s color-selection logic change.

- [ ] **Step 1: Write the failing test**

Add to `src/canvas.rs`'s `#[cfg(test)] mod tests`, immediately after the existing `braille_last_written_dot_wins_the_cells_color` test:

```rust
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
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --lib canvas::tests::braille_last_written_wins_even_when_earlier_in_scan_order`
Expected: FAIL — the current `blit_braille` picks whichever subpixel the scan visits last (here, `(1,3)`/red), not whichever was written last (`(0,0)`/blue), so this assertion fails against today's code.

- [ ] **Step 3: Change `grid`'s representation to track write order**

In `src/canvas.rs`, change the `Canvas` struct:

```rust
pub struct Canvas {
    width: u16,
    height: u16,
    mode: CanvasMode,
    subpixels_x: u16,
    subpixels_y: u16,
    grid: Vec<Option<Color>>, // len = grid_width() * grid_height()
}
```

to:

```rust
pub struct Canvas {
    width: u16,
    height: u16,
    mode: CanvasMode,
    subpixels_x: u16,
    subpixels_y: u16,
    grid: Vec<Option<(Color, u64)>>, // (color, write-sequence number)
    next_seq: u64,
}
```

In `Canvas::new`, change the final struct literal to add `next_seq: 0,` alongside the existing fields (the `grid: vec![None; grid_w * grid_h]` line's element type now infers as `Option<(Color, u64)>` automatically from the struct's new field type — no change needed to that line itself).

- [ ] **Step 4: Update `set_pixel` to stamp a write-sequence number**

Change:

```rust
    pub fn set_pixel(&mut self, x: u16, y: u16, color: Color) {
        if x < self.grid_width() && y < self.grid_height() {
            let idx = self.index(x, y);
            self.grid[idx] = Some(color);
        }
    }
```

to:

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

`clear_pixel` needs no change (`self.grid[idx] = None;` already works for either element type).

- [ ] **Step 5: Update `blit_half_block` to unwrap the new tuple shape**

Change the two lines:

```rust
                let top = self.grid[self.index(cx, cy * 2)];
                let bottom = self.grid[self.index(cx, cy * 2 + 1)];
```

to:

```rust
                let top = self.grid[self.index(cx, cy * 2)].map(|(c, _)| c);
                let bottom = self.grid[self.index(cx, cy * 2 + 1)].map(|(c, _)| c);
```

The rest of `blit_half_block` is unchanged — `top`/`bottom` are still plain `Option<Color>` after this line, matching what the existing `match (top, bottom) { ... }` below already expects.

- [ ] **Step 6: Fix `blit_braille`'s color selection to track write order**

Change:

```rust
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
                            alpha: 1.0,
                        },
                    );
                }
```

to:

```rust
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
```

- [ ] **Step 7: Run tests to verify they pass**

Run: `cargo test --lib canvas::tests`
Expected: PASS — the whole `canvas` test module, including both the pre-existing `braille_last_written_dot_wins_the_cells_color` test (still correct, now genuinely exercising write order rather than coincidentally agreeing with scan order) and the new regression test from Step 1.

- [ ] **Step 8: Run clippy and fmt**

Run: `cargo clippy --lib -- -D warnings` and `cargo fmt --check -- src/canvas.rs`
Expected: both clean.

- [ ] **Step 9: Commit**

```bash
git add src/canvas.rs
git commit -m "fix(core): make Canvas Braille color selection genuinely last-write-wins

blit_braille picked a cell's color from whichever subpixel the
row/col scan visited last, not whichever was actually written last —
despite a comment claiming last-write-wins. A wireframe edge landing
inside an already-filled region lost its color to the fill unless it
happened to land on that specific subpixel, found by the depth &
perspective spike's final review via a real tools/visual-snapshot
capture (3 of a test cube's 12 wireframe edges were invisible).
Fixed by stamping a write-sequence number per subpixel and picking
the highest-sequence color among a cell's set subpixels."
```

---

### Task 5: Final verification

**Files:** none (verification only).

- [ ] **Step 1: Full test suite**

Run: `cargo test`
Expected: full suite green, including every new test from Tasks 1-4.

- [ ] **Step 2: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
Expected: both clean.

- [ ] **Step 3: Build every target**

Run: `cargo build --all-targets`
Expected: succeeds, including `examples/depth_spike.rs` (untouched by this plan, confirming this Arc introduced no accidental breakage to the spike's own reference code).

- [ ] **Step 4: Confirm no visual-snapshot regression risk**

`examples/depth_spike.rs` was not modified to consume `src/perspective.rs`/the new `Canvas::fill_polygon` in this plan (deliberate — see this plan's Global Constraints). Since no example's rendering behavior changed, `tools/visual-snapshot` is not required for this plan's own verification. Note this plainly in your final report rather than skipping the question — the next Arc (Falcon redesign, once it actually renders with this module) is where a fresh visual capture becomes necessary again.

## Final verification (whole plan)

- [ ] `cargo test` — full suite green.
- [ ] `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` — clean.
- [ ] `cargo build --all-targets` — library, examples, benches all compile.
- [ ] Every `Camera`/`Canvas::fill_polygon` test from Tasks 1-3 passes with the exact hand-verified values in this plan (not just "some value").
- [ ] The Braille color-selection regression test from Task 4 passes, and the pre-existing `braille_last_written_dot_wins_the_cells_color` test still passes unmodified.
- [ ] Per `.claude/rules/git-github-standards.md`: open a PR from this Arc's worktree branch to `main`, wait for all four required checks green, squash-merge, then remove the worktree via `ExitWorktree`.
