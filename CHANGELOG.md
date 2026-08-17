# Changelog

All notable changes to `ttui` are documented in this file. Format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
this project follows the SemVer policy defined in
`.claude/rules/code-forge.md`.

## [Unreleased]

### Fixed

- `app::run` now checks `should_quit()` after `on_tick`, so a
  timer-driven app (a splash screen, an idle timeout, a finished
  animation) can exit without waiting for a keypress it may never
  receive (#30). No public API change; apps that never quit from
  `on_tick` are unaffected.

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
