# Falcon HUD System — Design

**Status:** draft, pending your review before we move to planning.
**Date:** 2026-08-12
**Relationship to prior specs:** the deferred second half of
`2026-08-11-falcon-windshield-redesign-design.md` (PR #101, merged) —
that Arc explicitly scoped out "the three distinct 3D HUD states" as a
Non-goal, deliberately, so the windshield shipped as a complete,
coherent visual change on its own. This Arc adds those three states:
Hyperdrive trajectory line, Sensors radar sweep, Weapons reticle — one
rendered at a time, matching whichever console panel is currently
focused. Builds on `src/perspective.rs` (`Camera`/`Point3`/`Line3`,
`2026-08-11-perspective-projection-graduation-design.md`, PR #100) and
directly reuses the `render_canopy` pattern (`Line3` → `Camera::
project_line` → `Canvas::line` in Braille mode) the windshield Arc
already proved out, including its final-review-fixed subpixel
clip-bound arithmetic (see Slice 1).

## Problem

The windshield redesign gave Falcon a real cockpit view, but the glass
itself is inert — nothing on it reflects which system (Hyperdrive,
Sensors, Weapons) is currently focused below. A real HUD overlay, one
state at a time, ties the windshield and the console together instead
of leaving them as two unrelated rendering layers stacked on top of
each other.

## Scope

**Tag: `coding`, TDD-exempt** — all changes are to `examples/falcon/`,
covered by the "Examples/demos" exception in
`.claude/rules/development-conventions.md`. **`tools/visual-snapshot`
is mandatory for every task and the final review** — this Arc directly
changes `examples/falcon/falcon.rs`'s `view()`/`on_tick()` render path
and `boot.rs`, the same trigger the windshield Arc's spec already
established.

Five slices, in dependency order:

1. **HUD animation state + dispatch skeleton** (`falcon.rs`) — the
   three phase fields, their `on_tick` advancement, and the
   `render_hud` dispatch method (bodies filled in by Slices 2-4).
2. **Hyperdrive trajectory line** (`falcon.rs`) — depends on 1.
3. **Sensors radar sweep** (`falcon.rs`) — depends on 1, independent of 2.
4. **Weapons reticle** (`falcon.rs`) — depends on 1, independent of 2-3.
5. **Boot integration** (`falcon.rs`, `boot.rs`) — depends on 1-4; wires
   `render_hud` into `render_windshield` behind a new `show_hud` gate
   and threads it through boot's existing phase 2/3 call sites.

## Design

### Slice 1: HUD animation state + dispatch skeleton

Three new `f32` fields on `Falcon`, each advanced unconditionally every
tick (regardless of which panel is focused — an unfocused state's
animation keeps running in the background, so tabbing back to it never
shows a reset), wrapped mod `TAU` to keep them bounded over a long
session:

```rust
hyperdrive_phase: f32,
sensor_sweep_angle: f32,
weapons_pulse_phase: f32,
```

Initialized to `0.0` in `new()`. `on_tick` gains:

```rust
const HYPERDRIVE_PHASE_SPEED: f32 = 1.5; // radians/sec
const SENSOR_SWEEP_SPEED: f32 = std::f32::consts::TAU / 4.0; // one revolution per ~4s
const WEAPONS_PULSE_SPEED: f32 = 3.0; // radians/sec

self.hyperdrive_phase =
    (self.hyperdrive_phase + HYPERDRIVE_PHASE_SPEED * elapsed.as_secs_f32())
        % std::f32::consts::TAU;
self.sensor_sweep_angle =
    (self.sensor_sweep_angle + SENSOR_SWEEP_SPEED * elapsed.as_secs_f32())
        % std::f32::consts::TAU;
self.weapons_pulse_phase =
    (self.weapons_pulse_phase + WEAPONS_PULSE_SPEED * elapsed.as_secs_f32())
        % std::f32::consts::TAU;
```

Dispatch:

```rust
fn render_hud(&self, area: Rect, buf: &mut LayerStack) {
    match PANELS[self.focused] {
        PanelKind::Hyperdrive => self.render_hud_hyperdrive(area, buf),
        PanelKind::Sensors => self.render_hud_sensors(area, buf),
        PanelKind::Weapons => self.render_hud_weapons(area, buf),
    }
}
```

**Mandatory pattern for every HUD render method (Slices 2-4):** each
one draws via its own `Canvas::new(area.width, area.height,
CanvasMode::Braille)`, projects with `center_x = area.width as f32 /
2.0` / `center_y = area.height as f32 / 2.0` (**area-relative**, not
absolute — matching `render_canopy`'s coordinate convention, not
`render_starfield`'s), and calls `Camera::project_line` with clip
bounds `area.width as f32 - 1.0 / 2.0` / `area.height as f32 - 1.0 /
4.0` (**not** bare `area.width as f32` / `area.height as f32`) — this
is the exact closed-interval/subpixel fix the windshield Arc's final
review found and fixed for `render_canopy` (canopy pillars silently
culled at 80-column terminals otherwise). Copy `render_canopy`'s
current call shape verbatim for every `project_line` call in this Arc;
do not reintroduce the unfixed bare-width form.

Each render method ends with `canvas.blit(buf, area.x, area.y)`, same
as `render_canopy`.

### Slice 2: Hyperdrive trajectory line

A single beam from just past the near plane into deep space, angled
off-axis so it reads as a calculated jump vector rather than a straight
line dead ahead — split into fixed dash segments with a brightness wave
traveling along them, driven by `hyperdrive_phase`:

```rust
const HYPERDRIVE_DASH_COUNT: usize = 8;
const HYPERDRIVE_START: Point3 = Point3 { x: 0.0, y: 0.0, z: 2.5 };
const HYPERDRIVE_END: Point3 = Point3 { x: 6.0, y: 2.0, z: 22.0 };

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
        let brightness = 0.3 + 0.7 * (0.5 + 0.5 * (self.hyperdrive_phase - phase_offset).sin());
        let color = ttui::easing::lerp_color(self.theme.background, self.theme.accent, brightness);
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

`theme.accent` (gold) — visually distinct from the canopy's
`theme.secondary` (green) and the starfield's white/gray.

### Slice 3: Sensors radar sweep

A circle facing the camera dead-on at a fixed depth. The sweep is a
single radial line from the circle's center to its current angle on the
circumference, plus a decaying trail of previous angles — classic
radar-persistence, not a bare line snapping around:

```rust
const SENSOR_PLANE_Z: f32 = 6.0;
const SENSOR_RADIUS: f32 = 3.0;
const SENSOR_TRAIL_COUNT: usize = 4;
const SENSOR_TRAIL_STEP: f32 = 0.25; // radians between trailing lines

fn render_hud_sensors(&self, area: Rect, buf: &mut LayerStack) {
    let center_x = area.width as f32 / 2.0;
    let center_y = area.height as f32 / 2.0;
    let mut canvas = Canvas::new(area.width, area.height, CanvasMode::Braille);
    let center = Point3 { x: 0.0, y: 0.0, z: SENSOR_PLANE_Z };
    for k in 0..=SENSOR_TRAIL_COUNT {
        let angle = self.sensor_sweep_angle - k as f32 * SENSOR_TRAIL_STEP;
        let tip = Point3 {
            x: SENSOR_RADIUS * angle.cos(),
            y: SENSOR_RADIUS * angle.sin(),
            z: SENSOR_PLANE_Z,
        };
        let brightness = 1.0 - (k as f32 / (SENSOR_TRAIL_COUNT + 1) as f32);
        let color = ttui::easing::lerp_color(self.theme.background, self.theme.secondary, brightness);
        let line = Line3 { start: center, end: tip };
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

`k = 0` is the current sweep position at full brightness; `k = 1..=4`
are progressively dimmer trailing lines. `theme.secondary` (green) —
same tone as the canopy, so the sweep reads as part of the same
instrument layer rather than a competing color.

### Slice 4: Weapons reticle

Four independent corner brackets (L-shapes, not a closed outline) at
the corners of a square whose size breathes via `weapons_pulse_phase`:

```rust
const WEAPONS_PLANE_Z: f32 = 5.0;
const WEAPONS_BASE_HALF_SIZE: f32 = 2.0;
const WEAPONS_PULSE_AMPLITUDE: f32 = 0.15;
const WEAPONS_BRACKET_LEN: f32 = 0.7;

fn render_hud_weapons(&self, area: Rect, buf: &mut LayerStack) {
    let center_x = area.width as f32 / 2.0;
    let center_y = area.height as f32 / 2.0;
    let half =
        WEAPONS_BASE_HALF_SIZE * (1.0 + WEAPONS_PULSE_AMPLITUDE * self.weapons_pulse_phase.sin());
    let mut canvas = Canvas::new(area.width, area.height, CanvasMode::Braille);
    let corners = [(-half, -half), (half, -half), (half, half), (-half, half)];
    for &(cx, cy) in &corners {
        let dx = if cx < 0.0 { WEAPONS_BRACKET_LEN } else { -WEAPONS_BRACKET_LEN };
        let dy = if cy < 0.0 { WEAPONS_BRACKET_LEN } else { -WEAPONS_BRACKET_LEN };
        let corner = Point3 { x: cx, y: cy, z: WEAPONS_PLANE_Z };
        let horiz = Line3 {
            start: corner,
            end: Point3 { x: cx + dx, y: cy, z: WEAPONS_PLANE_Z },
        };
        let vert = Line3 {
            start: corner,
            end: Point3 { x: cx, y: cy + dy, z: WEAPONS_PLANE_Z },
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

`theme.tertiary` (red) — "weapons hot," distinct from both other
states.

### Slice 5: Boot integration

`render_windshield` gains a `show_hud: bool` parameter:

```rust
fn render_windshield(
    &self,
    area: Rect,
    buf: &mut LayerStack,
    canopy_edges_shown: usize,
    show_hud: bool,
) {
    // ...existing bg fill + render_starfield + render_canopy, unchanged...
    if show_hud {
        self.render_hud(area, buf);
    }
}
```

Three call sites:

- `render_dashboard` (`falcon.rs`) — always `true`. This is the only
  change `render_dashboard` itself needs; it's used both for the normal
  post-boot frame and, unchanged, inside boot's own `[0.85, 1.0]`
  fade-scratch render.
- `boot.rs`, `[0.1, 0.4)` windshield-power-on phase — `false`. The HUD
  has no reveal animation of its own; it simply isn't part of this
  phase.
- `boot.rs`, `[0.4, 0.85)` console-reveal phase — `true`. The HUD pops
  in fully formed at the same moment (`progress = 0.4`) the first
  console panel does — one more system coming online alongside the
  panels, not a separate boot beat. No new phase boundary, no reveal
  fraction for the HUD itself.

No other change to `boot.rs`'s phase structure, timing, or the
`[0.85, 1.0]` fade mechanic — the HUD rides that whole-frame fade like
everything else in `render_dashboard`, since it renders inside the
scratch buffer that phase already composites and dims.

## Non-goals

- **Any new user input or interaction.** The HUD is purely atmospheric
  — no "fire," "lock," or "sweep" key. Sensors doesn't detect anything,
  Weapons doesn't track a target, Hyperdrive doesn't complete a jump.
  Matches Falcon's existing scope (every panel still reads "(not yet
  built)").
- **`Polygon3`/`Canvas::fill_polygon`.** All three states use only
  `Line3`/`project_line`/`Canvas::line` — the pattern already proven
  end-to-end by `render_canopy`, including its fixed subpixel clip
  bounds. No filled shape is needed for any of the three concepts.
- **Any `src/` change.** Everything here consumes `src/perspective.rs`
  and `Canvas` as committed; no new primitive, no bug fix.
- **Pausing an unfocused state's animation**, or any lock-on/detection
  state machine — decided explicitly during brainstorming in favor of
  always-advancing, stateless-per-frame animation.
- **Changing `boot.rs`'s phase boundaries or timing.** Only the
  `show_hud` argument is threaded through the two existing windshield
  call sites inside boot; the phase percentages, the console reveal
  mechanic, and the fade mechanic are untouched.

## Testing

Per `.claude/rules/development-conventions.md`: example code,
TDD-exempt, correctness checked by running. **`tools/visual-snapshot`
is mandatory for every task and the final review.** Each task/review
should capture at minimum:

- Each of the three HUD states individually (Tab-cycling through all
  three), confirming distinct geometry, distinct color, and visible
  motion between two captures a fraction of a second apart.
- An idle-time capture confirming an *unfocused* state's animation
  phase has visibly changed between two captures taken a few seconds
  apart while a different panel stayed focused the whole time (proves
  the always-advancing decision, not just that the focused state
  moves).
- A boot-sequence capture confirming the initially-focused panel's HUD
  state appears at `progress = 0.4`, alongside the first console panel,
  not before and not after.
- At least one capture at 80 columns (the terminal size the windshield
  Arc's canopy bug lived at) confirming no HUD element is asymmetrically
  culled the same way.

## Critical files

- `examples/falcon/falcon.rs` — new fields, `on_tick` additions,
  `render_hud` + three `render_hud_*` methods, `render_windshield`'s
  new parameter.
- `examples/falcon/boot.rs` — two call-site updates (no phase
  restructuring).

## Verification

- `cargo build --example falcon` / `cargo clippy --all-targets -- -D
  warnings` / `cargo fmt --check` — clean.
- `tools/visual-snapshot` captures per the Testing section above,
  `Read` and confirmed by whoever runs them, not just claimed.
- Manual key-logic check (via the capture script's `key` steps):
  Tab/Shift+Tab still cycle focus correctly, and the HUD swaps to match
  the newly-focused panel within the same frame the console panel
  itself re-highlights.
