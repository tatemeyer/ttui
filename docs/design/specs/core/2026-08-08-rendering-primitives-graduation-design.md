# Rendering Primitives Graduation — Design

**Status:** draft, pending your review before we move to planning.
**Date:** 2026-08-08
**Relationship to prior specs:** graduates three of the six levers
explored by the rendering-fidelity spike
(`2026-08-08-rendering-fidelity-spike-design.md`, PR #92) into real,
committed, TDD-covered core API, per that spec's "Recommendations
(post-spike)" section — specifically its graduation-ranking items 1-3
(`CellStyle` attributes, the `Canvas` sub-cell primitive, gradient
color ramps). Builds on the unchanged Rev A
(`2026-08-04-ttui-core-framework-design.md`) pipeline, Rev B
(`2026-08-05-ttui-rev-b-vision-alignment-design.md`) `Theme`, and Arc
0's `CellStyle`/`easing::lerp_color`
(`2026-08-06-core-capabilities-design.md`).

**Dependency:** this Arc assumes PR #92 has already merged to `main` —
it builds directly on that PR's `src/canvas.rs`/`src/blend.rs`
prototypes and its recommendations write-up. Do not start
implementation before that PR lands.

## Context / Motivation

The spike deliberately shipped prototype-quality code exempt from TDD,
explicitly not a committed API — every new module says so in its own
doc comment. Its recommendations ranked three of its six levers as
safe, high-confidence candidates to commit for real: full `CellStyle`
attributes, the `Canvas` sub-cell primitive, and gradient color ramps.
This spec is that graduation: same capabilities, this time as tested,
documented, stable public API that other Arcs (starting with the
widget-rebuild Arc that follows this one) can build on with confidence.

Alpha blending (lever 5) and any `Cell`-shape change are explicitly
excluded — the spike flagged that lever as needing its own dedicated
spec given the structural risk, and it's sequenced later as its own
Arc rather than folded in here.

## Scope

**Tag: `coding`.** Full TDD applies, no exceptions — this is committed
core, not a spike.

### 1. `CellStyle.intensity: Intensity` (replaces `CellStyle.bold: bool`)

```rust
/// Text intensity — a single SGR axis; a cell can be bold, dim, or
/// neither, never more than one at once.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Intensity {
    #[default]
    Normal,
    Bold,
    Dim,
}

pub struct CellStyle {
    pub intensity: Intensity,  // was: pub bold: bool
    pub underline: bool,
    pub italic: bool,
    pub reverse: bool,
    pub strikethrough: bool,
}
```

**This is the one genuinely breaking change in this Arc.** `bold` is
the only intensity-related field that exists on `main` today (the
spike's `underline`/`italic`/`reverse`/`strikethrough` fields are new
additions, not migrations) — so this is the cheapest point this
migration will ever be: fix it before a second bolted-on `dim: bool`
field ever ships and the wrong shape spreads further. Every existing
`CellStyle { bold: ... }` literal and `.style.bold` read across the
codebase (13 files reference bold in some form as of this writing)
migrates to `Intensity::Bold`/`Intensity::Normal` in the same task that
introduces the enum — Rust's exhaustiveness rule makes this mandatory,
same as every prior `CellStyle` field addition in this project's
history.

`render_diff` (`src/terminal.rs`) changes its bold tracker from
`Option<bool>` to `Option<Intensity>`, emitting `Attribute::Bold`,
`Attribute::Dim`, or `Attribute::NormalIntensity` on transition — a
three-way version of the existing two-way pattern, not a new pattern.

### 2. `underline`/`italic`/`reverse`/`strikethrough` — committed, tested

Same fields, same `render_diff` wiring shape the spike prototyped
(`Underlined`/`NoUnderline`, `Italic`/`NoItalic`, `Reverse`/`NoReverse`,
`CrossedOut`/`NotCrossedOut`) — this time with full unit test coverage
of `render_diff`'s coalescing behavior for all five style axes
together (the spike shipped zero tests for this, by design; this Arc
is exactly the "do it for real" pass).

### 3. `Canvas` (`src/canvas.rs`) — committed public API

One type, matching the spike's shape and your explicit call to keep it
that way rather than splitting into `HalfBlockCanvas`/`BrailleCanvas`:

```rust
pub enum CanvasMode { HalfBlock, Braille }
pub struct Canvas { /* ... */ }
impl Canvas {
    pub fn new(width: u16, height: u16, mode: CanvasMode) -> Self
    pub fn set_pixel(&mut self, x: u16, y: u16, color: Color)
    pub fn clear_pixel(&mut self, x: u16, y: u16)
    pub fn line(&mut self, x0: u16, y0: u16, x1: u16, y1: u16, color: Color)
    pub fn rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: Color)
    pub fn fill_rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: Color)
    pub fn blit(&self, buf: &mut Buffer, x: u16, y: u16)
}
```

Signatures unchanged from the spike. What changes: full TDD coverage
(pure unit tests — no real-TTY exception applies, same footing as
`layout`/`buffer`'s existing tests), `#[warn(missing_docs)]`-satisfying
doc comments on every `pub` item (already true in the spike, carries
forward), and the two minor correctness notes the spike's task reviews
already flagged get addressed as part of writing real tests this time:
`rect()`'s double-drawn corners (harmless today, but a test now pins
the behavior explicitly rather than leaving it implicit), and the
`u16` bounds arithmetic in `rect`/`fill_rect` gets `saturating_add`
instead of raw `+` so a canvas positioned near `u16::MAX` degrades
instead of panicking in a debug build — cheap correctness now that
this is committed code, not disposable prototype.

### 4. Gradient border on `Theme`/`Block`

```rust
pub struct Theme {
    // ...unchanged fields...
    pub primary_end: Option<Color>,  // new
}
```

`Block::render` (`src/widgets/block.rs`): when `theme.primary_end` is
`Some`, the border ring's color is `easing::lerp_color(primary,
primary_end, t)` where `t` is the cell's position around the
perimeter (0.0 at the ring's start, 1.0 at its end) instead of the
current flat `primary` color; `None` (the zero-value default, same as
every other `Theme` field today) preserves the exact current flat-color
behavior — every existing themed app (Omnitrix/TARDIS/Smash Crabs)
keeps rendering identically unless it explicitly opts in by setting
`primary_end`. No widget call-site signature changes; `Block::new()`
already takes `.theme(&theme)`, this is purely a new optional `Theme`
field the same widget call already reads.

**Perimeter position math:** reuses the position formula the spike's
`draw_gradient_ring` prototyped (`(x - area.x)/width + (y -
area.y)/height`, clamped to `0..1`) rather than inventing a new one —
that formula was already validated visually in the spike's showcase.

**Explicitly not built here:** a standalone gradient-fill helper
independent of `Block`/`Theme` (e.g. for filling an arbitrary region,
not just a border ring). Nothing in this Arc's scope needs one; add it
if/when Arc B's widget rebuilds actually need it, not speculatively.

## Non-goals

- Real per-cell alpha or any `Cell`-shape change (sequenced as its own
  later Arc).
- Rebuilding `TimeRotor`/`EnergyCore`/`DamageMeter`/`Roundel` on these
  primitives (the next Arc in the sequence — this one only makes the
  capability available).
- A separate `dim: bool` field — superseded by the `Intensity` enum.
- Any change to `Buffer`, `LayerStack`, or the diff algorithm.

## Testing

Per `.claude/rules/development-conventions.md`: `coding`-tagged, TDD
mandatory, no exceptions apply (not config/git-adjacent, not an
example/demo, not real-TTY-only beyond `terminal.rs`'s existing
draw_diff exception, not a research spike). Test cases:

- `Intensity`/`CellStyle` — default value, equality, and a regression
  test confirming every migrated call site's rendered SGR bytes are
  unchanged from before the migration (same technique the render-diff-
  performance Arc's tests already use: count specific CSI byte
  sequences, not whole-stream equality).
- `render_diff` — extend the existing coalescing test suite to cover
  all three `Intensity` transitions (`Normal→Bold`, `Bold→Dim`,
  `Dim→Normal`) plus the four boolean attributes, individually and in
  combination.
- `Canvas` — `set_pixel`/`clear_pixel` bounds behavior, `line` traced
  through multiple octants, `rect`/`fill_rect` correctness, `blit`'s
  half-block 4-case match and braille last-write-wins rule — the exact
  cases the spike's task reviews already traced by hand, now codified
  as real tests instead of reviewer-verified-by-inspection.
- `Block`/`Theme` gradient — a themed `Block::render` with
  `primary_end: Some(...)` produces a lerped color at a known
  perimeter position; `primary_end: None` produces byte-for-byte
  identical output to today's flat-color rendering (the critical
  regression guarantee for the three existing themed apps).

## Critical files

- `src/buffer.rs` — `Intensity` enum, `CellStyle` field change.
- `src/terminal.rs` — `render_diff`'s intensity tracking becomes
  three-way; new unit tests.
- `src/canvas.rs` — promoted from spike prototype to committed API;
  full test suite added; `saturating_add` fix in `rect`/`fill_rect`.
- `src/theme.rs` — `Theme.primary_end: Option<Color>`.
- `src/widgets/block.rs` — gradient-ring rendering when
  `primary_end` is set.
- Every file with a `CellStyle { bold: ... }` literal or `.style.bold`
  read (enumerated precisely in the implementation plan, mirroring how
  the spike's own `CellStyle` extension task enumerated its six sites).

## Verification

- `cargo test` — full suite green, including all new tests above.
- `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` —
  clean (hard gates now — the spike's exemptions don't apply here).
- `cargo build --examples` — all example apps compile against the
  migrated `Intensity` enum with zero behavior change (verified by the
  regression tests above, not just "it compiles").
- `cargo run --example omnitrix` / `tardis` / `smash_crabs` — manual
  visual check confirming no regression: every existing bold-rendered
  cell (borders, highlighted rows, etc.) still renders bold, since
  `Intensity::Bold` is a drop-in replacement for `bold: true`.
