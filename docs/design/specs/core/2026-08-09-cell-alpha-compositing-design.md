# Cell Alpha Compositing — Design

**Status:** draft, pending your review before we move to planning.
**Date:** 2026-08-09
**Relationship to prior specs:** closes the one lever the rendering-
fidelity spike (`2026-08-08-rendering-fidelity-spike-design.md`)
explicitly deferred — "real alpha/blend compositing" — flagged there
and again in `2026-08-08-rendering-primitives-graduation-design.md` as
needing "its own dedicated spec given the `Cell`-shape cost." This is
that spec. Builds on `LayerStack`/`composite()`
(`2026-08-05-buffer-layering-compositing-design.md`,
`2026-08-05-buffer-layering-followups-design.md`) and `easing::
lerp_color` (Arc 0). Does not touch `Intensity`/`CellStyle` (Arc A) or
any widget's rendering logic beyond the mechanical migration below.

## Context / Motivation

`LayerStack::composite()` today is a hard cutout: for each position, it
scans layers top-to-bottom and the first non-`Cell::default()` cell
wins outright — no color math, no partial coverage. The spike's own
particle-trail prototype demonstrated that a per-cell color *can*
already fade smoothly over time, entirely without a `Cell`-level alpha
field, as long as both fade endpoints are real `Color::Rgb` values
(`easing::lerp_color` only interpolates between those). The specific
thing that breaks is fading *toward or from* `Color::Reset` — the
default/unset color most of this codebase's "untouched" cells carry —
because `Color::Reset` isn't a real color with RGB channels to
interpolate.

That reframes what this Arc actually needs to deliver: not just "a way
to blend two known colors" (already possible via `lerp_color`), but a
per-cell notion of **coverage** — how much a cell actually occludes
whatever is beneath it — that composites correctly across an arbitrary
number of `LayerStack` layers, independent of whether either side of a
blend happens to be a real color. That is what a persistent `alpha`
field on `Cell` provides, and why this is scoped as a real `Cell`-
shape change rather than another blend-call parameter (`blend_over`'s
existing shape already covers the "blend two known buffers with one
scalar" case and is not superseded by this — see Non-goals).

## Scope

**Tag: `coding`.** Full TDD applies, no exceptions.

### 1. `Cell` gains `alpha: f32`

```rust
pub struct Cell {
    pub symbol: char,
    pub fg: Color,
    pub bg: Color,
    pub style: CellStyle,
    pub alpha: f32, // 0.0 = fully transparent, 1.0 = fully opaque
}
```

**The one invariant everything else depends on:** `Cell::default().alpha
== 0.0`. `Buffer::new()` fills every cell with `Cell::default()`, and
`LayerStack::composite()`'s entire transparency model — "an unpainted
layer cell lets what's beneath it show" — depends on that sentinel
staying transparent. Flipping this to `1.0` (the intuitively "safer"
default for a cell you're about to customize) would make every fresh,
untouched buffer position opaque, silently breaking every existing
multi-layer app the moment this field lands. `Default` stays wired to
`0.0` specifically to preserve that invariant, not out of any general
principle that transparent should be the default.

**The consequence that invariant forces:** because `Cell::default()`
is transparent, any *real* painted cell must set `alpha: 1.0`
explicitly — it cannot rely on `..Default::default()` to fill the
field in, because that would silently produce an **invisible cell**,
not a compile error. This is a materially worse failure mode than any
previous mechanical migration in this project (a wrong `Intensity`
default just meant "not bold"; a wrong `alpha` default means "not
there"). The implementation plan's migration task must specifically
hunt down every `Cell { ..., ..Default::default() }` (or equivalent
partial-spread) construction site — not just every exhaustive literal,
which the compiler already forces to a decision — and deliberately add
`alpha: 1.0` to each. Exhaustive literals (no spread) get a hard
compile error and are the safe case; spread-using sites are where this
actually bites, and known sites include `src/widgets/text.rs` and
`src/particles.rs` at minimum — the plan enumerates the full set.

### 2. `LayerStack::composite()` becomes real top-to-bottom alpha accumulation

Replace the current "first non-default cell wins" scan with Porter-Duff
"over" compositing, walking layers top-to-bottom and tracking how much
of the final pixel is still "up for grabs":

```
for each position (x, y):
    remaining = 1.0        // opacity budget not yet claimed by a layer above
    result = Cell::default() // stays default (alpha 0.0) if nothing ever contributes
    for layer in layers, top to bottom:
        if remaining <= 0.0: break          // fully covered already, nothing beneath can show
        cell = layer.get(x, y)
        if cell.alpha <= 0.0: continue       // this layer didn't paint here
        contribution = cell.alpha * remaining
        result = blend(result, cell, contribution)   // see color/glyph rules below
        remaining *= 1.0 - cell.alpha
    composite.set(x, y, result)
```

**This is a deliberately exact generalization, not a new algorithm
bolted alongside the old one.** When every cell in a `LayerStack` has
`alpha: 1.0` (true for every existing app once the migration lands),
`contribution` is always `1.0 * remaining_before_this_layer`, `remaining`
drops to `0.0` on the first hit, and the loop breaks immediately after
the first non-transparent cell — byte-for-byte the same "topmost wins"
result the current implementation produces, for the same reason (early
exit on full coverage). The existing single-layer fast path
(`self.layers.len() == 1 → return self.layers[0].clone()`) is
untouched — a lone layer never needs blending against anything beneath
it, so its cells (alpha included) pass straight through.

**Color blending:** `fg`/`bg` blend via `easing::lerp_color(result.{fg,bg},
cell.{fg,bg}, contribution)` when both sides are `Color::Rgb`; for any
non-`Rgb` pairing, `lerp_color`'s existing fallback (return the target
color outright) applies — consistent with every other place in this
codebase that already accepts that limitation (`Theme.primary_end`,
`EnergyCore`, `DamageMeter`), not a new inconsistency introduced here.

**Glyph/style cannot blend** (a terminal cell shows exactly one
character). The layer contributing the most weight to the final pixel
determines `symbol`/`style`: the first layer (walking top-down) whose
`contribution >= 0.5` wins outright; if no single layer ever crosses
that threshold, the topmost non-transparent contributor's `symbol`/
`style` is used. This is the same "alpha >= 0.5 picks the glyph" rule
`blend_over` (the spike's prototype) already established — kept
consistent rather than inventing a second convention.

**Output alpha:** every position that received any contribution is
written with `alpha: 1.0` in the composited result (it is now real,
opaque terminal content — see §3); positions no layer ever touched
stay `Cell::default()` (`alpha: 0.0`), unchanged from today.

### 3. Nothing downstream of `composite()` changes

`diff()`, `CellDiff`, `render_diff`, and everything in `src/terminal.rs`
operate purely on the single, already-flattened `Buffer` a `LayerStack`
composites down to — a real terminal has no concept of partial
coverage, so by the time cells reach that pipeline they are, and
always were conceptually, fully opaque. `alpha` is a compositing-time-
only concept; nothing past `composite()` needs to read it, and this
spec makes no changes to any of those files. `diff()`'s existing
`PartialEq`-based change detection continues to work unmodified,
since `#[derive(PartialEq)]` on `Cell` automatically includes the new
field, and `composite()` deterministically produces `alpha: 1.0` or
`0.0` for identical input frames (never a stray fractional leftover),
preserving frame-to-frame diff stability.

## Non-goals

- **Not superseding `src/blend.rs`'s `blend_over`/`fade_toward`.**
  Those operate on two known `Buffer`s with a single scalar alpha
  passed at the call site — a genuinely different tool for "blend
  these two specific frames," independent of `LayerStack`. This spec
  doesn't touch that module. (`blend.rs` remains spike-only/
  uncommitted regardless — whether it graduates is a separate,
  future decision, not part of this Arc.)
- **No per-cell alpha *animation* helpers** (e.g. a "fade this region
  in over N ticks" widget-level utility) — this spec delivers the
  primitive `Cell.alpha` and correct `composite()` behavior; anything
  that drives alpha values over time is an app/widget concern for a
  future Arc, same app-space boundary this project has held
  consistently since Rev B.
- **No change to `Buffer::new()`'s signature or `Cell::default()`'s
  other fields** — only the new `alpha` field's value is decided here.
- **No attempt to give `render_diff`/the terminal writer any notion of
  transparency** — real terminals don't have one; see §3.

## Testing

Per `.claude/rules/development-conventions.md`: `coding`-tagged, full
TDD, no exceptions.

- **`Cell`/`Default`** — `Cell::default().alpha == 0.0`; a cell built
  via `Cell { alpha: 1.0, ..Default::default() }` is genuinely opaque
  by the fields that matter to compositing.
- **`composite()` regression suite (the critical guarantee):** every
  existing `LayerStack`/`composite` test in `src/buffer.rs` must keep
  passing unchanged once every test's `Cell` literals are migrated to
  `alpha: 1.0` — this is the concrete, provable "byte-identical to
  before" check the algorithm's design promises, not just an assertion
  in prose.
- **New alpha-specific tests:** two layers, top cell at `alpha: 0.5`
  over a known base color, asserts the exact `lerp_color` result at
  `t=0.5`; three-plus layers with varying alpha, asserting the
  accumulated `remaining` budget produces the mathematically correct
  blend (hand-computed, not just "looks plausible"); a fully-opaque
  layer over other layers correctly early-exits (verified by a
  bottom layer whose color would produce a *wrong* test result if it
  were incorrectly included); non-`Rgb` color pairs fall back per
  `lerp_color`'s existing documented behavior; the `>= 0.5` glyph/style
  selection rule at exactly the boundary and on both sides of it.
- **Migration regression:** `cargo build --all-targets` succeeding is
  the acceptance bar that every `Cell` construction site in the
  workspace was found and updated — a missed spread-based site would
  either fail to compile (if it happened to also need other new-field
  handling) or, worse, silently compile with an invisible cell, which
  is exactly why the migration task enumerates every spread-using site
  by hand rather than trusting the compiler to catch all of them.

## Critical files

- `src/buffer.rs` — `Cell.alpha` field, `Default` impl,
  `LayerStack::composite()` rewrite, new tests.
- Every file with a `Cell { ... }` construction site — enumerated
  precisely in the implementation plan, with explicit call-outs for
  every spread-based (`..Default::default()` or similar) site, which
  carry the real risk this spec discusses at length above.

## Verification

- `cargo test` — full suite green, including every existing
  `LayerStack`/`composite` test (unchanged assertions) and all new
  alpha-specific tests.
- `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` —
  clean.
- `cargo build --all-targets` — the whole workspace compiles, meaning
  every `Cell` construction site was found.
- `cargo run --example omnitrix` / `tardis` / `smash_crabs` /
  `launcher` — manual visual check confirming zero regression: every
  app's current visual output is unchanged, since every existing cell
  now carries `alpha: 1.0` and the new `composite()` algorithm
  degenerates to today's exact behavior at that value.
