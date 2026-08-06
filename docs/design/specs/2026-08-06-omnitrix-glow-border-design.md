# Omnitrix Glow Border (Issue #41) — Design

**Status:** draft, pending your review.
**Date:** 2026-08-06
**Relationship to prior specs:** implements the Core-wave ticket of Arc 1
(Omnitrix) from `2026-08-06-example-apps-roadmap-design.md` (issue #41,
tracking #52). Depends on Arc 0's `Cell.style.bold`
(`2026-08-06-core-capabilities-design.md`, issue #34), now shipped via
PR #84. Builds on the Rev B `Theme`/`BorderSet` abstraction
(`2026-08-05-ttui-rev-b-vision-alignment-design.md`) and
`examples/omnitrix.rs`'s existing "Recharge Pulse" animation.

## Problem

`examples/omnitrix.rs` renders its "Recharge Pulse" glow as a continuous
sine-wave brightness ramp on `Theme.primary`, feeding `Block::render`'s
border cells. This is Rev B's original placeholder; the roadmap's vision-doc
source describes the glow differently: *"glow effects achieved by layering
ANSI bold text over bright color variants."* `Cell.style.bold` (Arc 0)
exists but nothing in `Theme` or `Block` uses it yet — this spec closes that
gap for the border.

## Scope

Three small, coupled changes: a new `Theme` field, `Block::render` reading
it, and Omnitrix computing it. No other widgets, no title-text styling, no
change to the existing color-pulse math.

## Design

### `Theme` (`src/theme.rs`)

Add one field:

```rust
pub struct Theme {
    pub background: Color,
    pub primary: Color,
    pub secondary: Color,
    pub tertiary: Color,
    pub accent: Color,
    pub border: BorderSet,
    pub border_bold: bool,
}
```

Defaults to `false` in `Theme::default()`, consistent with every other
field's zero-effect default (`Color::Reset`, `BorderSet::default()`).

### `Block::render` (`src/widgets/block.rs`)

Border-glyph cells (the four edges and four corners, currently all built
through the shared `plain()` closure) get `style.bold` set from
`theme.border_bold` (or `false` when `self.theme` is `None`, matching the
existing no-theme defaults). Title-text cells do **not** — the vision-doc
quote is specifically about the border glow, and leaving title styling
alone avoids a second unrelated behavior change riding on this ticket.

Concretely, `plain()` currently backs both border and title cells with the
same `fg`/`bg`. It splits into a border-cell constructor (adds
`style: CellStyle { bold }`) while the title loop keeps building its
`Cell` the way it does today (default, non-bold style) — the two cases no
longer need to be identical past `fg`/`bg`.

### `examples/omnitrix.rs`

`theme()` already computes `brightness: f32` (0.0-1.0, from the sine wave)
each frame before deriving `primary`. Add one derived field to the
returned `Theme`:

```rust
border_bold: brightness > 0.6,
```

Chosen threshold: top ~40% of the pulse cycle. No other change to the
pulse's math, phase advancement, or tick handling — the border flips to
bold as the glow nears peak brightness and back to plain as it fades,
layering the bold-over-bright-color effect the vision doc describes
without replacing the existing continuous color fade.

## Testing

`Theme` and `Block` changes are `coding`-tagged, TDD applies
(`.claude/rules/development-conventions.md`):

- `src/theme.rs`: `Theme::default().border_bold == false`.
- `src/widgets/block.rs`: with a theme whose `border_bold: true`, rendered
  border cells (edge + corner) have `style.bold == true`; a title cell
  rendered alongside them has `style.bold == false`. Existing "without
  theme" test continues to assert border cells' `style.bold == false`
  (already true by `Cell::default()`, but worth an explicit assertion
  given the new field).

`examples/omnitrix.rs`'s threshold logic is example code, not
unit-tested per the TDD exceptions in `development-conventions.md`
("Examples/demos... correctness is checked by running the example, not
asserting on it") — verified manually per this spec's Verification
section below and PR #84's carried-over open item ("bold-cell rendering
has no current example consumer... worth a quick manual check when
Omnitrix's glow-border ticket (#41) picks this up").

## Verification

- `cargo test`, `cargo clippy`, `cargo fmt --check` green.
- `cargo run --example omnitrix`: visually confirm the border bolds in and
  out in sync with the brightness pulse, readable in a real terminal, no
  flicker/artifacts at the bold/non-bold transition.
- Check off #41 on tracking issue #52 once merged.
