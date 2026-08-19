# Changelog

All notable changes to `ttui` are documented in this file. Format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
this project follows the SemVer policy defined in
`.claude/rules/code-forge.md`.

## [Unreleased]

### Added

- `transition::Phases<N>` — subdivides a `Transition`'s `0..1` progress
  into `N` phases and answers, in one call, which phase a progress value
  falls in and how far through that phase it is (`at`), replacing the
  boundary arithmetic each themed app hand-rolls with every boundary
  written twice. Build it from cumulative phase ends (`Phases::new`) or
  from one duration per phase (`Phases::from_durations`, normalised by
  their total); both are `const fn`, so an app can declare its phases as
  a `const`. A boundary belongs to the later phase, mirroring the
  `if progress < 0.1` branching it replaces. `semver:minor` — a new
  `pub` item, no existing signature changed. The `falcon`, `omnitrix`,
  `tardis` and `smash_crabs` boot sequences now derive their phases from
  it; that migration is a pure refactor and changes no rendered output.
- `easing::scale_color` — multiplies an `Rgb` color's channels by a
  factor, `1.0` unchanged and `0.0` black, replacing the three private
  copies this repo had grown (`camera`, `widgets::roundel`, `launcher`),
  one of which — `camera`'s — read its factor inverted, so the same call
  meant opposite things depending on which copy you landed on. `factor`
  is deliberately not clamped: above `1.0` the color brightens and each
  channel saturates at `255` rather than wrapping, because clamping
  would silently turn a deliberate highlight into a no-op. A non-`Rgb`
  color passes through untouched at every factor — its brightness was
  never disclosed by the terminal, so there is nothing honest to scale.
  That differs from `lerp_color`'s midpoint switch (#122) on purpose: a
  lerp has two endpoints and can pick the nearer one, a scale has none.
  `semver:minor` — a new `pub` item, no existing signature changed.
- `noise::scatter` — deterministically maps a `u32` seed to an offset in
  `-spread/2 .. spread/2`, the jitter four bundled apps (`depth_spike`,
  `falcon`, `mission_control`, `showcase`'s telemetry) each carried a
  byte-identical private copy of, to place stars and scatter glyphs. A
  move, not a rewrite: the hash and its constants are reproduced
  verbatim, so every existing layout is bit-for-bit unchanged.
  `semver:minor` — a new `pub` item, no existing signature changed.
- `Buffer::blit` — draws one `Buffer` into another with its top-left at
  `(x, y)`, the Buffer-to-Buffer counterpart to `Canvas::blit` that the
  engine simply did not have, and which `omnitrix`, `tardis` and
  `smash_crabs` each hand-rolled identically. Argument order mirrors
  `Canvas::blit(&self, buf, x, y)` so the two read the same way round.
  It clips explicitly instead of relying on `Buffer::set`'s bounds
  behaviour, which checks only the flat index and so silently wraps an
  out-of-range `x` onto a later row (#161) — `set` itself is unchanged.
  `semver:minor` — a new `pub` item, no existing signature changed.

### Fixed

- `app::run` now checks `should_quit()` after `on_tick`, so a
  timer-driven app (a splash screen, an idle timeout, a finished
  animation) can exit without waiting for a keypress it may never
  receive (#30). No public API change; apps that never quit from
  `on_tick` are unaffected.
- `easing::lerp_color` now respects `t` for color pairs it cannot
  interpolate. It previously returned `to` for every `t` whenever either
  color was not `Rgb`, so a gradient rendered flat at its end color and
  a fade snapped to its target on the first frame; such a pair now
  switches at the midpoint, making `t = 0` the source and `t = 1` the
  target (#122). No public API change, and no shade the terminal did not
  choose is ever emitted — `Color::Reset` and named colors still cannot
  be interpolated componentwise. `Rgb` pairs are unaffected, so no
  bundled example's output changes.

## [1.0.0] - 2026-08-14

### Added

- Release governance: SemVer policy, label taxonomy, filing-work rule
  (`.claude/rules/code-forge.md`).
- `GlitchBuffer::with_alpha` — partial-transparency glitch overlays (#115).
- `perspective::ProjectLineParams` (#119).
- `showcase` — the flagship demo reel: a mascot-hosted tile menu of 5
  vignettes, run via `cargo run --bin showcase`.
- `tools/visual-snapshot`'s `--bin` flag — captures a `[[bin]]` target
  (e.g. `showcase`), alongside the existing `--example` capture path.
- `showcase`'s mascot: idle breathing animation and a redesigned
  two-tone eye with a genuine blink.
- `showcase`'s Assembly Line vignette reworked: a real crate sprite
  (replacing a plain glyph row), and the mascot now slides to and
  reaches down for a caught crate instead of pose-flashing in place.
- `BorderSet::single_line()` / `BorderSet::ascii()` presets (#130).

### Changed

- **Breaking:** `Theme.border_bold: bool` → `Theme.border_style: CellStyle` (#108).
- **Breaking:** `Camera::project_line` now takes `ProjectLineParams` instead of 7 positional `f32`s (#119).
- **Breaking:** `BorderSet.corner: char` → `top_left`/`top_right`/`bottom_left`/`bottom_right`; `BorderSet::default()` now returns `single_line()` (real box-drawing glyphs) instead of the old ASCII look (#130).
