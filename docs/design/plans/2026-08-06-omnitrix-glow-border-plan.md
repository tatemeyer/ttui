# Omnitrix Glow Border (Issue #41) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `docs/design/specs/2026-08-06-omnitrix-glow-border-design.md`
(GitHub issue #41, tracking #52): adopt `Cell.style.bold` (Arc 0) in
Omnitrix's border glow, replacing the pure-color pulse with a
brightness-threshold bold toggle layered over it.

**Architecture:** Three sequential tasks (each depends on the previous):
add `Theme.border_bold` (and fix every exhaustive `Theme { .. }` literal
this forces), teach `Block::render` to apply it to border cells only, then
wire Omnitrix's existing brightness calculation to it. Unlike Arc 0's
file-disjoint tasks, these touch overlapping/dependent files in sequence,
not concurrently.

**Tech Stack:** Rust, `crossterm` (unchanged). No new dependencies.

## Global Constraints

- TDD mandatory for Tasks 1-2 (`coding`-tagged, no exception applies).
  Task 3 (`examples/omnitrix.rs`) is example code — per
  `.claude/rules/development-conventions.md`'s TDD exceptions, verified
  by running the example, not unit tested.
- Inline `#[cfg(test)] mod tests` per module — no new `tests/` directory.
- `cargo fmt` / `cargo clippy --all-targets` clean after every task.
- No new dependencies.
- Border glow bold applies to border glyphs only — title text drawn by
  `Block::render` must stay non-bold regardless of `border_bold`.

---

### Task 1: `Theme.border_bold` field

**Files:**
- Modify: `src/theme.rs`
- Modify: `examples/smash_crabs.rs` (mechanical exhaustive-literal fix)
- Modify: `examples/omnitrix.rs` (mechanical exhaustive-literal fix —
  placeholder `false`; Task 3 replaces this with real logic)
- Modify: `src/widgets/block.rs` (mechanical exhaustive-literal fix in
  its existing test; Task 2 adds the real bold-behavior tests)

**Interfaces produced:**
```rust
pub struct Theme {
    pub background: Color,
    pub primary: Color,
    pub secondary: Color,
    pub tertiary: Color,
    pub accent: Color,
    pub border: BorderSet,
    pub border_bold: bool,
}
```
`Theme::default().border_bold == false`.

- [ ] **Step 1: Write the failing test** — add to `src/theme.rs`'s
  existing `mod tests`:

```rust
#[test]
fn default_theme_border_bold_is_false() {
    assert!(!Theme::default().border_bold);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib theme::tests::default_theme_border_bold_is_false`
Expected: FAIL (`no field \`border_bold\` on type \`Theme\``)

- [ ] **Step 3: Implement** — in `src/theme.rs`, add the field to the
  `Theme` struct definition and to `impl Default for Theme`:

```rust
pub struct Theme {
    pub background: Color,
    pub primary: Color,
    pub secondary: Color,
    pub tertiary: Color,
    pub accent: Color,
    pub border: BorderSet,
    pub border_bold: bool,
}
```
```rust
impl Default for Theme {
    fn default() -> Self {
        Theme {
            background: Color::Reset,
            primary: Color::Reset,
            secondary: Color::Reset,
            tertiary: Color::Reset,
            accent: Color::Reset,
            border: BorderSet::default(),
            border_bold: false,
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib theme::tests`
Expected: PASS (including the pre-existing `default_theme_uses_reset_colors_and_default_border`)

- [ ] **Step 5: Fix every exhaustive `Theme { .. }` literal the compiler
  now flags.** Run `cargo build --examples 2>&1` and `cargo test --lib 2>&1`
  to find them — expected sites (confirm this list is exhaustive via the
  build errors):
  - `examples/smash_crabs.rs`, `arena_theme()`: add `border_bold: false,`
    after the `border: BorderSet { .. }` field (Smash Crabs has no glow
    border — out of this ticket's scope, stays `false`).
  - `examples/omnitrix.rs`, `theme()`: add `border_bold: false,` after
    the `border: BorderSet { .. }` field — placeholder only, Task 3
    replaces this literal `false` with real logic. Do not compute
    brightness-threshold logic here; that belongs to Task 3.
  - `src/widgets/block.rs`, `with_theme_border_uses_theme_glyphs_and_colors`
    test: add `border_bold: false,` after its `border: BorderSet { .. }`
    field — this existing test doesn't assert on bold, so `false` keeps
    it unchanged; Task 2 adds new tests that do assert on bold.
  Do not change any other behavior in these files — purely additive.

- [ ] **Step 6: Run full check**

Run: `cargo test --lib && cargo build --examples && cargo fmt && cargo clippy --all-targets`
Expected: all pass, no warnings.

- [ ] **Step 7: Commit**

```bash
git add src/theme.rs examples/smash_crabs.rs examples/omnitrix.rs src/widgets/block.rs
git commit -m "feat(theme): add Theme.border_bold field"
```

---

### Task 2: `Block::render` applies `border_bold` to border cells only

**Files:**
- Modify: `src/widgets/block.rs`

**Interfaces consumed:**
- `Theme.border_bold: bool` (Task 1)
- `CellStyle { pub bold: bool }` (`src/buffer.rs`, from Arc 0 — already
  `#[derive(Clone, Copy, PartialEq, Debug, Default)]`)

**Interfaces produced:** no new public API — `Block::render`'s existing
signature (`fn render(&self, area: Rect, buf: &mut Buffer) -> Rect`) is
unchanged; only its internal cell construction changes.

- [ ] **Step 1: Write the failing tests** — add to `src/widgets/block.rs`'s
  existing `mod tests`:

```rust
#[test]
fn border_cells_are_bold_when_theme_border_bold_is_true() {
    let theme = Theme {
        background: Color::Black,
        primary: Color::Green,
        secondary: Color::Reset,
        tertiary: Color::Reset,
        accent: Color::Reset,
        border: BorderSet {
            horizontal: '=',
            vertical: '#',
            corner: '*',
        },
        border_bold: true,
    };
    let mut buf = Buffer::new(4, 3);
    let area = Rect {
        x: 0,
        y: 0,
        width: 4,
        height: 3,
    };

    Block::new().theme(&theme).render(area, &mut buf);

    assert!(buf.get(0, 0).style.bold); // corner
    assert!(buf.get(1, 0).style.bold); // horizontal edge
    assert!(buf.get(0, 1).style.bold); // vertical edge
}

#[test]
fn title_cells_are_not_bold_even_when_theme_border_bold_is_true() {
    let theme = Theme {
        background: Color::Black,
        primary: Color::Green,
        secondary: Color::Reset,
        tertiary: Color::Reset,
        accent: Color::Reset,
        border: BorderSet {
            horizontal: '=',
            vertical: '#',
            corner: '*',
        },
        border_bold: true,
    };
    let mut buf = Buffer::new(6, 3);
    let area = Rect {
        x: 0,
        y: 0,
        width: 6,
        height: 3,
    };

    Block::new().title("Hi").theme(&theme).render(area, &mut buf);

    assert!(!buf.get(1, 0).style.bold); // 'H'
    assert!(!buf.get(2, 0).style.bold); // 'i'
}
```

  Also extend the existing `without_theme_border_colors_are_reset` test
  with one more assertion (append, don't remove the existing ones):

```rust
assert!(!buf.get(0, 0).style.bold);
```

- [ ] **Step 2: Run tests to verify the two new ones fail**

Run: `cargo test --lib widgets::block::tests`
Expected: `border_cells_are_bold_when_theme_border_bold_is_true` and
`title_cells_are_not_bold_even_when_theme_border_bold_is_true` FAIL
(`style.bold` is `false` — nothing sets it yet); the extended
`without_theme_border_colors_are_reset` PASSES already (Cell::default()
is non-bold).

- [ ] **Step 3: Implement** — in `src/widgets/block.rs`:

Add `CellStyle` to the existing import:
```rust
use crate::buffer::{Buffer, Cell, CellStyle};
```

Replace the theme-unpacking line and `plain()` closure:
```rust
let (border, fg, bg, border_bold) = match self.theme {
    Some(t) => (t.border, t.primary, t.background, t.border_bold),
    None => (BorderSet::default(), Color::Reset, Color::Reset, false),
};
let plain = || Cell {
    symbol: ' ',
    fg,
    bg,
    style: CellStyle { bold: border_bold },
    ..Default::default()
};
```

Leave every border/corner `Cell { symbol: ..., ..plain() }` call
unchanged — they now inherit `style.bold` from `plain()` automatically.

In the title-drawing loop, override `style` back to non-bold explicitly
(explicit fields win over `..plain()` in Rust's struct-update syntax):
```rust
buf.set(
    area.x + 1 + i as u16,
    area.y,
    Cell {
        symbol: ch,
        style: CellStyle::default(),
        ..plain()
    },
);
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib widgets::block::tests`
Expected: PASS (all tests in the module, old and new)

- [ ] **Step 5: Run full check**

Run: `cargo test --lib && cargo build --examples && cargo fmt && cargo clippy --all-targets`
Expected: all pass, no warnings.

- [ ] **Step 6: Commit**

```bash
git add src/widgets/block.rs
git commit -m "feat(widgets): Block border cells honor Theme.border_bold"
```

---

### Task 3: Omnitrix glow border wiring

**Files:**
- Modify: `examples/omnitrix.rs`

**Interfaces consumed:**
- `Theme.border_bold: bool` (Task 1), read by `Block::render` (Task 2)
- Existing `brightness: f32` local already computed in `Omnitrix::theme()`

No new tests — `examples/omnitrix.rs` is example code, verified by
running it per the TDD exceptions in `development-conventions.md`, same
as every other change to this file to date.

- [ ] **Step 1: Implement** — in `examples/omnitrix.rs`'s `theme()`
  method, replace the Task 1 placeholder `border_bold: false,` with:

```rust
border_bold: brightness > 0.6,
```

  placed alongside the existing `border: BorderSet { .. }` field, right
  after it, in the returned `Theme { .. }` literal. No other line in
  `theme()` changes — `brightness` is already computed above this
  literal by the existing sine-wave calculation.

- [ ] **Step 2: Build**

Run: `cargo build --example omnitrix`
Expected: compiles cleanly.

- [ ] **Step 3: `cargo fmt && cargo clippy --all-targets`**

Expected: clean, no warnings.

- [ ] **Step 4: Manual verification** (real-terminal check, not
  automatable — per this ticket's spec and PR #84's carried-over open
  item):

Run: `cargo run --example omnitrix`

Confirm, watching for a few pulse cycles:
- The border color still fades smoothly (dim green to bright green) as
  before — the existing pulse is unchanged.
- The border glyphs additionally switch to bold as the color nears peak
  brightness (roughly the top 40% of each cycle) and back to non-bold as
  it dims — reads as the glow "catching" rather than a hard flicker.
- The title text ("Omnitrix") and body text never go bold — only the
  border ring does.
- Press `q`: the app still exits cleanly, no panic, no leftover bold/color
  attributes bleeding into the shell prompt after exit (this is the
  `terminal.rs` attribute-reset wiring from Arc 0 Task 1 — confirms it
  still works with a real bold-toggling consumer).

- [ ] **Step 5: Commit**

```bash
git add examples/omnitrix.rs
git commit -m "feat(omnitrix): adopt Cell style/bold for the glow border (#41)"
```

---

## Self-Review

**Spec coverage:** Theme field (Task 1) — covered. Block border-only bold
application (Task 2) — covered, including the title-must-stay-non-bold
requirement. Omnitrix threshold wiring (Task 3) — covered, threshold
value (0.6) matches the spec verbatim. Verification section (cargo
test/clippy/fmt + manual run) — covered across all three tasks' final
steps plus Task 3's dedicated manual-check step.

**Placeholder scan:** no TBD/TODO; every step has literal code or an
exact command. Task 1's `false` in `omnitrix.rs` is explicitly flagged
as a temporary placeholder with the task that replaces it named, not an
unresolved placeholder in the plan itself.

**Type consistency:** `Theme.border_bold: bool` (Task 1) is the exact
type/name `Block::render` reads in Task 2 and `examples/omnitrix.rs`
writes in Task 3 — no renames across tasks. `CellStyle { bold: bool }`
matches its existing Arc 0 definition in `src/buffer.rs`, not redefined
here.
