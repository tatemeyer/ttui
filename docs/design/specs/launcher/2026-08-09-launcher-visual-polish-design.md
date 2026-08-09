# Launcher Visual Polish — Design

**Status:** draft, pending your review before we move to planning.
**Date:** 2026-08-09
**Relationship to prior specs:** closes two gaps between the original
`2026-08-08-cross-app-launcher-design.md` and what actually shipped,
and applies two capabilities that didn't exist when that spec was
written — the gradient-border option (`Theme.primary_end`) from
`2026-08-08-rendering-primitives-graduation-design.md` (PR #93) and
the demonstrated-in-practice pattern of driving a widget's animation
state from a persistent, tick-updated struct field (established across
the rendering-fidelity spike and Arc A/B). No `src/` (core framework)
changes — this is entirely `examples/launcher/` work, same as the
original launcher spec.

## Context / Motivation

The original launcher spec describes two visual details its own Design
section commits to that the shipped code does not actually implement:

- **"background layer — a slow starfield/void (reusing `particles` for
  drift)"** (Design §4). `examples/launcher/nexus.rs::starfield()` as
  shipped is a static hash-grid: every star's `(x, y)` is fixed per
  frame, only its twinkle brightness animates. No `ParticleSystem`, no
  drift, no movement.
- **"the focused portal enlarges/pulses via `easing` hover"** (Design
  §4). `examples/launcher/nexus.rs::portals()` computes one `box_w`/
  `box_h` and applies it identically to all three slots regardless of
  focus — only the pulse (border glow, glyph brightness) is
  implemented. Nothing enlarges.

A third item, the "dive into portal" flourish on launch, was
explicitly marked optional in the original spec ("allowed but not
required") and was never built — this spec builds it now.

A fourth item is a genuine new opportunity, not a gap:
`examples/launcher/portal.rs`'s `Theme` construction hardcodes
`primary_end: None` (line 34) because that field didn't exist yet when
this file was written. It does now.

This spec closes all four.

## Scope

**Tag: `coding`.** TDD applies per this project's existing carve-out
for examples: pixel/visual output is a demo exception (verified by
running), but `Launcher`'s *state machine* — already unit-tested today
for location-toggling — is not, and this spec's state additions follow
that same precedent.

### 1. Starfield: static hash-grid → real `ParticleSystem` drift

`Launcher` (`examples/launcher/main.rs`) gains a persistent field:

```rust
starfield: ParticleSystem,
```

**The constraint this design works around:** `App::on_tick(&mut self,
elapsed: Duration)` does not receive the terminal's current size —
only `view(&self, area: Rect, ...)` does, and `view` takes `&self`, so
it cannot mutate `starfield` to (re)seed it against the real area. This
spec does not change the `App` trait to fix this (out of scope — a
core, not example, change) and instead accepts an approximation: stars
are spawned against a fixed, generous virtual space (`STARFIELD_W:
u16 = 250`, `STARFIELD_H: u16 = 80` — comfortably larger than any
realistic terminal), not the real area. `ParticleSystem::render`
already silently skips any particle outside the buffer it's handed, so
this degrades gracefully — on a small terminal, most seeded stars
simply never draw, at the cost of iterating a few dozen inert
particles per frame (cheap; this is not thousands of particles).

Each tick, in `on_tick`, while in the nexus: if `starfield.len() <
TARGET_STAR_COUNT` (a constant, e.g. `60`), spawn enough new `Particle`s
to reach it. Each spawned star gets:
- a pseudo-random position within the virtual space (same
  index-derived hash technique the current `starfield()` already
  uses for its twinkle phase, applied to position instead);
- a slow pseudo-random drift velocity (small `vx`/`vy`, order
  0.3-1.0 cells/second, direction derived from the same hash);
- a `lifetime` of ~30 seconds, so the field continuously turns over
  rather than drifting into a static-looking steady state, and
  naturally self-heals via the existing "top up to target count" rule
  — no explicit wraparound-at-edges logic is needed, since
  `ParticleSystem` exposes no way to mutate an in-flight particle's
  position (only `spawn`/`update`/`render`/`len`/`is_empty` are
  public) and a plain "let it expire and respawn elsewhere" turnover is
  simpler and sufficient for a background flourish;
- a color/symbol chosen the same way the current twinkle-brightness
  hash does today (varying levels of dim blue-white), fixed for that
  star's lifetime (no continuous per-frame twinkle animation —
  traded away for real drift, per this spec's scope; a star's
  brightness no longer breathes, but the field now visibly moves,
  which is what both the original spec and this one actually ask for).

`nexus::render` drops its `starfield(scene, phase)` function entirely
and instead takes a `&ParticleSystem` parameter, calling
`starfield.render(&mut scene)` after `fill_void`. `nexus::render`'s
signature becomes `render(selected, starfield: &ParticleSystem, fade,
area, buf)` (the unused `phase` parameter it previously used only for
starfield twinkle is removed from `render` itself; `phase` is still
threaded to `portals()` for the pulse animation, unchanged).

### 2. Real enlarge-on-focus

In `nexus.rs::portals()`, compute two box sizes instead of one:

```rust
let base_w = box_w; // existing computation, unchanged
let base_h = box_h;
let focus_w = (base_w + 2).min(slot_w.saturating_sub(1));
let focus_h = (base_h + 1).min(h.saturating_sub(2));
```

for each portal `i`, use `focus_w`/`focus_h` when `i == selected`,
`base_w`/`base_h` otherwise, re-centering `slot_x`/`top` per-portal
using whichever size is in effect for that iteration (the existing
centering formulas, just recomputed per-portal instead of once).

**Explicitly not built:** an eased size transition between focused and
unfocused. Cell-grid resizing does not sub-pixel-interpolate the way a
pixel-based UI would, and the existing brightness pulse already
carries the "this one is alive" animated feeling — an instant size
snap on focus-change reads cleanly in a terminal; adding tween state
for it would be speculative polish with no clear payoff.

### 3. Dive-in flourish on launch

`Launcher` gains:

```rust
diving: Option<(usize, Transition, ParticleSystem)>,
```

`Action::Launch(i)`'s handling in `apply()` changes from immediately
setting `self.active` to instead setting `self.diving = Some((i,
Transition::start(DIVE_DURATION), burst))`, where `burst` is a fresh
`ParticleSystem` with ~16 particles spawned outward from a fixed point
(see below), `DIVE_DURATION` a constant (`Duration::from_millis(400)`).
The actual app swap — `self.active = Some(make_app(i)); self.location
= location_of(i);` — moves out of `apply()` into `on_tick`, firing once
`diving`'s `Transition::is_complete()` (after `on_tick` advances both
the transition and the burst particle system's `update`).

**Burst origin, and why it's approximate:** the same `on_tick`-has-no-
area constraint from §1 applies here — `apply()` (called from
`update()`) has no access to the real terminal size either, so the
burst cannot originate from portal `i`'s exact on-screen position.
Instead it originates from a fixed offset from a nominal center point:
`x_offset = (i as f32 - 1.0) * 20.0` cells from center, `y` at nominal
center — approximating the three portals' left/center/right layout
without depending on the real area. This is stated as a known
approximation, not silently passed off as pixel-exact.

`view()`'s nexus-render branch, while `diving` is `Some`, renders the
nexus (dimmed via `camera::dim` as `1.0 - transition.progress()`
advances) with the burst's `ParticleSystem` composited on top — the
same dim-then-reveal shape the existing return-fade already uses,
reused here for the opposite direction (revealing the destination
rather than the nexus).

### 4. Gradient portal borders

In `portal.rs`, change:

```rust
primary_end: None,
```

to:

```rust
primary_end: if focused {
    Some(dim_color(accent, 0.3 + 0.7 * pulse))
} else {
    None
},
```

The focused portal's border now genuinely lerps from `border` (already
an accent-derived color) toward a pulse-modulated brighter/dimmer
variant of `accent` across the ring, via `Block::render`'s existing
gradient support — the same `pulse` value already driving
`border_bold`/glyph brightness, so this is one more expression of an
animation state that already exists, not new state. Unfocused portals
keep `primary_end: None` (flat, cheap, matching their already-dimmed,
deliberately-quieter presentation).

## Non-goals

- No `App` trait changes (e.g. threading `area` into `on_tick`) —
  the approximations in §1/§3 are the accepted cost of staying within
  the existing trait shape.
- No change to `route()`'s key→`Action` mapping — `Enter` still maps
  to `Action::Launch(i)`; only what `apply()`/`on_tick` do with that
  action changes.
- No smooth eased resize for enlarge-on-focus (§2).
- No changes to any of the three sub-apps (Omnitrix/TARDIS/Smash
  Crabs) or their own boot/transition sequences — those play exactly
  as before once the dive completes and `self.active` is set.
- No alpha/blend-based effects (`src/blend.rs`) — that module is still
  spike-only, not committed API; this spec uses only `camera::dim`,
  `ParticleSystem`, `Transition`, and `Theme.primary_end`, all
  already-committed primitives.

## Testing

Per `.claude/rules/development-conventions.md`'s examples carve-out:
pixel/visual output (the starfield's look, the enlarge amount, the
burst's visual shape, the gradient's exact color) is verified by
running (`cargo run --example launcher`), not asserted on. `Launcher`'s
*state machine* is not exempt — it's plain Rust state already
unit-tested today (`apply_launch_and_return_toggle_location`), and this
spec's additions follow that precedent:

- A new test asserts that immediately after `apply(Action::Launch(i))`,
  `diving.is_some()` and `active.is_none()` (the dive has started, the
  app hasn't swapped in yet).
- A new test asserts that after `on_tick`ing forward by more than
  `DIVE_DURATION`, `active.is_some()`, `location` matches `i`, and
  `diving.is_none()` (the dive completed and the swap fired).
- `route()`'s existing 8 tests are unchanged — `route()` itself is not
  touched by this spec.
- `nexus_render_does_not_panic_across_sizes` (existing) gets its call
  sites updated for `nexus::render`'s new signature (a `&ParticleSystem`
  parameter) and continues to assert no panic across the same size
  matrix, now also exercising the enlarge-on-focus sizing math at each
  size (including the smallest, `12x10`, where `focus_w`/`focus_h`'s
  clamps are most likely to matter).

## Critical files

- `examples/launcher/main.rs` — `Launcher.starfield`/`diving` fields,
  `apply()`/`on_tick`/`view()` changes, new tests.
- `examples/launcher/nexus.rs` — `starfield()` removed,
  `render()`/`portals()` signature and sizing changes.
- `examples/launcher/portal.rs` — `primary_end` gradient on focus.

## Verification

- `cargo test` — full suite green, including the new `Launcher` state
  tests.
- `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` —
  clean.
- `cargo build --examples` — the three sub-apps' own standalone
  examples are untouched by this spec and continue to build/run
  exactly as before.
- `cargo run --example launcher` — manual visual check: stars visibly
  drift rather than sitting static; the focused portal is visibly
  larger than the other two and its border shows a genuine color
  gradient (not flat); pressing `Enter` shows a brief particle burst
  before the chosen app's own boot sequence begins; `F12`/app-`q`/
  nexus-`q` routing still behaves exactly as before (unchanged by this
  spec).
