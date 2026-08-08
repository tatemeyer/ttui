# Render Diff Performance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:test-driven-development for the `coding` tasks and superpowers:executing-plans to drive the plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Implement `docs/design/specs/core/2026-08-08-render-diff-performance-design.md`:
extract a writer-generic `render_diff`, coalesce redundant cursor/SGR
output, buffer stdout, and add a `criterion` benchmark that quantifies
the improvement — closing Rev B's open `draw_diff` performance risk with
measured numbers.

**Architecture:** One coding Slice (the `render_diff` extraction +
coalescing, TDD) followed by one wiring Slice (buffered stdout, no new
logic) and one measurement Slice (the benchmark, an examples/demo-style
TDD exception). The public `draw_diff` signature is unchanged; every
example app keeps compiling untouched.

**Tech Stack:** Rust, `crossterm` 0.27 (`queue!`, `Attribute::
NormalIntensity`), `criterion` 0.5 (dev-dependency, `harness = false`).

## Global Constraints

- `draw_diff`'s public signature does not change — `render_diff` is
  purely additive.
- Behavior-preserving: each cell still renders with its exact
  `fg`/`bg`/`bold`; only redundant control bytes are removed. The
  coalescing tests are the regression guard.
- TDD for the `render_diff`/coalescing task (writer-generic → testable
  off a real TTY, so the real-TTY exception does **not** apply to it).
- Never use `execute!` inside `render_diff`; `queue!` only, no flush.

---

## Slice 1: `render_diff` extraction + coalescing (`coding`, TDD)

### Task 1: Extract `render_diff` and coalesce cursor/SGR output

**Files:**
- Modify: `src/terminal.rs`

`coding`-tagged, **TDD required** (Gated tier). Write the tests first,
watch them fail, then implement.

- [x] **Step 1 (RED): Add inline `#[cfg(test)]` tests** in
  `src/terminal.rs` that call `render_diff(&mut Vec::<u8>::new(), …)`
  and assert on the captured bytes by counting CSI sequences:
  - empty diffs → zero bytes written;
  - one diff → one `MoveTo` (`H`-terminated CSI), one
    `SetForegroundColor`, one `SetBackgroundColor`, one intensity
    attribute, and the glyph present;
  - two contiguous same-row same-styled diffs → exactly **one**
    `MoveTo` and one each of fg/bg/intensity, both glyphs present;
  - a diff with an x-gap or a new row → a second `MoveTo`;
  - a mid-run `fg` change → `SetForegroundColor` emitted twice;
  - bold off→on→off → `Bold` then `NormalIntensity` present.
  These must not compile/pass yet (no `render_diff`).
- [x] **Step 2 (GREEN): Implement `pub fn render_diff(writer: &mut impl
  Write, diffs: &[CellDiff]) -> std::io::Result<()>`** using `queue!`
  and `Option`-tracked `last_fg`/`last_bg`/`last_bold`, per the spec's
  coalescing rules; `NormalIntensity` for bold-off so colors aren't
  clobbered. Give it a `///` doc line (satisfies `missing_docs`). Make
  all Step-1 tests pass.
- [x] **Step 3 (REFACTOR): Rewrite `draw_diff`** to
  `render_diff(&mut self.out, diffs)?; self.out.flush()`. Confirm the
  existing example apps still type-check against the unchanged
  signature.

---

## Slice 2: Buffer stdout (`coding`, real-TTY exception)

### Task 2: Wrap `Terminal.out` in `BufWriter<Stdout>`

**Files:**
- Modify: `src/terminal.rs`

`coding`-tagged but **real-TTY exception applies** — this is terminal
I/O wiring with no off-TTY-assertable logic of its own (the byte
generation is already covered by Slice 1's tests). Verified manually
per the real-TTY policy.

- [x] **Step 1: Change `out: Stdout` to `out: BufWriter<Stdout>`,**
  constructing it in `new()` (`BufWriter::new(stdout())`). The
  `execute!` calls in `new()` and `Drop` are unchanged (they flush
  themselves; `BufWriter: Write`). Add the `use std::io::BufWriter`.
- [x] **Step 2: Confirm `draw_diff`'s single `self.out.flush()`** now
  flushes the `BufWriter`, so the many `queue!` writes become few OS
  writes per frame.

---

## Slice 3: Benchmark harness (`research`/`coding`, demo exception)

### Task 3: Add `criterion` benchmark with a naive baseline

**Files:**
- Modify: `Cargo.toml`
- Add: `benches/render.rs`

Benchmark code — the "examples/demos" spirit of the TDD exception
applies (correctness is Slice 1's unit tests; the bench only measures).

- [x] **Step 1: Add to `Cargo.toml`** `criterion = "0.5"` under
  `[dev-dependencies]` and a `[[bench]]` entry
  (`name = "render"`, `harness = false`).
- [x] **Step 2: Write `benches/render.rs`** that builds a source
  `Buffer` and `next` buffers for three diff profiles (full-frame
  repaint, sparse scatter, dense contiguous region) via
  `ttui::buffer::diff`, and benches `ttui::terminal::render_diff` into a
  reused `Vec<u8>` for each profile.
- [x] **Step 3: Add a private `render_diff_naive`** in the bench file
  (old per-cell `MoveTo` + SGR-0 + colors, writing to `Vec<u8>`) and
  bench it on the identical diff sets, so the report shows the
  before/after ratio.

---

## Verification (whole plan)

- [x] `cargo test` green, including the new `render_diff` coalescing
  tests (the two `#[ignore]`d real-TTY tests unchanged).
- [x] `cargo clippy --all-targets -- -D warnings` and
  `cargo fmt --check` clean.
- [x] `cargo build --examples` — all example apps compile against the
  unchanged `draw_diff`.
- [x] `cargo bench` runs and shows `render_diff` doing meaningfully less
  work than `render_diff_naive` on the contiguous/shared-style
  profiles.
- [ ] Manual (real-TTY): `cargo run --example omnitrix` — breathing
  border animates with no visual regression, input still instant.
  **Pending:** the CI/dev environment is headless (no TTY), so this must
  be run locally before merge and the result noted in the PR's
  Verification section, per `development-conventions.md`'s real-TTY
  policy.
