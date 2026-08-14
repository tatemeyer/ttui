# Pre-v1 Fix Wave — Design

**Status:** draft, pending review before we move to planning.
**Date:** 2026-08-13
**Relationship to prior work:** sub-project #3 of the TTUI v1.0.0
initiative. Fixes the exact 3 issues the sweep audit (#2) triaged into
the `v1-blocking` queue: #108, #115, #119 — no other scope. Per
`.claude/rules/code-forge.md`'s filing-work rule, these were routed to
the full brainstorm cycle (not a direct fix) because each is a real
API design decision, not a mechanical change.

## Problem

Three pieces of `ttui`'s public API surface (`Theme`, `GlitchBuffer`,
`Camera::project_line`) have known, already-identified rough edges
that are free to fix now (pre-1.0, per `code-forge.md`'s SemVer
policy) and costly to fix later. Each was deliberately left open by an
earlier Arc pending exactly the condition that now holds: `Theme`
grows a new attribute need, `GlitchBuffer` gets a real consumer that
needs partial transparency, and `Camera::project_line` gets a real
non-spike consumer (Falcon's HUD).

## Scope

**Tag: `coding`, TDD mandatory** — no exemption; all three slices
touch `src/` and change tested behavior.

Three independent slices (different subsystems, no shared code or
ordering dependency — any could ship alone):

1. **`Theme.border_bold: bool` → `Theme.border_style: CellStyle`**
   (closes #108).
2. **`GlitchBuffer::with_alpha`** builder method (closes #115).
3. **`Camera::project_line`'s `ProjectLineParams`** struct (closes
   #119).

## Design

### Slice 1: `Theme.border_style: CellStyle`

`src/theme.rs`'s `Theme` struct field changes from:
```rust
pub border_bold: bool,
```
to:
```rust
/// Style applied to every border cell (not title cells — see
/// `Block::render`). Reuses `Cell`'s own style type rather than a
/// narrower bool, so future border attributes (underline, etc.) need
/// no further `Theme` field growth.
pub border_style: CellStyle,
```
`Theme::default()`'s `border_bold: false` becomes
`border_style: CellStyle::default()`.

`src/widgets/block.rs`'s `render()` currently destructures `t.border_bold`
(a `bool`) and uses it directly to pick `Intensity::Bold` vs.
`Intensity::Normal` for the `plain` cell closure:
```rust
let (border, fg, bg, border_bold, border_thick, primary_end) = match self.theme {
    Some(t) => (t.border, t.primary, t.background, t.border_bold, t.border_thick, t.primary_end),
    None => (BorderSet::default(), Color::Reset, Color::Reset, false, false, None),
};
// ...
let plain = |x: u16, y: u16| Cell {
    symbol: ' ',
    fg: ring_fg(x, y),
    bg,
    style: CellStyle {
        intensity: if border_bold { Intensity::Bold } else { Intensity::Normal },
        ..Default::default()
    },
    alpha: 1.0,
};
```
Changes to destructure `t.border_style` (a `CellStyle`) directly and
use it as `plain`'s base style, dropping the intensity-only
reconstruction:
```rust
let (border, fg, bg, border_style, border_thick, primary_end) = match self.theme {
    Some(t) => (t.border, t.primary, t.background, t.border_style, t.border_thick, t.primary_end),
    None => (BorderSet::default(), Color::Reset, Color::Reset, CellStyle::default(), false, None),
};
// ...
let plain = |x: u16, y: u16| Cell {
    symbol: ' ',
    fg: ring_fg(x, y),
    bg,
    style: border_style,
    alpha: 1.0,
};
```
This is a strict behavioral no-op for every existing caller — a
`CellStyle` with only `intensity` set (via `Intensity::Bold`/`Normal`,
everything else default) renders identically to the old bool-driven
construction, since `plain`'s old style block only ever set
`intensity` and left every other `CellStyle` field at its default.

**Every call site constructing a `Theme { .. }` or (in `block.rs`'s
own tests) directly testing `border_bold` must update** — 11 files,
confirmed via `grep -rln border_bold src/ examples/`:
`src/theme.rs`, `src/widgets/block.rs` (struct field use + 8 test
literals), `src/widgets/cockpit_panel.rs`, `src/widgets/smash_border.rs`,
`examples/control_panel.rs`, `examples/falcon/falcon.rs`,
`examples/launcher/portal.rs`, `examples/mission_control.rs`,
`examples/omnitrix/omnitrix.rs`, `examples/smash_crabs/smash_crabs.rs`,
`examples/tardis/tardis.rs`.

Two shapes of update, both mechanical:
- **Static `false`** (7 of 9 non-block.rs sites): `border_bold: false`
  → `border_style: CellStyle::default()`.
- **Conditional expression** (2 sites — `examples/launcher/portal.rs`:
  `border_bold: focused && pulse > 0.5`; `examples/omnitrix/omnitrix.rs`:
  `border_bold: brightness > 0.6`): becomes
  ```rust
  border_style: CellStyle {
      intensity: if <same condition> { Intensity::Bold } else { Intensity::Normal },
      ..Default::default()
  },
  ```
  (both files already import or can import `Intensity` from
  `crate::buffer` / `ttui::buffer`, matching `block.rs`'s existing
  import).

`block.rs`'s own test module has 8 `border_bold: true`/`false`
literals across its `Theme { .. }` test fixtures — each becomes
`border_style: CellStyle { intensity: Intensity::Bold, ..Default::default() }`
(for `true`) or `border_style: CellStyle::default()` (for `false`,
already-default so no `intensity` override needed). No test's
*assertion* logic changes — every test still checks the same rendered
`Cell`'s `style.intensity`, just via the new field's construction.

### Slice 2: `GlitchBuffer::with_alpha`

`src/glitch.rs`'s `GlitchBuffer` struct gains a field and a builder
method; `render()`'s signature is untouched:
```rust
pub struct GlitchBuffer {
    transition: Option<Transition>,
    alpha: f32,
}

impl GlitchBuffer {
    pub fn new() -> Self {
        GlitchBuffer { transition: None, alpha: 1.0 }
    }

    /// Sets the alpha every rendered glitch cell carries, for a
    /// partially-transparent effect (e.g. "static laid over the
    /// readout, not fully opaque"). Defaults to `1.0` (fully opaque) —
    /// existing callers that never call this see no behavior change.
    pub fn with_alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha;
        self
    }
    // trigger/tick/clear/is_active unchanged
```
`render()`'s cell construction changes its hardcoded `alpha: 1.0` to
`alpha: self.alpha`:
```rust
Cell {
    symbol: glyph,
    fg: color,
    bg: Color::Reset,
    alpha: self.alpha,
    ..Default::default()
},
```
Purely additive: `GlitchBuffer::new()` is unchanged, every existing
caller (Falcon, TARDIS) that never calls `.with_alpha(...)` renders
identically to today (`alpha` defaults to `1.0`, same as the current
hardcoded value). `semver:minor`, not `major` — correcting the sweep
audit's original triage on issue #115, which assumed a breaking
signature change before this design settled on the additive builder
shape.

### Slice 3: `Camera::project_line`'s `ProjectLineParams`

New struct in `src/perspective.rs`, alongside `Point3`/`Line3`/`Polygon3`:
```rust
/// Screen/subpixel parameters for `Camera::project_line` — bundles
/// everything except the line itself, since Falcon's HUD (the first
/// real consumer beyond the depth/perspective spike) call sites were
/// becoming hard to read with 7 trailing positional `f32`s.
#[derive(Clone, Copy, Debug)]
pub struct ProjectLineParams {
    /// Screen-space projection center, x.
    pub center_x: f32,
    /// Screen-space projection center, y.
    pub center_y: f32,
    /// Visible screen width in cells, for clipping.
    pub screen_w: f32,
    /// Visible screen height in cells, for clipping.
    pub screen_h: f32,
    /// Canvas subpixel columns per cell.
    pub subpixels_x: f32,
    /// Canvas subpixel rows per cell.
    pub subpixels_y: f32,
    /// Minimum projected scale for the line to be drawn at all.
    pub min_scale: f32,
}
```
`project_line`'s signature changes from 8 positional arguments to 2:
```rust
pub fn project_line(&self, line: Line3, params: ProjectLineParams) -> Option<(u16, u16, u16, u16)> {
    let (sx0, sy0, scale0) = self.project(line.start, params.center_x, params.center_y)?;
    let (sx1, sy1, scale1) = self.project(line.end, params.center_x, params.center_y)?;
    if scale0.max(scale1) < params.min_scale {
        return None;
    }
    let (cx0, cy0, cx1, cy1) = clip_to_screen(sx0, sy0, sx1, sy1, params.screen_w, params.screen_h)?;
    Some((
        (cx0 * params.subpixels_x).round().clamp(0.0, u16::MAX as f32) as u16,
        (cy0 * params.subpixels_y).round().clamp(0.0, u16::MAX as f32) as u16,
        (cx1 * params.subpixels_x).round().clamp(0.0, u16::MAX as f32) as u16,
        (cy1 * params.subpixels_y).round().clamp(0.0, u16::MAX as f32) as u16,
    ))
}
```
The `#[allow(clippy::too_many_arguments)]` attribute is removed — no
longer needed at 2 arguments.

**Both real call sites update** (confirmed via `grep -rn
"\.project_line(" src/ examples/`): `examples/falcon/falcon.rs:365`
and `examples/falcon/hud.rs:31`, each currently passing the same 7
trailing values (`area.width as f32 / 2.0, area.height as f32 / 2.0,
area.width as f32 - 1.0/2.0, area.height as f32 - 1.0/4.0, 2.0, 4.0,
0.0` at the `hud.rs` site) positionally — both become a single
`ProjectLineParams { center_x: ..., center_y: ..., screen_w: ...,
screen_h: ..., subpixels_x: 2.0, subpixels_y: 4.0, min_scale: 0.0 }`
literal.

**`project_polygon` is explicitly untouched** — it has a similarly
long parameter list, but no finding was filed against it, and
extending scope to a function nobody flagged is exactly the kind of
unrequested drift YAGNI exists to prevent. If it needs the same
treatment later, that's a new finding through the normal process, not
scope creep here.

**All 4 existing `project_line` tests** (`src/perspective.rs`'s
`#[cfg(test)] mod tests`) update their call sites from positional args
to a `ProjectLineParams { .. }` literal — no assertion logic changes,
same inputs/outputs, just restructured as named fields instead of
positions.

## Non-goals

- **`project_polygon`'s parameter list.** Not filed, not touched.
- **Any behavior change to the projection/clipping math itself** in
  Slice 3 — this is a pure signature restructuring, not a fix to
  `project`/`clip_to_screen`.
- **A `CellStyle`-wide review of `Theme`'s other fields** (`border`,
  `border_thick`, etc.) — Slice 1 fixes exactly the field the sweep
  audit flagged, not a broader `Theme` redesign.
- **Adding alpha support to any other overlay type** (`GlitchBuffer`'s
  sibling systems, if any) — Slice 2 is scoped to the one struct the
  finding named.
- **Re-triaging any other sweep-audit finding.** Only #108/#115/#119
  are in scope; the 13 `semver:patch` issues stay in the post-v1 queue
  untouched.

## Testing

`coding`-tagged, TDD mandatory, per `.claude/rules/development-
conventions.md` — no exemption applies to any of these three slices.

- **Slice 1:** existing `block.rs` tests (`border_cells_are_bold_when_
  theme_border_bold_is_true`, `title_cells_are_not_bold_even_when_
  theme_border_bold_is_true`, and `theme.rs`'s `default_theme_border_
  bold_is_false`) are the regression coverage — updated to construct/
  assert via `border_style` instead of `border_bold`, same behavioral
  claims. No new test needed beyond confirming these still pass
  post-rename (a pure field/type rename with preserved semantics
  doesn't need new coverage, the existing tests already prove the
  rendered-cell behavior).
- **Slice 2:** new test(s) proving `with_alpha` actually changes
  rendered cells' `alpha` field (e.g. `gb.with_alpha(0.5)` then
  `render()` then assert `cell.alpha == 0.5`), plus a test confirming
  the *default* (no `.with_alpha()` call) still renders `alpha: 1.0`
  (guards the "existing callers unaffected" claim, not just assumes
  it).
- **Slice 3:** the 4 existing `project_line` tests, updated to the new
  signature, are the regression coverage (same inputs, same expected
  outputs, just called via the struct). No new test needed — this is
  a pure signature restructuring with unchanged internal logic.

## Critical files

- `src/theme.rs` — `Theme.border_style: CellStyle` (Slice 1).
- `src/widgets/block.rs` — `render()`'s destructuring + `plain` closure
  + 8 test literals (Slice 1).
- `src/widgets/cockpit_panel.rs`, `src/widgets/smash_border.rs` — one
  `Theme { .. }` literal each (Slice 1).
- `examples/control_panel.rs`, `examples/falcon/falcon.rs`,
  `examples/launcher/portal.rs`, `examples/mission_control.rs`,
  `examples/omnitrix/omnitrix.rs`, `examples/smash_crabs/smash_crabs.rs`,
  `examples/tardis/tardis.rs` — one `Theme { .. }` literal each
  (Slice 1).
- `src/glitch.rs` — `GlitchBuffer.alpha` field, `with_alpha`, `render()`
  (Slice 2).
- `src/perspective.rs` — `ProjectLineParams`, `project_line()`'s
  signature, 4 test call sites (Slice 3).
- `examples/falcon/falcon.rs`, `examples/falcon/hud.rs` — `project_line`
  call sites (Slice 3).

## Verification

- `cargo build --all-targets` / `cargo clippy --all-targets -- -D
  warnings` / `cargo fmt --check` — clean.
- `cargo test` — full suite green, including every updated test in
  `block.rs`, `theme.rs`, `glitch.rs`, `perspective.rs`.
- Per `.claude/rules/development-conventions.md`'s visual-review
  mandate: Slice 1 touches `src/widgets/` (rendering-affecting) —
  `tools/visual-snapshot` must run against at least one themed example
  with `border_bold: true`-equivalent state (e.g. `falcon` or
  `omnitrix`) and the resulting PNG/GIF read before approving. Slice 2
  touches `src/glitch.rs` (rendering-affecting) — same requirement,
  against an example that triggers a glitch (Falcon's percussive-
  maintenance mechanic). Slice 3 touches `src/perspective.rs`, not
  itself in the mandated file list, but its only real consumer
  (Falcon's HUD) is — capture Falcon's HUD states too, confirming the
  restructured `project_line` calls still render identically to before.
- Close issues #108, #115, #119 by referencing them in each slice's
  commit, per `code-forge.md`'s filing-work convention.
