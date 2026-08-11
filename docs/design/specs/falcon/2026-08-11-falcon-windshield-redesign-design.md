# Falcon Windshield Redesign — Design

**Status:** draft, pending your review before we move to planning.
**Date:** 2026-08-11
**Relationship to prior specs:** revises the shipped Falcon dashboard
hub (`2026-08-09-falcon-dashboard-hub-design.md`, PR #98) — replaces
its full-screen 3-panel layout with a first-person cockpit windshield
(drifting starfield + wireframe canopy frame, both real 3D-projected)
over a slim console strip, per the reference-image-driven redesign
brainstormed after the hub shipped. Keeps the hub's mechanics intact:
`CockpitPanel`, `GlitchBuffer`-driven percussive maintenance, Tab/
Shift+Tab panel cycling, the boot sequence's overall phase structure.
Builds directly on `src/perspective.rs` (`Camera`/`Point3`/`Line3`,
`2026-08-11-perspective-projection-graduation-design.md`, PR #100) —
the first real consumer of that module. **First of two Arcs**: this
one covers the windshield, canopy, console re-layout, and boot rework;
a follow-up covers the three distinct per-focus 3D HUD states
(Hyperdrive trajectory line, Sensors radar sweep, Weapons reticle) —
deliberately deferred so this Arc ships a complete, coherent visual
change on its own, matching the hub-first-then-details split already
used for the original Falcon build and for Falcon's own sub-app
content.

## Problem

The shipped hub fills the whole screen with three side-by-side panels
— it doesn't read as a cockpit at all, just a dashboard. The reference
image that motivated this redesign shows a first-person view out a
canopy window into space, with a slim instrument console along the
bottom edge. This spec translates that into TTUI: real 3D-projected
parallax (not the flat 2D drift the original brainstorm considered
before `src/perspective.rs` existed) for the starfield, a wireframe
canopy frame using the same real projection, and the existing console
mechanics shrunk into a strip instead of filling the screen.

## Scope

**Tag: `coding`, TDD-exempt** — all changes are to `examples/falcon/`,
covered by the "Examples/demos" exception in
`.claude/rules/development-conventions.md`. **`tools/visual-snapshot`
is mandatory for every task and the final review in this Arc's plan**
— unlike the graduation Arc this builds on, this one directly changes
an example's `view()`/`on_tick()`, which is the other half of the
"Visual review" rule's disjunction (rendering-affecting `src/` code OR
an example's render loop) and unambiguously applies here.

Four slices, in dependency order:

1. **Camera + starfield** (`examples/falcon/falcon.rs`)
2. **Canopy frame** (`examples/falcon/falcon.rs`) — depends on 1 for
   the shared `Camera`.
3. **Console strip re-layout** (`examples/falcon/falcon.rs`) —
   independent of 1-2, but sequenced after them so the windshield
   exists before the layout split that makes room for it.
4. **Boot sequence rework** (`examples/falcon/boot.rs`) — depends on
   1-3.

## Design

### Slice 1: Camera + starfield

```rust
fn falcon_camera() -> Camera {
    Camera {
        near: 0.5,
        focal_length: 8.0,
    }
}
```

Same values already validated throughout `src/perspective.rs`'s own
test suite and `depth_spike.rs`.

```rust
const STAR_COUNT: usize = 60;
const STAR_SPEED: f32 = 3.0; // z-units/second
const STAR_RESPAWN_Z: f32 = 20.0;

struct Star {
    x: f32,
    y: f32,
    z: f32,
}

fn scatter(seed: u32, spread: f32) -> f32 {
    let h = (seed.wrapping_mul(2_654_435_761)) ^ (seed.wrapping_mul(40_503).rotate_left(13));
    ((h % 10_000) as f32 / 10_000.0 - 0.5) * spread
}
```

Identical shape to `depth_spike.rs`'s `Star`/`scatter` (deterministic,
no RNG dependency, same posture as every prior Arc) — `STAR_COUNT` is
lower (60 vs the spike's 80) since the windshield only occupies part
of the screen (see Slice 3), not the whole terminal.

`Falcon` gains `camera: Camera` and `stars: Vec<Star>` fields,
initialized via `falcon_camera()` and the same seeded-scatter
construction pattern `depth_spike.rs::DepthSpike::new()` uses.
`on_tick` gains the same per-star `z -= STAR_SPEED * elapsed` and
respawn-past-`STAR_RESPAWN_Z` logic, keyed off the existing
`tick_count` field (already present, already incremented once per
tick) instead of a new counter.

**Rendering** (new method, called from a windshield-rendering entry
point added in Slice 3):

```rust
fn render_starfield(&self, area: Rect, buf: &mut LayerStack) {
    let center_x = area.x as f32 + area.width as f32 / 2.0;
    let center_y = area.y as f32 + area.height as f32 / 2.0;
    for star in &self.stars {
        let p = Point3 { x: star.x, y: star.y, z: star.z };
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

Same glyph/brightness-from-`scale` mapping `depth_spike.rs` already
validated (both by hand-trace and by a real `tools/visual-snapshot`
capture during the graduation Arc's final review) — reused verbatim,
not reinvented. The bounds check is against `area` (the windshield
sub-region from Slice 3), not the whole terminal, since `center_x`/
`center_y` are now `area`-relative.

### Slice 2: Canopy frame

```rust
const CANOPY_NEAR_Z: f32 = 2.0;
const CANOPY_FAR_Z: f32 = 10.0;
const CANOPY_HALF_W: f32 = 5.0;
const CANOPY_HALF_H: f32 = 3.0;

/// The canopy's 8 corners: two parallel rectangles (near/far) of the
/// same world-space size, connected by 4 verticals — the perspective
/// convergence comes entirely from the projection (constant world
/// width at greater `z` reads as narrower on screen), not from
/// shrinking the far rectangle's world-space size. Index order:
/// `i = (dx_idx*2 + dy_idx)*2 + z_idx` for `dx_idx, dy_idx, z_idx`
/// each in `{0 (near/-), 1 (far/+)}` — identical convention to
/// `depth_spike.rs::cube_vertices`, since the topology (two parallel
/// rectangles joined by 4 edges) is the same shape, just non-cubic.
fn canopy_vertices() -> [Point3; 8] {
    let mut v = [Point3 { x: 0.0, y: 0.0, z: 0.0 }; 8];
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

/// Same edge topology as `depth_spike.rs::CUBE_EDGES` (4 near-rect
/// edges via dx/dy pairs, 4 far-rect edges, 4 near-to-far connectors).
const CANOPY_EDGES: [(usize, usize); 12] = [
    (0, 4), (1, 5), (2, 6), (3, 7), // edges along dx
    (0, 2), (1, 3), (4, 6), (5, 7), // edges along dy
    (0, 1), (2, 3), (4, 5), (6, 7), // near-to-far connectors
];
```

**Rendering** (new method):

```rust
fn render_canopy(&self, area: Rect, buf: &mut LayerStack, edges_shown: usize) {
    let center_x = area.x as f32 + area.width as f32 / 2.0;
    let center_y = area.y as f32 + area.height as f32 / 2.0;
    let verts = canopy_vertices();
    let mut canvas = Canvas::new(area.width, area.height, CanvasMode::Braille);
    for &(a, b) in CANOPY_EDGES.iter().take(edges_shown) {
        let line = Line3 { start: verts[a], end: verts[b] };
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

`edges_shown: usize` (normally `12`, the full frame) exists so Slice 4's
boot sequence can progressively reveal the canopy edge-by-edge, the
same "reveal N of M" technique the console panels' boot phase already
uses — not a new pattern. `theme.secondary` (the sickly-green tone) for
the frame color, distinct from the starfield's white/gray and the
console's amber, so the three windshield layers (stars, frame, later
the HUD) stay visually separable.

No fill — a real canopy is a structural frame, not a solid panel; only
`Canvas::line` is used, `Canvas::fill_polygon` has no role in this
slice.

### Slice 3: Console strip re-layout

`render_dashboard` currently fills the entire `area` with the 3-panel
layout (`src/examples/falcon/falcon.rs:137-187` as of this writing).
Split `area` into a windshield region and a console region first, and
scope the existing panel logic to the console region only — `panel_slots`/
`panel_box` themselves need **no changes** (their existing clamping,
fixed during the original hub's final review, already degrades
correctly to whatever `slot` they're given):

```rust
fn render_dashboard(&self, area: Rect, buf: &mut LayerStack) {
    let regions = Layout::new(
        Direction::Vertical,
        vec![Constraint::Percentage(78), Constraint::Fill(1)],
    )
    .split(area);
    let windshield = regions[0];
    let console = regions[1];

    self.render_windshield(windshield, buf, 12); // full canopy, no boot reveal in progress

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
    let mut panel_inners = [Rect { x: 0, y: 0, width: 0, height: 0 }; 3];
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
            gb.render(panel_inners[i], self.theme.tertiary, self.tick_count, overlay);
        }
    }
    self.particles.render(overlay);
}

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

`Constraint::Percentage(78)`/`Constraint::Fill(1)` (rather than two
percentages) guarantees the two regions sum to exactly `area.height`
with no rounding gap, matching `Layout::split`'s existing `Fill`
semantics. `render_windshield` takes `canopy_edges_shown` so Slice 4's
boot phase can call it directly with a partial reveal count; the
normal (post-boot) call from `render_dashboard` always passes the full
`12`.

`update`'s WHACK handler (`falcon.rs:206-225`) reads `self.last_area`
via `Self::panel_slots(self.last_area.get())` — this must change to
`Self::panel_slots(<the console sub-region of self.last_area.get()>)`,
recomputing the same vertical split `render_dashboard` uses, so the
WHACK spark still spawns at the focused panel's *current* (now
smaller, repositioned) on-screen center rather than a stale full-screen
position.

### Slice 4: Boot sequence rework

Current phases (`boot.rs:5-92`): `[0.0,0.1)` pilot light, `[0.1,0.7)`
console panel reveal, `[0.7,1.0]` whole-frame dim-to-bright. New
phases, with a windshield-power-on phase inserted:

- **`[0.0, 0.1)`** — pilot light, unchanged (`boot.rs:5-20`).
- **`[0.1, 0.4)`** — windshield powers on. Remap to `wave = (progress
  - 0.1) / 0.3`. Starfield renders at full (all stars, no per-star
  reveal — already visually diffuse, doesn't need one). Canopy renders
  via `render_canopy(windshield_area, buf, (wave * 12.0).ceil() as
  usize)` — the same "reveal N of M, rounding up so the first edge
  appears almost immediately" technique the console phase's `panels_shown`
  already uses (`(wave * 3.0).ceil() as usize` there), just with 12
  edges instead of 3 panels.
- **`[0.4, 0.85)`** — console panel reveal, same rivet-by-rivet +
  static-burst mechanic as today (`boot.rs:22-57`), remapped to this
  narrower window (`wave = (progress - 0.4) / 0.45`) and operating on
  the console sub-region (`Layout::split`'s second region) instead of
  the full `area`.
- **`[0.85, 1.0]`** — whole-frame dim-to-bright fade, same technique as
  today (`boot.rs:59-91`, scratch-`LayerStack`-then-composite pattern
  — still required for the same reason: `render_dashboard` pushes its
  own glitch/particle layer, so dimming must happen on a flattened
  copy, not `buf` directly), remapped to `fade = (progress - 0.85) /
  0.15`.

### Slice ordering note for the implementation plan

Slices 1-2 add new rendering methods without wiring them into `view()`
yet (matching the original hub's plan's pattern of building pieces
before assembly) — Slice 3 is where `render_dashboard` actually calls
`render_windshield`, and is also where the layout split itself lands.
Slice 4 is the only slice touching `boot.rs`.

## Non-goals

- **The three distinct 3D HUD states** (Hyperdrive trajectory line,
  Sensors radar sweep, Weapons reticle) — explicitly deferred to a
  follow-up Arc, per this spec's header. This Arc's windshield has no
  HUD overlay at all.
- **Any `src/` change.** Everything here consumes `src/perspective.rs`
  and `Canvas` as committed; no new primitive, no bug fix.
- **Near-plane/screen-edge clipping decisions beyond what
  `src/perspective.rs` already provides.** The canopy's `min_scale` is
  `0.0` (never LOD-skipped) since its fixed near/far `z` values never
  approach the near plane or shrink to illegibility — no tuning needed
  this Arc.
- **Sub-app content** (Hyperdrive/Sensors/Weapons' actual
  functionality) — still deferred from the original hub spec, still
  not this Arc's concern.

## Testing

Per `.claude/rules/development-conventions.md`: example code,
TDD-exempt, correctness checked by running. **`tools/visual-snapshot`
is mandatory for every task and the final review** (this Arc directly
changes `examples/falcon/falcon.rs`'s `view()`/`on_tick()` and
`boot.rs`'s render path — the "Visual review" rule's example-render-loop
trigger applies unambiguously, unlike the graduation Arc this builds
on). Each task/review should capture at minimum: the post-boot
dashboard (windshield + console strip together), and — for Slice 4 —
a multi-frame GIF spanning enough real wall-clock time to show all
four boot phases in sequence (pilot light → windshield power-on →
console reveal → dim-to-bright).

## Critical files

- `examples/falcon/falcon.rs` — `falcon_camera`, `Star`/`scatter`,
  `render_starfield`, canopy geometry + `render_canopy`,
  `render_windshield`, `render_dashboard`'s layout split, `update`'s
  WHACK handler's console-region recomputation.
- `examples/falcon/boot.rs` — four-phase rework.

## Verification

- `cargo build --example falcon` / `cargo clippy --all-targets -- -D
  warnings` / `cargo fmt --check` — clean.
- `tools/visual-snapshot` capture of the post-boot dashboard: confirms
  the windshield (starfield + canopy frame) occupies the top ~78% of
  the screen with real parallax/perspective, the console strip
  occupies the bottom with the same three panels as before (just
  smaller), and the focused panel still visibly enlarges/brightens.
- `tools/visual-snapshot` capture spanning the boot sequence: confirms
  all four phases play in order and the canopy frame visibly builds up
  edge-by-edge during the windshield-power-on phase, not all-at-once.
- Manual key-logic check (via the capture script's `key` steps):
  Tab/Shift+Tab still cycle focus correctly on the now-smaller console
  panels; Space still triggers WHACK at the focused panel's correct
  (repositioned) on-screen location, confirmed by the spark burst
  appearing in the console strip, not floating in the windshield area
  above it.
