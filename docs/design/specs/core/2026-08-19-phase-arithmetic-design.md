# Phase Arithmetic — Design

**Status:** draft, pending review before we move to planning.
**Date:** 2026-08-19
**Relationship to prior work:** first Arc of the v1.1 initiative, and the
first Arc chosen by the "future of TTUI" brainstorm rather than by a
vision doc or a bug. Extends `src/transition.rs`. Consumes the framing in
`docs/briefings/2026-08-16-future-of-ttui.md`.

## What this Arc follows from

The brainstorm settled the question that briefing deliberately left open.
**TTUI is a personal engine for making themed terminal apps.** The apps
are the point; the library exists to serve them. That resolves several
things the briefing flagged as ambiguous — the app-specific widget
sprawl is the product working as intended, the examples stay
first-class, and publishing to crates.io was about discipline and
permanence rather than adoption.

It also gives the engine a job it is not currently doing: **making the
next app cheaper to build.**

## Problem

Every themed app hand-rolls the same arithmetic.

A timed sequence — a boot animation, a screen transition, a multi-stage
effect — is driven by one `Transition` producing an overall `progress` in
`0..1`. Each app then subdivides that range into phases *by hand*, and
inside each phase recomputes how far along it is:

```rust
// examples/falcon/boot.rs
if progress < 0.1  { ...; return; }
if progress < 0.4  { let wave = (progress - 0.1) / 0.3;  ... return; }
if progress < 0.85 { let wave = (progress - 0.4) / 0.45; ... return; }
let fade = ((progress - 0.85) / 0.15).clamp(0.0, 1.0);
```

**Ten such expressions exist across four apps** — `falcon` x3,
`omnitrix` x3, `tardis` x3, `smash_crabs` x1 — and it is not only boot
code: screen transitions and effects use the same shape
(`omnitrix.rs:262`, `smash_crabs.rs:353`, `tardis.rs:284` and `:316`).

Three things are wrong with it, in increasing order of how quietly they
bite:

1. **The formula is rewritten every time.** `(p - start) / (end - start)`
   is simple enough to get right and tedious enough to keep writing.
2. **Clamping is inconsistent.** `omnitrix/boot.rs:22` clamps;
   `falcon/boot.rs:42` does not. Both feed values that happen to be
   defended downstream, so neither is a live bug — but which one is
   correct is currently a per-call-site accident.
3. **Each boundary is written twice** — once in the phase test
   (`if progress < 0.4`) and once in the next phase's local-progress
   formula (`(progress - 0.4) / 0.45`) — with nothing keeping the two in
   agreement, and the second also encoding the *following* boundary as a
   width. Editing a boundary means finding two or three places.

That third point is the real defect. Which phase you are in and how far
into it you are are two answers to one question, and today they are
derived independently.

## Scope

**Tag: `coding`.** TDD test-first, no exceptions.

**Autonomy tier: Gated** — PR with all four required checks green.

**SemVer: `semver:minor`.** This adds a wholly new `pub` item and changes
no existing signature, which `code-forge.md` sizes as additive. It is the
first change to warrant a minor bump since v1.0.0, and the reason this
Arc is v1.1 rather than a patch.

### In scope

- One new public type in `src/transition.rs` turning overall progress
  into `(phase index, progress within that phase)`.
- Migrating the ten existing call sites onto it.

### Out of scope

- **Any hub / sub-screen abstraction.** The brainstorm considered and
  rejected it: six apps have a `Screen`-style enum with a `match` in
  `update` and `view`, but the arms call bespoke per-app methods. That is
  idiomatic Rust, not duplication, and extracting it would mean generics
  and a trait for almost no gain — while abstracting away exactly the
  part that makes each app *themed*.
- **Owning the `Transition` or the tick loop.** A `BootSequence` driving
  its own timing was considered and rejected as reaching into how each
  app stores and advances state.
- **Changing any app's visible output.** This is a refactor: every
  migrated call site must render identically.

## Approach

```rust
/// Subdivides a `0..1` progress range at fixed boundaries, so "which
/// phase" and "how far into it" come from one declaration instead of
/// being derived separately at each site.
pub struct Phases<const N: usize> { /* bounds: [f32; N] */ }

impl<const N: usize> Phases<N> {
    /// Boundaries in ascending order, each in `0..1`. `N` boundaries
    /// describe `N + 1` phases.
    pub const fn new(bounds: [f32; N]) -> Self;

    /// The phase `progress` falls in, and how far through that phase it
    /// is — always clamped to `0..1`.
    pub fn at(&self, progress: f32) -> (usize, f32);
}
```

Declared once per sequence, as a `const`:

```rust
const BOOT: Phases<3> = Phases::new([0.1, 0.4, 0.85]);

let (phase, t) = BOOT.at(progress);
match phase {
    0 => { /* pre-reveal */ }
    1 => { /* canopy wireframe reveals across t */ }
    2 => { /* panels reveal across t */ }
    _ => { /* final fade across t */ }
}
```

Each boundary now appears exactly once, and the local `t` cannot disagree
with the phase, because both come from the same call.

`const fn new` matters: boundaries want to be `const` items alongside the
other tuning constants each app already keeps at the top of its file, not
values rebuilt every frame.

### Decisions inside the design

- **Clamped, always.** `at` returns `t` in `0..1`, ending the
  per-call-site inconsistency. A caller who genuinely wants unclamped
  overshoot can compute it; nobody currently does.
- **`N + 1` phases from `N` boundaries**, the final phase running to 1.0.
  This matches every existing sequence, all of which end in a phase with
  no upper boundary.
- **Out-of-range progress saturates** rather than panicking: below 0
  gives `(0, 0.0)`, at or above 1 gives `(N, 1.0)`.
- **Lives in `src/transition.rs`**, next to `Transition`, because the two
  are used together — `PHASES.at(t.progress())`. That file is 91 lines,
  far below the 500-line ceiling.

### What this does not fix

Worth stating plainly so the Arc is not oversold: **this would not have
prevented #117.** That bug was a final phase whose local progress
legitimately starts at 0, dimming the screen for one frame at the
boundary — inherent to phase-local progress, not an error in deriving it.
`Phases` makes boundaries single-sourced and clamping uniform. It does
not make the values chosen for each phase correct.

## Verification

- **TDD test-first**, covering: `N + 1` phases from `N` boundaries;
  clamping at both ends; out-of-range saturation; a boundary value itself
  landing in the *later* phase (`at(0.4)` with bounds `[0.1, 0.4, 0.85]`
  is phase 2 at `t = 0.0`, matching today's `if progress < 0.4` tests);
  and a zero-boundary `Phases<0>` behaving as one phase.
- **Migration is a refactor and must be provably invisible.** Every
  migrated app is captured before and after with `tools/visual-snapshot`
  and the frames compared. Boot sequences are short relative to capture
  cadence (#131), so the comparison rests on scenario captures plus the
  per-frame measurements this project already uses — not on catching one
  specific boot frame.
- Four required checks green, and `CHANGELOG.md` gains an `Added` entry
  under `[Unreleased]` for the new public type.

## Rejected alternatives

- **A free `phase_progress(progress, start, end) -> f32`.** Names the
  formula but leaves each boundary written twice, which is the actual
  defect. Considered and rejected in the brainstorm.
- **A `BootSequence` owning phases and timing.** Reaches into each app's
  state and tick handling; over-extraction. Rejected.
- **A hub / sub-screen framework.** See "Out of scope".
- **A slice-backed `Phases<'a>`** rather than const-generic. Avoids the
  `N` parameter but cannot be a `const` item as naturally, which is how
  every app will want to declare it.

## Open questions for planning

1. **Migration order and batching** — ten call sites across five apps.
   One PR, or one per app with its own visual review?
2. **Whether the non-boot call sites migrate too.** `omnitrix.rs:262`,
   `smash_crabs.rs:353` and `tardis.rs:284/316` use the same shape for
   screen transitions and effects. They should, but they are lower value
   than the boot sequences and could be deferred.
3. **Whether `Phases` should expose its boundaries** for a caller wanting
   a phase's absolute range. No current call site needs it, and adding it
   later is additive.
4. **Whether to accept durations as well as fractions.** `smash_crabs`
   does not write its boundaries as fractions at all — it declares them
   as durations and divides at runtime:

   ```rust
   let t1 = BOOT_FLASH_MS as f32 / BOOT_TOTAL_MS as f32;
   let t2 = (BOOT_FLASH_MS + BOOT_CLAW_MS) as f32 / BOOT_TOTAL_MS as f32;
   ```

   That is arguably the more natural way to author a boot sequence —
   "flash for 200ms, then claw for 800ms" — and it is a *third* spelling
   of the same idea, alongside inline fractions and inline widths. A
   `Phases::from_durations([...])` constructor would cover it and remove
   the running-total arithmetic above. Deferred to planning because it
   widens the API beyond the single type this design proposes, and
   because `Phases::new` already accepts these values as a `const`
   expression; the question is whether the app should still be doing the
   division itself.
