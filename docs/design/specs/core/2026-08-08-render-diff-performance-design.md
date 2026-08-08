# Render Diff Performance — Design

**Status:** draft, pending your review before we move to planning.
**Date:** 2026-08-08
**Relationship to prior specs:** closes the one concrete performance
risk that `docs/design/specs/core/2026-08-05-ttui-rev-b-vision-alignment-design.md`
("Validation plan") explicitly left open — that `Terminal::draw_diff`'s
per-cell `execute!` pattern "must be measured under real animated load,
resolving the open risk … with actual numbers instead of assumption."
Rev A/Rev B and their `draw_diff` guarantees are otherwise unchanged;
this is a behavior-preserving optimization plus the measurement harness
Rev B asked for.

## Context / Motivation

`src/terminal.rs::draw_diff` (as shipped) does, for every changed cell:

```rust
for d in diffs {
    execute!(self.out,
        cursor::MoveTo(d.x, d.y),
        SetAttribute(Attribute::Reset),
        SetAttribute(attr),
        SetForegroundColor(d.cell.fg),
        SetBackgroundColor(d.cell.bg),
        Print(d.cell.symbol))?;
}
self.out.flush()
```

Three costs compound here, and Rev B flagged the first as an unmeasured
risk:

1. **`execute!` flushes on every call.** `crossterm::execute!` = queue +
   *immediate flush*, so N changed cells means N `write`/flush syscalls
   per frame (the trailing `self.out.flush()` is then redundant). Under
   an animated tick load (Omnitrix's breathing border, Smash Crabs' hit
   flash) this is the per-frame syscall count Rev B wanted quantified.
2. **Full SGR re-emission per cell.** `MoveTo`, a full attribute reset,
   and both colors are re-sent for every cell even when consecutive
   cells are contiguous and share styling — the common case for a
   redrawn region.
3. **No output buffering.** `self.out` is a bare `Stdout`; even with
   `queue!` each `write!` still hits the OS.

Note the subtlety that forces (2)'s waste: `SetAttribute(Attribute::
Reset)` is ANSI SGR 0, which resets **colors too**, so the code *has*
to re-send `fg`/`bg` after it every cell. Fixing (2) therefore requires
changing how "not bold" is expressed.

## Scope of this spec

1. **Extract a writer-generic encoder** `render_diff(writer: &mut impl
   Write, diffs: &[CellDiff])` that `draw_diff` calls with its buffered
   stdout. This makes the exact byte output assertable in unit tests
   and benchmarkable off a real TTY (against `Vec<u8>`/`io::sink()`) —
   the enabling refactor for everything below.
2. **Coalesce redundant output** inside `render_diff`: one `MoveTo` per
   contiguous run, and `fg`/`bg`/intensity emitted only when they
   change from the previously-emitted cell.
3. **Buffer stdout** (`BufWriter<Stdout>`) and flush once per frame in
   `draw_diff`, replacing the per-cell `execute!` flushes with `queue!`.
4. **A `criterion` benchmark** (`benches/render.rs`) measuring
   `render_diff` under representative diff profiles, with the old
   per-cell/full-SGR encoder inlined as a baseline so the before/after
   ratio is the actual number Rev B asked for.

Explicitly **out of scope:** changing `draw_diff`'s public signature,
changing the `diff()` algorithm or `CellDiff` shape, wide-grapheme /
double-width cell handling (ttui is single-width today), and any new
`CellStyle` attributes beyond the existing `bold`.

## Design

### 1. `render_diff` — the writer-generic encoder

```rust
/// Encodes `diffs` as terminal control sequences into `writer`,
/// coalescing redundant cursor moves and SGR changes. Does not flush;
/// `draw_diff` wraps this with a buffered stdout and one flush.
pub fn render_diff(writer: &mut impl Write, diffs: &[CellDiff])
    -> std::io::Result<()>
```

This is an **additive** public function — no existing signature
changes, so "public API unchanged" holds for every current caller. It
is `pub` (not private) specifically so `benches/render.rs`, a separate
crate, can measure it; it is documented as the lower-level primitive
that `draw_diff` builds on. Uses `queue!` throughout, never `execute!`,
and never flushes.

`draw_diff` becomes a thin wrapper:

```rust
pub fn draw_diff(&mut self, diffs: &[CellDiff]) -> std::io::Result<()> {
    render_diff(&mut self.out, diffs)?;
    self.out.flush()
}
```

### 2. Coalescing rules

`render_diff` walks `diffs` in order (`diff()` already emits row-major,
so same-row neighbors are adjacent) tracking previously-emitted state:

- **Cursor:** emit `MoveTo(d.x, d.y)` unless this diff is immediately to
  the right of the previous one on the same row (`d.y == prev.y &&
  d.x == prev.x + 1`) — after `Print`, the cursor already sits there.
  A run can never extend past the last column (the next cell would be a
  new row), so terminal autowrap is never relied upon.
- **Foreground:** track `last_fg: Option<Color>`; emit
  `SetForegroundColor(d.cell.fg)` only when it differs from `last_fg`.
- **Background:** symmetric, `last_bg: Option<Color>`.
- **Intensity:** track `last_bold: Option<bool>`; on change emit
  `SetAttribute(Bold)` (bold on) or `SetAttribute(NormalIntensity)`
  (bold off). `NormalIntensity` (SGR 22) clears bold **without**
  resetting colors — this is what lets `fg`/`bg` be tracked
  independently, unlike the old `Attribute::Reset`.
- **Glyph:** always `Print(d.cell.symbol)`.

Because every field is `Option`-initialised to `None`, the **first**
emitted cell sends `fg`, `bg`, and an explicit intensity attribute,
establishing a fully-known SGR state with no dependence on whatever the
terminal's prior state was. Each frame's `render_diff` starts fresh, so
the first changed cell of every frame re-establishes state — no styling
bleeds between frames, matching the shipped behavior.

**Behavior preservation:** every cell still ends up rendered with
exactly its `fg`/`bg`/`bold`; only redundant control bytes are removed.
`CellStyle` has only `bold`, and nothing in the crate emits
italic/underline, so dropping the blanket SGR-0 reset loses nothing.

### 3. Buffered stdout

`Terminal.out` changes from `Stdout` to `BufWriter<Stdout>`
(constructed in `new()`, used unchanged by the `execute!` calls in
`new()`/`Drop`, which flush themselves). This turns the many small
`queue!` writes into few OS writes, flushed once per `draw_diff`.

### 4. Benchmark

`benches/render.rs` (criterion, `harness = false`) builds a source
`Buffer` and several `next` buffers producing representative diff sets —
e.g. a full-screen repaint, a sparse scattered update, and a
dense-contiguous-region update — then benches `render_diff` into a
reused `Vec<u8>` for each. A private `render_diff_naive` (the old
per-cell `MoveTo`+SGR-0+colors approach, writing to the same `Vec<u8>`)
is benched on the identical diff sets so the report shows the
before/after ratio directly. This is the "actual numbers" deliverable.

## Testing

Per `.claude/rules/development-conventions.md`:

- **`render_diff` and coalescing are `coding`-tagged; TDD required.**
  Because `render_diff` writes to `&mut impl Write`, its output is
  fully testable against a `Vec<u8>` with no real TTY — so this is
  *not* covered by the real-TTY exception. Write tests first:
  - empty diffs → no output;
  - one diff → exactly one `MoveTo`, one `fg`, one `bg`, one intensity,
    the glyph;
  - two contiguous same-styled diffs → exactly **one** `MoveTo` and
    **one** each of `fg`/`bg`/intensity, two glyphs;
  - a positional gap or row change → a second `MoveTo`;
  - a color change mid-run → `fg` (or `bg`) re-emitted;
  - bold toggled on then off → `Bold` then `NormalIntensity` emitted.
  Assertions count occurrences of the specific CSI byte sequences in
  the captured `Vec<u8>`, not exact whole-stream equality (robust to
  ordering of the color/attr triple).
- **`draw_diff`'s stdout wiring stays under the real-TTY exception** —
  the two existing `#[ignore]`d terminal tests are unchanged; the
  buffering change is verified manually per the real-TTY policy (run
  an example and confirm no visual regression).
- **The benchmark is not a test** — it does not run under `cargo test`
  and adds no assertion; correctness is the unit tests above, the
  benchmark only produces numbers.

## Critical files

- `src/terminal.rs` — add `pub fn render_diff`, rewrite `draw_diff` as a
  wrapper, change `out` to `BufWriter<Stdout>`, add the coalescing unit
  tests inline.
- `Cargo.toml` — add `criterion` dev-dependency and the `[[bench]]`
  entry.
- `benches/render.rs` — new benchmark harness with the naive baseline.

## Verification

- `cargo test` — full suite green, including the new `render_diff`
  coalescing tests.
- `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
  clean (the `missing_docs` gate is satisfied by `render_diff`'s `///`).
- `cargo build --examples` — all example apps still compile against the
  unchanged `draw_diff` signature.
- `cargo bench` — runs, and the report shows `render_diff` doing
  meaningfully less work than the inlined naive baseline on the
  contiguous/shared-style profiles (the concrete number Rev B's
  validation plan required).
- Manual (real-TTY exception): `cargo run --example omnitrix` — the
  breathing border animates with no visual regression and input still
  feels instant, confirming the buffering/coalescing change is
  transparent to rendered output.
