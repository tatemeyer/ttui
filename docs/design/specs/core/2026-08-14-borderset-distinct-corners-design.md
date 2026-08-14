# BorderSet Distinct Corner Glyphs Design

**Status:** approved (brainstorming complete 2026-08-14)

Resolves GitHub issue #130 (`semver:major`, `v1-blocking`) — the last
remaining `v1-blocking` item before the TTUI v1.0.0 tag. Surfaced
during the Showcase Polish Arc's brainstorming: the user wants real
box-drawing borders with genuinely distinct corner glyphs (`┌┐└┘`),
which `ttui::theme::BorderSet`'s current shape can't express.

## Problem

`BorderSet` (`src/theme.rs`) has a single `corner: char` field, reused
at all 4 corner positions by every consumer:

- `Block::render` (`src/widgets/block.rs`) — 4 separate corner-setting
  call sites (2 per ring, for both the normal ring and the optional
  outward "thick" ring), all reading `border.corner`.
- `SmashBorder::render` (`src/widgets/smash_border.rs`) — its middle
  ring reads `theme.border.horizontal`/`vertical`/`corner`; the other
  two rings use their own hardcoded chars, unrelated to `BorderSet`.

There is no way to render a proper box-drawing border (`┌` top-left,
`┐` top-right, `└` bottom-left, `┘` bottom-right) — only a single
glyph repeated at every corner.

## Design

**`BorderSet` gains 4 named corner fields**, replacing `corner: char`:

```rust
pub struct BorderSet {
    pub horizontal: char,
    pub vertical: char,
    pub top_left: char,
    pub top_right: char,
    pub bottom_left: char,
    pub bottom_right: char,
}
```

**Two named constructors, both `const fn`:**

```rust
impl BorderSet {
    pub const fn single_line() -> Self {
        BorderSet {
            horizontal: '─',
            vertical: '│',
            top_left: '┌',
            top_right: '┐',
            bottom_left: '└',
            bottom_right: '┘',
        }
    }

    pub const fn ascii() -> Self {
        BorderSet {
            horizontal: '-',
            vertical: '|',
            top_left: '+',
            top_right: '+',
            bottom_left: '+',
            bottom_right: '+',
        }
    }
}

impl Default for BorderSet {
    fn default() -> Self {
        Self::single_line()
    }
}
```

`BorderSet::default()` changing from `ascii()`-equivalent to
`single_line()` is a deliberate, in-scope behavior change — every app
currently relying on `BorderSet::default()` (see Migration below)
gets real box-drawing borders with zero code changes on its part.

## Migration (all ~13 call sites, mechanical per site)

**No code changes needed** — these already construct `BorderSet::default()`
explicitly inside a full `Theme` literal, and render their borders via
`Block` (which does read `theme.border`), so they pick up
`single_line()`'s box-drawing corners automatically:
`showcase/showcase.rs`, `examples/control_panel.rs`,
`examples/mission_control.rs`.
(`src/widgets/cockpit_panel.rs` is not actually a `BorderSet` consumer —
`CockpitPanel::render` hardcodes its own `'+'`/`'¤'` corner glyphs and
never reads `theme.border`; its only `BorderSet::default()` usage is in
its `#[cfg(test)] test_theme()` helper, which doesn't affect rendering.
`examples/falcon/falcon.rs` constructs `BorderSet::default()` too, but
its only bordered widget — its HUD panels (`falcon.rs:271`) and boot
screen (`boot.rs:48`) — is `CockpitPanel`, not `Block`, so it inherits
the same non-consumption: falcon's panel corners are unaffected by this
change, for the identical reason as `cockpit_panel.rs` above.)

**Mechanical field-rename, same visual result** — these already hand-write
a custom `BorderSet` literal with `horizontal`/`vertical`/a single
`corner` expression; migrate by replacing `corner: <expr>` with all 4
new fields set to that identical `<expr>`, preserving today's exact
rendered look:

- `examples/tardis/tardis.rs`, `examples/omnitrix/omnitrix.rs`,
  `examples/smash_crabs/smash_crabs.rs` — static `corner: '+'` (or
  similar single char) → all 4 fields get that same char.
- `examples/launcher/portal.rs` — **dynamic** corner (`if focused {
  '◆' } else { '·' }`); all 4 fields get that identical expression,
  preserving the existing focus-indicator behavior exactly (all 4
  corners already show the same glyph today, so this is still a
  faithful 1:1 migration, not a behavior change).

**Rendering-logic changes** (not just call-site literals):

- `Block::render`'s 4 corner-setting call sites (2 per ring × 2
  rings) each switch from `border.corner` to the field matching that
  specific corner's position (`top_left` for the ring's own top-left
  cell, etc.) — mechanical, no structural change to `draw_ring`'s
  shape.
- `SmashBorder::render`'s middle-ring tuple
  (`(theme.border.horizontal, theme.border.vertical,
  theme.border.corner, theme.primary)`) can no longer carry a single
  corner char in one tuple slot; the ring-drawing loop's corner-cell
  calls need to reference the correct field directly instead of
  destructuring a single `c`. The outer/inner rings' hardcoded chars
  (`'#'`/`'-'`/`':'`/`'.'`) are untouched — they never went through
  `BorderSet`.

## Testing

Full TDD — this is core `src/` library code, no exemption. New tests
for `single_line()`, `ascii()`, and `Default::default()` matching
`single_line()`. `Block`'s and `SmashBorder`'s existing corner tests
(currently asserting one shared corner value, e.g.
`assert_eq!(buf.get(0, 0).symbol, '*')`) extend to assert all 4
corners independently, using 4 visually distinct test glyphs so a
test can't pass by accident if two corners were swapped.

Mandatory `tools/visual-snapshot` capture + review (per
`development-conventions.md`'s "Visual review" convention) for at
least one app using the new `single_line()` default (e.g. `showcase`)
and one using a custom preserved glyph (e.g. `tardis`), confirming
real distinct box-drawing corners render and no `font8x8` glyph-
coverage gap is hit for `┌┐└┘` (not on the project's known-gap list,
but unverified until actually captured).

## Out of scope

- No change to `Block`'s or `SmashBorder`'s other rendering behavior
  (title placement, `border_thick`, `primary_end` gradient, ring
  bevel colors) — only corner-glyph plumbing.
- No new preset beyond `single_line()`/`ascii()` (e.g. a double-line
  `╔╗╚╝` variant) — YAGNI, nothing in this Arc's scope calls for one.
- `showcase/mascot.rs`'s pixel-tile rendering is unrelated (solid-
  color `Cell` fills, not `BorderSet`-driven) and untouched.
