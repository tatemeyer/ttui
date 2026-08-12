# Falcon HUD System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add three distinct, animated, 3D-projected HUD states to the Falcon windshield (Hyperdrive trajectory line, Sensors radar sweep, Weapons reticle), one rendered at a time matching the currently-focused console panel.

**Architecture:** Each HUD state is its own render method reusing the `Line3` → `Camera::project_line` → `Canvas::line` pattern `render_canopy` already proved out, driven by its own always-advancing `f32` phase field on `Falcon`. A dispatch method picks the right one by focus; `render_windshield` gains a `show_hud` gate so boot can control when the HUD comes online.

**Tech Stack:** Rust, `ttui::perspective` (`Camera`/`Point3`/`Line3`), `ttui::canvas` (`Canvas`/`CanvasMode::Braille`), `ttui::easing::lerp_color`.

## Global Constraints

- **Tag: `coding`, TDD-exempt.** All changes are to `examples/falcon/`, covered by the "Examples/demos" exception in `.claude/rules/development-conventions.md` — no failing-test-first step; verification is "it builds, and a real visual capture confirms it looks right."
- **`tools/visual-snapshot` is mandatory for every task and the final review** — not optional. Every task below has a dedicated capture-and-`Read` step.
- **Mandatory pattern for every HUD render method:** `center_x = area.width as f32 / 2.0` / `center_y = area.height as f32 / 2.0` (area-relative, matching `render_canopy` — **not** `render_starfield`'s absolute convention), and `Camera::project_line` clip bounds of `area.width as f32 - 1.0 / 2.0` / `area.height as f32 - 1.0 / 4.0` (**not** bare `area.width as f32` / `area.height as f32`). This is the exact fix the windshield Arc's final review made to `render_canopy` after finding canopy pillars silently culled at 80-column terminals — copy `render_canopy`'s current call shape verbatim, do not reintroduce the unfixed bare-width form.
- **No `Polygon3`/`Canvas::fill_polygon` anywhere in this plan.** All three HUD states use only `Line3`/`project_line`/`Canvas::line`.
- **No `src/` changes.** Everything consumes `src/perspective.rs`/`Canvas` as already committed.
- **No new user input/interaction**, no pausing an unfocused state's animation, no `boot.rs` phase-boundary or timing changes beyond threading one new argument through two existing call sites.

---

### Task 1: Hyperdrive trajectory line

**Files:**
- Modify: `examples/falcon/falcon.rs`

**Interfaces:**
- Consumes: `Falcon` struct (`falcon.rs:139-157`), `new()` (`falcon.rs:160-192`), `on_tick` (`falcon.rs:440-469`), `view()` (`falcon.rs:423-430`), `theme.accent` (`Theme` field, already exists), `Camera`/`Point3`/`Line3` (already imported at `falcon.rs:10`), `Canvas`/`CanvasMode` (already imported at `falcon.rs:6`), `ttui::easing::lerp_color` (used elsewhere in `boot.rs`, needs a `use ttui::easing;` or fully-qualified call in `falcon.rs` — this file doesn't import it yet).
- Produces: `hyperdrive_phase: f32` field on `Falcon`, `fn render_hud_hyperdrive(&self, area: Rect, buf: &mut LayerStack)` — both consumed by Task 4.

- [ ] **Step 1: Add the Hyperdrive constants**

Near the top of `examples/falcon/falcon.rs`, immediately after the existing `CANOPY_HALF_H` const (`falcon.rs:30`), add:

```rust
const HYPERDRIVE_PHASE_SPEED: f32 = 1.5; // radians/sec
const HYPERDRIVE_DASH_COUNT: usize = 8;
const HYPERDRIVE_START: Point3 = Point3 {
    x: 0.0,
    y: 0.0,
    z: 2.5,
};
const HYPERDRIVE_END: Point3 = Point3 {
    x: 6.0,
    y: 2.0,
    z: 22.0,
};
```

- [ ] **Step 2: Add the `hyperdrive_phase` field**

In the `Falcon` struct (`falcon.rs:139-157`), add a new field right after `camera: Camera,`:

```rust
    hyperdrive_phase: f32,
```

In `new()` (`falcon.rs:171-191`), add the matching initializer right after `camera: falcon_camera(),`:

```rust
            hyperdrive_phase: 0.0,
```

- [ ] **Step 3: Advance `hyperdrive_phase` every tick**

In `on_tick` (`falcon.rs:440-469`), add this near the top of the method body (right after `self.tick_count += 1;` is fine — order relative to the glitch/particle/star logic already there doesn't matter):

```rust
        self.hyperdrive_phase = (self.hyperdrive_phase
            + HYPERDRIVE_PHASE_SPEED * elapsed.as_secs_f32())
            % std::f32::consts::TAU;
```

- [ ] **Step 4: Add `render_hud_hyperdrive`**

Add this new method inside `impl Falcon` (place it after `render_windshield`, `falcon.rs:364-379`, is a reasonable spot):

```rust
    fn render_hud_hyperdrive(&self, area: Rect, buf: &mut LayerStack) {
        let center_x = area.width as f32 / 2.0;
        let center_y = area.height as f32 / 2.0;
        let mut canvas = Canvas::new(area.width, area.height, CanvasMode::Braille);
        let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
        for i in 0..HYPERDRIVE_DASH_COUNT {
            let t0 = i as f32 / HYPERDRIVE_DASH_COUNT as f32;
            let t1 = (i + 1) as f32 / HYPERDRIVE_DASH_COUNT as f32;
            let seg = Line3 {
                start: Point3 {
                    x: lerp(HYPERDRIVE_START.x, HYPERDRIVE_END.x, t0),
                    y: lerp(HYPERDRIVE_START.y, HYPERDRIVE_END.y, t0),
                    z: lerp(HYPERDRIVE_START.z, HYPERDRIVE_END.z, t0),
                },
                end: Point3 {
                    x: lerp(HYPERDRIVE_START.x, HYPERDRIVE_END.x, t1),
                    y: lerp(HYPERDRIVE_START.y, HYPERDRIVE_END.y, t1),
                    z: lerp(HYPERDRIVE_START.z, HYPERDRIVE_END.z, t1),
                },
            };
            let phase_offset = i as f32 * (std::f32::consts::TAU / HYPERDRIVE_DASH_COUNT as f32);
            let brightness =
                0.3 + 0.7 * (0.5 + 0.5 * (self.hyperdrive_phase - phase_offset).sin());
            let color =
                ttui::easing::lerp_color(self.theme.background, self.theme.accent, brightness);
            if let Some((x0, y0, x1, y1)) = self.camera.project_line(
                seg,
                center_x,
                center_y,
                area.width as f32 - 1.0 / 2.0,
                area.height as f32 - 1.0 / 4.0,
                2.0,
                4.0,
                0.0,
            ) {
                canvas.line(x0, y0, x1, y1, color);
            }
        }
        canvas.blit(buf, area.x, area.y);
    }
```

- [ ] **Step 5: Temporarily wire it into `view()` for verification**

In `view()` (`falcon.rs:423-430`), the method currently ends with:

```rust
        self.render_dashboard(area, buf);
    }
```

Change it to:

```rust
        self.render_dashboard(area, buf);
        let (windshield, _) = Self::windshield_console_split(area);
        self.render_hud_hyperdrive(windshield, buf); // TEMPORARY — Task 4 replaces this with real focus-based dispatch
    }
```

- [ ] **Step 6: Build, lint, format**

Run: `cargo build --example falcon`
Expected: succeeds, no warnings.

Run: `cargo clippy --example falcon -- -D warnings`
Expected: clean.

Run: `cargo fmt --check -- examples/falcon/falcon.rs`
Expected: clean (no output).

- [ ] **Step 7: Capture and verify visually**

Run (from the repo root):
```
cargo run -p visual-snapshot -- --example falcon --size 100x35 --script <(echo '[{"wait_ms": 1600}, {"wait_ms": 300}]') --out /tmp/hyperdrive.gif
```
(If your shell doesn't support `<()` process substitution, write the script array to a temp `.json` file first and pass its path instead.)

`Read` the resulting GIF frames (split with `ffmpeg -i /tmp/hyperdrive.gif frame-%02d.png` if needed). Confirm:
- A gold (`theme.accent`-colored), dashed diagonal beam is visible in the windshield area, extending up and to the right into the starfield.
- Comparing the two post-boot frames (300ms apart), the brightness pattern along the dashes has visibly shifted — proving the traveling-pulse animation is live, not a static image.

- [ ] **Step 8: Commit**

```bash
git add examples/falcon/falcon.rs
git commit -m "feat(falcon): add animated Hyperdrive trajectory-line HUD state (temporary view() wire-in)"
```

---

### Task 2: Sensors radar sweep

**Files:**
- Modify: `examples/falcon/falcon.rs`

**Interfaces:**
- Consumes: same as Task 1 (this task does not depend on Task 1's own additions — independent, sequenced after it only to keep the diff history simple).
- Produces: `sensor_sweep_angle: f32` field, `fn render_hud_sensors(&self, area: Rect, buf: &mut LayerStack)` — consumed by Task 4.

- [ ] **Step 1: Add the Sensors constants**

Immediately after Task 1's `HYPERDRIVE_END` const, add:

```rust
const SENSOR_SWEEP_SPEED: f32 = std::f32::consts::TAU / 4.0; // one revolution per ~4s
const SENSOR_PLANE_Z: f32 = 6.0;
const SENSOR_RADIUS: f32 = 3.0;
const SENSOR_TRAIL_COUNT: usize = 4;
const SENSOR_TRAIL_STEP: f32 = 0.25; // radians between trailing lines
```

- [ ] **Step 2: Add the `sensor_sweep_angle` field**

In the `Falcon` struct, add right after `hyperdrive_phase: f32,` (Task 1):

```rust
    sensor_sweep_angle: f32,
```

In `new()`, add right after `hyperdrive_phase: 0.0,`:

```rust
            sensor_sweep_angle: 0.0,
```

- [ ] **Step 3: Advance `sensor_sweep_angle` every tick**

In `on_tick`, add right after Task 1's `hyperdrive_phase` update:

```rust
        self.sensor_sweep_angle = (self.sensor_sweep_angle
            + SENSOR_SWEEP_SPEED * elapsed.as_secs_f32())
            % std::f32::consts::TAU;
```

- [ ] **Step 4: Add `render_hud_sensors`**

Add this new method, right after `render_hud_hyperdrive` (Task 1):

```rust
    fn render_hud_sensors(&self, area: Rect, buf: &mut LayerStack) {
        let center_x = area.width as f32 / 2.0;
        let center_y = area.height as f32 / 2.0;
        let mut canvas = Canvas::new(area.width, area.height, CanvasMode::Braille);
        let center = Point3 {
            x: 0.0,
            y: 0.0,
            z: SENSOR_PLANE_Z,
        };
        for k in 0..=SENSOR_TRAIL_COUNT {
            let angle = self.sensor_sweep_angle - k as f32 * SENSOR_TRAIL_STEP;
            let tip = Point3 {
                x: SENSOR_RADIUS * angle.cos(),
                y: SENSOR_RADIUS * angle.sin(),
                z: SENSOR_PLANE_Z,
            };
            let brightness = 1.0 - (k as f32 / (SENSOR_TRAIL_COUNT + 1) as f32);
            let color =
                ttui::easing::lerp_color(self.theme.background, self.theme.secondary, brightness);
            let line = Line3 {
                start: center,
                end: tip,
            };
            if let Some((x0, y0, x1, y1)) = self.camera.project_line(
                line,
                center_x,
                center_y,
                area.width as f32 - 1.0 / 2.0,
                area.height as f32 - 1.0 / 4.0,
                2.0,
                4.0,
                0.0,
            ) {
                canvas.line(x0, y0, x1, y1, color);
            }
        }
        canvas.blit(buf, area.x, area.y);
    }
```

- [ ] **Step 5: Temporarily wire it in alongside Task 1's**

In `view()`, change:

```rust
        self.render_dashboard(area, buf);
        let (windshield, _) = Self::windshield_console_split(area);
        self.render_hud_hyperdrive(windshield, buf); // TEMPORARY — Task 4 replaces this with real focus-based dispatch
    }
```

to:

```rust
        self.render_dashboard(area, buf);
        let (windshield, _) = Self::windshield_console_split(area);
        self.render_hud_hyperdrive(windshield, buf); // TEMPORARY — Task 4 replaces this with real focus-based dispatch
        self.render_hud_sensors(windshield, buf); // TEMPORARY — Task 4 replaces this with real focus-based dispatch
    }
```

(Both render on top of each other for now — that's expected and fine; Task 4 replaces both lines with the real one-at-a-time dispatch.)

- [ ] **Step 6: Build, lint, format**

Run: `cargo build --example falcon`
Expected: succeeds, no warnings.

Run: `cargo clippy --example falcon -- -D warnings`
Expected: clean.

Run: `cargo fmt --check -- examples/falcon/falcon.rs`
Expected: clean.

- [ ] **Step 7: Capture and verify visually**

Capture two post-boot frames ~300ms apart (same technique as Task 1). `Read` both. Confirm:
- A green (`theme.secondary`-colored) radial sweep line is visible, with a visible dimmer trailing wedge behind it (not just one bare line).
- The sweep's angle has visibly rotated between the two frames.
- The Hyperdrive beam from Task 1 is still visible too (both are temporarily wired in simultaneously) — confirms Task 2 didn't break Task 1.

- [ ] **Step 8: Commit**

```bash
git add examples/falcon/falcon.rs
git commit -m "feat(falcon): add animated Sensors radar-sweep HUD state (temporary view() wire-in)"
```

---

### Task 3: Weapons reticle

**Files:**
- Modify: `examples/falcon/falcon.rs`

**Interfaces:**
- Consumes: same as Tasks 1-2 (independent of both, sequenced after them only for diff-history simplicity).
- Produces: `weapons_pulse_phase: f32` field, `fn render_hud_weapons(&self, area: Rect, buf: &mut LayerStack)` — consumed by Task 4.

- [ ] **Step 1: Add the Weapons constants**

Immediately after Task 2's `SENSOR_TRAIL_STEP` const, add:

```rust
const WEAPONS_PULSE_SPEED: f32 = 3.0; // radians/sec
const WEAPONS_PLANE_Z: f32 = 5.0;
const WEAPONS_BASE_HALF_SIZE: f32 = 2.0;
const WEAPONS_PULSE_AMPLITUDE: f32 = 0.15;
const WEAPONS_BRACKET_LEN: f32 = 0.7;
```

- [ ] **Step 2: Add the `weapons_pulse_phase` field**

In the `Falcon` struct, add right after `sensor_sweep_angle: f32,` (Task 2):

```rust
    weapons_pulse_phase: f32,
```

In `new()`, add right after `sensor_sweep_angle: 0.0,`:

```rust
            weapons_pulse_phase: 0.0,
```

- [ ] **Step 3: Advance `weapons_pulse_phase` every tick**

In `on_tick`, add right after Task 2's `sensor_sweep_angle` update:

```rust
        self.weapons_pulse_phase = (self.weapons_pulse_phase
            + WEAPONS_PULSE_SPEED * elapsed.as_secs_f32())
            % std::f32::consts::TAU;
```

- [ ] **Step 4: Add `render_hud_weapons`**

Add this new method, right after `render_hud_sensors` (Task 2):

```rust
    fn render_hud_weapons(&self, area: Rect, buf: &mut LayerStack) {
        let center_x = area.width as f32 / 2.0;
        let center_y = area.height as f32 / 2.0;
        let half = WEAPONS_BASE_HALF_SIZE
            * (1.0 + WEAPONS_PULSE_AMPLITUDE * self.weapons_pulse_phase.sin());
        let mut canvas = Canvas::new(area.width, area.height, CanvasMode::Braille);
        let corners = [(-half, -half), (half, -half), (half, half), (-half, half)];
        for &(cx, cy) in &corners {
            let dx = if cx < 0.0 {
                WEAPONS_BRACKET_LEN
            } else {
                -WEAPONS_BRACKET_LEN
            };
            let dy = if cy < 0.0 {
                WEAPONS_BRACKET_LEN
            } else {
                -WEAPONS_BRACKET_LEN
            };
            let corner = Point3 {
                x: cx,
                y: cy,
                z: WEAPONS_PLANE_Z,
            };
            let horiz = Line3 {
                start: corner,
                end: Point3 {
                    x: cx + dx,
                    y: cy,
                    z: WEAPONS_PLANE_Z,
                },
            };
            let vert = Line3 {
                start: corner,
                end: Point3 {
                    x: cx,
                    y: cy + dy,
                    z: WEAPONS_PLANE_Z,
                },
            };
            for seg in [horiz, vert] {
                if let Some((x0, y0, x1, y1)) = self.camera.project_line(
                    seg,
                    center_x,
                    center_y,
                    area.width as f32 - 1.0 / 2.0,
                    area.height as f32 - 1.0 / 4.0,
                    2.0,
                    4.0,
                    0.0,
                ) {
                    canvas.line(x0, y0, x1, y1, self.theme.tertiary);
                }
            }
        }
        canvas.blit(buf, area.x, area.y);
    }
```

- [ ] **Step 5: Temporarily wire it in alongside Tasks 1-2's**

In `view()`, add a third temporary line:

```rust
        self.render_dashboard(area, buf);
        let (windshield, _) = Self::windshield_console_split(area);
        self.render_hud_hyperdrive(windshield, buf); // TEMPORARY — Task 4 replaces this with real focus-based dispatch
        self.render_hud_sensors(windshield, buf); // TEMPORARY — Task 4 replaces this with real focus-based dispatch
        self.render_hud_weapons(windshield, buf); // TEMPORARY — Task 4 replaces this with real focus-based dispatch
    }
```

- [ ] **Step 6: Build, lint, format**

Run: `cargo build --example falcon`
Expected: succeeds, no warnings.

Run: `cargo clippy --example falcon -- -D warnings`
Expected: clean.

Run: `cargo fmt --check -- examples/falcon/falcon.rs`
Expected: clean.

- [ ] **Step 7: Capture and verify visually**

Capture two post-boot frames ~300ms apart. `Read` both. Confirm:
- A red (`theme.tertiary`-colored) set of 4 corner brackets is visible, forming a square outline made of L-shapes (not a closed rectangle).
- The square's size has visibly changed slightly between the two frames (the breathing pulse).
- Hyperdrive's beam and Sensors' sweep are both still visible too — confirms Task 3 didn't break Tasks 1-2.

- [ ] **Step 8: Commit**

```bash
git add examples/falcon/falcon.rs
git commit -m "feat(falcon): add animated Weapons reticle HUD state (temporary view() wire-in)"
```

---

### Task 4: Real assembly — focus-based dispatch + boot integration

**Files:**
- Modify: `examples/falcon/falcon.rs`
- Modify: `examples/falcon/boot.rs`

**Interfaces:**
- Consumes: `render_hud_hyperdrive`/`render_hud_sensors`/`render_hud_weapons` (Tasks 1-3, exact signatures `fn(&self, area: Rect, buf: &mut LayerStack)`), `PanelKind`/`PANELS` (already exist, `falcon.rs:66-87`), `render_windshield` (`falcon.rs:364-379`), `render_dashboard`'s call to `render_windshield` (`falcon.rs:229`), `boot.rs`'s two `render_windshield` calls (`boot.rs:33` and `boot.rs:40`).
- Produces: `fn render_hud(&self, area: Rect, buf: &mut LayerStack)`, `render_windshield`'s new 4-parameter signature `fn render_windshield(&self, area: Rect, buf: &mut LayerStack, canopy_edges_shown: usize, show_hud: bool)` — both are the final, permanent interface; no later task changes them.

- [ ] **Step 1: Add the `render_hud` dispatch method**

Add this new method in `falcon.rs`, right after `render_hud_weapons` (Task 3):

```rust
    fn render_hud(&self, area: Rect, buf: &mut LayerStack) {
        match PANELS[self.focused] {
            PanelKind::Hyperdrive => self.render_hud_hyperdrive(area, buf),
            PanelKind::Sensors => self.render_hud_sensors(area, buf),
            PanelKind::Weapons => self.render_hud_weapons(area, buf),
        }
    }
```

- [ ] **Step 2: Give `render_windshield` a `show_hud` parameter**

Current `render_windshield` (`falcon.rs:364-379`):

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

Change the signature and add the HUD call at the end:

```rust
    fn render_windshield(
        &self,
        area: Rect,
        buf: &mut LayerStack,
        canopy_edges_shown: usize,
        show_hud: bool,
    ) {
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
        if show_hud {
            self.render_hud(area, buf);
        }
    }
```

- [ ] **Step 3: Update `render_dashboard`'s call site**

In `render_dashboard` (`falcon.rs:229`), change:

```rust
        self.render_windshield(windshield, buf, 12);
```

to:

```rust
        self.render_windshield(windshield, buf, 12, true);
```

- [ ] **Step 4: Remove Tasks 1-3's temporary `view()` wiring**

In `view()`, remove all three temporary lines added by Tasks 1-3, restoring it to:

```rust
        self.render_dashboard(area, buf);
    }
```

- [ ] **Step 5: Update `boot.rs`'s two `render_windshield` call sites**

In `examples/falcon/boot.rs`, the windshield-power-on phase (currently `boot.rs:33`):

```rust
            self.render_windshield(windshield, buf, edges_shown);
```

becomes:

```rust
            self.render_windshield(windshield, buf, edges_shown, false);
```

And the console-reveal phase (currently `boot.rs:40`):

```rust
            self.render_windshield(windshield, buf, 12);
```

becomes:

```rust
            self.render_windshield(windshield, buf, 12, true);
```

- [ ] **Step 6: Build, lint, format**

Run: `cargo build --all-targets`
Expected: succeeds, no warnings.

Run: `cargo clippy --all-targets -- -D warnings`
Expected: clean.

Run: `cargo fmt --check`
Expected: clean.

- [ ] **Step 7: Capture and verify visually — focus-based dispatch**

Capture a post-boot script that Tabs through all three panels, e.g. steps: `{"wait_ms": 1600}` (post-boot), `{"wait_ms": 200}`, `{"key": "Tab"}`, `{"wait_ms": 200}`, `{"key": "Tab"}`, `{"wait_ms": 200}`. `Read` each frame. Confirm:
- Frame at initial focus (Hyperdrive, index 0): only the gold beam is visible in the windshield, no sweep or reticle.
- Frame after first Tab (Sensors): only the green sweep is visible, the beam is gone.
- Frame after second Tab (Weapons): only the red brackets are visible, the sweep is gone.

- [ ] **Step 8: Capture and verify visually — idle background animation**

With focus left on Hyperdrive (index 0) after boot, capture two frames several seconds apart (e.g. `{"wait_ms": 1600}`, `{"wait_ms": 3000}`) — no Tab presses. Then Tab to Sensors and capture one more frame. Confirm the Sensors sweep, when it finally appears, is NOT at its initial angle (`0.0`) — it should be wherever `SENSOR_SWEEP_SPEED * ~4.6s` puts it, proving its angle field kept advancing in `on_tick` the whole time Hyperdrive was focused and Sensors wasn't being rendered at all.

- [ ] **Step 9: Capture and verify visually — boot timing**

Capture a script spanning boot's phase 2→3 boundary precisely: steps sized around `BOOT_TOTAL_MS = 1400`'s `progress = 0.4` mark (560ms into boot) — e.g. `{"wait_ms": 500}`, `{"wait_ms": 100}`, `{"wait_ms": 100}`. `Read` all three frames. Confirm the HUD (whichever state matches `self.focused`'s initial value, index 0 = Hyperdrive) is absent in the first frame (progress ≈0.36, still phase 2) and present by the third (progress ≈0.57, into phase 3) — appearing at the same beat the first console panel does, not before.

- [ ] **Step 10: Capture and verify visually — 80 columns**

Capture a single post-boot frame (`{"wait_ms": 1600}`) at `--size 80x24`. `Read` it. Confirm the visible HUD state (Hyperdrive, the default focus) renders without any asymmetric culling (compare the beam's two halves relative to the windshield's center — both should be present; if only a partial beam renders, check the clip-bound arithmetic in Step matches the Global Constraints' mandatory pattern exactly).

- [ ] **Step 11: Commit**

```bash
git add examples/falcon/falcon.rs examples/falcon/boot.rs
git commit -m "feat(falcon): wire real focus-based HUD dispatch into windshield + boot, retire temporary wiring"
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
Expected: full suite green — this plan adds no new unit tests (example code, TDD-exempt) and touches no `src/` file, so nothing should change here versus `main` post-#101.

- [ ] **Step 4: One more full `tools/visual-snapshot` capture of the finished result**

Run a capture spanning the full boot sequence, a Tab through all three HUD states, and a few seconds of idle time on the last one (long enough to see its own animation continue). `Read` it. This is the final, whole-Arc confirmation — a single artifact demonstrating boot, all three HUD states, and their focus-driven swap all together. Reference this capture in the PR's Verification section.

## Final verification (whole plan)

- [ ] `cargo build --all-targets` succeeds.
- [ ] `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` — clean.
- [ ] `cargo test` — full suite green, unchanged from `main`.
- [ ] At least one `tools/visual-snapshot` capture from Task 5 is referenced in the PR description, showing all three HUD states and the boot-timed reveal.
- [ ] Per `.claude/rules/git-github-standards.md`: open a PR from this Arc's worktree branch to `main`, wait for all four required checks green, squash-merge, then remove the worktree via `ExitWorktree` (per the now-documented squash-merge resolution: verify via `gh pr view --json state,mergedAt,mergeCommit`, then retry with `discard_changes: true` if the tool's own ancestry check false-positives).
