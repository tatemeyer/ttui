# Changelog

All notable changes to `ttui` are documented in this file. Format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
this project follows the SemVer policy defined in
`.claude/rules/code-forge.md`.

## [Unreleased]

### Added

- Release governance: SemVer policy, label taxonomy, filing-work rule
  (`.claude/rules/code-forge.md`).
- `GlitchBuffer::with_alpha` — partial-transparency glitch overlays (#115).
- `perspective::ProjectLineParams` (#119).

### Changed

- **Breaking:** `Theme.border_bold: bool` → `Theme.border_style: CellStyle` (#108).
- **Breaking:** `Camera::project_line` now takes `ProjectLineParams` instead of 7 positional `f32`s (#119).
