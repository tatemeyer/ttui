# Phase Arithmetic Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the engine the arithmetic four apps each rebuilt by hand —
one `Phases` type that turns overall progress into `(phase, progress
within phase)` — and migrate all ten call sites onto it without changing
a single rendered pixel.

**Architecture:** Three shipping slices, each its own PR. Slice 1 adds
the type and changes no app. Slice 2 migrates the boot sequences. Slice 3
migrates the non-boot transitions and can be dropped without leaving the
tree inconsistent. Slice 4 is close-out.

**Tech Stack:** Rust (stable 1.91.1, edition 2021), `std::time::Duration`,
`tools/visual-snapshot` for before/after comparison.

**Design:** `docs/design/specs/core/2026-08-19-phase-arithmetic-design.md`

## Global Constraints

- **TDD test-first** for every `coding`-tagged task. No exceptions.
- **Autonomy tier Gated** — each slice's PR needs `build`/`test`/
  `clippy`/`fmt` green.
- **`semver:minor`.** New `pub` item, no existing signature changed.
  `CHANGELOG.md` gains an `Added` entry under `[Unreleased]`.
- **Migration must be invisible.** Every migrated app is captured before
  and after and the frames compared. A migration that changes output is a
  bug in the migration, not an improvement — revert and re-derive.
- **Do not touch the hub/sub-screen shape.** Explicitly out of scope per
  the design; six apps' `Screen` enums stay exactly as they are.

## Resolved from the design's open questions

- **Q1 (batching):** one PR per slice. Slice 2's four app migrations
  share one PR and one visual-review pass, the same shape as the Falcon
  batch (#151) — they are mechanical and each app is captured anyway.
- **Q2 (non-boot call sites):** yes, they migrate — but as Slice 3, so
  they can be deferred without stranding Slice 2.
- **Q3 (exposing ends):** not now. No call site needs it and adding it
  later is additive.

---

## Slice 1 — The `Phases` type (`coding`)

Adds the API. Changes no application code, so nothing can regress
visually.

### Task 1: `Phases::new` and `at`

**Files:** `src/transition.rs`

**Interfaces:**
```rust
pub struct Phases<const N: usize> { ends: [f32; N] }

impl<const N: usize> Phases<N> {
    pub const fn new(ends: [f32; N]) -> Self;
    pub fn at(&self, progress: f32) -> (usize, f32);
}
```

- [ ] **Step 1:** Write the failing tests first. `at` on
      `Phases::new([0.1, 0.4, 0.85, 1.0])` must give: `0.0 -> (0, 0.0)`;
      `0.05 -> (0, 0.5)`; `0.1 -> (1, 0.0)` (a boundary belongs to the
      *later* phase, matching today's `if progress < 0.1` tests);
      `0.25 -> (1, 0.5)`; `0.4 -> (2, 0.0)`; `1.0 -> (3, 1.0)`.
- [ ] **Step 2:** Add saturation tests: `-1.0 -> (0, 0.0)` and
      `2.0 -> (N - 1, 1.0)`. No panics on any finite input.
- [ ] **Step 3:** Add a `Phases<1>` test — `new([1.0])` is one phase
      spanning the whole range, `at(0.5) -> (0, 0.5)`.
- [ ] **Step 4:** Implement. `at` is a linear scan over `ends` (N is
      tiny; a binary search would be premature). `t` is always clamped.
- [ ] **Step 5:** `#![warn(missing_docs)]` is on — every `pub` item gets
      a single-line `///` per `development-conventions.md`.

### Task 2: `Phases::from_durations`

**Files:** `src/transition.rs`

**Interfaces:**
```rust
pub const fn from_durations(durations: [Duration; N]) -> Self;
```

- [ ] **Step 1:** Failing test first: `from_durations([200ms, 800ms,
      600ms, 500ms])` must produce ends `[0.0952…, 0.4762…, 0.7619…,
      1.0]` (each cumulative sum over the 2100ms total), asserted with an
      epsilon.
- [ ] **Step 2:** Add a test that it agrees with `new` — equal durations
      give evenly spaced ends, e.g. four equal durations produce
      `[0.25, 0.5, 0.75, 1.0]`.
- [ ] **Step 3:** Add a test that the last end is exactly `1.0`, not
      `0.9999…`, so the final phase is reachable.
- [ ] **Step 4:** Implement as `const fn`: sum `as_nanos()` into a
      `u128`, then a second pass writing `acc as f32 / total as f32`.
      This shape is already verified to compile and evaluate in a `const`
      item on stable 1.91.1.
- [ ] **Step 5:** Decide and document the zero-total case (all durations
      zero). Simplest defensible answer: every end is `1.0`, so `at`
      reports the final phase — a zero-length sequence is already over.
      Add a test pinning whatever is chosen.

### Task 3: Document and record

**Files:** `src/transition.rs`, `CHANGELOG.md`

- [ ] **Step 1:** Module-level `//!` note on `transition.rs` covering why
      `Phases` lives beside `Transition` — they are used together as
      `PHASES.at(t.progress())`.
- [ ] **Step 2:** `CHANGELOG.md` `Added` entry under `[Unreleased]` for
      `transition::Phases`, noting both constructors.
- [ ] **Step 3:** Four gates green; open the Slice 1 PR.

---

## Slice 2 — Migrate the boot sequences (`coding`)

Seven call sites across four apps. Each is a pure refactor.

### Task 4: `falcon`

**Files:** `examples/falcon/boot.rs`, `examples/falcon/falcon.rs`

- [ ] **Step 1:** Capture `falcon-glitch-burst` before the change and
      record per-frame non-black percentages as the baseline.
- [ ] **Step 2:** Declare `const BOOT: Phases<4> = Phases::new([0.1, 0.4,
      0.85, 1.0]);` beside the other constants and rewrite `render_boot`'s
      three hand-computed locals to use `BOOT.at(progress)`.
- [ ] **Step 3:** Keep `BOOT_FADE_FLOOR` exactly as-is — it is #117's fix
      and is about the *value* of the fade, not how its `t` is derived.
- [ ] **Step 4:** Re-capture and compare against Step 1's baseline.
      Any per-frame difference beyond the app's own animation noise is a
      migration bug.

### Task 5: `omnitrix`

**Files:** `examples/omnitrix/boot.rs`

- [ ] **Step 1:** Baseline capture of `omnitrix-dial-rotate`. Frame 0 is
      the known-black pre-script frame (#139) — expected, not a defect.
- [ ] **Step 2:** Migrate both sites. Note the first is inverted
      (`1.0 - progress / 0.4`), so it becomes `1.0 - t` for phase 0 —
      the inversion is the app's, not the helper's.
- [ ] **Step 3:** Re-capture and compare. `omnitrix` has been
      pixel-stable across this whole session, so any difference is real.

### Task 6: `tardis`

**Files:** `examples/tardis/boot.rs`

- [ ] **Step 1:** Baseline capture of `tardis-console-idle`.
- [ ] **Step 2:** Migrate `boot.rs:104`'s `push_progress`.
- [ ] **Step 3:** Re-capture and compare.

### Task 7: `smash_crabs` — the duration case

**Files:** `examples/smash_crabs/smash_crabs.rs`

**Interfaces:** this is `from_durations`' motivating call site.

- [ ] **Step 1:** `smash_crabs` has no `.plumb` scenario. Either author
      one first or capture it directly with `visual-snapshot`, and record
      the baseline. Read `.plumb/SCENARIOS.md` before writing any script:
      `BOOT_TOTAL_MS` is 2100, so the first wait must clear it.
- [ ] **Step 2:** Replace the runtime `t1`/`t2` division with
      `const BOOT: Phases<4> = Phases::from_durations([...])` built from
      the existing `BOOT_FLASH_MS`/`BOOT_CLAW_MS`/`BOOT_TITLE_MS`/
      `BOOT_FLARE_MS` constants.
- [ ] **Step 3:** `BOOT_TOTAL_MS` may become unused once the division is
      gone — if so remove it, since `from_durations` derives the total.
      Check for other readers first.
- [ ] **Step 4:** Re-capture and compare.

### Task 8: Ship Slice 2

- [ ] **Step 1:** Four gates green.
- [ ] **Step 2:** PR recording, per app, the before/after per-frame
      measurements and which frames were read directly. Per
      `development-conventions.md` this is rendering-affecting work and
      the captures must be *read*, not just measured.

---

## Slice 3 — Migrate the non-boot transitions (`coding`)

Four sites using the same shape for screen transitions and effects.
Independently droppable.

### Task 9: `omnitrix.rs:262`, `smash_crabs.rs:353`, `tardis.rs:284` and `:316`

**Files:** `examples/omnitrix/omnitrix.rs`, `examples/smash_crabs/smash_crabs.rs`, `examples/tardis/tardis.rs`

- [ ] **Step 1:** For each site, identify what the surrounding sequence's
      phases actually are — these are mid-app transitions, so some may be
      a single phase with an offset rather than a genuine multi-phase
      sequence. **A site that is not really phased should be left alone**
      and the reason recorded; forcing `Phases` onto a lone
      `(p - a) / b` would be worse than the status quo.
- [ ] **Step 2:** Migrate the ones that are genuinely phased, each with a
      before/after capture of its app.
- [ ] **Step 3:** Four gates green; PR with the same evidence shape as
      Slice 2.

---

## Slice 4 — Close out

### Task 10: Verify the whole Arc

- [ ] **Step 1:** Confirm every remaining `(progress - N) / M` expression
      in `examples/` and `showcase/` is either migrated or explicitly
      justified in Task 9 Step 1. `grep -rn "progress - 0\.\|progress /
      0\."` should come back empty or fully accounted for.
- [ ] **Step 2:** Run all five `.plumb` scenarios and read every contact
      sheet. Exit code 0 is not evidence.
- [ ] **Step 3:** Update `docs/design/README.md` if this Arc warrants a
      line, and mark the design's open questions resolved.
- [ ] **Step 4:** Decide whether v1.1.0 is cut here or accumulates more
      Arcs first — `Phases` alone is a thin release, and the brainstorm
      framed v1.1 as "the engine gets better at what it repeatedly does",
      which may want a second Arc before tagging.
