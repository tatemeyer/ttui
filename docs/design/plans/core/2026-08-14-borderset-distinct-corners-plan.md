# BorderSet Distinct Corner Glyphs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give `ttui::theme::BorderSet` 4 distinct corner fields (replacing its single shared `corner: char`), so `Block`/`SmashBorder` can render real box-drawing corners (`┌┐└┘`) instead of one glyph repeated everywhere — resolving GitHub issue #130, the last `v1-blocking` item before the v1.0.0 tag.

**Architecture:** A breaking, additive-shape change to a `pub struct` (`semver:major` per `code-forge.md`). Two named `const fn` constructors (`single_line()`, `ascii()`) minimize the migration burden for the 5 call sites that already use `BorderSet::default()`. The 8 remaining call sites (widgets' own corner-drawing logic, plus example apps with custom glyphs) get mechanical, meaning-preserving updates.

**Tech Stack:** Rust, no new dependencies.

## Global Constraints

- **Full TDD is mandatory for Tasks 1-3** (`src/theme.rs`, `src/widgets/block.rs`, `src/widgets/smash_border.rs`) — this is core library code, no exemption applies.
- **Task 4 (example-app call sites) is TDD-exempt** — demo code verified by building + `tools/visual-snapshot` review, matching every other example-app task in this project's history.
- **`cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` must stay clean** after every task.
- **`tools/visual-snapshot` capture + `Read`-and-review is mandatory** for Task 5, per `development-conventions.md`'s visual-review convention (this touches every themed app's border rendering).
- **Exact field names, glyphs, and existing-value transcriptions below are load-bearing** — copied verbatim from the approved design spec and from direct inspection of each file's current state; don't re-derive them.
- **This resolves issue #130** — close it via the final commit's `Closes #130.` trailer (auto-closes on merge), not a manual `gh issue close`.

---

### Task 1: `BorderSet`'s new shape

**Files:**
- Modify: `src/theme.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces: `BorderSet { horizontal: char, vertical: char, top_left: char, top_right: char, bottom_left: char, bottom_right: char }` (replacing the old `corner: char` field), `BorderSet::single_line() -> Self`, `BorderSet::ascii() -> Self`, `impl Default for BorderSet` delegating to `single_line()` — consumed by Tasks 2-4.

- [ ] **Step 1: Write the failing tests**

In `src/theme.rs`'s existing `#[cfg(test)] mod tests` block, leave the
existing `default_border_set_matches_todays_hardcoded_glyphs` test in
place for now (Step 3 removes it) and add the 3 new preset tests below
it:

```rust
    #[test]
    fn single_line_uses_real_box_drawing_glyphs() {
        let b = BorderSet::single_line();
        assert_eq!(b.horizontal, '─');
        assert_eq!(b.vertical, '│');
        assert_eq!(b.top_left, '┌');
        assert_eq!(b.top_right, '┐');
        assert_eq!(b.bottom_left, '└');
        assert_eq!(b.bottom_right, '┘');
    }

    #[test]
    fn ascii_uses_a_plus_at_every_corner() {
        let b = BorderSet::ascii();
        assert_eq!(b.horizontal, '-');
        assert_eq!(b.vertical, '|');
        assert_eq!(b.top_left, '+');
        assert_eq!(b.top_right, '+');
        assert_eq!(b.bottom_left, '+');
        assert_eq!(b.bottom_right, '+');
    }

    #[test]
    fn default_matches_single_line() {
        assert_eq!(BorderSet::default(), BorderSet::single_line());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib theme::`
Expected: FAIL to compile — `single_line`/`ascii` don't exist yet, and
`BorderSet`'s old `corner` field means the old
`default_border_set_matches_todays_hardcoded_glyphs` test (still
referencing `b.corner`) won't compile either.

- [ ] **Step 3: Remove the old test, update `BorderSet`'s struct and impls**

Delete the old `default_border_set_matches_todays_hardcoded_glyphs`
test entirely (its assertions are superseded by
`ascii_uses_a_plus_at_every_corner` and `single_line_uses_real_box_drawing_glyphs`
above — `BorderSet::default()` no longer matches the old ASCII glyphs,
so the old test's premise is gone).

Replace the `BorderSet` struct and its `Default` impl:

```rust
/// The glyphs a bordered widget draws its edges/corners with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderSet {
    /// Top/bottom edge glyph.
    pub horizontal: char,
    /// Left/right edge glyph.
    pub vertical: char,
    /// Top-left corner glyph.
    pub top_left: char,
    /// Top-right corner glyph.
    pub top_right: char,
    /// Bottom-left corner glyph.
    pub bottom_left: char,
    /// Bottom-right corner glyph.
    pub bottom_right: char,
}

impl BorderSet {
    /// Real box-drawing glyphs (`┌┐└┘─│`) — the default border look.
    pub const fn single_line() -> Self {
        BorderSet {
            horizontal: '─',
            vertical: '│',
            top_left: '┌',
            top_right: '┐',
            bottom_left: '└',
            bottom_right: '┘',
        }
    }

    /// Plain ASCII (`-|+`), the same `+` at every corner — for apps
    /// that want the pre-1.0 look.
    pub const fn ascii() -> Self {
        BorderSet {
            horizontal: '-',
            vertical: '|',
            top_left: '+',
            top_right: '+',
            bottom_left: '+',
            bottom_right: '+',
        }
    }
}

impl Default for BorderSet {
    fn default() -> Self {
        Self::single_line()
    }
}
```

Also update `default_theme_uses_reset_colors_and_default_border`'s
existing assertion (`assert_eq!(t.border, BorderSet::default());`) —
no change needed there, it already compares against
`BorderSet::default()` generically and will pass once the struct
compiles.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib theme::`
Expected: all `theme::` tests pass, including the 3 new ones.

- [ ] **Step 5: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
Expected: both clean. (This will surface every downstream compile
error from `src/widgets/block.rs`, `src/widgets/smash_border.rs`, and
every example app still using the old `corner` field — that's
expected at this point in the plan; Tasks 2-4 fix them. Confirm the
*only* errors are "no field `corner`"/"missing fields" errors in the
files this plan's later tasks will touch, not something unexpected.)

- [ ] **Step 6: Commit**

```bash
git add src/theme.rs
git commit -m "feat(core): give BorderSet 4 distinct corner fields

Replaces the single corner: char (reused at all 4 positions) with
top_left/top_right/bottom_left/bottom_right, plus single_line()/
ascii() presets. BorderSet::default() now returns single_line() (real
box-drawing) instead of the old ASCII '+' — a deliberate behavior
change every app using BorderSet::default() picks up automatically.

Breaking change to ttui's public API surface (#130, semver:major)."
```

---

### Task 2: `Block::render` migration

**Files:**
- Modify: `src/widgets/block.rs`

**Interfaces:**
- Consumes: `BorderSet`'s new fields (Task 1).
- Produces: no new public interface — `Block::render`'s signature is unchanged.

- [ ] **Step 1: Update `draw_ring`'s 4 corner-setting calls**

In `src/widgets/block.rs`, `draw_ring`'s 4 `buf.set` calls for corners
currently all read `symbol: border.corner`. Update each to the field
matching its actual position:

```rust
            buf.set(
                ring_area.x,
                ring_area.y,
                Cell {
                    symbol: border.top_left,
                    ..plain(ring_area.x, ring_area.y)
                },
            );
            buf.set(
                ring_area.x + ring_area.width - 1,
                ring_area.y,
                Cell {
                    symbol: border.top_right,
                    ..plain(ring_area.x + ring_area.width - 1, ring_area.y)
                },
            );
            buf.set(
                ring_area.x,
                ring_area.y + ring_area.height - 1,
                Cell {
                    symbol: border.bottom_left,
                    ..plain(ring_area.x, ring_area.y + ring_area.height - 1)
                },
            );
            buf.set(
                ring_area.x + ring_area.width - 1,
                ring_area.y + ring_area.height - 1,
                Cell {
                    symbol: border.bottom_right,
                    ..plain(
                        ring_area.x + ring_area.width - 1,
                        ring_area.y + ring_area.height - 1,
                    )
                },
            );
```

(Only the 4 `symbol:` values change — everything else in `draw_ring`,
including the horizontal/vertical edge loops above these calls, is
untouched.)

- [ ] **Step 2: Update the 9 existing test literals**

Every test in `src/widgets/block.rs`'s `#[cfg(test)] mod tests` that
constructs a `BorderSet { horizontal: '=', vertical: '#', corner: '*' }`
literal (9 occurrences, all identical in shape) needs updating. Since
every existing test uses `'*'` as its single corner value and only
ever asserts corner `(0, 0)` (or `(0,0)` on the outer ring for the
thick-border test) — **not** all 4 independently — replace each
occurrence's literal with:

```rust
            border: BorderSet {
                horizontal: '=',
                vertical: '#',
                top_left: '*',
                top_right: '*',
                bottom_left: '*',
                bottom_right: '*',
            },
```

This preserves every existing test's assertions unchanged (they only
ever check `buf.get(0, 0).symbol` against `'*'`, which is still
`top_left`).

- [ ] **Step 3: Add a test asserting all 4 corners render independently**

Add a new test to the same module, using 4 visually distinct corner
glyphs so the test can't pass if two corners were accidentally
swapped:

```rust
    #[test]
    fn all_four_corners_render_their_own_distinct_glyph() {
        let theme = Theme {
            background: Color::Black,
            primary: Color::Green,
            secondary: Color::Reset,
            tertiary: Color::Reset,
            accent: Color::Reset,
            primary_end: None,
            border: BorderSet {
                horizontal: '=',
                vertical: '#',
                top_left: '1',
                top_right: '2',
                bottom_left: '3',
                bottom_right: '4',
            },
            border_style: CellStyle::default(),
            border_thick: false,
        };
        let mut buf = Buffer::new(4, 3);
        let area = Rect {
            x: 0,
            y: 0,
            width: 4,
            height: 3,
        };

        Block::new().theme(&theme).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, '1'); // top-left
        assert_eq!(buf.get(3, 0).symbol, '2'); // top-right
        assert_eq!(buf.get(0, 2).symbol, '3'); // bottom-left
        assert_eq!(buf.get(3, 2).symbol, '4'); // bottom-right
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib widgets::block::`
Expected: all existing tests (updated in Step 2) plus the new Step 3
test pass.

- [ ] **Step 5: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
Expected: both clean (aside from remaining, expected errors in
`src/widgets/smash_border.rs` and example apps not yet migrated —
Tasks 3-4 fix those).

- [ ] **Step 6: Commit**

```bash
git add src/widgets/block.rs
git commit -m "feat(core): migrate Block::render to BorderSet's 4 corner fields

Each of draw_ring's 4 corner cells now reads its own matching field
(top_left/top_right/bottom_left/bottom_right) instead of one shared
value. Existing tests updated to the new literal shape; a new test
asserts all 4 corners render independently, using 4 distinct glyphs
so a swapped-corner bug can't pass silently."
```

---

### Task 3: `SmashBorder::render` migration

**Files:**
- Modify: `src/widgets/smash_border.rs`

**Interfaces:**
- Consumes: `BorderSet`'s new fields (Task 1).
- Produces: no new public interface — `SmashBorder::render`'s signature is unchanged.

- [ ] **Step 1: Change the middle ring's corner representation from one `char` to `[char; 4]`**

In `src/widgets/smash_border.rs`, `render`'s `rings` array currently
has type `[(char, char, char, Color); 3]`, with a single shared corner
char per ring (the outer `'#'` ring and inner `'-'`/`':'`/`'.'` ring
don't come from `BorderSet` and keep the same glyph at all 4 corners
today; only the middle ring's corner comes from `theme.border`, which
now has 4 distinct values). Change the array's type and the 3 ring
definitions:

```rust
        let rings: [(char, char, [char; 4], Color); 3] = [
            ('#', '#', ['#', '#', '#', '#'], theme.accent),
            (
                theme.border.horizontal,
                theme.border.vertical,
                [
                    theme.border.top_left,
                    theme.border.top_right,
                    theme.border.bottom_left,
                    theme.border.bottom_right,
                ],
                theme.primary,
            ),
            ('-', ':', ['.', '.', '.', '.'], theme.tertiary),
        ];
```

Update the loop destructuring and the 4 corner-setting `buf.set` calls
(they currently all read `symbol: c` from a single destructured `c:
char` — change the loop variable name and each call to read the
matching array index, in the same position order as the existing 4
calls: top-left, top-right, bottom-left, bottom-right):

```rust
        let mut inner = area;
        for (h, v, corners, color) in rings {
            if inner.width < 2 || inner.height < 2 {
                break;
            }
            for x in inner.x..inner.x + inner.width {
                buf.set(
                    x,
                    inner.y,
                    Cell {
                        symbol: h,
                        fg: color,
                        bg: Color::Reset,
                        alpha: 1.0,
                        ..Default::default()
                    },
                );
                buf.set(
                    x,
                    inner.y + inner.height - 1,
                    Cell {
                        symbol: h,
                        fg: color,
                        bg: Color::Reset,
                        alpha: 1.0,
                        ..Default::default()
                    },
                );
            }
            for y in inner.y..inner.y + inner.height {
                buf.set(
                    inner.x,
                    y,
                    Cell {
                        symbol: v,
                        fg: color,
                        bg: Color::Reset,
                        alpha: 1.0,
                        ..Default::default()
                    },
                );
                buf.set(
                    inner.x + inner.width - 1,
                    y,
                    Cell {
                        symbol: v,
                        fg: color,
                        bg: Color::Reset,
                        alpha: 1.0,
                        ..Default::default()
                    },
                );
            }
            buf.set(
                inner.x,
                inner.y,
                Cell {
                    symbol: corners[0], // top-left
                    fg: color,
                    bg: Color::Reset,
                    alpha: 1.0,
                    ..Default::default()
                },
            );
            buf.set(
                inner.x + inner.width - 1,
                inner.y,
                Cell {
                    symbol: corners[1], // top-right
                    fg: color,
                    bg: Color::Reset,
                    alpha: 1.0,
                    ..Default::default()
                },
            );
            buf.set(
                inner.x,
                inner.y + inner.height - 1,
                Cell {
                    symbol: corners[2], // bottom-left
                    fg: color,
                    bg: Color::Reset,
                    alpha: 1.0,
                    ..Default::default()
                },
            );
            buf.set(
                inner.x + inner.width - 1,
                inner.y + inner.height - 1,
                Cell {
                    symbol: corners[3], // bottom-right
                    fg: color,
                    bg: Color::Reset,
                    alpha: 1.0,
                    ..Default::default()
                },
            );

            inner = Rect {
                x: inner.x + 1,
                y: inner.y + 1,
                width: inner.width.saturating_sub(2),
                height: inner.height.saturating_sub(2),
            };
        }

        inner
```

- [ ] **Step 2: Update the existing test's `BorderSet` literal**

In `test_theme()` (the module's test helper), update the `border`
literal:

```rust
            border: BorderSet {
                horizontal: '=',
                vertical: '|',
                top_left: '+',
                top_right: '+',
                bottom_left: '+',
                bottom_right: '+',
            },
```

This preserves `draws_three_concentric_rings_and_returns_shrunk_inner_area`'s
existing assertion (`buf.get(1, 1).symbol` == `'+'`, the middle
ring's top-left corner) unchanged.

- [ ] **Step 3: Add a test asserting the middle ring's 4 corners are independently addressable**

```rust
    #[test]
    fn middle_ring_renders_all_four_corners_from_their_own_field() {
        let mut theme = test_theme();
        theme.border = BorderSet {
            horizontal: '=',
            vertical: '|',
            top_left: '1',
            top_right: '2',
            bottom_left: '3',
            bottom_right: '4',
        };
        let mut buf = Buffer::new(12, 10);
        let area = Rect {
            x: 0,
            y: 0,
            width: 12,
            height: 10,
        };

        SmashBorder::new().render(area, &theme, &mut buf);

        // Middle ring sits 1 cell inward from the outer ring's own
        // bounds (0..12, 0..10) -> middle ring spans (1..11, 1..9).
        assert_eq!(buf.get(1, 1).symbol, '1'); // top-left
        assert_eq!(buf.get(10, 1).symbol, '2'); // top-right
        assert_eq!(buf.get(1, 8).symbol, '3'); // bottom-left
        assert_eq!(buf.get(10, 8).symbol, '4'); // bottom-right
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib widgets::smash_border::`
Expected: existing tests (updated in Step 2) plus the new Step 3 test
pass.

- [ ] **Step 5: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
Expected: both clean (aside from remaining, expected errors in
example apps not yet migrated — Task 4 fixes those).

- [ ] **Step 6: Commit**

```bash
git add src/widgets/smash_border.rs
git commit -m "feat(core): migrate SmashBorder::render to BorderSet's 4 corner fields

The middle ring's corner representation changes from a single shared
char to a [char; 4] (one per corner, sourced from BorderSet's 4
fields); the other two hardcoded rings keep repeating their own single
glyph at all 4 corners, unrelated to BorderSet. New test asserts the
middle ring's 4 corners are independently addressable."
```

---

### Task 4: Example-app call-site migrations

**Files:**
- Modify: `examples/tardis/tardis.rs`
- Modify: `examples/omnitrix/omnitrix.rs`
- Modify: `examples/smash_crabs/smash_crabs.rs`
- Modify: `examples/launcher/portal.rs`
- Verify (no code change expected): `showcase/showcase.rs`,
  `examples/control_panel.rs`, `examples/mission_control.rs`,
  `examples/falcon/falcon.rs`, `src/widgets/cockpit_panel.rs`

**Interfaces:**
- Consumes: `BorderSet`'s new fields (Task 1); `Block`/`SmashBorder`'s
  migrated rendering (Tasks 2-3).
- Produces: nothing consumed by later tasks — this task is
  purely a migration pass.

- [ ] **Step 1: `examples/tardis/tardis.rs`**

Its current `border: BorderSet { horizontal: '=', vertical: '#', corner:
'+' }` literal becomes:

```rust
        border: BorderSet {
            horizontal: '=',
            vertical: '#',
            top_left: '+',
            top_right: '+',
            bottom_left: '+',
            bottom_right: '+',
        },
```

- [ ] **Step 2: `examples/omnitrix/omnitrix.rs`**

Same shape, same values (`horizontal: '='`, `vertical: '#'`, corner
`'+'` at all 4 positions):

```rust
            border: BorderSet {
                horizontal: '=',
                vertical: '#',
                top_left: '+',
                top_right: '+',
                bottom_left: '+',
                bottom_right: '+',
            },
```

- [ ] **Step 3: `examples/smash_crabs/smash_crabs.rs`**

Current literal has `vertical: '|'` (not `'#'` — different from
tardis/omnitrix, don't transcribe the wrong one):

```rust
        border: BorderSet {
            horizontal: '=',
            vertical: '|',
            top_left: '+',
            top_right: '+',
            bottom_left: '+',
            bottom_right: '+',
        },
```

- [ ] **Step 4: `examples/launcher/portal.rs`**

This one has a **dynamic** corner (`if focused { '◆' } else { '·' }`),
not a static char — preserve the exact expression at all 4 fields so
the focus-indicator behavior is unchanged:

```rust
        border: BorderSet {
            horizontal: '─',
            vertical: '│',
            top_left: if focused { '◆' } else { '·' },
            top_right: if focused { '◆' } else { '·' },
            bottom_left: if focused { '◆' } else { '·' },
            bottom_right: if focused { '◆' } else { '·' },
        },
```

- [ ] **Step 5: Verify the 5 zero-change sites actually need no edits**

`showcase/showcase.rs`, `examples/control_panel.rs`,
`examples/mission_control.rs`, `examples/falcon/falcon.rs`, and
`src/widgets/cockpit_panel.rs` all construct `border:
BorderSet::default()` already — confirm (`grep -n "border: BorderSet"`
each file) they're unchanged from what this plan's brainstorming
observed, and that they compile cleanly once Task 1 lands, with no
edits needed. If any of them turns out to have changed since this
plan was written (e.g. gained its own custom `BorderSet` literal),
treat that as a real gap — migrate it the same way as Steps 1-4 above,
using whatever its actual current corner value is, and note the
discrepancy in your task report.

- [ ] **Step 6: Build and lint the whole workspace**

Run: `cargo build --all-targets`
Expected: succeeds — this is the first point every call site in the
workspace compiles again since Task 1 landed.

Run: `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
Expected: both clean.

- [ ] **Step 7: Full workspace test suite**

Run: `cargo test --workspace`
Expected: full suite green, including every test from Tasks 1-3.

- [ ] **Step 8: Commit**

```bash
git add examples/tardis/tardis.rs examples/omnitrix/omnitrix.rs \
        examples/smash_crabs/smash_crabs.rs examples/launcher/portal.rs
git commit -m "feat(core): migrate example apps to BorderSet's 4 corner fields

tardis/omnitrix/smash_crabs repeat their existing single corner char
across all 4 new fields; launcher/portal repeats its dynamic
focus-indicator expression the same way. showcase/control_panel/
mission_control/falcon/cockpit_panel needed no changes — they already
use BorderSet::default(), which now returns real box-drawing corners
automatically."
```

---

### Task 5: Final verification

**Files:** none (verification only).

- [ ] **Step 1: Full workspace build, lint, format, test**

Run: `cargo build --all-targets`, `cargo clippy --all-targets -- -D
warnings`, `cargo fmt --check`, `cargo test --workspace`
Expected: all clean; full suite green.

- [ ] **Step 2: Capture and verify visually — the new default**

Capture `showcase` (uses `BorderSet::default()`, i.e. `single_line()`)
at the menu screen:

```
cargo run -p visual-snapshot -- --bin showcase --size 100x30 --script <script.json> --out <path>.gif
```

(use a script waiting past the 1200ms boot, e.g. `[{"wait_ms": 1500}]`,
per this project's established `showcase` capture convention).
`Read` the result. Confirm the tile menu's borders render with real
distinct box-drawing corners (`┌┐└┘`), not a repeated `+`/`*`, and
that no `font8x8` glyph-coverage error was hit for these glyphs (the
design spec flagged this as unverified-until-captured).

- [ ] **Step 3: Capture and verify visually — a custom-preserved glyph**

Capture `tardis` (custom `'+'`-at-every-corner, unchanged look) at its
hub screen — a single-frame capture is enough, no script steps needed.
`Read` the result. Confirm all 4 corners still show `'+'` (same as
before this Arc), proving the migration preserved tardis's exact
visual appearance rather than accidentally picking up box-drawing
glyphs.

- [ ] **Step 4: Record the result**

No additional commit for this step. If either capture reveals a
problem (e.g. a `font8x8` glyph-coverage gap for `┌┐└┘`, or tardis's
corners not matching), that's a finding — file it as a GitHub issue
and triage it per `code-forge.md`'s rule. If both captures confirm the
expected behavior, record that plainly.

## Final verification (whole plan)

- [ ] `cargo build --all-targets` / `cargo clippy --all-targets -- -D
      warnings` / `cargo fmt --check` — all clean.
- [ ] `cargo test --workspace` — full suite green, including all new
      tests from Tasks 1-3.
- [ ] Both Task 5 `tools/visual-snapshot` captures were actually `Read`
      and reviewed.
- [ ] The final commit's message includes `Closes #130.` so the issue
      auto-closes on merge (do not manually `gh issue close` it).
- [ ] Per `.claude/rules/git-github-standards.md`: this Arc is
      `coding`-tagged (Gated tier) — open a PR from this worktree's
      branch to `main`, wait for all four required checks green,
      squash-merge, then remove the worktree via `ExitWorktree`.
- [ ] Once merged, re-check for any other open `v1-blocking` issues
      before cutting the v1.0.0 tag — this plan closes #130, which was
      the last one known at the time this plan was written, but
      confirm nothing new was filed in the meantime.
