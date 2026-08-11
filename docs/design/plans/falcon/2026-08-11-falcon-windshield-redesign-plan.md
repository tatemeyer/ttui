# Falcon Windshield Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the shipped Falcon dashboard hub's full-screen 3-panel layout with a first-person cockpit windshield — a real 3D-projected drifting starfield and wireframe canopy frame (`src/perspective.rs`) over a slim console strip — plus a boot-sequence rework adding a windshield-power-on phase.

**Architecture:** All changes are to `examples/falcon/{falcon.rs,boot.rs}`. No `src/` changes. Built incrementally: Tasks 1-2 add the starfield/canopy rendering with a temporary `view()` wire-in for visual verification (matching the pattern `examples/depth_spike.rs`'s own plan used); Task 3 does the real assembly (windshield/console layout split) and removes the temporary wiring; Task 4 reworks the boot sequence to match.

**Tech Stack:** Rust, existing `ttui` core (`app`, `buffer`, `canvas`, `layout`, `perspective`, `theme`, `transition`, `glitch`, `particles`), `tools/visual-snapshot` for verification.

## Global Constraints

- Example code (`examples/falcon/`): TDD-exempt per `.claude/rules/development-conventions.md`'s "Examples/demos" exception — no failing-test-first step, verification is "it builds, and a real visual capture confirms it looks right."
- **`tools/visual-snapshot` is a hard requirement for every task and the final review in this plan** — this Arc directly changes `examples/falcon/falcon.rs`'s `view()`/`on_tick()` and `boot.rs`'s render path, which unambiguously triggers `.claude/rules/development-conventions.md`'s "Visual review" rule. "Reasoned through it, no PTY available" is not an acceptable substitute now that this tool exists — every task below has a dedicated capture-and-`Read`-the-image step, not an optional one.
- `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` are hard gates on every task.
- No new dependency, no `src/` changes.
- One worktree for this whole plan, created via `superpowers:using-git-worktrees` before Task 1, per `.claude/rules/git-github-standards.md`.
- `coding`-tagged (git-adjacent convention still applies even though TDD is exempt) → **Gated** autonomy tier: ships as a PR to `main` with all four required checks green, squash-merged at the end.
- Spec being implemented: `docs/design/specs/falcon/2026-08-11-falcon-windshield-redesign-design.md`.

---

### Task 1: Camera + starfield (temporary wire-in for verification)

**Files:**
- Modify: `examples/falcon/falcon.rs`

**Interfaces:**
- Consumes: `ttui::perspective::{Camera, Point3}` (new import, existing module).
- Produces: `falcon_camera()`, `Star`, `scatter()`, `Falcon.camera`/`Falcon.stars`, `render_starfield()` — Task 3 wires this into the real windshield rendering; this task's own `view()` call is temporary and removed there.

- [ ] **Step 1: Add imports and constants**

At the top of `examples/falcon/falcon.rs`, add to the `use` block:

```rust
use ttui::perspective::{Camera, Point3};
```

Add near the existing `IDLE_FLICKER_*`/`WHACK_*` constants:

```rust
const STAR_COUNT: usize = 60;
const STAR_SPEED: f32 = 3.0; // z-units/second
const STAR_RESPAWN_Z: f32 = 20.0;
```

- [ ] **Step 2: Add `falcon_camera`, `Star`, `scatter`**

Add near `falcon_theme()`:

```rust
fn falcon_camera() -> Camera {
    Camera {
        near: 0.5,
        focal_length: 8.0,
    }
}

struct Star {
    x: f32,
    y: f32,
    z: f32,
}

/// Deterministic pseudo-random scatter for star placement — no RNG
/// dependency, matching every prior Arc's posture.
fn scatter(seed: u32, spread: f32) -> f32 {
    let h = (seed.wrapping_mul(2_654_435_761)) ^ (seed.wrapping_mul(40_503).rotate_left(13));
    ((h % 10_000) as f32 / 10_000.0 - 0.5) * spread
}
```

- [ ] **Step 3: Add `camera`/`stars` fields**

Change the `Falcon` struct to add two fields (after `theme`):

```rust
pub(crate) struct Falcon {
    theme: Theme,
    camera: Camera,
    stars: Vec<Star>,
    focused: usize,
    // ...unchanged fields below (last_area, glitches, particles, tick_count, booting, quit)...
}
```

Change `Falcon::new()` to initialize them (after `theme: falcon_theme(),`):

```rust
    pub(crate) fn new() -> Self {
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
        Falcon {
            theme: falcon_theme(),
            camera: falcon_camera(),
            stars,
            focused: 0,
            // ...unchanged fields below...
```

- [ ] **Step 4: Add star tick logic to `on_tick`**

At the end of `on_tick` (after the existing `self.particles.update(elapsed);` line), add:

```rust
        let dz = STAR_SPEED * elapsed.as_secs_f32();
        for (i, star) in self.stars.iter_mut().enumerate() {
            star.z -= dz;
            if star.z <= self.camera.near {
                let seed = i as u32;
                star.z = STAR_RESPAWN_Z;
                star.x = scatter(seed.wrapping_add(self.tick_count as u32), 16.0);
                star.y = scatter(
                    seed.wrapping_add(self.tick_count as u32).wrapping_add(1_000),
                    10.0,
                );
            }
        }
```

- [ ] **Step 5: Add `render_starfield`**

Add this method to `impl Falcon` (after `render_dashboard`, before the closing brace):

```rust
    fn render_starfield(&self, area: Rect, buf: &mut LayerStack) {
        let center_x = area.x as f32 + area.width as f32 / 2.0;
        let center_y = area.y as f32 + area.height as f32 / 2.0;
        for star in &self.stars {
            let p = Point3 {
                x: star.x,
                y: star.y,
                z: star.z,
            };
            let Some((sx, sy, scale)) = self.camera.project(p, center_x, center_y) else {
                continue;
            };
            let x = sx.round();
            let y = sy.round();
            if x < area.x as f32
                || y < area.y as f32
                || x >= (area.x + area.width) as f32
                || y >= (area.y + area.height) as f32
            {
                continue;
            }
            let symbol = if scale > 3.0 {
                '@'
            } else if scale > 1.5 {
                '*'
            } else {
                '.'
            };
            let brightness = (scale * 50.0).clamp(25.0, 255.0) as u8;
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

- [ ] **Step 6: Temporarily wire into `view` for verification**

In `impl App for Falcon`'s `view` method, add a call after `self.render_dashboard(area, buf);` (this line and this step's comment are removed in Task 3 once the real windshield/console split lands):

```rust
    fn view(&self, area: Rect, buf: &mut LayerStack) {
        self.last_area.set(area);
        if let Some(t) = &self.booting {
            self.render_boot(area, t.progress(), buf);
            return;
        }
        self.render_dashboard(area, buf);
        self.render_starfield(area, buf); // TEMPORARY — Task 3 replaces this with the real windshield/console layout split
    }
```

- [ ] **Step 7: Build**

Run: `cargo build --example falcon`
Expected: compiles cleanly.

- [ ] **Step 8: Capture and verify with `tools/visual-snapshot`**

Run:
```
cargo run -p visual-snapshot -- --example falcon --size 100x35 --script <a script with a couple of wait_ms steps, e.g. [{"wait_ms": 1500}, {"wait_ms": 1500}]> --out <path>.gif
```
(`falcon` boots first — the script's `wait_ms` needs to span past `BOOT_TOTAL_MS` (1400ms) for the starfield to actually be visible over the existing dashboard, not just the boot sequence.) `Read` the resulting image. Expected: the existing 3-panel dashboard renders as before, now with drifting star glyphs (`.`/`*`/`@`) scattered across the whole screen on top of it — confirms the projection math and rendering are wired correctly in isolation, even though the layout itself hasn't changed yet.

- [ ] **Step 9: Run clippy and fmt**

Run: `cargo clippy --example falcon -- -D warnings` and `cargo fmt --check -- examples/falcon/falcon.rs`
Expected: both clean.

- [ ] **Step 10: Commit**

```bash
git add examples/falcon/falcon.rs
git commit -m "feat(falcon): add real 3D-projected starfield (temporary view() wire-in)

Uses src/perspective.rs's Camera::project directly — same Star/
scatter/tick pattern examples/depth_spike.rs already validated.
Temporarily rendered over the existing full-screen dashboard for
visual verification; Task 3 wires it into the real windshield layout."
```

---

### Task 2: Canopy frame (temporary wire-in for verification)

**Files:**
- Modify: `examples/falcon/falcon.rs`

**Interfaces:**
- Consumes: `ttui::perspective::Line3` (new import), `ttui::canvas::{Canvas, CanvasMode}` (new import), `self.camera` (Task 1).
- Produces: `canopy_vertices()`, `CANOPY_EDGES`, `render_canopy()` — Task 3 wires this into the real windshield rendering; this task's own `view()` call is temporary and removed there.

- [ ] **Step 1: Add imports and canopy geometry**

Add to the `use` block:

```rust
use ttui::canvas::{Canvas, CanvasMode};
use ttui::perspective::{Camera, Line3, Point3};
```

(combining with Task 1's `Camera, Point3` import into one `use ttui::perspective::{...}` line.)

Add near the star constants:

```rust
const CANOPY_NEAR_Z: f32 = 2.0;
const CANOPY_FAR_Z: f32 = 10.0;
const CANOPY_HALF_W: f32 = 5.0;
const CANOPY_HALF_H: f32 = 3.0;

/// The canopy's 8 corners: two parallel rectangles (near/far) of the
/// same world-space size, connected by 4 verticals — the perspective
/// convergence comes entirely from the projection, not from shrinking
/// the far rectangle's world-space size. Index order:
/// `i = (dx_idx*2 + dy_idx)*2 + z_idx` for `dx_idx, dy_idx, z_idx`
/// each in `{0 (near/-), 1 (far/+)}`.
fn canopy_vertices() -> [Point3; 8] {
    let mut v = [Point3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    }; 8];
    let mut i = 0;
    for dx in [-CANOPY_HALF_W, CANOPY_HALF_W] {
        for dy in [-CANOPY_HALF_H, CANOPY_HALF_H] {
            for z in [CANOPY_NEAR_Z, CANOPY_FAR_Z] {
                v[i] = Point3 { x: dx, y: dy, z };
                i += 1;
            }
        }
    }
    v
}

/// 4 near-rectangle edges (dx/dy pairs), 4 far-rectangle edges, 4
/// near-to-far connectors — same topology as a cube's 12 edges.
const CANOPY_EDGES: [(usize, usize); 12] = [
    (0, 4), (1, 5), (2, 6), (3, 7), // edges along dx
    (0, 2), (1, 3), (4, 6), (5, 7), // edges along dy
    (0, 1), (2, 3), (4, 5), (6, 7), // near-to-far connectors
];
```

- [ ] **Step 2: Add `render_canopy`**

Add this method to `impl Falcon` (after `render_starfield`):

```rust
    fn render_canopy(&self, area: Rect, buf: &mut LayerStack, edges_shown: usize) {
        let center_x = area.x as f32 + area.width as f32 / 2.0;
        let center_y = area.y as f32 + area.height as f32 / 2.0;
        let verts = canopy_vertices();
        let mut canvas = Canvas::new(area.width, area.height, CanvasMode::Braille);
        for &(a, b) in CANOPY_EDGES.iter().take(edges_shown) {
            let line = Line3 {
                start: verts[a],
                end: verts[b],
            };
            if let Some((x0, y0, x1, y1)) = self.camera.project_line(
                line,
                center_x,
                center_y,
                area.width as f32,
                area.height as f32,
                2.0,
                4.0,
                0.0,
            ) {
                canvas.line(x0, y0, x1, y1, self.theme.secondary);
            }
        }
        canvas.blit(buf, area.x, area.y);
    }
```

- [ ] **Step 3: Temporarily wire into `view` for verification**

Change the temporary line Task 1 added in `view`:

```rust
        self.render_starfield(area, buf); // TEMPORARY — Task 3 replaces this with the real windshield/console layout split
```

to:

```rust
        self.render_starfield(area, buf); // TEMPORARY — Task 3 replaces this with the real windshield/console layout split
        self.render_canopy(area, buf, 12); // TEMPORARY — same
```

- [ ] **Step 4: Build**

Run: `cargo build --example falcon`
Expected: compiles cleanly.

- [ ] **Step 5: Capture and verify with `tools/visual-snapshot`**

Run the same capture command style as Task 1's Step 8. `Read` the resulting image. Expected: the wireframe canopy (a near rectangle and a far rectangle connected by 4 diagonal edges, all in the theme's secondary/green color) now renders across the whole screen, over the dashboard and starfield, converging visibly toward the far rectangle — confirms `project_line`'s clipping and the canopy's geometry are correct.

- [ ] **Step 6: Run clippy and fmt**

Run: `cargo clippy --example falcon -- -D warnings` and `cargo fmt --check -- examples/falcon/falcon.rs`
Expected: both clean.

- [ ] **Step 7: Commit**

```bash
git add examples/falcon/falcon.rs
git commit -m "feat(falcon): add wireframe canopy frame (temporary view() wire-in)

Two same-size rectangles at different z, connected by 4 edges — same
topology as depth_spike.rs's cube, drawn via Camera::project_line's
real screen-edge clipping. Temporarily layered over the existing
dashboard for visual verification; Task 3 wires it into the real
windshield layout."
```

---

### Task 3: Real windshield/console assembly

**Files:**
- Modify: `examples/falcon/falcon.rs`

**Interfaces:**
- Consumes: `render_starfield` (Task 1), `render_canopy` (Task 2), `panel_slots`/`panel_box`/`CockpitPanel` (existing, unchanged).
- Produces: `render_windshield()` — Task 4's boot rework calls this directly with a partial `canopy_edges_shown`.

This is the task where the redesign actually replaces the old full-screen layout — the temporary `view()` wiring from Tasks 1-2 is removed here, not left behind.

- [ ] **Step 1: Add `render_windshield` and rewrite `render_dashboard`**

Add this method to `impl Falcon` (after `render_canopy`):

```rust
    fn render_windshield(&self, area: Rect, buf: &mut LayerStack, canopy_edges_shown: usize) {
        let bg = Cell {
            symbol: ' ',
            fg: Color::Reset,
            bg: self.theme.background,
            alpha: 1.0,
            ..Default::default()
        };
        for y in 0..area.height {
            for x in 0..area.width {
                buf.set(area.x + x, area.y + y, bg.clone());
            }
        }
        self.render_starfield(area, buf);
        self.render_canopy(area, buf, canopy_edges_shown);
    }
```

Replace `render_dashboard`'s entire body:

```rust
    fn render_dashboard(&self, area: Rect, buf: &mut LayerStack) {
        let regions = Layout::new(
            Direction::Vertical,
            vec![Constraint::Percentage(78), Constraint::Fill(1)],
        )
        .split(area);
        let windshield = regions[0];
        let console = regions[1];

        self.render_windshield(windshield, buf, 12);

        let bg = Cell {
            symbol: ' ',
            fg: self.theme.primary,
            bg: self.theme.background,
            alpha: 1.0,
            ..Default::default()
        };
        for y in 0..console.height {
            for x in 0..console.width {
                buf.set(console.x + x, console.y + y, bg.clone());
            }
        }

        let slots = Self::panel_slots(console);
        let mut panel_inners = [Rect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }; 3];
        for (i, kind) in PANELS.iter().enumerate() {
            let focused = i == self.focused;
            let panel_box = Self::panel_box(slots[i], focused);
            let inner = CockpitPanel::new(focused).render(panel_box, &self.theme, buf);
            panel_inners[i] = inner;
            Text::new(kind.name()).render(inner, buf);
            if inner.height > 1 {
                let hint = Rect {
                    x: inner.x,
                    y: inner.y + 1,
                    width: inner.width,
                    height: 1,
                };
                Text::new("(not yet built)").render(hint, buf);
            }
        }

        let overlay = buf.push_layer();
        for (i, gb) in self.glitches.iter().enumerate() {
            if gb.is_active() {
                gb.render(
                    panel_inners[i],
                    self.theme.tertiary,
                    self.tick_count,
                    overlay,
                );
            }
        }
        self.particles.render(overlay);
    }
```

The only real change from the current version: the leading `Layout::split` call and using `windshield`/`console` sub-rects instead of `area` directly for the background fill and `Self::panel_slots(...)` call. Everything after that (the panel loop, glitch overlay, particle render) is unchanged.

- [ ] **Step 2: Remove Tasks 1-2's temporary `view()` wiring**

Change `view` back to a single `render_dashboard` call (removing both TEMPORARY lines Tasks 1-2 added):

```rust
    fn view(&self, area: Rect, buf: &mut LayerStack) {
        self.last_area.set(area);
        if let Some(t) = &self.booting {
            self.render_boot(area, t.progress(), buf);
            return;
        }
        self.render_dashboard(area, buf);
    }
```

- [ ] **Step 3: Fix the WHACK handler's console-region recomputation**

`update`'s `KeyCode::Char(' ')` arm currently computes `Self::panel_slots(self.last_area.get())` — this must recompute the same vertical split `render_dashboard` uses, so the WHACK spark spawns at the focused panel's current (now smaller, repositioned) location, not a stale full-screen one. Change:

```rust
            KeyCode::Char(' ') if self.glitches[self.focused].is_active() => {
                self.glitches[self.focused].clear();
                let slots = Self::panel_slots(self.last_area.get());
                let panel_box = Self::panel_box(slots[self.focused], true);
```

to:

```rust
            KeyCode::Char(' ') if self.glitches[self.focused].is_active() => {
                self.glitches[self.focused].clear();
                let regions = Layout::new(
                    Direction::Vertical,
                    vec![Constraint::Percentage(78), Constraint::Fill(1)],
                )
                .split(self.last_area.get());
                let slots = Self::panel_slots(regions[1]);
                let panel_box = Self::panel_box(slots[self.focused], true);
```

(the rest of the arm, computing `cx`/`cy` and spawning particles, is unchanged.)

- [ ] **Step 4: Build**

Run: `cargo build --example falcon`
Expected: compiles cleanly.

- [ ] **Step 5: Capture and verify with `tools/visual-snapshot`**

Capture the post-boot dashboard (script needs `wait_ms` past `BOOT_TOTAL_MS`, same as Task 1's Step 8) plus a couple of `key: "Tab"` steps and a `key: " "` step to exercise focus-cycling and WHACK. `Read` the resulting frames. Expected: the windshield (starfield + canopy) now occupies roughly the top 78% of the screen, with the three console panels compressed into a slim strip along the bottom — not filling the whole screen anymore. Tab visibly moves the enlarged/bright panel among the three console panels. If a glitch happens to be active on the focused panel when the Space step fires, confirm the spark burst appears within the console strip, not floating up in the windshield area above it — this is the concrete check that Step 3's region recomputation is correct, not just "compiles."

- [ ] **Step 6: Run clippy and fmt**

Run: `cargo clippy --example falcon -- -D warnings` and `cargo fmt --check -- examples/falcon/falcon.rs`
Expected: both clean.

- [ ] **Step 7: Commit**

```bash
git add examples/falcon/falcon.rs
git commit -m "feat(falcon): assemble the real windshield/console layout

Splits the screen into a windshield region (starfield + canopy,
~78%) and a console strip (the existing 3-panel mechanics,
unchanged) via Layout::split — replaces the full-screen 3-panel
dashboard. The WHACK handler's panel-position lookup is updated to
match, so sparks spawn at the panel's real (now smaller) location."
```

---

### Task 4: Boot sequence rework

**Files:**
- Modify: `examples/falcon/boot.rs`

**Interfaces:**
- Consumes: `render_windshield` (Task 3), `panel_slots`/`panel_box` (existing).
- Produces: no new public API — `render_boot`'s internal phase structure changes from 3 phases to 4.

- [ ] **Step 1: Replace `render_boot`'s body**

Replace the entire body of `render_boot` in `examples/falcon/boot.rs`:

```rust
    pub(crate) fn render_boot(&self, area: Rect, progress: f32, buf: &mut LayerStack) {
        let regions = Layout::new(
            Direction::Vertical,
            vec![Constraint::Percentage(78), Constraint::Fill(1)],
        )
        .split(area);
        let windshield = regions[0];
        let console = regions[1];

        if progress < 0.1 {
            let cx = area.x + area.width / 2;
            let cy = area.y + area.height / 2;
            buf.set(
                cx,
                cy,
                Cell {
                    symbol: '\u{2022}', // '•'
                    fg: self.theme.primary,
                    bg: Color::Reset,
                    alpha: 1.0,
                    ..Default::default()
                },
            );
            return;
        }

        if progress < 0.4 {
            let wave = (progress - 0.1) / 0.3;
            let edges_shown = ((wave * 12.0).ceil() as usize).min(12);
            self.render_windshield(windshield, buf, edges_shown);
            return;
        }

        if progress < 0.85 {
            self.render_windshield(windshield, buf, 12);

            let wave = (progress - 0.4) / 0.45;
            let panels_shown = ((wave * 3.0).ceil() as usize).min(3);
            let slots = Self::panel_slots(console);
            let mut newest: Option<(usize, Rect)> = None;
            for (i, kind) in PANELS.iter().enumerate().take(panels_shown) {
                let panel_box = Self::panel_box(slots[i], false);
                let inner = CockpitPanel::new(false).render(panel_box, &self.theme, buf);
                Text::new(kind.name()).render(inner, buf);
                newest = Some((i, inner));
            }
            if let Some((newest_index, inner)) = newest {
                let local_wave = ((wave * 3.0) - newest_index as f32).clamp(0.0, 1.0);
                let mut burst = GlitchBuffer::new();
                let burst_duration = Duration::from_millis(300);
                burst.trigger(burst_duration);
                burst.tick(Duration::from_secs_f32(
                    local_wave * burst_duration.as_secs_f32(),
                ));
                let overlay = buf.push_layer();
                burst.render(inner, self.theme.tertiary, self.tick_count, overlay);
            }
            return;
        }

        let fade = ((progress - 0.85) / 0.15).clamp(0.0, 1.0);
        let mut scratch = LayerStack::new(area.width, area.height);
        self.render_dashboard(
            Rect {
                x: 0,
                y: 0,
                width: area.width,
                height: area.height,
            },
            &mut scratch,
        );
        let composited = scratch.composite();
        for y in 0..area.height {
            for x in 0..area.width {
                let real = composited.get(x, y);
                let dimmed = Cell {
                    symbol: real.symbol,
                    fg: ttui::easing::lerp_color(self.theme.background, real.fg, fade),
                    bg: ttui::easing::lerp_color(self.theme.background, real.bg, fade),
                    style: real.style,
                    alpha: 1.0,
                };
                buf.set(area.x + x, area.y + y, dimmed);
            }
        }
    }
```

Four phases, replacing the old three: `[0.0,0.1)` pilot light (unchanged, still centered in the full `area`), `[0.1,0.4)` windshield power-on (canopy edges reveal `(wave*12.0).ceil()` at a time, starfield renders at full immediately), `[0.4,0.85)` console panel reveal (same rivet-by-rivet + static-burst mechanic as before, now operating on `console` instead of the full `area`, with the already-complete windshield kept visible above it), `[0.85,1.0]` whole-frame dim-to-bright fade (unchanged mechanic — the scratch-`LayerStack`-then-composite pattern is still required for the same reason: `render_dashboard` pushes its own glitch/particle layer).

- [ ] **Step 2: Build**

Run: `cargo build --example falcon`
Expected: compiles cleanly.

- [ ] **Step 3: Capture and verify the full boot sequence with `tools/visual-snapshot`**

Run a capture spanning enough real time to cover all four phases — `BOOT_TOTAL_MS` is 1400ms, so a script with several `wait_ms` steps totaling at least ~1600-1800ms (a few hundred ms past completion) at moderate intervals (e.g. `[{"wait_ms": 200}, {"wait_ms": 200}, {"wait_ms": 200}, {"wait_ms": 200}, {"wait_ms": 200}, {"wait_ms": 200}, {"wait_ms": 400}]`) produces a multi-frame GIF. `Read` the resulting frames in order. Expected: pilot light appears alone first; the canopy frame visibly builds up edge-by-edge (not all 12 at once) while the starfield is already fully present; the console panels then snap in one at a time beneath the now-complete windshield; the whole frame finally brightens from dim to full color. Confirm no phase's content is missing or appears out of order.

- [ ] **Step 4: Run clippy and fmt**

Run: `cargo clippy --example falcon -- -D warnings` and `cargo fmt --check -- examples/falcon/boot.rs`
Expected: both clean.

- [ ] **Step 5: Commit**

```bash
git add examples/falcon/boot.rs
git commit -m "feat(falcon): rework boot sequence for the windshield layout

Four phases instead of three: pilot light (unchanged), a new
windshield-power-on phase revealing the canopy frame edge-by-edge
while the starfield shows at full immediately, the existing
console-panel reveal (now scoped to the console strip), and the
existing whole-frame dim-to-bright fade."
```

---

### Task 5: Final verification

**Files:** none (verification only).

- [ ] **Step 1: Build every target**

Run: `cargo build --all-targets`
Expected: succeeds.

- [ ] **Step 2: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
Expected: both clean.

- [ ] **Step 3: Full test suite**

Run: `cargo test`
Expected: full suite green — this plan adds no new unit tests (example code, TDD-exempt) and touches no `src/` file, so nothing should change here at all versus `main`.

- [ ] **Step 4: One more full `tools/visual-snapshot` capture of the finished result**

Run a capture spanning the full boot sequence plus a few seconds of post-boot idle time (long enough to see the starfield's parallax and at least one idle-flicker cycle on a console panel). `Read` it. This is the final, whole-Arc confirmation — a single artifact demonstrating boot, windshield, console, and their interaction all together. Attach or reference this capture in the PR's Verification section.

## Final verification (whole plan)

- [ ] `cargo build --all-targets` succeeds.
- [ ] `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` — clean.
- [ ] `cargo test` — full suite green, unchanged from `main`.
- [ ] At least one `tools/visual-snapshot` capture from Task 5 is referenced in the PR description, showing the finished windshield + console strip + boot sequence.
- [ ] Per `.claude/rules/git-github-standards.md`: open a PR from this Arc's worktree branch to `main`, wait for all four required checks green, squash-merge, then remove the worktree via `ExitWorktree`.
