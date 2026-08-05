# TTUI Core Framework Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Structure note:** This plan is organized as **Arcs → Slices → Tasks**
> per `docs/design/README.md`, not the flat "Task N" list the
> `writing-plans` skill defaults to. Tasks still follow the skill's
> bite-sized TDD step structure; Arc/Slice headings are pure grouping.

**Goal:** Build TTUI v1 — a Rust terminal UI framework proven by a working
demo dashboard app (nested panes, `Text`/`List`/`Table`/`Block` widgets,
`Tab` focus switching, `Up`/`Down` navigation).

**Architecture:** Five-stage immediate-mode pipeline (App state → View
builder → Layout → Paint → Diff → Terminal writer), driven synchronously
by input events, never a polling tick. Full detail:
`docs/design/specs/2026-08-04-ttui-core-framework-design.md` (Rev A).

**Tech Stack:** Rust (stable, 2021 edition), `crossterm` for terminal I/O.

## Global Constraints

- Single crate (not a Cargo workspace) for v1. Package name `ttui`.
- Terminal I/O dependency is `crossterm = "0.27"` — no second terminal
  crate introduced.
- Windows-first (Windows Terminal/ConPTY, mintty) — no Linux/macOS
  testing required for v1, but avoid Windows-only APIs (crossterm already
  handles cross-platform differences).
- Redraw must be event-driven, never a fixed-tick poll loop — every task
  touching the event loop must preserve "handle event → redraw → flush"
  as one synchronous step.
- Every terminal-write path must flush immediately after writing — no
  buffering that delays visible output.
- Raw mode / alternate screen entry must always be paired with guaranteed
  restoration on exit, including panics (`Drop` guard + panic hook) — no
  task may bypass this.
- Widgets are stateless `(data, area) -> paint` functions — no widget
  type may hold state across frames.
- Full event-loop / real-terminal behavior is not unit-tested, per the
  spec's accepted testing gap — verified instead by the manual check in
  Task 17. Tests that do need a real terminal/TTY (raw-mode enter/exit,
  panic-hook behavior) are marked `#[ignore]` and run manually with
  `cargo test -- --ignored`, not as part of a headless `cargo test`.

---

## Arc 1: Foundation — Project Setup, Cell Buffer, Diffing

### Slice 1.1: Project scaffolding

**Tags:** coding, git-adjacent

#### Task 1: Initialize the Cargo project

**Files:**
- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Modify: `.gitignore` (append Rust section: `/target`)
- Create: `tests/README.md`

**Interfaces:**
- Consumes: nothing (first task)
- Produces: an empty `ttui` library crate that later tasks add modules
  to, plus a `tests/` integration-test placeholder (see
  `docs/design/specs/2026-08-04-testing-verification-conventions-design.md`).

- [ ] **Step 1: Create `Cargo.toml`**

```toml
[package]
name = "ttui"
version = "0.1.0"
edition = "2021"

[dependencies]
crossterm = "0.27"
```

- [ ] **Step 2: Create `src/lib.rs`**

```rust
// Modules are added by later tasks as each one lands.
```

- [ ] **Step 3: Append a Rust section to `.gitignore`**

The repo's `.gitignore` already exists with code-review-graph/OS/
editor/Python-tooling/installer-backup sections — append to it, do not
overwrite (the code-review-graph section in particular is
installer-managed and must not be lost):

```
# Rust
/target
```

- [ ] **Step 4: Create `tests/README.md`**

```markdown
# Integration tests

Not used yet. Unit tests live inline via `#[cfg(test)] mod tests` in
each module — see
`docs/design/specs/2026-08-04-testing-verification-conventions-design.md`.
This directory is for integration tests that exercise the crate as an
external consumer would, via the public `ttui::` API across module
boundaries. Add a test file here the first time one is actually
needed, not before.
```

- [ ] **Step 5: Verify the crate builds**

Run: `cargo build`
Expected: builds successfully with no errors (crossterm downloads and
compiles as a dependency).

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src/lib.rs .gitignore tests/README.md
git commit -m "chore: initialize ttui crate with crossterm dependency"
```

### Slice 1.2: Cell buffer & diffing

**Tags:** coding

#### Task 2: `Cell` and `Buffer` types

**Files:**
- Create: `src/buffer.rs`
- Modify: `src/lib.rs`
- Test: `src/buffer.rs` (inline `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `crossterm::style::Color` (external crate type).
- Produces: `Cell { symbol: char, fg: Color, bg: Color }` (derives
  `Clone, PartialEq, Debug`), `Buffer::new(width: u16, height: u16) ->
  Buffer`, `Buffer::get(&self, x: u16, y: u16) -> &Cell`,
  `Buffer::set(&mut self, x: u16, y: u16, cell: Cell)`, public fields
  `Buffer.width: u16`, `Buffer.height: u16`.

- [ ] **Step 1: Write the failing test**

```rust
// src/buffer.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_buffer_is_filled_with_default_cells() {
        let buf = Buffer::new(3, 2);
        assert_eq!(buf.width, 3);
        assert_eq!(buf.height, 2);
        assert_eq!(*buf.get(0, 0), Cell::default());
        assert_eq!(*buf.get(2, 1), Cell::default());
    }

    #[test]
    fn set_then_get_returns_the_cell() {
        let mut buf = Buffer::new(3, 2);
        let cell = Cell { symbol: 'x', fg: crossterm::style::Color::Red, bg: crossterm::style::Color::Reset };
        buf.set(1, 1, cell.clone());
        assert_eq!(*buf.get(1, 1), cell);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test buffer::`
Expected: FAIL to compile — `Cell`, `Buffer` not defined yet.

- [ ] **Step 3: Write the implementation**

```rust
// src/buffer.rs (above the tests module)
use crossterm::style::Color;

#[derive(Clone, PartialEq, Debug)]
pub struct Cell {
    pub symbol: char,
    pub fg: Color,
    pub bg: Color,
}

impl Default for Cell {
    fn default() -> Self {
        Cell { symbol: ' ', fg: Color::Reset, bg: Color::Reset }
    }
}

#[derive(Clone, Debug)]
pub struct Buffer {
    pub width: u16,
    pub height: u16,
    cells: Vec<Cell>,
}

impl Buffer {
    pub fn new(width: u16, height: u16) -> Self {
        Buffer {
            width,
            height,
            cells: vec![Cell::default(); width as usize * height as usize],
        }
    }

    pub fn get(&self, x: u16, y: u16) -> &Cell {
        &self.cells[self.index(x, y)]
    }

    pub fn set(&mut self, x: u16, y: u16, cell: Cell) {
        let idx = self.index(x, y);
        self.cells[idx] = cell;
    }

    fn index(&self, x: u16, y: u16) -> usize {
        y as usize * self.width as usize + x as usize
    }
}
```

- [ ] **Step 4: Add the module to `src/lib.rs`**

```rust
pub mod buffer;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test buffer::`
Expected: PASS (2 tests)

- [ ] **Step 6: Commit**

```bash
git add src/buffer.rs src/lib.rs
git commit -m "feat: add Cell and Buffer cell-grid types"
```

#### Task 3: Buffer diffing

**Files:**
- Modify: `src/buffer.rs`
- Test: `src/buffer.rs` (inline `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `Cell`, `Buffer` (Task 2).
- Produces: `CellDiff { x: u16, y: u16, cell: Cell }` (derives `Debug,
  PartialEq`), `diff(prev: &Buffer, next: &Buffer) -> Vec<CellDiff>`.

- [ ] **Step 1: Write the failing test**

```rust
// add to the existing tests module in src/buffer.rs
#[test]
fn diff_returns_only_changed_cells() {
    let prev = Buffer::new(2, 1);
    let mut next = Buffer::new(2, 1);
    let cell = Cell { symbol: 'x', fg: Color::Reset, bg: Color::Reset };
    next.set(1, 0, cell.clone());

    let diffs = diff(&prev, &next);

    assert_eq!(diffs, vec![CellDiff { x: 1, y: 0, cell }]);
}

#[test]
fn diff_of_identical_buffers_is_empty() {
    let a = Buffer::new(2, 2);
    let b = Buffer::new(2, 2);
    assert!(diff(&a, &b).is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test buffer::`
Expected: FAIL to compile — `CellDiff`, `diff` not defined yet.

- [ ] **Step 3: Write the implementation**

```rust
// src/buffer.rs, below the Buffer impl block
#[derive(Debug, PartialEq)]
pub struct CellDiff {
    pub x: u16,
    pub y: u16,
    pub cell: Cell,
}

pub fn diff(prev: &Buffer, next: &Buffer) -> Vec<CellDiff> {
    let mut out = Vec::new();
    for y in 0..next.height {
        for x in 0..next.width {
            let n = next.get(x, y);
            if n != prev.get(x, y) {
                out.push(CellDiff { x, y, cell: n.clone() });
            }
        }
    }
    out
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test buffer::`
Expected: PASS (4 tests total in this module)

- [ ] **Step 5: Commit**

```bash
git add src/buffer.rs
git commit -m "feat: add Buffer diffing"
```

---

## Arc 2: Terminal I/O

### Slice 2.1: Terminal wrapper with panic-safe raw mode

**Tags:** coding

#### Task 4: `Terminal` — raw mode / alt screen enter and exit

**Files:**
- Create: `src/terminal.rs`
- Modify: `src/lib.rs`
- Test: `src/terminal.rs` (inline `#[cfg(test)] mod tests`, `#[ignore]`)

**Interfaces:**
- Consumes: `crossterm::terminal`, `crossterm::cursor`, `crossterm::execute`.
- Produces: `Terminal::new() -> std::io::Result<Terminal>`,
  `Terminal::size(&self) -> std::io::Result<(u16, u16)>`, a `Drop` impl
  that restores the terminal.

- [ ] **Step 1: Write the failing test**

```rust
// src/terminal.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::terminal;

    #[test]
    #[ignore = "requires a real terminal (TTY); run with `cargo test -- --ignored`"]
    fn enter_and_drop_restores_raw_mode() {
        assert!(!terminal::is_raw_mode_enabled().unwrap());
        {
            let _term = Terminal::new().unwrap();
            assert!(terminal::is_raw_mode_enabled().unwrap());
        }
        assert!(!terminal::is_raw_mode_enabled().unwrap());
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test terminal:: -- --ignored`
Expected: FAIL to compile — `Terminal` not defined yet.

- [ ] **Step 3: Write the implementation**

```rust
// src/terminal.rs, above the tests module
use std::io::{stdout, Stdout};
use crossterm::{execute, terminal, cursor};

pub struct Terminal {
    out: Stdout,
}

impl Terminal {
    pub fn new() -> std::io::Result<Self> {
        terminal::enable_raw_mode()?;
        let mut out = stdout();
        execute!(out, terminal::EnterAlternateScreen, cursor::Hide)?;
        Ok(Terminal { out })
    }

    pub fn size(&self) -> std::io::Result<(u16, u16)> {
        terminal::size()
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(self.out, terminal::LeaveAlternateScreen, cursor::Show);
    }
}
```

- [ ] **Step 4: Add the module to `src/lib.rs`**

```rust
pub mod terminal;
```

- [ ] **Step 5: Run the test to verify it passes**

Run (in an actual terminal, not a headless shell): `cargo test terminal:: -- --ignored`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/terminal.rs src/lib.rs
git commit -m "feat: add Terminal with panic-safe raw mode enter/exit"
```

#### Task 5: `Terminal::draw_diff` — write and flush changed cells

**Files:**
- Modify: `src/terminal.rs`

**Interfaces:**
- Consumes: `CellDiff` (Task 3), `Terminal` (Task 4).
- Produces: `Terminal::draw_diff(&mut self, diffs: &[CellDiff]) ->
  std::io::Result<()>`.

- [ ] **Step 1: Write the implementation**

There is no real-terminal-free way to assert on actual escape-sequence
output without a full terminal emulator harness, which is out of scope
per the spec's accepted testing gap — this is verified manually in
Task 17. Implement directly:

```rust
// src/terminal.rs
use std::io::Write;
use crossterm::style::{SetForegroundColor, SetBackgroundColor, Print};
use crate::buffer::CellDiff;

impl Terminal {
    pub fn draw_diff(&mut self, diffs: &[CellDiff]) -> std::io::Result<()> {
        for d in diffs {
            execute!(
                self.out,
                cursor::MoveTo(d.x, d.y),
                SetForegroundColor(d.cell.fg),
                SetBackgroundColor(d.cell.bg),
                Print(d.cell.symbol),
            )?;
        }
        self.out.flush()
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build`
Expected: builds successfully.

- [ ] **Step 3: Commit**

```bash
git add src/terminal.rs
git commit -m "feat: add Terminal::draw_diff with immediate flush"
```

#### Task 6: `Terminal::next_event` — blocking input read

**Files:**
- Modify: `src/terminal.rs`

**Interfaces:**
- Consumes: `crossterm::event`.
- Produces: `Terminal::next_event(&self, timeout: std::time::Duration) ->
  std::io::Result<Option<crossterm::event::Event>>`.

- [ ] **Step 1: Write the implementation**

```rust
// src/terminal.rs
use std::time::Duration;
use crossterm::event::{self, Event};

impl Terminal {
    pub fn next_event(&self, timeout: Duration) -> std::io::Result<Option<Event>> {
        if event::poll(timeout)? {
            Ok(Some(event::read()?))
        } else {
            Ok(None)
        }
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build`
Expected: builds successfully.

- [ ] **Step 3: Commit**

```bash
git add src/terminal.rs
git commit -m "feat: add Terminal::next_event polling wrapper"
```

#### Task 7: Panic-safe cleanup hook

**Files:**
- Modify: `src/terminal.rs`
- Test: `src/terminal.rs` (inline `#[cfg(test)] mod tests`, `#[ignore]`)

**Interfaces:**
- Consumes: `std::panic`, `crossterm::terminal`.
- Produces: `pub fn install_panic_hook()`.

- [ ] **Step 1: Write the failing test**

```rust
// add to the existing tests module in src/terminal.rs
#[test]
#[ignore = "requires a real terminal (TTY); run with `cargo test -- --ignored`"]
fn panic_hook_disables_raw_mode_before_unwinding() {
    install_panic_hook();
    terminal::enable_raw_mode().unwrap();
    assert!(terminal::is_raw_mode_enabled().unwrap());

    let result = std::panic::catch_unwind(|| {
        panic!("simulated crash");
    });

    assert!(result.is_err());
    assert!(!terminal::is_raw_mode_enabled().unwrap());
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test terminal:: -- --ignored`
Expected: FAIL to compile — `install_panic_hook` not defined yet.

- [ ] **Step 3: Write the implementation**

```rust
// src/terminal.rs
pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(stdout(), terminal::LeaveAlternateScreen, cursor::Show);
        default_hook(info);
    }));
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run (in an actual terminal): `cargo test terminal:: -- --ignored`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/terminal.rs
git commit -m "feat: add panic-safe terminal cleanup hook"
```

---

## Arc 3: Layout Engine

### Slice 3.1: Constraint-based rect splitting

**Tags:** coding

#### Task 8: `Rect`, `Direction`, `Constraint`, and `Fixed`/`Percentage`/`Min` splitting

**Files:**
- Create: `src/layout.rs`
- Modify: `src/lib.rs`
- Test: `src/layout.rs` (inline `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: nothing.
- Produces: `Rect { x: u16, y: u16, width: u16, height: u16 }` (derives
  `Clone, Copy, Debug, PartialEq`), `Direction { Horizontal, Vertical }`,
  `Constraint { Fixed(u16), Percentage(u16), Min(u16), Fill(u16) }`,
  `Layout::new(direction: Direction, constraints: Vec<Constraint>) ->
  Layout`, `Layout::split(&self, area: Rect) -> Vec<Rect>`.

- [ ] **Step 1: Write the failing test**

```rust
// src/layout.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_constraints_split_horizontally() {
        let area = Rect { x: 0, y: 0, width: 10, height: 5 };
        let layout = Layout::new(Direction::Horizontal, vec![Constraint::Fixed(3), Constraint::Fixed(7)]);

        let rects = layout.split(area);

        assert_eq!(rects, vec![
            Rect { x: 0, y: 0, width: 3, height: 5 },
            Rect { x: 3, y: 0, width: 7, height: 5 },
        ]);
    }

    #[test]
    fn percentage_constraints_split_vertically() {
        let area = Rect { x: 0, y: 0, width: 4, height: 10 };
        let layout = Layout::new(Direction::Vertical, vec![Constraint::Percentage(40), Constraint::Percentage(60)]);

        let rects = layout.split(area);

        assert_eq!(rects, vec![
            Rect { x: 0, y: 0, width: 4, height: 4 },
            Rect { x: 0, y: 4, width: 4, height: 6 },
        ]);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test layout::`
Expected: FAIL to compile — `Rect`, `Direction`, `Constraint`, `Layout`
not defined yet.

- [ ] **Step 3: Write the implementation**

```rust
// src/layout.rs, above the tests module
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Constraint {
    Fixed(u16),
    Percentage(u16),
    Min(u16),
    Fill(u16),
}

pub struct Layout {
    direction: Direction,
    constraints: Vec<Constraint>,
    margin: u16,
    spacing: u16,
}

impl Layout {
    pub fn new(direction: Direction, constraints: Vec<Constraint>) -> Self {
        Layout { direction, constraints, margin: 0, spacing: 0 }
    }

    pub fn split(&self, area: Rect) -> Vec<Rect> {
        let total = match self.direction {
            Direction::Horizontal => area.width,
            Direction::Vertical => area.height,
        };

        let mut sizes = vec![0u16; self.constraints.len()];
        for (i, c) in self.constraints.iter().enumerate() {
            sizes[i] = match c {
                Constraint::Fixed(v) => *v,
                Constraint::Percentage(p) => (total as u32 * *p as u32 / 100) as u16,
                Constraint::Min(v) => *v,
                Constraint::Fill(_) => 0, // resolved in Task 9
            };
        }

        let mut rects = Vec::with_capacity(sizes.len());
        let mut offset = 0u16;
        for &size in &sizes {
            let rect = match self.direction {
                Direction::Horizontal => Rect { x: area.x + offset, y: area.y, width: size, height: area.height },
                Direction::Vertical => Rect { x: area.x, y: area.y + offset, width: area.width, height: size },
            };
            rects.push(rect);
            offset += size;
        }
        rects
    }
}
```

- [ ] **Step 4: Add the module to `src/lib.rs`**

```rust
pub mod layout;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test layout::`
Expected: PASS (2 tests)

- [ ] **Step 6: Commit**

```bash
git add src/layout.rs src/lib.rs
git commit -m "feat: add Layout with Fixed/Percentage/Min splitting"
```

#### Task 9: `Fill` weight distribution

**Files:**
- Modify: `src/layout.rs`
- Test: `src/layout.rs` (inline `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `Rect`, `Direction`, `Constraint`, `Layout` (Task 8).
- Produces: `Layout::split` now resolves `Constraint::Fill` by
  distributing remaining space by weight (signature unchanged).

- [ ] **Step 1: Write the failing test**

```rust
// add to the existing tests module in src/layout.rs
#[test]
fn fill_constraints_split_remaining_space_by_weight() {
    let area = Rect { x: 0, y: 0, width: 10, height: 1 };
    let layout = Layout::new(Direction::Horizontal, vec![Constraint::Fixed(4), Constraint::Fill(1), Constraint::Fill(1)]);

    let rects = layout.split(area);

    assert_eq!(rects, vec![
        Rect { x: 0, y: 0, width: 4, height: 1 },
        Rect { x: 4, y: 0, width: 3, height: 1 },
        Rect { x: 7, y: 0, width: 3, height: 1 },
    ]);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test layout::`
Expected: FAIL — `Fill` regions come back with `width: 0` (from Task 8's
placeholder resolution), not the expected 3/3 split.

- [ ] **Step 3: Update the implementation**

```rust
// src/layout.rs, replace the sizes-computation block inside Layout::split
let mut sizes = vec![0u16; self.constraints.len()];
let mut used = 0u16;
let mut fill_indices = Vec::new();
let mut fill_weight_total = 0u32;

for (i, c) in self.constraints.iter().enumerate() {
    match c {
        Constraint::Fixed(v) => { sizes[i] = *v; used += v; }
        Constraint::Percentage(p) => {
            let v = (total as u32 * *p as u32 / 100) as u16;
            sizes[i] = v; used += v;
        }
        Constraint::Min(v) => { sizes[i] = *v; used += v; }
        Constraint::Fill(w) => { fill_indices.push(i); fill_weight_total += *w as u32; }
    }
}

let remaining = total.saturating_sub(used);
if fill_weight_total > 0 {
    for &i in &fill_indices {
        if let Constraint::Fill(w) = self.constraints[i] {
            sizes[i] = (remaining as u32 * w as u32 / fill_weight_total) as u16;
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test layout::`
Expected: PASS (3 tests)

- [ ] **Step 5: Commit**

```bash
git add src/layout.rs
git commit -m "feat: resolve Fill constraints by weight in Layout::split"
```

#### Task 10: `margin` and `spacing`

**Files:**
- Modify: `src/layout.rs`
- Test: `src/layout.rs` (inline `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `Layout` (Tasks 8-9).
- Produces: `Layout::margin(self, m: u16) -> Layout`,
  `Layout::spacing(self, s: u16) -> Layout` (builder methods).

- [ ] **Step 1: Write the failing test**

```rust
// add to the existing tests module in src/layout.rs
#[test]
fn margin_and_spacing_shrink_and_separate_children() {
    let area = Rect { x: 0, y: 0, width: 10, height: 5 };
    let layout = Layout::new(Direction::Horizontal, vec![Constraint::Fixed(2), Constraint::Fixed(2)])
        .margin(1)
        .spacing(1);

    let rects = layout.split(area);

    assert_eq!(rects, vec![
        Rect { x: 1, y: 1, width: 2, height: 3 },
        Rect { x: 4, y: 1, width: 2, height: 3 },
    ]);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test layout::`
Expected: FAIL — no `margin`/`spacing` methods exist yet.

- [ ] **Step 3: Write the implementation**

```rust
// src/layout.rs, in the Layout impl block
pub fn margin(mut self, m: u16) -> Self { self.margin = m; self }
pub fn spacing(mut self, s: u16) -> Self { self.spacing = s; self }
```

```rust
// src/layout.rs, at the top of Layout::split, before computing `total`
let area = Rect {
    x: area.x + self.margin,
    y: area.y + self.margin,
    width: area.width.saturating_sub(self.margin * 2),
    height: area.height.saturating_sub(self.margin * 2),
};
let n = self.constraints.len() as u16;
let spacing_total = if n > 0 { self.spacing * (n - 1) } else { 0 };
```

```rust
// src/layout.rs, in Layout::split: subtract spacing_total from `total`
// before distributing sizes, and add self.spacing after each `offset +=`
let total = total.saturating_sub(spacing_total);
```

```rust
// src/layout.rs, in the rect-building loop at the end of Layout::split
offset += size + self.spacing;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test layout::`
Expected: PASS (4 tests)

- [ ] **Step 5: Commit**

```bash
git add src/layout.rs
git commit -m "feat: add margin and spacing to Layout"
```

---

## Arc 4: Widget Set

### Slice 4.1: Text widget

**Tags:** coding

#### Task 11: `Text` widget

**Files:**
- Create: `src/widgets/mod.rs`
- Create: `src/widgets/text.rs`
- Modify: `src/lib.rs`
- Test: `src/widgets/text.rs` (inline `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `Buffer`, `Cell` (Task 2), `Rect` (Task 8).
- Produces: `Text::new(content: &str) -> Text`, `Text::render(&self, area:
  Rect, buf: &mut Buffer)`.

- [ ] **Step 1: Write the failing test**

```rust
// src/widgets/text.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::Buffer;
    use crate::layout::Rect;

    #[test]
    fn renders_characters_left_to_right() {
        let mut buf = Buffer::new(5, 1);
        let area = Rect { x: 0, y: 0, width: 5, height: 1 };

        Text::new("hi").render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, 'h');
        assert_eq!(buf.get(1, 0).symbol, 'i');
        assert_eq!(buf.get(2, 0).symbol, ' '); // untouched, still default
    }

    #[test]
    fn truncates_content_wider_than_the_area() {
        let mut buf = Buffer::new(2, 1);
        let area = Rect { x: 0, y: 0, width: 2, height: 1 };

        Text::new("hello").render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, 'h');
        assert_eq!(buf.get(1, 0).symbol, 'e');
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test widgets::text::`
Expected: FAIL to compile — `Text` not defined yet.

- [ ] **Step 3: Write the implementation**

```rust
// src/widgets/text.rs, above the tests module
use crossterm::style::Color;
use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;

pub struct Text<'a> {
    content: &'a str,
    fg: Color,
    bg: Color,
}

impl<'a> Text<'a> {
    pub fn new(content: &'a str) -> Self {
        Text { content, fg: Color::Reset, bg: Color::Reset }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        for (i, ch) in self.content.chars().take(area.width as usize).enumerate() {
            buf.set(area.x + i as u16, area.y, Cell { symbol: ch, fg: self.fg, bg: self.bg });
        }
    }
}
```

```rust
// src/widgets/mod.rs
pub mod text;
```

- [ ] **Step 4: Add the module to `src/lib.rs`**

```rust
pub mod widgets;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test widgets::text::`
Expected: PASS (2 tests)

- [ ] **Step 6: Commit**

```bash
git add src/widgets/mod.rs src/widgets/text.rs src/lib.rs
git commit -m "feat: add Text widget"
```

### Slice 4.2: Block widget (opt-in border/title)

**Tags:** coding

#### Task 12: `Block` widget

**Files:**
- Create: `src/widgets/block.rs`
- Modify: `src/widgets/mod.rs`
- Test: `src/widgets/block.rs` (inline `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `Buffer`, `Cell` (Task 2), `Rect` (Task 8).
- Produces: `Block::new() -> Block`, `Block::title(self, t: &str) ->
  Block`, `Block::render(&self, area: Rect, buf: &mut Buffer) -> Rect`
  (returns the inner content area).

- [ ] **Step 1: Write the failing test**

```rust
// src/widgets/block.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::Buffer;
    use crate::layout::Rect;

    #[test]
    fn draws_border_and_returns_inner_area() {
        let mut buf = Buffer::new(4, 3);
        let area = Rect { x: 0, y: 0, width: 4, height: 3 };

        let inner = Block::new().render(area, &mut buf);

        assert_eq!(inner, Rect { x: 1, y: 1, width: 2, height: 1 });
        assert_eq!(buf.get(0, 0).symbol, '+');
        assert_eq!(buf.get(1, 0).symbol, '-');
        assert_eq!(buf.get(0, 1).symbol, '|');
    }

    #[test]
    fn title_is_drawn_on_the_top_border() {
        let mut buf = Buffer::new(6, 3);
        let area = Rect { x: 0, y: 0, width: 6, height: 3 };

        Block::new().title("Hi").render(area, &mut buf);

        assert_eq!(buf.get(1, 0).symbol, 'H');
        assert_eq!(buf.get(2, 0).symbol, 'i');
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test widgets::block::`
Expected: FAIL to compile — `Block` not defined yet.

- [ ] **Step 3: Write the implementation**

```rust
// src/widgets/block.rs, above the tests module
use crossterm::style::Color;
use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;

pub struct Block<'a> {
    title: Option<&'a str>,
}

impl<'a> Block<'a> {
    pub fn new() -> Self {
        Block { title: None }
    }

    pub fn title(mut self, t: &'a str) -> Self {
        self.title = Some(t);
        self
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) -> Rect {
        if area.width < 2 || area.height < 2 {
            return area;
        }
        let plain = || Cell { symbol: ' ', fg: Color::Reset, bg: Color::Reset };
        for x in area.x..area.x + area.width {
            buf.set(x, area.y, Cell { symbol: '-', ..plain() });
            buf.set(x, area.y + area.height - 1, Cell { symbol: '-', ..plain() });
        }
        for y in area.y..area.y + area.height {
            buf.set(area.x, y, Cell { symbol: '|', ..plain() });
            buf.set(area.x + area.width - 1, y, Cell { symbol: '|', ..plain() });
        }
        buf.set(area.x, area.y, Cell { symbol: '+', ..plain() });
        buf.set(area.x + area.width - 1, area.y, Cell { symbol: '+', ..plain() });
        buf.set(area.x, area.y + area.height - 1, Cell { symbol: '+', ..plain() });
        buf.set(area.x + area.width - 1, area.y + area.height - 1, Cell { symbol: '+', ..plain() });

        if let Some(title) = self.title {
            for (i, ch) in title.chars().take(area.width.saturating_sub(2) as usize).enumerate() {
                buf.set(area.x + 1 + i as u16, area.y, Cell { symbol: ch, ..plain() });
            }
        }

        Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        }
    }
}
```

Note: `Cell { symbol: '-', ..plain() }` requires `Cell` to be
constructible via struct-update syntax, which it already is (all fields
public, `plain()` returns an owned `Cell`).

```rust
// src/widgets/mod.rs
pub mod block;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test widgets::block::`
Expected: PASS (2 tests)

- [ ] **Step 5: Commit**

```bash
git add src/widgets/mod.rs src/widgets/block.rs
git commit -m "feat: add Block widget with opt-in border and title"
```

### Slice 4.3: List widget

**Tags:** coding

#### Task 13: `List` widget

**Files:**
- Create: `src/widgets/list.rs`
- Modify: `src/widgets/mod.rs`
- Test: `src/widgets/list.rs` (inline `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `Buffer`, `Cell` (Task 2), `Rect` (Task 8).
- Produces: `List::new(items: &[String], selected: usize) -> List`,
  `List::render(&self, area: Rect, buf: &mut Buffer)`.

- [ ] **Step 1: Write the failing test**

```rust
// src/widgets/list.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::Buffer;
    use crate::layout::Rect;
    use crossterm::style::Color;

    #[test]
    fn renders_each_item_on_its_own_row() {
        let items = vec!["one".to_string(), "two".to_string()];
        let mut buf = Buffer::new(5, 2);
        let area = Rect { x: 0, y: 0, width: 5, height: 2 };

        List::new(&items, 0).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, 'o');
        assert_eq!(buf.get(0, 1).symbol, 't');
    }

    #[test]
    fn selected_row_is_highlighted() {
        let items = vec!["one".to_string(), "two".to_string()];
        let mut buf = Buffer::new(5, 2);
        let area = Rect { x: 0, y: 0, width: 5, height: 2 };

        List::new(&items, 1).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).bg, Color::Reset);
        assert_eq!(buf.get(0, 1).bg, Color::White);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test widgets::list::`
Expected: FAIL to compile — `List` not defined yet.

- [ ] **Step 3: Write the implementation**

```rust
// src/widgets/list.rs, above the tests module
use crossterm::style::Color;
use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;

pub struct List<'a> {
    items: &'a [String],
    selected: usize,
}

impl<'a> List<'a> {
    pub fn new(items: &'a [String], selected: usize) -> Self {
        List { items, selected }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        for (row, item) in self.items.iter().take(area.height as usize).enumerate() {
            let (fg, bg) = if row == self.selected {
                (Color::Black, Color::White)
            } else {
                (Color::Reset, Color::Reset)
            };
            for x in 0..area.width {
                buf.set(area.x + x, area.y + row as u16, Cell { symbol: ' ', fg, bg });
            }
            for (i, ch) in item.chars().take(area.width as usize).enumerate() {
                buf.set(area.x + i as u16, area.y + row as u16, Cell { symbol: ch, fg, bg });
            }
        }
    }
}
```

```rust
// src/widgets/mod.rs
pub mod list;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test widgets::list::`
Expected: PASS (2 tests)

- [ ] **Step 5: Commit**

```bash
git add src/widgets/mod.rs src/widgets/list.rs
git commit -m "feat: add List widget with selection highlight"
```

### Slice 4.4: Table widget

**Tags:** coding

#### Task 14: `Table` widget

**Files:**
- Create: `src/widgets/table.rs`
- Modify: `src/widgets/mod.rs`
- Test: `src/widgets/table.rs` (inline `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `Buffer`, `Cell` (Task 2), `Rect` (Task 8).
- Produces: `Table::new(headers: &[String], rows: &[Vec<String>],
  selected: usize, col_width: u16) -> Table`, `Table::render(&self, area:
  Rect, buf: &mut Buffer)`.

- [ ] **Step 1: Write the failing test**

```rust
// src/widgets/table.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::Buffer;
    use crate::layout::Rect;
    use crossterm::style::Color;

    #[test]
    fn renders_header_row_then_data_rows() {
        let headers = vec!["Name".to_string()];
        let rows = vec![vec!["svc-a".to_string()], vec!["svc-b".to_string()]];
        let mut buf = Buffer::new(10, 3);
        let area = Rect { x: 0, y: 0, width: 10, height: 3 };

        Table::new(&headers, &rows, 0, 5).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, 'N'); // header row
        assert_eq!(buf.get(0, 1).symbol, 's'); // first data row
    }

    #[test]
    fn selected_data_row_is_highlighted_not_the_header() {
        let headers = vec!["Name".to_string()];
        let rows = vec![vec!["svc-a".to_string()], vec!["svc-b".to_string()]];
        let mut buf = Buffer::new(10, 3);
        let area = Rect { x: 0, y: 0, width: 10, height: 3 };

        Table::new(&headers, &rows, 1, 5).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).bg, Color::Reset);
        assert_eq!(buf.get(0, 1).bg, Color::Reset);
        assert_eq!(buf.get(0, 2).bg, Color::White);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test widgets::table::`
Expected: FAIL to compile — `Table` not defined yet.

- [ ] **Step 3: Write the implementation**

```rust
// src/widgets/table.rs, above the tests module
use crossterm::style::Color;
use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;

pub struct Table<'a> {
    headers: &'a [String],
    rows: &'a [Vec<String>],
    selected: usize,
    col_width: u16,
}

impl<'a> Table<'a> {
    pub fn new(headers: &'a [String], rows: &'a [Vec<String>], selected: usize, col_width: u16) -> Self {
        Table { headers, rows, selected, col_width }
    }

    fn render_row(&self, area: Rect, y: u16, cells: &[String], fg: Color, bg: Color, buf: &mut Buffer) {
        let mut x = area.x;
        for cell in cells {
            for i in 0..self.col_width {
                if x + i >= area.x + area.width {
                    break;
                }
                let ch = cell.chars().nth(i as usize).unwrap_or(' ');
                buf.set(x + i, y, Cell { symbol: ch, fg, bg });
            }
            x += self.col_width;
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        self.render_row(area, area.y, self.headers, Color::Reset, Color::Reset, buf);
        for (row_idx, row) in self.rows.iter().take(area.height.saturating_sub(1) as usize).enumerate() {
            let (fg, bg) = if row_idx == self.selected {
                (Color::Black, Color::White)
            } else {
                (Color::Reset, Color::Reset)
            };
            self.render_row(area, area.y + 1 + row_idx as u16, row, fg, bg, buf);
        }
    }
}
```

```rust
// src/widgets/mod.rs
pub mod table;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test widgets::table::`
Expected: PASS (2 tests)

- [ ] **Step 5: Commit**

```bash
git add src/widgets/mod.rs src/widgets/table.rs
git commit -m "feat: add Table widget with row selection highlight"
```

---

## Arc 5: App Runtime & Demo

### Slice 5.1: App trait and event loop

**Tags:** coding

#### Task 15: `App` trait and `run` event loop

**Files:**
- Create: `src/app.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Consumes: `Buffer`, `diff` (Tasks 2-3), `Rect` (Task 8), `Terminal`,
  `install_panic_hook` (Tasks 4-7).
- Produces: `trait App { fn update(&mut self, event: &crossterm::event::Event); fn view(&self, area: Rect, buf: &mut Buffer); fn should_quit(&self) -> bool; }`,
  `run<A: App>(app: &mut A) -> std::io::Result<()>`.

This task has no automated test — it wires together every earlier piece
into the live event loop, which per the spec's accepted testing gap is
verified manually (Task 17), not with `cargo test`.

- [ ] **Step 1: Write the implementation**

```rust
// src/app.rs
use std::time::Duration;
use crossterm::event::Event;
use crate::buffer::{diff, Buffer};
use crate::layout::Rect;
use crate::terminal::{install_panic_hook, Terminal};

pub trait App {
    fn update(&mut self, event: &Event);
    fn view(&self, area: Rect, buf: &mut Buffer);
    fn should_quit(&self) -> bool;
}

pub fn run<A: App>(app: &mut A) -> std::io::Result<()> {
    install_panic_hook();
    let mut term = Terminal::new()?;

    let (w, h) = term.size()?;
    let mut prev = Buffer::new(w, h);
    let mut next = Buffer::new(w, h);
    app.view(Rect { x: 0, y: 0, width: w, height: h }, &mut next);
    term.draw_diff(&diff(&prev, &next))?;
    prev = next;

    loop {
        if let Some(event) = term.next_event(Duration::from_millis(250))? {
            app.update(&event);
            if app.should_quit() {
                break;
            }
            let (w, h) = term.size()?;
            let mut next = Buffer::new(w, h);
            app.view(Rect { x: 0, y: 0, width: w, height: h }, &mut next);
            term.draw_diff(&diff(&prev, &next))?;
            prev = next;
        }
    }
    Ok(())
}
```

- [ ] **Step 2: Add the module to `src/lib.rs`**

```rust
pub mod app;
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build`
Expected: builds successfully.

- [ ] **Step 4: Commit**

```bash
git add src/app.rs src/lib.rs
git commit -m "feat: add App trait and event-driven run loop"
```

### Slice 5.2: Demo dashboard app

**Tags:** coding

#### Task 16: Demo app — panes, focus, navigation

**Files:**
- Create: `examples/demo.rs`

**Interfaces:**
- Consumes: `App`, `run` (Task 15), `Layout`, `Direction`, `Constraint`,
  `Rect` (Tasks 8-10), `Text`, `Block`, `List`, `Table` (Tasks 11-14),
  `Buffer` (Task 2).
- Produces: nothing consumed by later tasks — this is the v1 success
  criterion itself.

`update`'s selection-clamping logic (`.min(len - 1)`) assumes non-empty
`list_items`/`table_rows`, which the hardcoded demo data guarantees; this
is app code, not framework code, so it doesn't need to defend against
inputs the demo never produces.

- [ ] **Step 1: Write the demo app**

```rust
// examples/demo.rs
use ttui::app::{run, App};
use ttui::buffer::Buffer;
use ttui::layout::{Constraint, Direction, Layout, Rect};
use ttui::widgets::{block::Block, list::List, table::Table, text::Text};
use crossterm::event::{Event, KeyCode};

#[derive(PartialEq, Clone, Copy)]
enum Focus {
    List,
    Table,
}

struct Demo {
    list_items: Vec<String>,
    list_selected: usize,
    table_headers: Vec<String>,
    table_rows: Vec<Vec<String>>,
    table_selected: usize,
    focus: Focus,
    quit: bool,
}

impl Demo {
    fn new() -> Self {
        Demo {
            list_items: vec!["Alpha".into(), "Beta".into(), "Gamma".into()],
            list_selected: 0,
            table_headers: vec!["Name".into(), "Status".into()],
            table_rows: vec![
                vec!["svc-a".into(), "ok".into()],
                vec!["svc-b".into(), "down".into()],
            ],
            table_selected: 0,
            focus: Focus::List,
            quit: false,
        }
    }
}

impl App for Demo {
    fn update(&mut self, event: &Event) {
        let Event::Key(k) = event else { return };
        match k.code {
            KeyCode::Tab => {
                self.focus = match self.focus {
                    Focus::List => Focus::Table,
                    Focus::Table => Focus::List,
                };
            }
            KeyCode::Down => match self.focus {
                Focus::List => self.list_selected = (self.list_selected + 1).min(self.list_items.len() - 1),
                Focus::Table => self.table_selected = (self.table_selected + 1).min(self.table_rows.len() - 1),
            },
            KeyCode::Up => match self.focus {
                Focus::List => self.list_selected = self.list_selected.saturating_sub(1),
                Focus::Table => self.table_selected = self.table_selected.saturating_sub(1),
            },
            KeyCode::Char('q') => self.quit = true,
            _ => {}
        }
    }

    fn view(&self, area: Rect, buf: &mut Buffer) {
        let rows = Layout::new(Direction::Vertical, vec![Constraint::Fill(1), Constraint::Fixed(1)]).split(area);
        let cols = Layout::new(Direction::Horizontal, vec![Constraint::Percentage(40), Constraint::Fill(1)]).split(rows[0]);

        let list_inner = Block::new().title("Items").render(cols[0], buf);
        List::new(&self.list_items, self.list_selected).render(list_inner, buf);

        let table_inner = Block::new().title("Services").render(cols[1], buf);
        Table::new(&self.table_headers, &self.table_rows, self.table_selected, 8).render(table_inner, buf);

        Text::new("Tab: switch focus | Up/Down: navigate | q: quit").render(rows[1], buf);
    }

    fn should_quit(&self) -> bool {
        self.quit
    }
}

fn main() -> std::io::Result<()> {
    let mut demo = Demo::new();
    run(&mut demo)
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build --examples`
Expected: builds successfully.

- [ ] **Step 3: Commit**

```bash
git add examples/demo.rs
git commit -m "feat: add demo dashboard app"
```

### Slice 5.3: Manual verification against the v1 success criteria

**Tags:** coding, admin

#### Task 17: Run the demo and check it against the spec's success criteria

**Files:**
- None (manual verification task, no code changes).

**Interfaces:**
- Consumes: the whole crate.
- Produces: nothing — this is the plan's final acceptance check.

- [ ] **Step 1: Run the demo in a real terminal**

Run: `cargo run --example demo`

- [ ] **Step 2: Check each item from the spec's "Success criteria" section**

  - [ ] Multiple panes are visible via nested layout splits (List pane,
    Table pane, footer text row).
  - [ ] At least one `Block`-bordered pane is visible (both panes are,
    here).
  - [ ] `Text`, `List`, and `Table` widgets are all visibly rendering.
  - [ ] Pressing `Tab` moves the highlighted selection between the List
    pane and the Table pane.
  - [ ] Pressing `Up`/`Down` moves the highlighted row within whichever
    pane is currently focused.
  - [ ] Pressing `q` exits cleanly and the terminal is restored to its
    normal (non-raw, non-alternate-screen) state — the shell prompt is
    usable immediately after exit, with no leftover garbled state.

- [ ] **Step 3: Check terminal-safety on a crash path**

  Temporarily insert `panic!("manual crash check")` at the top of
  `Demo::update`, run `cargo run --example demo`, press any key to
  trigger it, and confirm the terminal is restored (not stuck in raw
  mode / alternate screen) even though the app panicked. Then remove the
  `panic!` line — it was only for this check, not a permanent change.

- [ ] **Step 4: Record the result**

If every box in Step 2 and the Step 3 check pass, v1 is done. If
anything fails, open a follow-up task (or fix inline, per
`superpowers:executing-plans`/`superpowers:subagent-driven-development`
convention) before considering this plan complete — do not mark this
task done with a known failure.
