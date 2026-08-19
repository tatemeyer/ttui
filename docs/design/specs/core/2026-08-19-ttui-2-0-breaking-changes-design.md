# TTUI 2.0 — Breaking Changes Design

**Status:** approved, ready for planning.

## Goal

Fix `Table`'s single-`col_width` limitation (#170), and use the major
version it requires to land every other breaking change worth making —
so the next one is years away rather than weeks.

## What drove this

`ttui` is published, and `parallax-panopticon` — the Parallax cockpit —
is the first thing outside this repo to depend on it as a crate. #170 is
the first bug that consumer found.

`Table::new(headers, rows, selected, col_width)` takes one width for
every column, and `render_row` truncates every cell to it. The cockpit
wants a work-pane row like:

```
#165  open   agent  on-checks  verifiable  3/0/1  tardis-console-idle claims…
```

Six narrow columns and one wide one. At `col_width: 12` the title is cut
to `tardis-conso`; at `col_width: 90` the number column is 90 cells wide
and one row fills the screen. There is no value that renders this.

The fix requires changing `Table::new`'s signature, which is a major
bump under `code-forge.md`. Rather than pay that for one widget, 2.0.0
becomes a deliberate release train: **batch the breaking changes now,
then iterate additively in 2.x.**

## Scope

Seven slices. Two carry real design (Table's column model, and the #161
decision); the rest are mechanical.

| # | slice | breaking? | design weight |
|---|---|---|---|
| 1 | `#[non_exhaustive]` on four public enums | yes | trivial |
| 2 | Retire the `blend` module | yes | trivial |
| 3 | `Buffer::set` bounds contract (#161) | yes, if Option 1 | real |
| 4 | Themeable selection highlight (`List`/`Dial`/`Table`) | no | light |
| 5 | `visual-snapshot`: map U+2026 | no (tooling) | trivial |
| 6 | **Table column model (#170)** | yes | **real** |
| 7 | Cut 2.0.0 | — | — |

### Ordering, and why it is not by size

- **Slice 1 first.** `#[non_exhaustive]` must land before anything that
  might want a new `Constraint` variant — including Slice 6.
- **Slice 4 before Slice 6.** Slice 4 puts `.theme()` on `Table`. Running
  it after the column work would mean re-opening `table.rs` and
  rewriting its tests twice.
- **Slice 5 before Slice 6.** Slice 6 introduces an ellipsis, and
  without Slice 5 every capture of a truncated table hard-errors — see
  "The ellipsis breaks the capture pipeline" below.
- Slices 2 and 3 are independent and can slot anywhere before 7.

## Slice 1 — `#[non_exhaustive]` on four public enums

`Intensity`, `CanvasMode`, `Direction`, `Constraint`. One attribute
each.

This is the highest leverage-to-effort item in the release, and 2.0.0 is
the **only** moment it is cheap. Under `code-forge.md`, adding a variant
to an exhaustive public enum is a breaking change. `Constraint` is the
one that stings: `Max`, `Ratio` and `Length` are all plausible, and each
would currently cost its own major bump.

`#[non_exhaustive]` restricts *exhaustive matching* by downstream
crates; it does not restrict variant construction. Verified there is no
in-repo exhaustive match on any of the four outside `src/` — and
`examples/` are separate crates that link the library, so they would
have been affected if any existed.

**Downstream cost, stated plainly:** any consumer with an exhaustive
`match` on these — possibly `parallax-panopticon` — needs a `_ => {}`
arm. Cheap, but a real edit, and unpleasant to discover mid-migration.

## Slice 2 — Retire the `blend` module

`src/blend.rs`'s own header says it is "Spike-only, and now historical":
the rendering-fidelity spike's recommendation was adopted, and
`LayerStack::composite` now does real Porter-Duff "over" compositing on
`Cell::alpha` rather than the hard-cutout rule the spike measured
against.

Shipping uncommitted spike code as public API is exactly what a major
version should clean up. Delete the module and its `pub mod`; port
`examples/render_spike.rs`, the only consumer, to `LayerStack::composite`.

## Slice 3 — `Buffer::set` bounds contract (#161)

`Buffer::get`/`set` document *"Panics if out of bounds"*. True for `y`,
false for `x`:

```rust
fn index(&self, x: u16, y: u16) -> usize {
    y as usize * self.width as usize + x as usize   // no x < width check
}
```

On a 4x3 buffer, `set(5, 0, ..)` indexes `0 * 4 + 5 = 5`, which is
`(1, 1)`. No panic, and a neighbouring row is silently corrupted.

**Decision: Option 1 (a real bounds check), gated on a benchmark.**

The three filed options were a real check, a `debug_assert!`, or a
documentation correction. A major release is the only time the first is
affordable, because it changes `set`'s observable panic behaviour.

The objection to Option 1 is cost: `set` is on the renderer's hottest
path. That objection is currently an assumption, and will be settled
with a number instead.

**`benches/render.rs` cannot settle it.** Checked rather than assumed:
its three profiles build their `Vec<CellDiff>` *before* `b.iter()`, and
the timed closure runs only `render_diff`/`render_diff_naive` — the ANSI
writer. `Buffer::set` appears solely in that untimed setup, so the
existing benchmark would report "no change" no matter what `index` does.
Treating a flat result from it as evidence would be worse than not
measuring at all.

The procedure is therefore:

1. **Add a `set`-focused benchmark** — a timed loop that fills a
   `Buffer` cell by cell, which is what a paint pass actually does.
   This is a prerequisite task, not a side errand.
2. Record a baseline on `main`.
3. Add the bounds check; re-run.
4. If the difference is in the noise, ship Option 1.
5. If it is measurable, fall back to Option 2 (`debug_assert!`) and
   record the measurement in the PR as the reason.

**Accepted risk:** if the benchmark rejects Option 1, the work is spent
and 2.0.0 ships Option 2 anyway. This is worth it — the bench run is
cheap, and a recorded measurement is worth more to a future reader than
a recorded opinion.

Whichever lands, the doc comment must end up describing what the code
actually does. The present state — a documented panic that does not
happen — is the actual defect.

## Slice 4 — Themeable selection highlight

`List`, `Dial` and `Table` each hardcode the identical highlight and
accept no colours at all:

```rust
let (fg, bg) = if i == self.selected {
    (Color::Black, Color::White)
} else {
    (Color::Reset, Color::Reset)
};
```

In an engine whose stated purpose is themed terminal apps, a selection
highlight that cannot match its app's palette is a real gap — and
`Block` already has `.theme()`, so the crate is inconsistent with itself.

Each gains `.theme(&Theme)`. Selected renders `accent` on `background`;
unselected renders `primary`. Omitting `.theme()` keeps today's colours,
so this slice is additive on its own and only rides the major because
`Table`'s constructor is changing anyway.

Worth noting for its own sake: this is the third time in two Arcs that
three widgets were found to have each grown the same private thing —
the same shape as the ten duplicates removed in the Shared Utilities
Arc. Worth watching for as a recurring pattern rather than treating each
instance as a surprise.

## Slice 5 — `visual-snapshot`: map U+2026

Probed rather than assumed:

```
UNMAPPED U+2026 '…'
MAPPED   U+002E '.'   U+003E '>'   U+007E '~'   U+00B7 '·'   U+00BB '»'
UNMAPPED U+2192 '→'
```

The rasterizer hard-errors on unmapped codepoints. Truncation is
`Table`'s normal state, `development-conventions.md` requires capturing
rendering changes, and the consumer that drove this Arc is table-heavy —
so choosing `…` without this slice would make the one widget being fixed
the one widget that cannot be screenshotted.

Add an 8x8 bitmap for U+2026 to the glyph table. There is precedent: the
rasterizer already generates Braille Patterns algorithmically rather
than from `font8x8`. `tools/visual-snapshot` is internal dev tooling,
explicitly outside the SemVer policy.

The alternative — picking a mapped ASCII marker like `»` — was rejected.
A font limitation in a dev tool should not dictate the library's
typography when closing the gap is this cheap.

## Slice 6 — Table column model (#170)

```rust
Table::new(&headers, &rows, selected)
    .widths(&[Constraint::Fixed(6), Constraint::Fixed(6), Constraint::Fill(1)])
    .spacing(1)
    .theme(&theme)
    .render(area, buf);
```

### Geometry is delegated, not reimplemented

Column rects come from
`Layout::new(Direction::Horizontal, widths).spacing(gap).split(area)` —
one `Rect` per column, each cell rendered into its own. `Table` computes
no positions itself, which is the same rule the rest of the engine
follows.

Three things fall out of reuse rather than being designed:

- `Fill(1)` **is** #170's "one wide column", and it adapts to terminal
  width, which no fixed number can.
- `Layout` already has `spacing()`, so the column gap costs nothing new.
- Callers already know `Constraint` from `Layout`; there is no second
  vocabulary to learn.

**Rejected: a Table-local `ColumnWidth` enum.** Its only real argument
was `Auto` (size to widest cell), which `Constraint` cannot express
today. But Slice 1 makes `Constraint` `#[non_exhaustive]`, so
`Constraint::Auto` can be added later as a *minor* bump. Approach A
forecloses nothing; a parallel enum would be permanent.

**Rejected: plain `&[u16]`.** Simplest, but it does not solve #170 — the
wide column needs an absolute number, so the table stops adapting to
terminal width. It would need a sentinel (`0` means fill?), which is a
worse `Constraint`.

**Inherited wart, accepted:** `Constraint::Min(v)` is documented as
"currently treated as exactly this many — no growth beyond it", which is
the wrong behaviour for a table column. Not fixed here; fixing `Min`
across the whole layout engine is its own Arc.

### Defaults

| builder | omitted behaviour |
|---|---|
| `.widths()` | `Fill(1)` per column, from `headers.len()` — today's equal split |
| `.spacing()` | `0` — today's flush columns |
| `.theme()` | today's `Color::Reset` / Black-on-White |

An untouched call site renders identically apart from the constructor
arity.

### Count mismatch

**`headers.len()` defines the column count.** Today `render_row`
iterates each row's own cells, so a row carrying more cells than there
are headers renders extra columns off the end of the header row. Under
the new model a column without a header has no rect, so those cells are
dropped. Stated explicitly because it is a silent behaviour change that
no signature captures.

When `widths.len() != headers.len()`, render `min(len)` columns and
ignore the rest.

Not a panic: that would be a rendering-time crash in a UI library
triggered by data shape, which is the worst place to be strict.
Not silent padding either: that invents columns the caller did not ask
for. Truncating is visible in the output and cannot take the app down.

### Clipping is explicit, and tested

`Layout::split` does not clamp — `Fixed` constraints exceeding the area
return rects past its right edge:

```rust
offset += size + self.spacing;   // no bound against area.width
```

Writing those unclipped lands in `Buffer::set`, which wraps onto the
next row (#161). `Table` therefore clips each column against its own
area, with a test that an over-wide `Fixed` set leaves the row below
untouched.

This is the identical failure `Buffer::blit` was built to avoid in the
Shared Utilities Arc, where instrumentation showed three hand-rolled
copies were one mis-sized buffer away from smearing a row. Left
implicit, it returns as a rendering artefact with no stack trace.

### Unicode

`cell.chars().nth(i)` re-walks the string per character (O(n²)) and
counts `char`s rather than display width, so CJK and combining marks
misalign every column after them. It becomes a single forward walk
accumulating display width.

**This requires a `unicode-width` dependency — `ttui`'s second ever,
after `crossterm`.** That is the sharpest cost in this design: the crate
loses its single-dependency property. Accepted deliberately, because a
table that misaligns on non-Latin text is not a table. `unicode-width`
is small, stable, and near-universal in this space.

### Ellipsis

Content exceeding its column ends with `…` (U+2026), so a cut cell is
distinguishable from one whose data simply ends. Requires Slice 5.

A column too narrow to hold content plus marker degrades to plain
truncation rather than rendering a lone `…`, which would carry no
information at all.

## Testing

Red-first, per `development-conventions.md`. The tests that matter:

- A `Fixed`+`Fill` mix produces #170's layout — the case that has no
  representable answer today.
- No `.widths()` matches today's equal split byte-for-byte. This is the
  characterisation test for the whole slice; it is worthless written
  afterwards.
- `widths.len() != headers.len()` renders `min(len)` and does not panic,
  in both directions.
- An over-wide `Fixed` set leaves the next row untouched (the #161
  shape).
- A wide-glyph string (CJK) leaves its neighbouring column aligned.
- Ellipsis appears only when content actually exceeds the column; a
  1-cell column degrades to truncation.
- `Buffer::set` out-of-range `x` panics (Slice 3, if Option 1 lands).

Visual review is mandatory for Slices 4 and 6 — both are
rendering-affecting. `examples/demo.rs` is the only in-repo `Table`
consumer and is captured before and after.

**A same-code control run is required** before concluding any capture
difference is real. Two runs of an identical binary produce different
frames; this was established in the Shared Utilities Arc, where a
100%-different frame turned out to be a boot flash landing one frame
later.

## Migration

The whole in-repo cost is `examples/demo.rs` — the only `Table::new`
call site outside `table.rs`'s own tests, and itself slated for
retirement in #83.

```rust
// 1.x
Table::new(&headers, &rows, selected, 12).render(area, buf);

// 2.0
Table::new(&headers, &rows, selected)
    .widths(&[Constraint::Fill(1); 3])
    .render(area, buf);
```

`CHANGELOG.md` gets a **Breaking** entry per slice, and the 2.0.0
section leads with a migration table, since that is what a consumer
reads first.

## Risks

1. **`unicode-width` ends the single-dependency property.** Judged worth
   it; noted because it is the kind of thing that is invisible in a diff
   and obvious in a dependency tree.
2. **The #161 benchmark may reject Option 1**, spending the work to ship
   Option 2. Accepted above.
3. **Downstream `match` breakage from Slice 1** — cheap to fix, annoying
   to discover. Should be called out first in the 2.0.0 release notes,
   not buried.
4. **A second major soon after** would waste this batching. Mitigated by
   Slice 1: with `Constraint` non-exhaustive, the most likely future
   breaking change (a new constraint variant) becomes additive.

## Out of scope

- `Constraint::Auto` — deferred to a later minor; Slice 1 makes that
  possible.
- Fixing `Constraint::Min`'s no-growth behaviour across the layout
  engine — its own Arc.
- Theming the remaining widgets. `Roundel` and friends already take
  colours from the caller; only the three selectable widgets share the
  hardcoded highlight, and only those are in scope.
- Retiring `examples/demo.rs` (#83) — tracked separately, and it is the
  migration's test case here.

## Open questions

None. `Auto` (deferred), the ellipsis character (Slice 5), the width
vocabulary (Approach A), mismatch behaviour (truncate), and the
`unicode-width` dependency (accepted) were all resolved during
brainstorming. #161's option is resolved as a decision procedure — run
the benchmark, take Option 1 unless it measurably hurts — rather than
left open.
