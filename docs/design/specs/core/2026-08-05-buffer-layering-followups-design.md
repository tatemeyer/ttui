# Buffer Layering Follow-ups — Design

**Status:** draft, pending your review before we move to planning.
**Date:** 2026-08-05
**Relationship to prior specs:** addendum to
`2026-08-05-buffer-layering-compositing-design.md` (the `LayerStack`/
`composite()` design, PR #32). That spec and its shipped code are
unchanged in their guarantees; this covers three follow-ups its final
whole-branch review parked as deferred, non-blocking items, now picked
up on the same branch.

## Scope of this spec

1. `LayerStack` API cleanup: remove redundant `width`/`height` fields,
   rewrite `composite()` to reverse-iterate and break early, add
   `layer_count()` and a read-only `layer()` accessor.
2. A stale-language fix in `docs/design/specs/2026-08-05-ttui-rev-b-vision-alignment-design.md`
   (one-line forward pointers, not a design decision — recorded here
   only so the implementation plan can reference it as a task).
3. A third example, `examples/smash_crabs.rs`, exercising the
   multi-layer path end-to-end — the one thing the final review found
   missing: neither `examples/demo.rs` nor `examples/omnitrix.rs` ever
   drives `push_layer()`/`composite()`'s multi-layer scan through a
   real render loop, only unit tests do.

## 1. `LayerStack` API cleanup

`LayerStack` (`src/buffer.rs`) currently stores its own `width`/`height`
alongside `layers: Vec<Buffer>`, duplicating `layers[0].width`/`height`
(public fields on `Buffer`) with no way for them to diverge — and, since
`LayerStack: Deref<Target = Buffer>`, the private fields shadow
`Buffer`'s public same-named fields depending on where an expression is
written. Both problems go away by dropping the two fields; every
internal read of them (`push_layer`, `composite`) becomes an explicit
`self.layers[0].width` / `self.layers[0].height` instead of relying on
`Deref`-based field autoderef.

`composite()`'s scan currently walks bottom-to-top, overwriting `cell`
on every non-default hit it sees. Walking top-to-bottom
(`self.layers.iter().rev()`) and stopping at the first non-default cell
produces an identical result under "last (topmost) non-default cell
wins," doing strictly less work.

`layer_count()` (returns `self.layers.len()`) and `layer()` (read-only
counterpart to `layer_mut`, same out-of-range panic behavior) close the
gap the final review flagged: an app using the pattern this spec's
predecessor recorded (app-local `const BACKGROUND: usize = 0;`-style
constants) has no way to confirm the stack is the depth it expects, or
to read a non-base layer without mutating it. Both are needed by the
example in section 3.

## 2. Rev B spec forward pointer

`docs/design/specs/2026-08-05-ttui-rev-b-vision-alignment-design.md`
describes buffer layering as "deferred, not designed" in two spots (the
"Deferred (documented, not designed): buffer layering" section, and a
bullet under "Explicitly deferred / open questions"). Both are now
stale. Fix: a one-line forward pointer at each spot to
`2026-08-05-buffer-layering-compositing-design.md`, leaving the
surrounding historical record intact rather than rewriting it.

## 3. `examples/smash_crabs.rs` — multi-layer end-to-end

**Theme choice: Smash Crabs, not TARDIS.** TARDIS's headline need (a
large virtual buffer plus a `Camera` projecting a viewport) is a
separate, still-undesigned deferred feature — building a real TARDIS
example now would either drag that abstraction into scope or fake it
unconvincingly. Smash Crabs' need — three explicit, named, z-ordered
buffers (background/UI/effects) composited before diffing — is exactly
what `LayerStack` already provides, nothing more.

The example follows `examples/omnitrix.rs`'s shape (tick-driven,
themed via `Theme`), addressing layers by app-local constants — the
pattern the predecessor spec's "App-space boundary" section describes
core as deliberately not knowing about:

```rust
const BACKGROUND: usize = 0;
const UI: usize = 1;
const EFFECTS: usize = 2;
```

Each frame, `view()` calls `push_layer()` twice (index 1, then index 2)
before painting, since `app.rs::run()` builds a fresh `LayerStack` every
frame:

- **Background (index 0, the base):** every cell painted with a themed
  arena backdrop (non-default `fg`/`bg` from `Theme`) — fully opaque, the
  floor everything else sits on.
- **UI (index 1):** a `Block`-framed "Fighters" panel in a corner
  (two `Text` lines: current HP for each of two fighters); the rest of
  this layer's cells are left at `Cell::default()`. Demonstrates
  fall-through: most of the screen shows the background layer through
  the UI layer's untouched cells.
- **Effects (index 2, topmost):** a transient "hit flash" — a small
  solid rectangle painted at a fixed position only while
  `flash_ticks_remaining > 0`, left entirely `Cell::default()`
  (transparent) otherwise. Pressing Space sets the counter and nicks
  the opponent's HP; `on_tick` (reusing the existing `tick_rate`/
  `on_tick` mechanism, unchanged since Rev B) decrements it each tick
  until it clears. Demonstrates both the top-wins case (the flash
  occludes background+UI while active) and the all-default/transparent
  case (background+UI show through again once it clears) — driven by
  real app state across real frames, not a unit test.

No core changes are needed beyond section 1's `LayerStack` additions;
`update()`/`tick_rate()`/`on_tick()` and all four widgets are used
exactly as they exist today.

## Testing

Per `.claude/rules/development-conventions.md`:

- Section 1 is `coding`-tagged, TDD required: write tests for
  `layer_count()`/`layer()` first (new behavior), then implement. The
  field removal and `composite()` rewrite are behavior-preserving
  refactors — the 6 existing `LayerStack`/`composite` tests in
  `src/buffer.rs` must keep passing unchanged as the regression check;
  they are not rewritten.
- Section 2 is pure docs, no TDD applies.
- Section 3 is an example/demo (explicit TDD exception) — correctness
  is checked by running it, not by asserting on it.

## Critical files

- `src/buffer.rs` — `LayerStack` field removal, `composite()` rewrite,
  `layer_count()`/`layer()`, two new tests.
- `docs/design/specs/2026-08-05-ttui-rev-b-vision-alignment-design.md`
  — two forward-pointer insertions.
- `examples/smash_crabs.rs` — new file.

## Verification

- `cargo test` — full suite green (existing `LayerStack`/`composite`
  tests unchanged in behavior, plus 2 new tests for
  `layer_count()`/`layer()`).
- `cargo fmt` / `cargo clippy --all-targets` clean.
- `cargo build --examples` — all three examples compile.
- Manual: `cargo run --example smash_crabs` — background renders
  themed and opaque everywhere; the fighter-status panel appears in its
  corner with the arena background visible everywhere else; pressing
  Space produces a brief flash that clears back to the arena/UI
  beneath it; `q` quits.
