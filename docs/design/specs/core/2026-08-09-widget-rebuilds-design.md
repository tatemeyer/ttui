# Widget Rebuilds on Rendering Primitives — Design

**Status:** draft, pending your review before we move to planning.
**Date:** 2026-08-09
**Relationship to prior specs:** consumes the rendering primitives
graduated in `2026-08-08-rendering-primitives-graduation-design.md`
(PR #93, merged) — specifically `Canvas` (`src/canvas.rs`) and
`easing::lerp_color` (Arc 0, unchanged) — to rebuild four existing
themed widgets that currently hand-roll effects those primitives now
provide for real. Builds on the unchanged Rev A
(`2026-08-04-ttui-core-framework-design.md`) widget model
(stateless `(data, area) -> paint calls`) and the unchanged `Theme`/
`Block` (Rev B, Arc A).

**Dependency:** this Arc assumes PR #93 has already merged to `main`
(it has) — it builds directly on `Canvas`'s committed API.

## Context / Motivation

Four themed widgets, one per example app plus `TimeRotor` (TARDIS),
predate the rendering-fidelity spike and its graduated primitives, and
each independently reinvents something `Canvas`/`lerp_color` now do
properly:

- **`TimeRotor`** (`src/widgets/time_rotor.rs`) renders one braille
  glyph per row, but the glyph's dot pattern comes from a hash of
  `(row, tick_count * speed)` — visual noise, not an actual rotating
  shape. It's supposed to read as a spinning rotor and doesn't.
- **`DamageMeter`** (`src/widgets/damage_meter.rs`) picks one of three
  hardcoded colors (white/yellow/red) via two threshold checks — a
  visible hard jump at 50% and 100% rather than a smooth escalation.
- **`EnergyCore`** (`src/widgets/energy_core.rs`) fills with a single
  flat color across the whole bar — no visual sense of "charging up."
- **`Roundel`** (`src/widgets/roundel.rs`) is a single glyph (`'O'`) at
  a scaled brightness — described in the original backlog as a
  "pulsing circular data node," but today it isn't circular at all,
  it's one character.

This spec rebuilds all four to actually use what's now available,
without changing three of their four public signatures.

## Scope

**Tag: `coding`.** Full TDD applies, no exceptions — same posture as
the Arc this one consumes, not the earlier spike.

### 1. `TimeRotor` — real rotation via `Canvas` (Braille)

Signature unchanged: `TimeRotor::new(speed).render(area, tick_count, buf)`.

Internally: treat the whole `area` as one `Canvas::new(area.width,
area.height, CanvasMode::Braille)`. Compute an angle from `tick_count`
and `speed` (`angle = tick_count as f32 * speed * ROTATION_RATE`, a
small constant tuned so the sweep is visible but not frantic at
`speed`'s default range). Draw one line from the canvas's center
outward at that angle via `Canvas::line`, plus its 180°-mirrored
counterpart (`angle + PI`), both using the same glyph color as before.
`blit` into `buf`. This produces a visible spinning blade through the
center — the geometric shape `TimeRotor`'s name already promises —
where before it was noise with no relationship to `speed` beyond
changing which hash bucket got hit.

**Non-goal:** multiple simultaneous blades, blade acceleration/easing,
or a 3D-ish perspective effect — one straight rotating line pair is
the full scope here.

### 2. `DamageMeter` — continuous color gradient

Signature unchanged: `DamageMeter::new(percent).render(area, buf)`.

Replace the three-branch `if/else if/else` with two `lerp_color` calls
gated on range: for `percent` in `0..50`, color = `lerp_color(white,
yellow, percent/50)`; for `50..100`, color = `lerp_color(yellow, red,
(percent-50)/50)`; for `percent >= 100`, color = solid `red` (matching
today's existing "stays red past 100%" behavior exactly, just smoothing
what leads into it). The `format!("{}%", percent)` text rendering and
`>100` uncapped-display behavior are otherwise unchanged.

### 3. `EnergyCore` — gradient fill instead of flat color

Signature unchanged: `EnergyCore::new(percent, color).render(area, buf)`.

For each filled column `x` (i.e. `x < filled_width`), fill color
becomes `lerp_color(color, Color::White, x as f32 / filled_width.max(1)
as f32)` instead of the current flat `self.color` — the fill visibly
brightens toward its leading edge, an automatic "charging up" look with
no new constructor parameter. The empty-track (`'░'`) rendering and the
100%-spark logic are unchanged. `Canvas` is deliberately not used here:
`HalfBlock` mode only doubles *vertical* resolution, which doesn't
improve a horizontal bar's fill precision, so there's nothing for it to
buy this widget.

### 4. `Roundel` — a real filled circle via `Canvas` (Braille), with a size parameter

**Signature change:** `Roundel::new(intensity, color)` becomes
`Roundel::new(intensity, color, radius: u16)` (a plain third
constructor argument, not a builder — matches this widget's existing
minimal, non-builder style, unlike `Block`/`Text`). `radius: 0`
preserves exactly today's single-glyph-at-center behavior (still `'O'`,
still just `buf.set` on one cell — this is the tested, working
fallback for tight layouts, not a degenerate case to special-case
away). `radius >= 1` switches to the `Canvas`-drawn-circle path:
`Canvas::new(area.width, area.height, CanvasMode::Braille)`, scan every
subpixel in the grid, `set_pixel` it if its distance from the canvas's
center is `<= radius` (in subpixel units, so the visual radius scales
sensibly with `Canvas`'s 2×4-subpixel-per-cell density), all at
`scale_color(color, intensity)` (the existing brightness-scaling
helper, unchanged) — then `blit`.

**Call-site changes (the only widget in this Arc with any):**
- `examples/tardis/artron_energy.rs` — the three energy-segment
  roundels (spaced `area.x + 4 + i * 4`, i.e. 4 cells apart) switch to
  `radius: 1` (a ~3-cell-wide circle) and widen their passed `Rect` to
  `width: 3, height: 3` (centered on the same `rx`/`ry` points as
  today) — 4-cell spacing comfortably fits a 3-cell circle.
- `examples/tardis/hub.rs` — the three ambient background-pulse
  roundels (spaced `area.width / 4` apart) switch to `radius: 1`
  identically. On a narrow terminal this spacing could pinch, but
  `hub.rs`'s roundels are decorative background elements already
  positioned by a coarse `/4` split, not precision-aligned to other
  content — a rare narrow-terminal case not designed around here.
- `examples/tardis/star_charts.rs` — the one-per-timeline-row roundel
  (rows exactly 1 cell apart) **stays at `radius: 0`** — the compact,
  unchanged single-glyph path — since a 3-cell circle would overlap the
  adjacent timeline row's text. This is the concrete reason `radius: 0`
  is a first-class supported mode, not a placeholder.

## Non-goals

- `Text`, `List`, `Table`, `Block`, `SmashBorder`, `AnalogToggle`,
  `DnaConsole`, `ScuttleCursor` — untouched, not part of this Arc's
  backlog item.
- No new core (`src/`, non-widget) API — every change in this spec is
  internal to the four widget files plus the three TARDIS call sites
  `Roundel`'s signature change forces. `Canvas`, `lerp_color`, and
  `Intensity` are consumed exactly as Arc A shipped them.
- No change to any of the three apps' visual *behavior* beyond these
  four widgets — `Roundel`'s call-site `Rect` widening is a mechanical
  consequence of its new size parameter, not a broader TARDIS redesign.

## Testing

Per `.claude/rules/development-conventions.md`: `coding`-tagged, TDD
mandatory, no exceptions. Each widget's existing test suite is the
regression baseline — new tests are added for the new behavior, and
every existing test's *intent* (not necessarily its exact assertion,
where the visual output genuinely changes) is preserved:

- **`TimeRotor`** — `renders_one_braille_glyph_per_row_at_the_center_column`
  no longer applies literally (rendering is no longer per-row/
  center-column-only once it's a whole-area `Canvas`) and is replaced
  by an equivalent-intent test: rendering produces at least one
  non-default braille cell somewhere in the area. `identical_inputs_
  render_identically` and `different_speeds_render_differently_for_
  the_same_tick_count` keep their exact intent (determinism, speed
  sensitivity) with assertions adjusted to the new rendering shape.
- **`DamageMeter`** — new tests pin the gradient at known points (e.g.
  25% renders partway between white and yellow, 75% renders partway
  between yellow and red); `zero_percent_renders_white`/`over_100_
  percent_renders_red_with_full_text` keep their exact assertions
  (both are still true under the new gradient, at its endpoints).
- **`EnergyCore`** — new test confirms the fill's rightmost filled
  column is visibly brighter than its leftmost; `zero_percent_renders_
  all_empty_track`/`full_percent_sparks_every_fourth_cell` keep their
  exact assertions where they don't touch fill color directly.
- **`Roundel`** — `radius: 0` path gets the exact existing four tests,
  renamed/confirmed unchanged (this is the regression guarantee for
  `star_charts.rs`). New tests for `radius >= 1`: renders multiple
  non-default cells (not just one), intensity still scales the drawn
  color, out-of-bounds/tiny `area` degrades without panicking (same
  discipline as every other widget's zero-size-area tests).

## Critical files

- `src/widgets/time_rotor.rs` — full render rewrite.
- `src/widgets/damage_meter.rs` — color-selection rewrite.
- `src/widgets/energy_core.rs` — fill-color rewrite.
- `src/widgets/roundel.rs` — signature change + full render rewrite.
- `examples/tardis/artron_energy.rs`, `examples/tardis/hub.rs`,
  `examples/tardis/star_charts.rs` — `Roundel::new`/`Rect` call-site
  updates (the latter unchanged in behavior, updated only because the
  signature changed).

## Verification

- `cargo test` — full suite green, including every new/adjusted test
  above.
- `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` —
  clean (hard gates, `coding`-tagged).
- `cargo build --all-targets` — all examples compile against
  `Roundel`'s new signature.
- `cargo run --example tardis` — manual visual check: the three
  `artron_energy`/`hub` roundels render as visible circles with no
  overlap at a normal terminal size; the `star_charts` roundel is
  unchanged (still a single pulsing glyph); `TimeRotor` (called from
  `examples/tardis/hub.rs:60` and `examples/tardis/artron_energy.rs:42`,
  signature unchanged so no call-site edits needed) visibly rotates
  rather than flickering randomly.
- `cargo run --example smash_crabs` — manual visual check: `DamageMeter`
  shows a smooth color ramp rather than two hard jumps.
- `cargo run --example omnitrix` — manual visual check: `EnergyCore`
  shows a visible brightness gradient across its fill rather than a
  flat bar.
