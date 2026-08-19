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
