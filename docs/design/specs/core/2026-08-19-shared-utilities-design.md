# Shared Utilities — Design

**Status:** draft, pending review before we move to planning.
**Date:** 2026-08-19
**Relationship to prior work:** second Arc of the v1.1 initiative, and
the Arc that takes it to a releasable v1.1.0. Follows
`2026-08-19-phase-arithmetic-design.md`, which extracted the first thing
every app rebuilt by hand. This extracts the rest of what an audit found.

## What this Arc follows from

Arc 1 established the shape: TTUI is a personal engine for making themed
terminal apps, so the engine's job is making the next app cheaper. Arc 1
took phase arithmetic. This Arc takes the three other things the apps —
and in one case the library itself — turned out to be rebuilding.

The candidates were found by audit, not by intuition, and two apparent
candidates were rejected on inspection (see "Rejected").

## Problem

### 1. Colour scaling exists three times, and one is inverted

| where | signature | what `1.0` means |
|---|---|---|
| `examples/launcher/main.rs` | `dim_color(c, f)` -> `× f` | full brightness |
| `src/widgets/roundel.rs` | `scale_color(c, intensity)` -> `× intensity` | full brightness |
| `src/camera.rs` | `scale_color(c, factor)` -> `× (1.0 - factor)` | **black** |

Two of the three are private duplicates *inside the library*. The third
is inverted relative to the other two.

This is not a tidiness problem. During #139's investigation the
inversion produced a wrong analysis that had to be corrected mid-flight:
`camera::dim(&scratch, 1.0)` reads as "full brightness" and means
"black". A convention that inverts between two functions with nearly the
same name is a trap that has already been sprung once.

### 2. `scatter` exists four times, byte-identical

```rust
fn scatter(seed: u32, spread: f32) -> f32 {
    let h = (seed.wrapping_mul(2_654_435_761)) ^ (seed.wrapping_mul(40_503).rotate_left(13));
    ((h % 10_000) as f32 / 10_000.0 - 0.5) * spread
}
```

Identical — including the magic constants — in `examples/depth_spike.rs`,
`examples/falcon/falcon.rs`, `examples/mission_control.rs` and
`showcase/telemetry.rs`. Deterministic scatter with no RNG dependency is
exactly the kind of thing a themed-app engine should hand you; instead
four apps carry their own copy, and a fifth app wanting stars has no
reason to think one exists.

### 3. `blit` exists three times, byte-identical

```rust
fn blit(scratch: &Buffer, area: Rect, buf: &mut Buffer) {
    for y in 0..scratch.height {
        for x in 0..scratch.width {
            buf.set(area.x + x, area.y + y, scratch.get(x, y).clone());
        }
    }
}
```

Identical in `examples/omnitrix`, `examples/smash_crabs` and
`examples/tardis`, and called from more sites again (both `boot.rs`
files use their app's copy).

`Canvas::blit` exists but blits a *`Canvas`*. Drawing a scratch `Buffer`
into another `Buffer` at an offset — the operation behind every
render-to-scratch-then-composite effect in the tree — is simply missing,
so three apps wrote it.

## Scope

**Tag: `coding`.** TDD test-first, no exceptions.

**Autonomy tier: Gated.**

**SemVer: `semver:minor`.** Three wholly new `pub` items; **no existing
signature changes** (see the `camera::dim` decision below). This Arc plus
Arc 1 is what makes v1.1.0 a release worth cutting rather than one type.

### In scope

- Three additive primitives, and migrating the ten duplicate definitions
  onto them.

### Out of scope

- **Unifying the noise hashes.** `src/glitch.rs` has its own inline hash,
  and `depth_spike`'s comment says `scatter` matches "the same style". It
  is a *different* function — three inputs `(x, y, tick_count)` on `u64`
  with different constants, versus one `u32` seed. Same style, different
  job. Considered and left alone.
- **Moving `lerp_color`.** It lives in `easing` and moving it would
  break. The new colour primitive joins it there rather than starting a
  second home.
- **Changing any app's visible output.** Refactor only.

## Approach

### `easing::scale_color`

```rust
/// Scales each channel of an `Rgb` colour by `factor`: `1.0` leaves it
/// unchanged, `0.0` is black.
pub fn scale_color(c: Color, factor: f32) -> Color;
```

Multiply semantics, matching two of the three existing implementations
and the plain reading of the name. Lives in `src/easing.rs` beside
`lerp_color`, so colour helpers keep one home.

**Non-`Rgb` returns unchanged**, and this deliberately differs from
`lerp_color`'s midpoint switch (#122) for a stateable reason: a lerp has
two endpoints and can honestly pick the nearer one, but *scaling* a
named colour has no meaningful answer at all — you cannot halve the
brightness of a colour whose RGB the terminal has not told you. Passing
it through unchanged is the only non-fabricating option.

### `camera::dim` keeps its signature

`camera::dim(&Buffer, factor)` is public and its `factor` means
*how much to dim* — the inverted sense. Flipping it would be a
`semver:major` break of a shipped API.

It therefore keeps its signature and meaning, and delegates internally:

```rust
scale_color(cell.fg, 1.0 - factor)
```

The inversion gets an explicit doc comment naming it as a trap rather
than leaving it to be rediscovered. `roundel`'s private copy and
`launcher`'s `dim_color` are deleted outright, since neither is public.

A future 2.0 could align `dim`; this Arc records the debt rather than
paying it with a major version.

### `noise::scatter`

```rust
/// Deterministic scatter in `[-spread/2, +spread/2]` from `seed`.
/// Same seed, same value — no RNG dependency and no per-frame state.
pub fn scatter(seed: u32, spread: f32) -> f32;
```

New `src/noise.rs`. Small, but a clear home: star placement, jitter and
telemetry wobble all want deterministic scatter, and `easing` is the
wrong shelf for it.

### `Buffer::blit`

```rust
/// Draws `self` into `dest` with its top-left at `(x, y)`, clipping at
/// `dest`'s bounds.
pub fn blit(&self, dest: &mut Buffer, x: u16, y: u16);
```

Deliberately mirrors `Canvas::blit(&self, buf, x, y)` — "draw me into
that" — so the two read the same way rather than being mutual opposites.
Call sites become `scratch.blit(buf, area.x, area.y)`.

The apps' copies take a `Rect` and use only its `x`/`y`; taking the two
coordinates is honest about what is used and matches `Canvas`.

**Clipping is a behaviour change, and an intentional one.** The apps'
copies call `buf.set` unconditionally, which is safe only because no
current call site blits out of bounds.

It is worth being precise about what they are relying on, because it is
not what `Buffer::set` documents. `set` says *"Panics if out of
bounds"*, and that holds for `y`, but not for `x`:

```rust
fn index(&self, x: u16, y: u16) -> usize {
    y as usize * self.width as usize + x as usize
}
```

On a 4x3 buffer, `set(5, 0, ..)` indexes `0 * 4 + 5 = 5`, which is a
valid cell — so it writes silently to `(1, 1)` rather than panicking. An
overflowing `x` wraps onto the next row. The existing helpers are one
mis-sized scratch buffer away from corrupting a neighbouring row with no
error.

Explicit clipping in `Buffer::blit` closes that, and matches
`Canvas::blit`, which is already tested for exactly this
(`blit_clips_to_the_target_buffers_bounds_without_panicking`). No
current call site blits out of bounds, so no output changes today.

**`Buffer::set`'s inaccurate contract is a separate finding** and is not
fixed here — tightening it would mean either a real bounds check on a
hot path or a documentation change, and that is its own decision. It
should be filed and triaged per `code-forge.md` rather than folded into
this Arc.

## Verification

- **TDD test-first** for all three primitives. `scale_color` at `1.0`,
  `0.0` and a midpoint, plus non-`Rgb` passthrough; `scatter`
  determinism, range, and that different seeds differ; `blit` including
  the clipping case.
- **`camera::dim` keeps a test pinning its inverted convention**, so the
  delegation cannot silently flip it. This is the one place where a
  wrong sign would be invisible to the capture comparison, because
  `dim` is used inside boot fades where the whole screen is dark anyway.
- **Migration must be invisible.** Every app touched is captured before
  and after, comparing against the boot-focused scripts Arc 1 built —
  the `.plumb` scenarios alone do not exercise boot.
- `CHANGELOG.md` gains `Added` entries for all three.

## Rejected

- **A starfield primitive.** Three apps have starfields, but they are
  genuinely different: `falcon` drifts its own `Star` struct in 3D,
  `launcher` spawns `Particle`s, `depth_spike` projects its own. Same
  idea, different implementations — unifying them would repeat the
  mistake the hub abstraction would have been in Arc 1.
- **A hint-bar helper.** Eight-plus screens draw a bottom hint row, but
  each is already one line of `Text::new(..).render(hint_row, buf)`.
  Only the `hint_row` rect computation repeats. Too thin to be worth an
  API.
- **Unifying `render_row`.** Two definitions, different signatures
  (`omnitrix` takes `y`, `fg` and `bg`; `smash_crabs` takes neither).
  Shared name, different jobs.
- **Flipping `camera::dim` to match.** Cleanest end state, but costs a
  major version for an engine with one consumer. Recorded as debt.

## Open questions for planning

1. **Migration batching** — ten definitions across seven files. Arc 1's
   answer (one PR per slice, apps batched within a slice) probably
   carries over.
2. **Whether `showcase` counts as an app for capture purposes.** It has
   no `.plumb` scenario, and `showcase/telemetry.rs` holds one of the
   four `scatter` copies. Arc 1 hit the same gap with `smash_crabs`.
3. **Whether `noise` should be one module or fold into an existing one.**
   One function is a thin module; the alternative is `easing`, which is
   the wrong shelf. Revisit if nothing else lands there.

## Filed during this design

- **#161** — `Buffer::get`/`set` document *"Panics if out of bounds"* but
  an overflowing `x` silently wraps onto the next row. Found while
  reasoning about `blit`'s clipping, reproduced with a throwaway test,
  and filed rather than folded in: the fix is a decision (real bounds
  check on a hot path, a `debug_assert`, or a documentation correction)
  rather than a patch this Arc should make on the way past.
