# Depth & Perspective Projection Spike Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prototype a fixed-forward pinhole-camera projection (points, lines, filled polygons, all sharing one depth-driven scale) in one showcase example, to learn whether real projection math produces convincing parallax and legible 3D shapes in a terminal, and recommend whether/how it graduates into a real TTUI primitive.

**Architecture:** All new code lands as prototype-quality free functions and structs directly in one new example, `examples/depth_spike.rs` — no `src/` changes at all this Arc (unlike the rendering-fidelity spike, nothing here needs a new core module or a `Cell`/`LayerStack` change; the projection math only produces coordinates that the already-committed `Canvas` and `Cell`-placement APIs already know how to render). `Canvas::line`/`set_pixel`/`blit` (Arc A) are reused as-is; polygon fill is new prototype code living in the example file, not a `Canvas` method, since it isn't a committed API yet.

**Tech Stack:** Rust, existing `ttui` core (`app`, `buffer`, `canvas`, `layout`, `easing`). No new dependency.

## Global Constraints

- **Research tag — TDD does NOT apply to any task in this plan**, per `.claude/rules/development-conventions.md`'s `research`-tagged exception. No task below has a failing-test-first step; verification is "it builds, it runs, you look at it," per the spec's own Testing section.
- `cargo fmt` / `cargo clippy --all-targets` is **not a hard gate** for this prototype file (spec's Verification section) — run it and fix free/trivial warnings, but do not block a task on it.
- **No `src/` changes this Arc.** Everything lives in `examples/depth_spike.rs`.
- Windows-first, `crossterm`-only posture unchanged — no new dependency.
- No RNG dependency — any scattered/pseudo-random positioning (e.g. star placement) uses a deterministic hash, matching every prior Arc's posture (see `src/glitch.rs`'s noise formula for precedent).
- One worktree for this whole Arc, created via `superpowers:using-git-worktrees` before Task 1, per `.claude/rules/git-github-standards.md`.
- This Arc is `research`-tagged → still **Gated** autonomy tier (`.claude/rules/git-github-standards.md`): ships as a PR to `main` with the four required checks green, squash-merged at the end — not a direct push, despite the TDD exemption above (TDD-exemption and merge-gating are separate axes).
- Spec being implemented: `docs/design/specs/core/2026-08-10-depth-perspective-projection-spike-design.md`.

---

### Task 1: Scaffold `examples/depth_spike.rs` + the projection core

**Files:**
- Create: `examples/depth_spike.rs`

**Interfaces:**
- Consumes: `ttui::app::{run, App}`, `ttui::buffer::{Cell, LayerStack}`, `ttui::layout::Rect` (all existing, unchanged).
- Produces: `Point3`, `project()`, `NEAR_PLANE`/`FOCAL_LENGTH`/`ASPECT_COMPENSATION` constants — every later task uses `project()` as the shared math core.

- [ ] **Step 1: Write the example**

```rust
// examples/depth_spike.rs
//
// SPIKE PROTOTYPE for the depth & perspective projection spike
// (docs/design/specs/core/2026-08-10-depth-perspective-projection-spike-design.md).
// Not a themed vision-doc app — a bare showcase proving out real
// projection math. This file grows across that spec's implementation
// plan; expect prototype-quality code throughout.

use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::style::Color;
use ttui::app::{run, App};
use ttui::buffer::{Cell, LayerStack};
use ttui::layout::Rect;

/// Camera fixed at the origin, looking down `+Z` — no position/
/// orientation changes this Arc (confirmed fixed-forward only).
const NEAR_PLANE: f32 = 0.5;
/// Controls field of view: larger = more zoomed in/narrower FOV.
const FOCAL_LENGTH: f32 = 8.0;
/// Corrects for terminal cells being roughly twice as tall as wide —
/// same compensation the `Dial` widget already applies to its ring
/// radius.
const ASPECT_COMPENSATION: f32 = 2.0;

/// A point in camera-relative 3D space.
#[derive(Clone, Copy, Debug)]
struct Point3 {
    x: f32,
    y: f32,
    z: f32,
}

/// Projects `p` to `(screen_x, screen_y, scale)` in cell coordinates,
/// where `scale` grows as the point gets nearer (drives size/
/// brightness falloff). Returns `None` if `p.z <= NEAR_PLANE` — behind
/// or at the camera, where projection is undefined/inverted.
fn project(p: Point3, center_x: f32, center_y: f32) -> Option<(f32, f32, f32)> {
    if p.z <= NEAR_PLANE {
        return None;
    }
    let ndc_x = p.x / p.z;
    let ndc_y = p.y / p.z;
    let scale = FOCAL_LENGTH / p.z;
    let screen_x = center_x + ndc_x * FOCAL_LENGTH * ASPECT_COMPENSATION;
    let screen_y = center_y - ndc_y * FOCAL_LENGTH;
    Some((screen_x, screen_y, scale))
}

struct DepthSpike {
    quit: bool,
}

impl DepthSpike {
    fn new() -> Self {
        DepthSpike { quit: false }
    }
}

impl App for DepthSpike {
    fn update(&mut self, event: &Event) {
        let Event::Key(k) = event else { return };
        if k.kind != KeyEventKind::Press {
            return;
        }
        if k.code == KeyCode::Char('q') {
            self.quit = true;
        }
    }

    fn view(&self, area: Rect, buf: &mut LayerStack) {
        // Sanity check: four points at increasing depth (plus one
        // behind the near plane, which must not render) along the
        // same x/y — nearer ones should land farther from center and
        // brighter; farther ones should converge toward center and
        // dim; the z=0.2 point must not appear at all.
        let center_x = area.width as f32 / 2.0;
        let center_y = area.height as f32 / 2.0;
        let test_points = [
            Point3 { x: 3.0, y: 0.0, z: 2.0 },
            Point3 { x: 3.0, y: 0.0, z: 5.0 },
            Point3 { x: 3.0, y: 0.0, z: 10.0 },
            Point3 { x: 3.0, y: 0.0, z: 0.2 },
        ];
        for p in test_points {
            let Some((sx, sy, scale)) = project(p, center_x, center_y) else {
                continue;
            };
            let x = sx.round();
            let y = sy.round();
            if x < 0.0 || y < 0.0 || x as u16 >= area.width || y as u16 >= area.height {
                continue;
            }
            let brightness = (scale * 40.0).clamp(40.0, 255.0) as u8;
            buf.set(
                x as u16,
                y as u16,
                Cell {
                    symbol: '*',
                    fg: Color::Rgb {
                        r: brightness,
                        g: brightness,
                        b: brightness,
                    },
                    bg: Color::Reset,
                    alpha: 1.0,
                    ..Default::default()
                },
            );
        }
    }

    fn should_quit(&self) -> bool {
        self.quit
    }
}

fn main() -> std::io::Result<()> {
    let mut app = DepthSpike::new();
    run(&mut app)
}
```

- [ ] **Step 2: Build**

Run: `cargo build --example depth_spike`
Expected: compiles cleanly.

- [ ] **Step 3: Run and sanity-check the projection**

Run: `cargo run --example depth_spike`
Expected: three `*` glyphs on screen, roughly in a horizontal line curving toward screen-center as depth increases (the `z=10` point closest to center, `z=2` farthest right and brightest), and only three — the fourth test point (`z=0.2`, behind the near plane) must not appear anywhere. Press `q` to quit. Record whether this matches expectations — it goes into Task 6's recommendations.

- [ ] **Step 4: Commit**

```bash
git add examples/depth_spike.rs
git commit -m "research(core): scaffold depth_spike example, add fixed-camera projection core"
```

---

### Task 2: Line projection wired into `Canvas`

**Files:**
- Modify: `examples/depth_spike.rs`

**Interfaces:**
- Consumes: `project()` (Task 1), `ttui::canvas::{Canvas, CanvasMode}` (existing, Arc A).
- Produces: `Line3`, `project_line()` — Task 5's cube wireframe uses both.

- [ ] **Step 1: Add `Line3` and `project_line`**

Add to `examples/depth_spike.rs`, after `project`:

```rust
/// A line segment in camera-relative 3D space.
#[derive(Clone, Copy, Debug)]
struct Line3 {
    start: Point3,
    end: Point3,
}

/// Projects `line` to `Canvas` subpixel coordinates for a canvas whose
/// subpixel grid is `subpixels_x`/`subpixels_y` per cell. Simplified
/// clipping for this spike: `None` if *either* endpoint is at/behind
/// the near plane — true near-plane segment clipping (computing the
/// intersection point) is a documented Non-goal unless this spike
/// finds it's actually needed.
fn project_line(
    line: Line3,
    center_x: f32,
    center_y: f32,
    subpixels_x: f32,
    subpixels_y: f32,
) -> Option<(u16, u16, u16, u16)> {
    let (sx0, sy0, _) = project(line.start, center_x, center_y)?;
    let (sx1, sy1, _) = project(line.end, center_x, center_y)?;
    Some((
        (sx0 * subpixels_x).round().max(0.0) as u16,
        (sy0 * subpixels_y).round().max(0.0) as u16,
        (sx1 * subpixels_x).round().max(0.0) as u16,
        (sy1 * subpixels_y).round().max(0.0) as u16,
    ))
}
```

- [ ] **Step 2: Add a temporary line test render and wire it into `view`**

Add this import to the top of the file:

```rust
use ttui::canvas::{Canvas, CanvasMode};
```

Add this method to `impl DepthSpike`:

```rust
    fn render_test_lines(&self, area: Rect, buf: &mut LayerStack) {
        let center_x = area.width as f32 / 2.0;
        let center_y = area.height as f32 / 2.0;
        let mut canvas = Canvas::new(area.width, area.height, CanvasMode::Braille);
        let lines = [
            Line3 {
                start: Point3 { x: -3.0, y: -2.0, z: 4.0 },
                end: Point3 { x: 3.0, y: -2.0, z: 4.0 },
            },
            Line3 {
                start: Point3 { x: -3.0, y: 2.0, z: 4.0 },
                end: Point3 { x: 3.0, y: 2.0, z: 4.0 },
            },
            Line3 {
                start: Point3 { x: -3.0, y: -2.0, z: 4.0 },
                end: Point3 { x: -5.0, y: -3.0, z: 10.0 },
            },
            Line3 {
                start: Point3 { x: 3.0, y: -2.0, z: 4.0 },
                end: Point3 { x: 5.0, y: -3.0, z: 10.0 },
            },
        ];
        for line in lines {
            if let Some((x0, y0, x1, y1)) =
                project_line(line, center_x, center_y, 2.0, 4.0)
            {
                canvas.line(x0, y0, x1, y1, Color::Rgb { r: 90, g: 180, b: 255 });
            }
        }
        canvas.blit(buf, area.x, area.y);
    }
```

(`2.0, 4.0` are `CanvasMode::Braille`'s subpixel factors — hardcoded here since this is a throwaway test render; Task 5's real scene reads them properly, see that task.)

In `view`, add a call to this method after the existing test-points loop:

```rust
        self.render_test_lines(area, buf);
```

- [ ] **Step 3: Build and run**

Run: `cargo build --example depth_spike && cargo run --example depth_spike`
Expected: four connected line segments forming a shape that reads as receding into the distance — the two segments connecting near corners to far corners should visibly converge rather than stay parallel, confirming perspective is being applied to lines, not just points. Press `q` to quit.

- [ ] **Step 4: Commit**

```bash
git add examples/depth_spike.rs
git commit -m "research(core): add Line3 projection wired into Canvas"
```

---

### Task 3: Polygon projection + scanline fill

**Files:**
- Modify: `examples/depth_spike.rs`

**Interfaces:**
- Consumes: `project()` (Task 1), `Canvas::set_pixel` (existing, Arc A).
- Produces: `Polygon3`, `fill_polygon()` — Task 5's cube front face uses this.

- [ ] **Step 1: Add `Polygon3` and `fill_polygon`**

Add to `examples/depth_spike.rs`, after `project_line`:

```rust
/// A flat-shaded polygon in camera-relative 3D space — 3+ vertices,
/// in order around the perimeter (not necessarily convex).
#[derive(Clone, Debug)]
struct Polygon3 {
    vertices: Vec<Point3>,
}

/// Projects and fills `polygon` into `canvas` via an even-odd scanline
/// fill (one horizontal span per canvas-subpixel row, between each
/// pair of edge crossings). Draws nothing if any vertex is at/behind
/// the near plane (same simplified clipping as `project_line` — any
/// one clipped vertex skips the whole polygon) or if fewer than 3
/// vertices remain.
fn fill_polygon(
    polygon: &Polygon3,
    canvas: &mut Canvas,
    center_x: f32,
    center_y: f32,
    subpixels_x: f32,
    subpixels_y: f32,
    color: Color,
) {
    let mut points: Vec<(f32, f32)> = Vec::with_capacity(polygon.vertices.len());
    for &v in &polygon.vertices {
        let Some((sx, sy, _)) = project(v, center_x, center_y) else {
            return;
        };
        points.push((sx * subpixels_x, sy * subpixels_y));
    }
    if points.len() < 3 {
        return;
    }
    let min_y = points
        .iter()
        .map(|p| p.1)
        .fold(f32::INFINITY, f32::min)
        .floor() as i32;
    let max_y = points
        .iter()
        .map(|p| p.1)
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil() as i32;
    for y in min_y.max(0)..=max_y {
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
                canvas.set_pixel(x, y as u16, color);
            }
            i += 2;
        }
    }
}
```

- [ ] **Step 2: Add a temporary polygon test render and wire it into `view`**

Add this method to `impl DepthSpike`, after `render_test_lines`:

```rust
    fn render_test_polygon(&self, area: Rect, buf: &mut LayerStack) {
        let center_x = area.width as f32 / 2.0;
        let center_y = area.height as f32 / 2.0;
        let mut canvas = Canvas::new(area.width, area.height, CanvasMode::Braille);
        let polygon = Polygon3 {
            vertices: vec![
                Point3 { x: -4.0, y: -2.5, z: 6.0 },
                Point3 { x: -4.0, y: 2.5, z: 6.0 },
                Point3 { x: 2.0, y: 3.0, z: 12.0 },
                Point3 { x: 2.0, y: -3.0, z: 12.0 },
            ],
        };
        fill_polygon(
            &polygon,
            &mut canvas,
            center_x,
            center_y,
            2.0,
            4.0,
            Color::Rgb { r: 60, g: 40, b: 100 },
        );
        canvas.blit(buf, area.x, area.y);
    }
```

In `view`, add a call after `self.render_test_lines(area, buf);`:

```rust
        self.render_test_polygon(area, buf);
```

- [ ] **Step 3: Build and run**

Run: `cargo build --example depth_spike && cargo run --example depth_spike`
Expected: a solid-filled quadrilateral appears, wider on the near edge (left) and narrower on the far edge (right) — confirming the fill correctly follows perspective-projected vertices rather than filling a flat rectangle. Press `q` to quit.

- [ ] **Step 4: Commit**

```bash
git add examples/depth_spike.rs
git commit -m "research(core): add Polygon3 projection and scanline fill"
```

---

### Task 4: Parallax starfield

**Files:**
- Modify: `examples/depth_spike.rs`

**Interfaces:**
- Consumes: `project()` (Task 1), `ttui::easing::lerp_color` (existing).
- Produces: `Star`, `DepthSpike.stars`, `DepthSpike.tick_count` — Task 5's final scene assembly reuses the star-rendering method this task adds.

- [ ] **Step 1: Add `Star` and replace `DepthSpike`'s fields**

Add to `examples/depth_spike.rs`, near the top (after the `Polygon3`/`fill_polygon` block):

```rust
const STAR_COUNT: usize = 80;
const STAR_SPEED: f32 = 4.0; // z-units/second
const STAR_RESPAWN_Z: f32 = 24.0;

/// One drifting star: a fixed `(x, y)` and a `z` that decreases over
/// time (drifting toward the camera), respawning far away once it
/// passes the near plane.
#[derive(Clone, Copy, Debug)]
struct Star {
    x: f32,
    y: f32,
    z: f32,
}

/// Deterministic pseudo-random scatter for star placement — no RNG
/// dependency, matching every prior Arc's posture (same style as
/// `src/glitch.rs`'s noise hash).
fn scatter(seed: u32, spread: f32) -> f32 {
    let h = (seed.wrapping_mul(2_654_435_761)) ^ (seed.wrapping_mul(40_503).rotate_left(13));
    ((h % 10_000) as f32 / 10_000.0 - 0.5) * spread
}
```

Change the `use` block to add `Duration`:

```rust
use std::time::Duration;
```

Change `DepthSpike`'s struct/constructor — `tick_count` is included now even though it's only read starting in Step 2 below, so the struct doesn't need a second, awkward revision mid-task:

```rust
struct DepthSpike {
    stars: Vec<Star>,
    tick_count: u32,
    quit: bool,
}

impl DepthSpike {
    fn new() -> Self {
        let stars = (0..STAR_COUNT)
            .map(|i| {
                let seed = i as u32;
                Star {
                    x: scatter(seed, 16.0),
                    y: scatter(seed.wrapping_add(1_000), 10.0),
                    z: 2.0 + (seed as f32 % 20.0),
                }
            })
            .collect();
        DepthSpike {
            stars,
            tick_count: 0,
            quit: false,
        }
    }
```

- [ ] **Step 2: Add the starfield renderer and tick logic**

Add this method to `impl DepthSpike`, after `render_test_polygon`:

```rust
    fn render_starfield(&self, area: Rect, buf: &mut LayerStack) {
        let center_x = area.width as f32 / 2.0;
        let center_y = area.height as f32 / 2.0;
        for star in &self.stars {
            let p = Point3 { x: star.x, y: star.y, z: star.z };
            let Some((sx, sy, scale)) = project(p, center_x, center_y) else {
                continue;
            };
            let x = sx.round();
            let y = sy.round();
            if x < 0.0 || y < 0.0 || x as u16 >= area.width || y as u16 >= area.height {
                continue;
            }
            // Nearer (larger scale) stars render as a brighter, denser
            // glyph; farther ones as a dim, sparse glyph — both driven
            // by the same projection-derived `scale`, not a separate
            // hand-tuned depth curve.
            let symbol = if scale > 3.0 {
                '@'
            } else if scale > 1.5 {
                '*'
            } else {
                '.'
            };
            let brightness = (scale * 60.0).clamp(30.0, 255.0) as u8;
            buf.set(
                x as u16,
                y as u16,
                Cell {
                    symbol,
                    fg: Color::Rgb {
                        r: brightness,
                        g: brightness,
                        b: brightness,
                    },
                    bg: Color::Reset,
                    alpha: 1.0,
                    ..Default::default()
                },
            );
        }
    }
```

Add `tick_rate`/`on_tick` to `impl App for DepthSpike`:

```rust
    fn tick_rate(&self) -> Option<Duration> {
        Some(Duration::from_millis(33))
    }

    fn on_tick(&mut self, elapsed: Duration) {
        let dz = STAR_SPEED * elapsed.as_secs_f32();
        for (i, star) in self.stars.iter_mut().enumerate() {
            star.z -= dz;
            if star.z <= NEAR_PLANE {
                let seed = i as u32;
                star.z = STAR_RESPAWN_Z;
                star.x = scatter(seed.wrapping_add(self.tick_count), 16.0);
                star.y = scatter(seed.wrapping_add(self.tick_count).wrapping_add(1_000), 10.0);
            }
        }
        self.tick_count = self.tick_count.wrapping_add(1);
    }
```

- [ ] **Step 3: Wire the starfield into `view`, replacing the Task 1 sanity-check points**

Replace `view`'s body (the sanity-check `test_points` loop) with:

```rust
    fn view(&self, area: Rect, buf: &mut LayerStack) {
        self.render_starfield(area, buf);
        self.render_test_lines(area, buf);
        self.render_test_polygon(area, buf);
    }
```

(The Task 1 sanity-check points already did their job confirming the projection math works in isolation — the starfield now exercises the same `project()` call continuously, animated, which is a stronger ongoing check.)

- [ ] **Step 4: Build and run**

Run: `cargo build --example depth_spike && cargo run --example depth_spike`
Expected: a field of drifting stars, each appearing to move outward from the center and grow/brighten as it approaches, then vanishing and being replaced by a new far, dim star elsewhere — real parallax, with nearer stars visibly moving faster across the screen than farther ones, entirely from the shared `project()`/`scale` math (no per-star speed tuning). Press `q` to quit. Record whether the parallax genuinely reads as convincing — this is the spec's central open question, and goes directly into Task 6's recommendations.

- [ ] **Step 5: Commit**

```bash
git add examples/depth_spike.rs
git commit -m "research(core): add tick-driven parallax starfield"
```

---

### Task 5: Wireframe + filled cube, final scene assembly

**Files:**
- Modify: `examples/depth_spike.rs`

**Interfaces:**
- Consumes: `project_line`/`Line3` (Task 2), `fill_polygon`/`Polygon3` (Task 3), `render_starfield` (Task 4).
- Produces: the final assembled `view()` — Task 6 only adds instrumentation/findings around this, no further structural changes.

- [ ] **Step 1: Add cube geometry and a drifting depth**

Add to `examples/depth_spike.rs`, near the star constants:

```rust
const CUBE_HALF: f32 = 2.0;
const CUBE_MIN_Z: f32 = 4.0;
const CUBE_MAX_Z: f32 = 14.0;
const CUBE_DRIFT_SPEED: f32 = 2.0; // z-units/second

/// The 8 corners of a cube of half-width `CUBE_HALF` centered at
/// `(0, 0, center_z)`. Index order: `i = (dx_idx*2 + dy_idx)*2 + dz_idx`
/// for `dx_idx, dy_idx, dz_idx` each in `{0 (-), 1 (+)}`.
fn cube_vertices(center_z: f32) -> [Point3; 8] {
    let mut v = [Point3 { x: 0.0, y: 0.0, z: 0.0 }; 8];
    let mut i = 0;
    for dx in [-CUBE_HALF, CUBE_HALF] {
        for dy in [-CUBE_HALF, CUBE_HALF] {
            for dz in [-CUBE_HALF, CUBE_HALF] {
                v[i] = Point3 { x: dx, y: dy, z: center_z + dz };
                i += 1;
            }
        }
    }
    v
}

/// The cube's 12 edges as index pairs into `cube_vertices`'s output.
const CUBE_EDGES: [(usize, usize); 12] = [
    (0, 4), (1, 5), (2, 6), (3, 7), // edges along dx
    (0, 2), (1, 3), (4, 6), (5, 7), // edges along dy
    (0, 1), (2, 3), (4, 5), (6, 7), // edges along dz
];
```

- [ ] **Step 2: Add the cube renderer**

Add this method to `impl DepthSpike`, after `render_starfield`:

```rust
    fn render_cube(&self, area: Rect, buf: &mut LayerStack, center_z: f32) {
        let center_x = area.width as f32 / 2.0;
        let center_y = area.height as f32 / 2.0;
        let verts = cube_vertices(center_z);
        let mut canvas = Canvas::new(area.width, area.height, CanvasMode::Braille);

        // Front face (both dz=- corners: indices 0,2,6,4, walked as a
        // perimeter), filled first so the wireframe draws on top of it.
        let front_face = Polygon3 {
            vertices: vec![verts[0], verts[2], verts[6], verts[4]],
        };
        fill_polygon(
            &front_face,
            &mut canvas,
            center_x,
            center_y,
            2.0,
            4.0,
            Color::Rgb { r: 40, g: 60, b: 100 },
        );

        for &(a, b) in &CUBE_EDGES {
            let line = Line3 { start: verts[a], end: verts[b] };
            if let Some((x0, y0, x1, y1)) =
                project_line(line, center_x, center_y, 2.0, 4.0)
            {
                canvas.line(x0, y0, x1, y1, Color::Rgb { r: 200, g: 220, b: 255 });
            }
        }

        canvas.blit(buf, area.x, area.y);
    }
```

- [ ] **Step 3: Drive the cube's depth over time and assemble the final scene**

Add `cube_z: f32` to `DepthSpike`'s struct and initialize it to `CUBE_MIN_Z` in `new()`. In `on_tick`, add (after the star-update loop, before `self.tick_count = ...`):

```rust
        self.cube_z += CUBE_DRIFT_SPEED * elapsed.as_secs_f32();
        if self.cube_z > CUBE_MAX_Z {
            self.cube_z = CUBE_MIN_Z;
        }
```

Replace `view`'s body entirely with the final assembled scene:

```rust
    fn view(&self, area: Rect, buf: &mut LayerStack) {
        self.render_starfield(area, buf);
        self.render_cube(area, buf, self.cube_z);
    }
```

Remove `render_test_lines` and `render_test_polygon`'s calls from `view` (already done by the full replacement above) — the methods themselves can stay in the file as dead code with a `#[allow(dead_code)]` if `cargo clippy` flags them, since this is prototype code and deleting working reference implementations mid-spike isn't worth the churn; or delete them if you'd rather keep the file lean. Either is fine for this task — record your choice in the commit message.

- [ ] **Step 4: Build and run the full showcase**

Run: `cargo build --example depth_spike && cargo run --example depth_spike`
Expected: a drifting starfield in the background, with a wireframe-and-filled cube drifting from far to near and looping, growing larger and passing near the edges of the screen as it approaches before resetting far away again. The cube's faces/edges should stay legible as a recognizable cube shape throughout its whole depth range, not degenerate into unreadable noise up close or an indistinct dot far away. Press `q` to quit. This is the concrete "does a projected shape stay legible" success criterion from the spec — record your honest assessment, it goes into Task 6.

- [ ] **Step 5: Commit**

```bash
git add examples/depth_spike.rs
git commit -m "research(core): add wireframe+filled cube, assemble full depth_spike scene"
```

---

### Task 6: Recommendations write-up

**Files:**
- Modify: `docs/design/specs/core/2026-08-10-depth-perspective-projection-spike-design.md` (append findings)

**Interfaces:**
- Consumes: the finished scene from Task 5 and the recorded observations from Tasks 1-5's "Expected" verification steps.
- Produces: the spec's filled-in "Recommendations (post-spike)" section.

- [ ] **Step 1: Replace the spec's placeholder "Recommendations" section**

The spec currently ends with:

```markdown
## Recommendations (post-spike)

Written after running `examples/depth_spike.rs`. Not yet available —
this section is appended once the spike is implemented and run, before
this spec is considered closed, matching the same deferred-until-run
convention as `2026-08-08-rendering-fidelity-spike-design.md`.
```

Replace it with real findings, using the observations recorded in Tasks 1-5's verification steps as your source material — do not write this section from theory alone; every claim must trace back to something actually observed running the example. Structure it the same way `2026-08-08-rendering-fidelity-spike-design.md`'s own "Recommendations (post-spike)" section is structured: answer each of the five questions from the design spec's Success Criteria section plainly (convincing depth? sane clipping? clean scanline fill? parallax for free? shape stays legible?), then a graduation recommendation — does this become a real `src/` module, under what name, and what (if anything) needs to change from the spike's exact math/API before it's committed with full TDD.

- [ ] **Step 2: Build check**

Run: `cargo build --examples`
Expected: all examples, including `depth_spike`, still compile.

- [ ] **Step 3: Commit**

```bash
git add docs/design/specs/core/2026-08-10-depth-perspective-projection-spike-design.md
git commit -m "research(core): record depth_spike recommendations"
```

---

## Final verification (whole plan)

- [ ] `cargo build --all-targets` succeeds.
- [ ] `cargo run --example depth_spike` shows the full assembled scene from Task 5 — starfield with real parallax, a drifting cube that stays legible across its depth range — animating smoothly, `q` quitting cleanly.
- [ ] `cargo build --examples` — all existing examples still compile, confirming this Arc's `examples/depth_spike.rs`-only scope didn't touch anything shared.
- [ ] `cargo test` — full existing suite still green (this plan adds no new unit tests per the `research` TDD exemption, and touches no `src/` files, so nothing should change here at all).
- [ ] The spec's "Recommendations (post-spike)" section is filled in with real findings, no placeholder text remaining.
- [ ] Per `.claude/rules/git-github-standards.md`: open a PR from this Arc's worktree branch to `main`, wait for the four required checks green, squash-merge, then remove the worktree via `ExitWorktree`.
