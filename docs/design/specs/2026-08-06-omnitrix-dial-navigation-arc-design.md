# Omnitrix Dial + Navigation Arc — Design

**Status:** draft, pending your review.
**Date:** 2026-08-06
**Relationship to prior specs:** an Arc-level spec (per `docs/design/README.md`'s
Arc/Slice/Task structure) bundling five roadmap/backlog tickets from Arc 1
(Omnitrix) into one design and one plan: issues **#87** (new core Dial
widget, filed during this brainstorm), **#45** (custom 2-char-thick
border), **#43** (`AppMode` enum), **#88** (Faceplate revamp to use Dial,
filed during this brainstorm), and **#44** (app-switch corruption
transition). Builds on the shipped Faceplate hub
(`2026-08-06-omnitrix-faceplate-design.md`, issue #42, PR #86), the glow
border (`2026-08-06-omnitrix-glow-border-design.md`, issue #41, PR #85),
and Arc 0's `Transition` helper (`src/transition.rs`, issue #38).

## Problem

#42 shipped Faceplate as a plain vertical list with a throwaway
`Screen`/`DnaSample` state shape. This Arc replaces that with the real
navigation primitives the roadmap's vision doc calls for: an actual
circular dial visual (not just a highlighted list row), the roadmap's
named `AppMode` state shape (`Faceplate`/`Brainstorm`/`Fasttrack`/`Upgrade`),
a "corruption" transition when switching between them, and the toy-box
double-border look. None of these five pieces has a design yet.

## Scope

Five slices, ordered by dependency (core additions first, then example
integration in dependency order):

1. **Dial widget** (`src/widgets/dial.rs`, #87) — new, core framework.
2. **Border renderer** (`src/theme.rs`, `src/widgets/block.rs`, #45) — new
   `Theme` field + `Block::render` extension. Independent of the rest.
3. **`AppMode` enum** (`examples/omnitrix.rs`, #43) — replaces #42's
   `Screen`/`DnaSample` with the roadmap's flat four-variant shape.
4. **Faceplate-to-Dial revamp** (`examples/omnitrix.rs`, #88) — swaps
   `List` for `Dial` in `AppMode::Faceplate` rendering. Depends on 1 and 3.
5. **Corruption transition** (`examples/omnitrix.rs`, #44) — depends on 3
   and Arc 0's `Transition` (done).

**Explicitly out of scope:** a gauge/progress mode for Dial (YAGNI, no
consumer yet); real sub-app content for Brainstorm/Fasttrack/Upgrade
(#48-50, separate tickets, still unbuilt — they stay placeholder screens);
any new dependency (the Braille-noise effect in Slice 5 uses a
deterministic position/tick hash, not an RNG crate, matching Arc 0's
existing no-new-dependency posture).

## Design

### Slice 1: Dial widget (`src/widgets/dial.rs`, #87)

```rust
pub struct Dial<'a> {
    items: &'a [String],
    selected: usize,
}

impl<'a> Dial<'a> {
    pub fn new(items: &'a [String], selected: usize) -> Self { ... }
    pub fn render(&self, area: Rect, buf: &mut Buffer) { ... }
}
```

No `Theme` parameter — matches `List`/`Table`/`Text`'s existing precedent
(only `Block` takes a theme in this codebase).

**Geometry:** center `(cx, cy)` is `area`'s center. `radius_y = (area.height
/ 2 - 1).max(1)` (1-row margin). `radius_x = (radius_y * 2.0)` — the `2.0`
factor compensates for terminal cells being roughly twice as tall as wide,
so the ring reads as circular rather than elliptical — clamped so labels
have room: `radius_x = radius_x.min(area.width / 2 - 1).max(1)`.

**Item placement:** for item `i` of `N`, `angle = i * 2π/N - π/2` (item 0
at 12 o'clock, increasing clockwise). Point: `(cx + radius_x *
angle.cos(), cy + radius_y * angle.sin())`, rounded to the nearest cell.

**Ring:** between each pair of adjacent items, subdivide the arc into 4
steps and draw a `.` at the 3 intermediate angles (same point formula) —
a dotted ring boundary, not a continuous stroke.

**Labels:** outward-flowing. If a point's `x >= cx`, the label is
left-aligned starting at the point (extends right); if `x < cx`,
right-aligned ending at the point (extends left). Clipped to stay inside
`area`'s left/right edges regardless of label length.

**Selection:** the selected item's label gets `List`'s existing highlight
convention (`Color::Black` on `Color::White`). A single pointer glyph
(`*`) is drawn at the midpoint between center and the selected item's
point (`radius * 0.5` along the same angle) — a fixed marker, not an
8-way directional needle (keeps the geometry to one formula, reused from
the item-point calculation).

**Testing:** `coding`-tagged, TDD applies. Tests assert on computed
positions for known `area`/`N`/`selected` combinations (e.g. a 3-item
dial's item 0 lands at the top-center column, item positions are
symmetric left/right for odd `N`), that ring dots never land exactly on
an item's point, and that the selected item's cells carry the highlight
colors while others don't.

### Slice 2: Border renderer (`src/theme.rs`, `src/widgets/block.rs`, #45)

Add `pub border_thick: bool` to `Theme`, defaulting `false` (same pattern
as #41's `border_bold` — mechanical exhaustive-literal fixes required
across `examples/smash_crabs.rs`, `examples/omnitrix.rs`, and `block.rs`'s
existing tests, exactly like #41's Task 1).

When `true`, `Block::render` draws a second border ring one cell further
outward from the existing one (same `BorderSet` glyphs, same `fg`/`bg`),
run through the same per-edge/corner cell-setting loop already in
`Block::render` at `area` expanded by 1 in each direction — a picture-frame
double edge. When `false` (default), behavior is byte-for-byte identical
to today. Note: drawing outward means the caller's `area` must have at
least 1 cell of margin available, or the outer ring clips at the buffer
edge (existing `Buffer::set` bounds behavior applies; no new panic risk,
since `LayerStack`/`Buffer` indexing is already bounds-checked by the
caller's `area` sizing convention used elsewhere in this codebase). The
*returned* inner `Rect` (content area) is unchanged — thickness affects
what's drawn outside `area`'s edge, not what's excluded from inside it.

**Testing:** `coding`-tagged, TDD applies. Tests assert the outer ring's
cells (one step beyond the normal border) carry the border glyphs when
`border_thick: true`, and are untouched (`Cell::default()`) when `false`
or theme-less.

### Slice 3: `AppMode` enum (`examples/omnitrix.rs`, #43)

Replaces `Screen`/`DnaSample` with:

```rust
#[derive(Clone, Copy, PartialEq)]
enum AppMode {
    Faceplate,
    Brainstorm,
    Fasttrack,
    Upgrade,
}
```

`Omnitrix.mode: AppMode` replaces `screen: Screen` (starts at
`AppMode::Faceplate`); `selected: usize` is unchanged (still drives
Faceplate's cycling). A `const SAMPLES: [&str; 3] = ["Brainstorm",
"Fasttrack", "Upgrade"]` (or equivalent) replaces `DnaSample::ALL`/`name()`
for display strings, since `AppMode` itself is now flat (no nested
`DnaSample` payload to carry a name via `match`).

Interaction carries over onto the new shape: on `AppMode::Faceplate`,
Tab/Shift+Tab cycle `selected` (wrapping, same arithmetic as #42); Enter
maps `selected` (0/1/2) to `AppMode::Brainstorm`/`Fasttrack`/`Upgrade` and
sets `mode` to it. On any non-`Faceplate` mode, Esc sets `mode` back to
`AppMode::Faceplate` (selection is preserved automatically — `selected`
is a separate field, untouched by mode changes, same invariant as #42).
`q` still quits unconditionally, from any mode.

Rendering: `AppMode::Faceplate` renders the selector (Slice 4 changes
this from `List` to `Dial`); the three other modes render the same
placeholder pattern #42 used (`Text::new(name)` + `"(not yet built)"` +
hint line), keyed by `mode` instead of a `Screen::Launched(DnaSample)`
payload.

**Testing:** example code, no `src/` changes in this slice — verified by
running, not unit tested, per the TDD exceptions in
`development-conventions.md` (same as #42).

### Slice 4: Faceplate-to-Dial revamp (`examples/omnitrix.rs`, #88)

`AppMode::Faceplate`'s rendering swaps `List::new(&names,
self.selected).render(list_area, buf)` for `Dial::new(&names,
self.selected).render(dial_area, buf)`. Interaction (Tab/Shift+Tab/Enter,
wraparound, hint line) is unchanged — only the widget call changes. The
hint-row layout math from #42 (list area shrunk by one row for the hint)
still applies, now sizing `dial_area` the same way `list_area` was sized.

**Testing:** example code, no `src/` changes — verified by running.

### Slice 5: Corruption transition (`examples/omnitrix.rs`, #44)

`Omnitrix` gains `transitioning_from: Option<(AppMode, Transition)>`
(starts `None`). On any mode switch (Enter from `Faceplate`, or Esc back
to it), `mode` updates immediately to the destination (as today), and
`transitioning_from = Some((old_mode, Transition::start(Duration::from_millis(500))))`
is set. While `transitioning_from.is_some()`, Tab/Shift+Tab/Enter/Esc are
ignored (prevents overlapping transitions or switching mid-animation); `q`
still quits unconditionally. `on_tick` calls `.1.tick(elapsed)` on
`transitioning_from` when present, and clears it to `None` once
`.1.is_complete()`.

**Rendering, two phases by `Transition::progress()`:**
- **Flash** (`progress` in `[0.0, 0.2)`): the whole inner area renders as
  solid Hazard Yellow (`Color::Yellow` background, space glyphs) — no
  content visible.
- **Wave** (`progress` in `[0.2, 1.0]`): remap to `wave = (progress - 0.2)
  / 0.8` in `[0, 1]`. `wave_row = (wave * inner.height) as u16`. Rows
  `0..wave_row` show the **new** mode's content (already revealed, rendered
  via the same per-mode rendering used outside a transition); the row(s) at
  `wave_row` (a 1-row band) show Braille-pattern noise on the Hazard
  Yellow background; rows `wave_row+1..inner.height` show the **old**
  mode's content (not yet revealed). This requires rendering both the old
  and new mode's content into two scratch buffers each frame during a
  transition, then compositing row-by-row — the existing per-mode
  rendering logic (Slice 3/4) is factored into a helper callable for
  either mode, taking `(mode, area) -> Buffer` rather than writing
  directly to the app's `LayerStack`.
- **Braille noise:** glyph per noisy cell is chosen from Unicode Braille
  patterns (`U+2800`-`U+28FF`, 256 dot combinations) via a deterministic
  hash of `(x, y, tick_count)` — no RNG dependency, matching Arc 0's
  no-new-dependency posture. Any cheap mixing function (e.g. wrapping
  multiply/XOR of the three inputs) that visually varies frame-to-frame
  and cell-to-cell is acceptable; exact hash constants are an
  implementation detail, not a design requirement.

**Testing:** example code, no `src/` changes — verified by running,
including watching a full transition play in both directions (Faceplate
→ a sample, and back).

## Verification

- `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D
  warnings` all green (Slices 1-2 have real unit tests; Slices 3-5 are
  example code).
- `cargo run --example omnitrix`: confirm the Dial renders as a legible
  ring with outward-flowing labels and a visible pointer at the
  selection; confirm Tab/Shift+Tab/Enter/Esc behave identically to #42's
  interaction contract; confirm the corruption transition plays on every
  mode switch (both directions), the yellow flash is visible, the Braille
  wave sweeps top-to-bottom, keys are ignored mid-transition, and `q`
  still quits cleanly with no leftover terminal attributes.
