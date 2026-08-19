# Shared Utilities Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the engine the three utilities its apps — and in one case
the library itself — each rebuilt by hand, remove all ten duplicate
definitions, and cut v1.1.0.

**Architecture:** Four slices, each its own PR. Slice 1 adds the
primitives and changes no consumer. Slice 2 migrates `src/`, which is the
riskiest step and carries the one bug a capture cannot catch. Slice 3
migrates the examples and `showcase`. Slice 4 sweeps, closes out, and
cuts the release.

**Tech Stack:** Rust (stable 1.91.1, edition 2021), `tools/visual-snapshot`.

**Design:** `docs/design/specs/core/2026-08-19-shared-utilities-design.md`

## Global Constraints

- **TDD test-first** for every `coding`-tagged task.
- **Autonomy tier Gated** — four checks green per PR.
- **`semver:minor`.** Three new `pub` items, no existing signature
  changed. `CHANGELOG.md` gains `Added` entries.
- **Migration must be invisible.** Every app touched is captured before
  and after. A migration that changes output is a bug in the migration —
  revert and re-derive, never adjust a baseline.
- **Do not fix #161 here.** `Buffer::set`'s inaccurate panic contract is
  filed and is its own decision. This Arc closes the *exposure* by giving
  `Buffer::blit` real clipping; it does not touch `set`.

## Resolved from the design's open questions

- **Q1 (batching):** one PR per slice; apps batched within a slice, as
  Arc 1 did.
- **Q2 (`showcase` captures):** `showcase` is a `[[bin]]`, so
  `visual-snapshot --bin showcase` captures it directly — the same route
  used to verify #131. Its Telemetry vignette must be reached, since
  that is where its `scatter` copy lives.
- **Q3 (`noise` module):** keep `src/noise.rs`. One function is thin, but
  `easing` is the wrong shelf and a clear home costs nothing.

---

## Slice 1 — The three primitives (`coding`)

Adds API only. No consumer changes, so nothing can regress.

### Task 1: `easing::scale_color`

**Files:** `src/easing.rs`

**Interfaces:**
```rust
pub fn scale_color(c: Color, factor: f32) -> Color;
```

- [ ] **Step 1:** Failing tests first — `factor = 1.0` returns the colour
      unchanged; `0.0` returns black; `0.5` halves each channel; a
      non-`Rgb` colour passes through untouched at every factor.
- [ ] **Step 2:** Add a test that `factor` is clamped, or that
      out-of-range values saturate rather than wrapping a `u8`. Decide
      which and pin it — `(300.0 * 2.0) as u8` saturating is a language
      detail worth asserting rather than assuming.
- [ ] **Step 3:** Implement. Multiply semantics.
- [ ] **Step 4:** Doc comment states the convention explicitly (`1.0`
      unchanged, `0.0` black) and why non-`Rgb` passes through — you
      cannot scale a colour whose RGB the terminal has not disclosed.
      Cross-reference `lerp_color`'s different policy so the two read as
      deliberate rather than inconsistent.

### Task 2: `noise::scatter`

**Files:** `src/noise.rs` (new), `src/lib.rs`

**Interfaces:**
```rust
pub fn scatter(seed: u32, spread: f32) -> f32;
```

- [ ] **Step 1:** Failing tests first — determinism (same seed, same
      value, twice); range (`|result| <= spread / 2.0` across many
      seeds); and that distinct seeds give distinct values for a decent
      sample.
- [ ] **Step 2:** Implement, preserving the *exact* existing body and its
      constants. This is a move, not a rewrite: changing the hash would
      change every star position in four apps.
- [ ] **Step 3:** Register the module in `src/lib.rs` with a `//!` header
      per `development-conventions.md`.

### Task 3: `Buffer::blit`

**Files:** `src/buffer.rs`

**Interfaces:**
```rust
pub fn blit(&self, dest: &mut Buffer, x: u16, y: u16);
```

- [ ] **Step 1:** Failing tests first — a full copy at `(0, 0)`; an
      offset copy; and **clipping**: a source larger than the remaining
      destination space must write only what fits and must not panic or
      wrap onto the next row.
- [ ] **Step 2:** Add the test that would have caught #161's shape —
      blitting at an `x` that overflows the destination width must leave
      the next row untouched.
- [ ] **Step 3:** Implement with an explicit bound. Mirror
      `Canvas::blit`'s argument order deliberately, and say so in the doc
      comment.

### Task 4: Record

- [ ] **Step 1:** `CHANGELOG.md` `Added` entries for all three.
- [ ] **Step 2:** Four gates green; open the Slice 1 PR.

---

## Slice 2 — Migrate the library (`coding`)

Two private duplicates inside `src/`. **This slice carries the one error
a capture cannot catch**, so it is separated from the example migrations.

### Task 5: `camera::dim` delegates without flipping

**Files:** `src/camera.rs`

- [ ] **Step 1:** **Before touching the body**, add a test pinning
      `dim`'s existing inverted convention: `dim(&buf, 0.0)` leaves
      colours unchanged and `dim(&buf, 1.0)` produces black. Run it
      against the current implementation and watch it pass — it is a
      characterisation test, and it is worthless if written afterwards.
- [ ] **Step 2:** Delete the private `scale_color` and have `dim` call
      `easing::scale_color(c, 1.0 - factor)`.
- [ ] **Step 3:** The pinning test must still pass. **A sign error here
      is invisible to visual review** — `dim` is used inside boot fades
      where the screen is dark anyway — so this test is the only thing
      standing between a flipped convention and a silent regression.
- [ ] **Step 4:** Doc comment on `dim` names the inversion explicitly:
      its `factor` is *how much to dim*, the opposite of
      `scale_color`'s. Reference #139, where the ambiguity already
      produced a wrong analysis.

### Task 6: `roundel` uses the shared primitive

**Files:** `src/widgets/roundel.rs`

- [ ] **Step 1:** Delete the private `scale_color`; call
      `easing::scale_color`. Same convention, so no call-site change.
- [ ] **Step 2:** `roundel` is a widget and rendering-affecting.
      **`tardis` is its only consumer** — `hub.rs`, `star_charts.rs` and
      `artron_energy.rs` all render one. Capture before and after.
      Note that `tardis-console-idle` settles on the Psychic Paper
      screen, which may not draw a `Roundel` at all: **check which
      captured frame actually contains one** before claiming the
      scenario covers this. If none does, drive `tardis` to its hub with
      a direct capture instead, and say which route was used.

### Task 7: Ship Slice 2

- [ ] **Step 1:** Four gates green.
- [ ] **Step 2:** PR stating explicitly that `camera::dim`'s public
      behaviour is unchanged, and pointing at the characterisation test
      as the evidence.

---

## Slice 3 — Migrate the examples (`coding`)

Eight duplicate definitions across six files. Pure refactor.

### Task 8: `scatter` x4

**Files:** `examples/depth_spike.rs`, `examples/falcon/falcon.rs`, `examples/mission_control.rs`, `showcase/telemetry.rs`

- [ ] **Step 1:** Baseline captures first: `falcon` and
      `mission_control` have `.plumb` scenarios; `depth_spike` needs a
      direct capture; `showcase` needs `--bin showcase` driven to the
      Telemetry vignette.
- [ ] **Step 2:** Delete all four copies, import `ttui::noise::scatter`.
- [ ] **Step 3:** Re-capture and compare. `scatter` places stars and
      jitter, so **any** change here is a real change — the hash must be
      byte-identical, and these captures are what prove it.

### Task 9: `blit` x3

**Files:** `examples/omnitrix/omnitrix.rs`, `examples/smash_crabs/smash_crabs.rs`, `examples/tardis/tardis.rs`, plus the `boot.rs` callers in each

- [ ] **Step 1:** Baseline captures, including the boot-focused scripts
      Arc 1 built — both `boot.rs` files call their app's local copy, so
      the `.plumb` scenarios alone do not cover this.
- [ ] **Step 2:** Delete the three copies; call sites become
      `scratch.blit(buf, area.x, area.y)`.
- [ ] **Step 3:** Re-capture and compare.

### Task 10: `launcher::dim_color`

**Files:** `examples/launcher/main.rs`, `examples/launcher/nexus.rs`, `examples/launcher/portal.rs`

- [ ] **Step 1:** Delete `dim_color`; call `easing::scale_color`. Same
      convention, so call sites are unchanged apart from the path.
- [ ] **Step 2:** **`launcher` cannot be captured** — its portal and
      starfield glyphs hit the rasterizer's known unmapped-glyph gap,
      which `development-conventions.md` covers explicitly. Reason from
      the code instead, as that rule directs, and record it in the PR
      rather than quietly skipping the review.

### Task 11: Ship Slice 3

- [ ] **Step 1:** Four gates green.
- [ ] **Step 2:** PR recording per-app before/after measurements, which
      frames were read directly, and the `launcher` exception.

---

## Slice 4 — Close out and cut v1.1.0

### Task 12: Verify the Arc

- [ ] **Step 1:** Sweep: `grep -rn "fn scatter\|fn blit\|fn dim_color\|fn scale_color" src/ examples/ showcase/` should return only the three new definitions plus `Canvas::blit` and `camera::dim`. Anything else is a missed copy.
- [ ] **Step 2:** Run all five `.plumb` scenarios and read every contact
      sheet. Exit code 0 is not evidence.
- [ ] **Step 3:** Mark the design's open questions resolved.

### Task 13: Cut v1.1.0

**Files:** `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`

- [ ] **Step 1:** Confirm the release is warranted. Both Arcs are
      additive-only: four new `pub` items (`Phases`, `scale_color`,
      `scatter`, `Buffer::blit`) and no signature changed. `[Unreleased]`
      also carries two behaviour fixes — `app::run` checking
      `should_quit` after `on_tick` (#30), and `lerp_color` respecting
      `t` for non-`Rgb` pairs (#122). New public API makes this a minor
      bump; neither fix changes a signature, so it is not a major.
- [ ] **Step 2:** Bump `version` to `1.1.0`; sync `Cargo.lock`.
- [ ] **Step 3:** Convert `CHANGELOG.md`'s `[Unreleased]` into a
      `## [1.1.0] - <date>` section. Check every entry accumulated since
      v1.0.0 is present and correctly categorised.
- [ ] **Step 4:** `cargo publish --dry-run` to catch packaging problems
      before the irreversible step.
- [ ] **Step 5:** **Human-tier — stop here and hand over.** Tagging
      `v1.1.0`, pushing the tag, cutting the GitHub release and running
      `cargo publish` are the user's to run: publishing to crates.io is
      irreversible and needs their credentials. Report the exact commands
      rather than running them.
