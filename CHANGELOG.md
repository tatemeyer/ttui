# Changelog

All notable changes to `ttui` are documented in this file. Format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
this project follows the SemVer policy defined in
`.claude/rules/code-forge.md`.

## [Unreleased]

## [2.0.0] - 2026-08-19

### Migration

| 1.x | 2.0 |
|---|---|
| `Table::new(headers, rows, selected, col_width)` | `Table::new(headers, rows, selected).widths(&vec![Constraint::Fixed(col_width); headers.len()])` reproduces 1.x geometry exactly |
| `blend::blend_over` | `LayerStack::composite` |
| `blend::fade_toward` | `easing::scale_color` |
| exhaustive `match` on `Constraint` / `Direction` / `CanvasMode` / `Intensity` | add a `_ => …` arm — all four are now `#[non_exhaustive]` |
| `List`/`Dial`/`Table` selection colors always black-on-white | unchanged unless you opt in via `.theme(&theme)` |
| `Buffer::get`/`set` documented a panic on out-of-range `x` but silently wrote/read a later row instead (#161) | debug builds now panic as documented; release builds still do not check, and the docs say so explicitly |

### Breaking

- `Intensity`, `CanvasMode`, `Direction` and `Constraint` are now
  `#[non_exhaustive]`. Downstream `match`es on them need a wildcard
  (`_ => …`) arm. This buys the ability to add variants — for example a
  content-sizing `Constraint::Auto` — in a *minor* release rather than
  another major.
- Removed the `blend` module. Its own documentation described it as
  "spike-only, and now historical": the rendering-fidelity spike's
  recommendation was adopted, and `LayerStack::composite` has done real
  Porter-Duff "over" compositing on `Cell::alpha` ever since. Callers
  should use `LayerStack::composite` (for `blend_over`) and
  `easing::scale_color` (for `fade_toward`).
- `Buffer::get`/`set` now bounds-check `x` in debug builds — previously
  only the flat index was checked, so on a 4x3 buffer `set(5, 0, ..)`
  silently wrote to `(1, 1)` instead of panicking as documented (#161).
  An A/B/A rerun of `benches/set.rs` in a single machine state
  (`debug_assert!` -> `assert!` -> `debug_assert!`) measured a real
  `assert!` at +10.1%/+17.0% (full_paint/single_cell) over
  `debug_assert!`, clearing the ~5% drift observed between the two
  `debug_assert!` runs, so the check is `debug_assert!`-gated rather
  than unconditional; release builds are unchanged and the docs now
  describe that release behavior explicitly instead of promising a
  panic that never happened.
- `List`, `Dial` and `Table` accept a `Theme` via `.theme(&theme)` for
  their selection highlight; all three previously hardcoded black-on-
  white and took no colours at all. Omitting `.theme()` renders exactly
  as 1.x did.
- **`Table::new` no longer takes `col_width`.** Columns are sized with
  `Constraint`s — the same vocabulary `Layout` uses — via
  `.widths(&[...])`, so a table can mix narrow fixed columns with one
  that takes the remaining width (#170). To reproduce 1.x geometry
  exactly, pass `.widths(&vec![Constraint::Fixed(col_width);
  headers.len()])` — see the Migration table above. **Omitting
  `.widths()` entirely is new behaviour, not equivalent to
  `col_width`:** it now defaults to `Fill(1)` per column, splitting the
  *whole* area evenly, whereas `col_width` gave every column exactly
  `col_width` cells and left any remaining area unpainted. `.spacing(n)`
  inserts a gap between columns, and cells that overflow their column
  are cut with an ellipsis instead of silently ending. Cells are
  measured by display width, so CJK no longer misaligns the columns
  after it; a combining mark keeps the columns after it aligned too,
  but — because `Cell::symbol` holds one `char` — it still overwrites
  its base glyph rather than combining with it, so full combining-mark
  rendering is not claimed. Every row's background span now derives
  from the column `Rect`s rather than from how many cells the row
  supplies, so a row with fewer cells than there are columns still
  paints its selection highlight across every column, not just the
  ones it has data for.

  The example below shows the new mixed-width capability (#170), not a
  migration — see the Migration table above for the identity port:

  ```rust
  // 1.x
  Table::new(&headers, &rows, selected, 12).render(area, buf);
  // 2.0 (#170's new capability: a narrow fixed column plus one that
  // takes the rest, not a port of the call above)
  Table::new(&headers, &rows, selected)
      .widths(&[Constraint::Fixed(6), Constraint::Fill(1)])
      .render(area, buf);
  ```

### Added

- Standard open-source project files: `CONTRIBUTING.md` (build, test, the
  four required checks, and the commit/test/doc/versioning conventions),
  `CODE_OF_CONDUCT.md` (Contributor Covenant 2.1), and `SECURITY.md`
  (private vulnerability reporting, and an honest statement of what the
  realistic attack surface of a terminal UI library actually is).
- A rustdoc landing page: `src/lib.rs` now carries a quick-start example,
  a pipeline diagram, and a guided tour of the modules, replacing a
  two-line crate summary. Both code samples are doctests, so the
  quick-start cannot silently rot.
- `README.md` gained crates.io/docs.rs/CI/MSRV/license badges, install
  instructions, the same quick-start, and the standard
  documentation/contributing/security/license sections.

### Changed

- **Declared MSRV: Rust 1.87.0**, via `rust-version` in `Cargo.toml`.
  Verified rather than assumed — 1.87 is the first release with
  `unsigned_is_multiple_of` stable, and 1.86 fails to compile the library.
  This documents the minimum that was already in force; it does not raise
  it. Future MSRV increases are treated as minor bumps.
- The published package no longer ships development-only material —
  `docs/`, `.claude/`, `.plumb/`, `.github/`, `CLAUDE.md` and
  `parallax.yaml` are now excluded, taking it from 229 files to 92. No
  code, example, test or bench is affected.

## [1.1.0] - 2026-08-19

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
