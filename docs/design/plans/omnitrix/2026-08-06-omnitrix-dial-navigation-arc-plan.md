# Omnitrix Dial + Navigation Arc Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `docs/design/specs/2026-08-06-omnitrix-dial-navigation-arc-design.md`
(issues **#87** Dial widget, **#45** thick border, **#43** `AppMode` enum,
**#88** Faceplate-to-Dial revamp, **#44** corruption transition): a new
core `Dial` widget, a `Theme.border_thick` double-border capability, and
Omnitrix's Faceplate hub rewired onto the roadmap's `AppMode` shape with
a real dial visual and a corruption transition between modes.

**Architecture:** Six tasks. Tasks 1-3 are core-framework (`src/`),
TDD-mandatory, independent of each other and of Tasks 4-6. Tasks 4-6 are
all `examples/omnitrix.rs`, strictly sequential (each depends on the
previous), and are example code — verified by running, not unit tested,
per this repo's TDD exceptions. Task order: Dial widget (1), Theme field
(2), Block rendering (3) — any order among 1-3 is fine, but 2 must
precede 3 since 3 consumes 2's field — then AppMode (4), Dial swap (5),
transition (6) in that fixed order since each rewrites the file the next
one edits.

**Tech Stack:** Rust, `crossterm` (unchanged). No new dependencies —
Slice 5's Braille noise uses a deterministic hash, not an RNG crate.

## Global Constraints

- TDD mandatory for Tasks 1-3 (`coding`-tagged, no exception applies).
  Tasks 4-6 (`examples/omnitrix.rs`) are example code — per
  `.claude/rules/development-conventions.md`'s TDD exceptions, verified
  by running the example, not unit tested.
- Inline `#[cfg(test)] mod tests` per module — no new `tests/` directory.
- `cargo fmt` / `cargo clippy --all-targets -- -D warnings` clean after
  every task.
- No new dependencies.
- Out of scope (do not build): a gauge/progress mode for `Dial`; real
  sub-app content for Brainstorm/Fasttrack/Upgrade (they stay
  placeholder screens); wiring `border_thick` into any example's theme
  (Task 2/3 add the capability only — no consumer in this plan, same as
  how `border_bold`'s Task 1 shipped a placeholder `false` before a
  separate ticket wired real logic).
- `Dial` takes no `Theme` parameter, matching `List`/`Table`/`Text`'s
  existing precedent (only `Block` takes a theme in this codebase).

---

### Task 1: Dial widget (`src/widgets/dial.rs`, #87)

**Files:**
- Create: `src/widgets/dial.rs`
- Modify: `src/widgets/mod.rs`

**Interfaces produced:**
```rust
pub struct Dial<'a> {
    items: &'a [String],
    selected: usize,
}

impl<'a> Dial<'a> {
    pub fn new(items: &'a [String], selected: usize) -> Self;
    pub fn render(&self, area: Rect, buf: &mut Buffer);
}
```

**Geometry reference** (all tests below are derived from these exact
formulas — do not approximate):
- `cx = area.x as f32 + area.width as f32 / 2.0`, `cy` likewise for height.
- `radius_y = ((area.height as i32 / 2 - 1).max(1)) as f32`
- `radius_x = (((radius_y as i32) * 2).min(area.width as i32 / 2 - 1).max(1)) as f32`
- item `i` of `n`: `angle = i as f32 * TAU / n as f32 - FRAC_PI_2`;
  point `(cx + radius_x * angle.cos(), cy + radius_y * angle.sin())`,
  each coordinate rounded with `f32::round` (ties away from zero) then
  cast to the cell position.
- ring dots: for each adjacent pair `(i, (i+1) % n)`, subdivide the arc
  into 4 steps and plot the 3 intermediate angles (`t = 0.25, 0.5, 0.75`)
  with the same point formula; wrap the second angle by `+ TAU` when
  `i + 1 == n` so interpolation goes forward around the circle instead
  of backward.
- pointer glyph for the selected item: same point formula with
  `radius_x * 0.5` / `radius_y * 0.5`.
- label direction: if the point's un-rounded `x >= cx`, left-align
  starting at the point (extends right); else right-align ending at the
  point (extends left). Clip any character whose column falls outside
  `[area.x, area.x + area.width)`.

- [ ] **Step 1: Write the failing tests** — create `src/widgets/dial.rs`
  with just the test module (no implementation yet):

```rust
use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;
use crossterm::style::Color;

pub struct Dial<'a> {
    items: &'a [String],
    selected: usize,
}

impl<'a> Dial<'a> {
    pub fn new(items: &'a [String], selected: usize) -> Self {
        Dial { items, selected }
    }

    pub fn render(&self, _area: Rect, _buf: &mut Buffer) {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_zero_lands_at_top_center_column() {
        let items = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let mut buf = Buffer::new(10, 8);
        let area = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 8,
        };

        Dial::new(&items, 0).render(area, &mut buf);

        assert_eq!(buf.get(5, 1).symbol, 'A');
    }

    #[test]
    fn items_are_symmetric_left_and_right_for_odd_item_count() {
        let items = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let mut buf = Buffer::new(10, 8);
        let area = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 8,
        };

        Dial::new(&items, 0).render(area, &mut buf);

        assert_eq!(buf.get(8, 6).symbol, 'B');
        assert_eq!(buf.get(2, 6).symbol, 'C');
    }

    #[test]
    fn ring_dots_never_land_on_an_item_point() {
        let items = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let mut buf = Buffer::new(10, 8);
        let area = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 8,
        };

        Dial::new(&items, 0).render(area, &mut buf);

        assert_eq!(buf.get(7, 1).symbol, '.');
        assert_eq!(buf.get(8, 3).symbol, '.');
        assert_eq!(buf.get(9, 4).symbol, '.');
    }

    #[test]
    fn selected_items_label_and_pointer_are_highlighted() {
        let items = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let mut buf = Buffer::new(10, 8);
        let area = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 8,
        };

        Dial::new(&items, 0).render(area, &mut buf);

        assert_eq!(buf.get(5, 1).fg, Color::Black);
        assert_eq!(buf.get(5, 1).bg, Color::White);
        assert_eq!(buf.get(5, 3).symbol, '*');
        assert_eq!(buf.get(8, 6).bg, Color::Reset);
    }

    #[test]
    fn labels_flow_outward_and_clip_at_area_edges() {
        let items = vec![
            "TOP".to_string(),
            "RIGHT".to_string(),
            "BOTTOM".to_string(),
            "LEFT".to_string(),
        ];
        let mut buf = Buffer::new(6, 4);
        let area = Rect {
            x: 0,
            y: 0,
            width: 6,
            height: 4,
        };

        Dial::new(&items, 0).render(area, &mut buf);

        // item1 "RIGHT" starts at column 5 flowing right; the area is
        // only 6 columns wide (0..5), so only 'R' fits.
        assert_eq!(buf.get(5, 2).symbol, 'R');
        // item3 "LEFT" ends at column 1 flowing left; only 'F' and 'T'
        // fit before the area's left edge clips the rest.
        assert_eq!(buf.get(0, 2).symbol, 'F');
        assert_eq!(buf.get(1, 2).symbol, 'T');
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib widgets::dial::tests`
Expected: all 5 tests FAIL (panic: `not implemented`).

- [ ] **Step 3: Implement** — replace the `render` method body (and add
  the free function below it) in `src/widgets/dial.rs`:

```rust
impl<'a> Dial<'a> {
    pub fn new(items: &'a [String], selected: usize) -> Self {
        Dial { items, selected }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let n = self.items.len();
        if n == 0 || area.width == 0 || area.height == 0 {
            return;
        }

        let cx = area.x as f32 + area.width as f32 / 2.0;
        let cy = area.y as f32 + area.height as f32 / 2.0;
        let radius_y = ((area.height as i32 / 2 - 1).max(1)) as f32;
        let radius_x = (((radius_y as i32) * 2)
            .min(area.width as i32 / 2 - 1)
            .max(1)) as f32;

        let angle_of = |i: usize| -> f32 {
            i as f32 * std::f32::consts::TAU / n as f32 - std::f32::consts::FRAC_PI_2
        };
        let point_at = |angle: f32, rx: f32, ry: f32| -> (f32, f32) {
            (cx + rx * angle.cos(), cy + ry * angle.sin())
        };

        for i in 0..n {
            let a0 = angle_of(i);
            let a1 = angle_of((i + 1) % n)
                + if i + 1 == n {
                    std::f32::consts::TAU
                } else {
                    0.0
                };
            for step in 1..4 {
                let t = step as f32 / 4.0;
                let angle = a0 + (a1 - a0) * t;
                let (x, y) = point_at(angle, radius_x, radius_y);
                let (px, py) = (x.round() as i32, y.round() as i32);
                if in_area(px, py, area) {
                    buf.set(
                        px as u16,
                        py as u16,
                        Cell {
                            symbol: '.',
                            ..Default::default()
                        },
                    );
                }
            }
        }

        for (i, item) in self.items.iter().enumerate() {
            let angle = angle_of(i);
            let (x, y) = point_at(angle, radius_x, radius_y);
            let (px, py) = (x.round() as i32, y.round() as i32);
            let selected = i == self.selected;
            let (fg, bg) = if selected {
                (Color::Black, Color::White)
            } else {
                (Color::Reset, Color::Reset)
            };

            let chars: Vec<char> = item.chars().collect();
            if x >= cx {
                for (offset, ch) in chars.iter().enumerate() {
                    let cell_x = px + offset as i32;
                    if in_area(cell_x, py, area) {
                        buf.set(
                            cell_x as u16,
                            py as u16,
                            Cell {
                                symbol: *ch,
                                fg,
                                bg,
                                ..Default::default()
                            },
                        );
                    }
                }
            } else {
                let len = chars.len() as i32;
                for (offset, ch) in chars.iter().enumerate() {
                    let cell_x = px - (len - 1 - offset as i32);
                    if in_area(cell_x, py, area) {
                        buf.set(
                            cell_x as u16,
                            py as u16,
                            Cell {
                                symbol: *ch,
                                fg,
                                bg,
                                ..Default::default()
                            },
                        );
                    }
                }
            }

            if selected {
                let (px2, py2) = point_at(angle, radius_x * 0.5, radius_y * 0.5);
                let (ppx, ppy) = (px2.round() as i32, py2.round() as i32);
                if in_area(ppx, ppy, area) {
                    buf.set(
                        ppx as u16,
                        ppy as u16,
                        Cell {
                            symbol: '*',
                            ..Default::default()
                        },
                    );
                }
            }
        }
    }
}

fn in_area(x: i32, y: i32, area: Rect) -> bool {
    x >= area.x as i32
        && x < area.x as i32 + area.width as i32
        && y >= area.y as i32
        && y < area.y as i32 + area.height as i32
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib widgets::dial::tests`
Expected: all 5 tests PASS.

- [ ] **Step 5: Register the module** — add to `src/widgets/mod.rs`:

```rust
pub mod block;
pub mod dial;
pub mod list;
pub mod table;
pub mod text;
```

- [ ] **Step 6: Run full check**

Run: `cargo test --lib && cargo build --examples && cargo fmt && cargo clippy --all-targets -- -D warnings`
Expected: all pass, no warnings.

- [ ] **Step 7: Commit**

```bash
git add src/widgets/dial.rs src/widgets/mod.rs
git commit -m "feat(widgets): add Dial navigation widget (#87)"
```

---

### Task 2: `Theme.border_thick` field (`src/theme.rs`, #45)

**Files:**
- Modify: `src/theme.rs`
- Modify: `examples/smash_crabs.rs` (mechanical exhaustive-literal fix)
- Modify: `examples/omnitrix.rs` (mechanical exhaustive-literal fix —
  placeholder `false`, no consumer in this plan)
- Modify: `src/widgets/block.rs` (mechanical exhaustive-literal fix on
  its 3 existing `Theme { .. }` test literals; Task 3 adds the real
  behavior tests)

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
    pub border_thick: bool,
}
```
`Theme::default().border_thick == false`.

- [ ] **Step 1: Write the failing test** — add to `src/theme.rs`'s
  existing `mod tests`:

```rust
#[test]
fn default_theme_border_thick_is_false() {
    assert!(!Theme::default().border_thick);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib theme::tests::default_theme_border_thick_is_false`
Expected: FAIL (`no field \`border_thick\` on type \`Theme\``)

- [ ] **Step 3: Implement** — in `src/theme.rs`, add the field to the
  `Theme` struct definition and to `impl Default for Theme`:

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub background: Color,
    pub primary: Color,
    pub secondary: Color,
    pub tertiary: Color,
    pub accent: Color,
    pub border: BorderSet,
    pub border_bold: bool,
    pub border_thick: bool,
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
            border_thick: false,
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib theme::tests`
Expected: PASS (all of `theme::tests`, old and new)

- [ ] **Step 5: Fix every exhaustive `Theme { .. }` literal the compiler
  now flags.** Run `cargo build --examples 2>&1` and `cargo test --lib 2>&1`
  to find them — expected sites:
  - `examples/smash_crabs.rs`, `arena_theme()` (currently ends with
    `border_bold: false,` before the closing brace): add
    `border_thick: false,` right after it.
  - `examples/omnitrix.rs`, `theme()` (currently ends with
    `border_bold: brightness > 0.6,`): add `border_thick: false,` right
    after it — placeholder only, no ticket in this plan wires it to real
    logic.
  - `src/widgets/block.rs`'s 3 existing test literals (`border_bold: false,`
    at one site, `border_bold: true,` at two sites): add
    `border_thick: false,` right after `border_bold` in each — none of
    these 3 pre-existing tests assert on thickness, so `false` keeps
    them unchanged; Task 3 adds new tests that do assert on it.
  Do not change any other behavior in these files — purely additive.

- [ ] **Step 6: Run full check**

Run: `cargo test --lib && cargo build --examples && cargo fmt && cargo clippy --all-targets -- -D warnings`
Expected: all pass, no warnings.

- [ ] **Step 7: Commit**

```bash
git add src/theme.rs examples/smash_crabs.rs examples/omnitrix.rs src/widgets/block.rs
git commit -m "feat(theme): add Theme.border_thick field (#45)"
```

---

### Task 3: `Block::render` draws a second ring when `border_thick` (`src/widgets/block.rs`, #45)

**Files:**
- Modify: `src/widgets/block.rs`

**Interfaces consumed:**
- `Theme.border_thick: bool` (Task 2)
- `Buffer.width: u16`, `Buffer.height: u16` (`src/buffer.rs`, already
  public fields) — used to clip the outer ring at the buffer's edge
  instead of panicking when the caller's `area` has no margin.

**Interfaces produced:** no new public API — `Block::render`'s existing
signature (`fn render(&self, area: Rect, buf: &mut Buffer) -> Rect`) is
unchanged; only its internal cell-drawing changes. The *returned* inner
`Rect` is unaffected by `border_thick` (matches the design spec: the
outer ring is drawn outside `area`, not subtracted from the inner
content area).

- [ ] **Step 1: Write the failing tests** — add to `src/widgets/block.rs`'s
  existing `mod tests`:

```rust
#[test]
fn thick_border_draws_a_second_ring_one_cell_outward() {
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
        border_bold: false,
        border_thick: true,
    };
    let mut buf = Buffer::new(6, 5);
    let area = Rect {
        x: 1,
        y: 1,
        width: 4,
        height: 3,
    };

    Block::new().theme(&theme).render(area, &mut buf);

    assert_eq!(buf.get(0, 0).symbol, '*'); // outer corner
    assert_eq!(buf.get(1, 0).symbol, '='); // outer top edge
    assert_eq!(buf.get(0, 1).symbol, '#'); // outer left edge
    assert_eq!(buf.get(0, 0).fg, Color::Green);
    assert_eq!(buf.get(0, 0).bg, Color::Black);
}

#[test]
fn thin_border_leaves_the_outward_ring_untouched() {
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
        border_bold: false,
        border_thick: false,
    };
    let mut buf = Buffer::new(6, 5);
    let area = Rect {
        x: 1,
        y: 1,
        width: 4,
        height: 3,
    };

    Block::new().theme(&theme).render(area, &mut buf);

    assert_eq!(*buf.get(0, 0), Cell::default());
}

#[test]
fn theme_less_border_leaves_the_outward_ring_untouched() {
    let mut buf = Buffer::new(6, 5);
    let area = Rect {
        x: 1,
        y: 1,
        width: 4,
        height: 3,
    };

    Block::new().render(area, &mut buf);

    assert_eq!(*buf.get(0, 0), Cell::default());
}
```

- [ ] **Step 2: Run tests to verify the new one fails**

Run: `cargo test --lib widgets::block::tests`
Expected: `thick_border_draws_a_second_ring_one_cell_outward` FAILS
(outer ring cells are still `Cell::default()` — symbol `' '`, not `'*'`
or `'='`); the other two PASS already (nothing draws outward yet).

- [ ] **Step 3: Implement** — in `src/widgets/block.rs`, replace the
  `render` method body:

```rust
pub fn render(&self, area: Rect, buf: &mut Buffer) -> Rect {
    if area.width < 2 || area.height < 2 {
        return area;
    }
    let (border, fg, bg, border_bold, border_thick) = match self.theme {
        Some(t) => (t.border, t.primary, t.background, t.border_bold, t.border_thick),
        None => (BorderSet::default(), Color::Reset, Color::Reset, false, false),
    };
    let plain = || Cell {
        symbol: ' ',
        fg,
        bg,
        style: CellStyle { bold: border_bold },
    };

    let draw_ring = |ring_area: Rect, buf: &mut Buffer| {
        for x in ring_area.x..ring_area.x + ring_area.width {
            buf.set(
                x,
                ring_area.y,
                Cell {
                    symbol: border.horizontal,
                    ..plain()
                },
            );
            buf.set(
                x,
                ring_area.y + ring_area.height - 1,
                Cell {
                    symbol: border.horizontal,
                    ..plain()
                },
            );
        }
        for y in ring_area.y..ring_area.y + ring_area.height {
            buf.set(
                ring_area.x,
                y,
                Cell {
                    symbol: border.vertical,
                    ..plain()
                },
            );
            buf.set(
                ring_area.x + ring_area.width - 1,
                y,
                Cell {
                    symbol: border.vertical,
                    ..plain()
                },
            );
        }
        buf.set(
            ring_area.x,
            ring_area.y,
            Cell {
                symbol: border.corner,
                ..plain()
            },
        );
        buf.set(
            ring_area.x + ring_area.width - 1,
            ring_area.y,
            Cell {
                symbol: border.corner,
                ..plain()
            },
        );
        buf.set(
            ring_area.x,
            ring_area.y + ring_area.height - 1,
            Cell {
                symbol: border.corner,
                ..plain()
            },
        );
        buf.set(
            ring_area.x + ring_area.width - 1,
            ring_area.y + ring_area.height - 1,
            Cell {
                symbol: border.corner,
                ..plain()
            },
        );
    };

    draw_ring(area, buf);

    if border_thick {
        let outer_x = area.x.saturating_sub(1);
        let outer_y = area.y.saturating_sub(1);
        let outer_w = (area.width + 2).min(buf.width.saturating_sub(outer_x));
        let outer_h = (area.height + 2).min(buf.height.saturating_sub(outer_y));
        if outer_w >= 2 && outer_h >= 2 {
            draw_ring(
                Rect {
                    x: outer_x,
                    y: outer_y,
                    width: outer_w,
                    height: outer_h,
                },
                buf,
            );
        }
    }

    if let Some(title) = self.title {
        for (i, ch) in title
            .chars()
            .take(area.width.saturating_sub(2) as usize)
            .enumerate()
        {
            buf.set(
                area.x + 1 + i as u16,
                area.y,
                Cell {
                    symbol: ch,
                    style: CellStyle::default(),
                    ..plain()
                },
            );
        }
    }

    Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    }
}
```

  Note: `outer_x`/`outer_y` use `saturating_sub(1)` and `outer_w`/`outer_h`
  are clamped against `buf.width`/`buf.height` — if `area` has no margin
  (e.g. `area.x == 0`), the outer ring clips at the buffer edge instead
  of underflowing/panicking, matching the design spec's stated behavior.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib widgets::block::tests`
Expected: PASS (all tests in the module, old and new)

- [ ] **Step 5: Run full check**

Run: `cargo test --lib && cargo build --examples && cargo fmt && cargo clippy --all-targets -- -D warnings`
Expected: all pass, no warnings.

- [ ] **Step 6: Commit**

```bash
git add src/widgets/block.rs
git commit -m "feat(widgets): Block draws a second ring when Theme.border_thick (#45)"
```

---

### Task 4: `AppMode` enum (`examples/omnitrix.rs`, #43)

**Files:**
- Modify: `examples/omnitrix.rs`

**Interfaces produced:** none public — `Omnitrix` and `AppMode` are
private to this example binary.

No new tests — example code, verified by running, per the TDD exceptions
in `development-conventions.md` (same as #42's original Faceplate task).

- [ ] **Step 1: Replace `DnaSample` and `Screen` with `AppMode`** — in
  `examples/omnitrix.rs`, replace the current:

```rust
#[derive(Clone, Copy, PartialEq)]
enum DnaSample {
    Brainstorm,
    Fasttrack,
    Upgrade,
}

impl DnaSample {
    const ALL: [DnaSample; 3] = [
        DnaSample::Brainstorm,
        DnaSample::Fasttrack,
        DnaSample::Upgrade,
    ];

    fn name(&self) -> &'static str {
        match self {
            DnaSample::Brainstorm => "Brainstorm",
            DnaSample::Fasttrack => "Fasttrack",
            DnaSample::Upgrade => "Upgrade",
        }
    }
}

enum Screen {
    Faceplate,
    Launched(DnaSample),
}
```

  with:

```rust
#[derive(Clone, Copy, PartialEq)]
enum AppMode {
    Faceplate,
    Brainstorm,
    Fasttrack,
    Upgrade,
}

const SAMPLES: [&str; 3] = ["Brainstorm", "Fasttrack", "Upgrade"];

impl AppMode {
    fn from_selected(selected: usize) -> Self {
        match selected {
            0 => AppMode::Brainstorm,
            1 => AppMode::Fasttrack,
            _ => AppMode::Upgrade,
        }
    }

    fn name(&self) -> &'static str {
        match self {
            AppMode::Faceplate => "Faceplate",
            AppMode::Brainstorm => "Brainstorm",
            AppMode::Fasttrack => "Fasttrack",
            AppMode::Upgrade => "Upgrade",
        }
    }
}
```

- [ ] **Step 2: Replace the `screen` field with `mode`** — change the
  `Omnitrix` struct definition:

```rust
struct Omnitrix {
    pulse_phase: f32,
    quit: bool,
    last_tick_started: Instant,
    perf_log: std::fs::File,
    selected: usize,
    mode: AppMode,
}
```

  and the corresponding field in the `Omnitrix { .. }` literal inside
  `new()`:

```rust
Omnitrix {
    pulse_phase: 0.0,
    quit: false,
    last_tick_started: Instant::now(),
    perf_log,
    selected: 0,
    mode: AppMode::Faceplate,
}
```

- [ ] **Step 3: Update `update()`** — replace the method body:

```rust
fn update(&mut self, event: &Event) {
    let Event::Key(k) = event else { return };
    if k.kind != KeyEventKind::Press {
        return;
    }
    if k.code == KeyCode::Char('q') {
        self.quit = true;
        return;
    }
    match self.mode {
        AppMode::Faceplate => match k.code {
            KeyCode::Tab => self.selected = (self.selected + 1) % SAMPLES.len(),
            KeyCode::BackTab => {
                self.selected = (self.selected + SAMPLES.len() - 1) % SAMPLES.len()
            }
            KeyCode::Enter => self.mode = AppMode::from_selected(self.selected),
            _ => {}
        },
        _ => {
            if k.code == KeyCode::Esc {
                self.mode = AppMode::Faceplate;
            }
        }
    }
}
```

- [ ] **Step 4: Update `view()`** — replace the method body:

```rust
fn view(&self, area: Rect, buf: &mut LayerStack) {
    let theme = self.theme();
    let inner = Block::new()
        .title("Omnitrix")
        .theme(&theme)
        .render(area, buf);

    match self.mode {
        AppMode::Faceplate => {
            let list_area = Rect {
                x: inner.x,
                y: inner.y,
                width: inner.width,
                height: inner.height.saturating_sub(1),
            };
            let hint_row = Rect {
                x: inner.x,
                y: inner.y + inner.height.saturating_sub(1),
                width: inner.width,
                height: inner.height.saturating_sub(1).min(1),
            };
            let names: Vec<String> = SAMPLES.iter().map(|s| s.to_string()).collect();
            List::new(&names, self.selected).render(list_area, buf);
            Text::new("Tab/Shift+Tab cycle * Enter launch * q quit").render(hint_row, buf);
        }
        _ => {
            let name_row = Rect {
                x: inner.x,
                y: inner.y,
                width: inner.width,
                height: inner.height.min(1),
            };
            let placeholder_row = Rect {
                x: inner.x,
                y: inner.y + 1,
                width: inner.width,
                height: inner.height.saturating_sub(2),
            };
            let hint_row = Rect {
                x: inner.x,
                y: inner.y + inner.height.saturating_sub(1),
                width: inner.width,
                height: inner.height.saturating_sub(1).min(1),
            };
            Text::new(self.mode.name()).render(name_row, buf);
            Text::new("(not yet built)").render(placeholder_row, buf);
            Text::new("Esc back * q quit").render(hint_row, buf);
        }
    }
}
```

- [ ] **Step 5: Build**

Run: `cargo build --example omnitrix`
Expected: compiles cleanly, no warnings.

- [ ] **Step 6: `cargo fmt && cargo clippy --all-targets -- -D warnings`**

Expected: clean, no warnings.

- [ ] **Step 7: Manual verification** (real-terminal check, not
  automatable — per this project's TDD exceptions for example code):

Run: `cargo run --example omnitrix`

Confirm:
- Behavior is identical to before this task: Faceplate shows the same
  3-row `List` (Brainstorm/Fasttrack/Upgrade), Tab/Shift+Tab cycle with
  wraparound, Enter launches the selected mode showing its name +
  "(not yet built)", Esc returns to Faceplate with `selected` preserved,
  `q` quits cleanly from any mode.
- No visual difference from before — this task only renames the state
  shape, it does not change rendering or interaction.

- [ ] **Step 8: Commit**

```bash
git add examples/omnitrix.rs
git commit -m "refactor(omnitrix): adopt AppMode enum in place of Screen/DnaSample (#43)"
```

---

### Task 5: Faceplate-to-Dial revamp (`examples/omnitrix.rs`, #88)

**Files:**
- Modify: `examples/omnitrix.rs`

**Interfaces consumed:** `Dial::new(items: &[String], selected: usize) -> Dial`,
`Dial::render(&self, area: Rect, buf: &mut Buffer)` (Task 1).

No new tests — example code, verified by running.

- [ ] **Step 1: Swap the `List` import for `Dial`** — change:

```rust
use ttui::widgets::{block::Block, list::List, text::Text};
```

  to:

```rust
use ttui::widgets::{block::Block, dial::Dial, text::Text};
```

- [ ] **Step 2: Swap the widget call in `view()`'s `AppMode::Faceplate`
  arm** — replace:

```rust
AppMode::Faceplate => {
    let list_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: inner.height.saturating_sub(1),
    };
    let hint_row = Rect {
        x: inner.x,
        y: inner.y + inner.height.saturating_sub(1),
        width: inner.width,
        height: inner.height.saturating_sub(1).min(1),
    };
    let names: Vec<String> = SAMPLES.iter().map(|s| s.to_string()).collect();
    List::new(&names, self.selected).render(list_area, buf);
    Text::new("Tab/Shift+Tab cycle * Enter launch * q quit").render(hint_row, buf);
}
```

  with:

```rust
AppMode::Faceplate => {
    let dial_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: inner.height.saturating_sub(1),
    };
    let hint_row = Rect {
        x: inner.x,
        y: inner.y + inner.height.saturating_sub(1),
        width: inner.width,
        height: inner.height.saturating_sub(1).min(1),
    };
    let names: Vec<String> = SAMPLES.iter().map(|s| s.to_string()).collect();
    Dial::new(&names, self.selected).render(dial_area, buf);
    Text::new("Tab/Shift+Tab cycle * Enter launch * q quit").render(hint_row, buf);
}
```

- [ ] **Step 3: Build**

Run: `cargo build --example omnitrix`
Expected: compiles cleanly, no warnings (confirms the now-unused `List`
import was fully removed, not just shadowed).

- [ ] **Step 4: `cargo fmt && cargo clippy --all-targets -- -D warnings`**

Expected: clean, no warnings.

- [ ] **Step 5: Manual verification**

Run: `cargo run --example omnitrix`

Confirm:
- Faceplate now renders as a circular dial: 3 outward-flowing labels
  (Brainstorm/Fasttrack/Upgrade) arranged around a ring of `.` dots, with
  Brainstorm at the top.
- The selected item's label is highlighted (black-on-white) and a `*`
  pointer glyph appears between the dial's center and the selected
  label.
- Tab/Shift+Tab move the highlight and pointer around the ring with the
  same wraparound as before; Enter launches the selected mode; Esc
  returns to the dial with the same item still highlighted.
- `q` quits cleanly, no panic, no leftover terminal attributes.

- [ ] **Step 6: Commit**

```bash
git add examples/omnitrix.rs
git commit -m "feat(omnitrix): render Faceplate as a Dial (#88)"
```

---

### Task 6: Corruption transition (`examples/omnitrix.rs`, #44)

**Files:**
- Modify: `examples/omnitrix.rs`

**Interfaces consumed:**
- `ttui::transition::Transition` (`src/transition.rs`, Arc 0, already
  shipped): `Transition::start(Duration) -> Transition`,
  `.tick(Duration)`, `.progress() -> f32`, `.is_complete() -> bool`.
- `ttui::buffer::{Buffer, Cell}` (Arc 0, already shipped).

No new tests — example code, verified by running, including watching a
full transition play in both directions.

- [ ] **Step 1: Update imports** — change:

```rust
use ttui::buffer::LayerStack;
```

  to:

```rust
use ttui::buffer::{Buffer, Cell, LayerStack};
use ttui::transition::Transition;
```

- [ ] **Step 2: Add transition state to `Omnitrix`** — change the struct
  definition:

```rust
struct Omnitrix {
    pulse_phase: f32,
    quit: bool,
    last_tick_started: Instant,
    perf_log: std::fs::File,
    selected: usize,
    mode: AppMode,
    transitioning_from: Option<(AppMode, Transition)>,
    tick_count: u64,
}
```

  and the two new fields in `new()`'s `Omnitrix { .. }` literal:

```rust
Omnitrix {
    pulse_phase: 0.0,
    quit: false,
    last_tick_started: Instant::now(),
    perf_log,
    selected: 0,
    mode: AppMode::Faceplate,
    transitioning_from: None,
    tick_count: 0,
}
```

- [ ] **Step 3: Add a `switch_mode` helper and wire it into `update()`**
  — replace the `update()` method body:

```rust
fn update(&mut self, event: &Event) {
    let Event::Key(k) = event else { return };
    if k.kind != KeyEventKind::Press {
        return;
    }
    if k.code == KeyCode::Char('q') {
        self.quit = true;
        return;
    }
    if self.transitioning_from.is_some() {
        return;
    }
    match self.mode {
        AppMode::Faceplate => match k.code {
            KeyCode::Tab => self.selected = (self.selected + 1) % SAMPLES.len(),
            KeyCode::BackTab => {
                self.selected = (self.selected + SAMPLES.len() - 1) % SAMPLES.len()
            }
            KeyCode::Enter => self.switch_mode(AppMode::from_selected(self.selected)),
            _ => {}
        },
        _ => {
            if k.code == KeyCode::Esc {
                self.switch_mode(AppMode::Faceplate);
            }
        }
    }
}

fn switch_mode(&mut self, next: AppMode) {
    let old = self.mode;
    self.mode = next;
    self.transitioning_from = Some((old, Transition::start(Duration::from_millis(500))));
}
```

  Note: the `transitioning_from.is_some()` early return sits after the
  `q` check (quit always works) and before the mode match (Tab/Shift+Tab/
  Enter/Esc are ignored mid-transition), matching the design spec.

- [ ] **Step 4: Tick the transition in `on_tick`** — append to the end
  of the existing `on_tick` method body (after the existing
  `self.pulse_phase += ...` line, before the closing brace):

```rust
self.tick_count += 1;

if let Some((_, transition)) = &mut self.transitioning_from {
    transition.tick(elapsed);
    if transition.is_complete() {
        self.transitioning_from = None;
    }
}
```

- [ ] **Step 5: Factor per-mode rendering into a scratch-buffer helper**
  — add these two methods to `impl Omnitrix` (alongside `theme()` and
  `switch_mode`):

```rust
fn render_mode_content(&self, mode: AppMode, area: Rect) -> Buffer {
    let mut buf = Buffer::new(area.width, area.height);
    let local = Rect {
        x: 0,
        y: 0,
        width: area.width,
        height: area.height,
    };
    match mode {
        AppMode::Faceplate => {
            let dial_area = Rect {
                x: local.x,
                y: local.y,
                width: local.width,
                height: local.height.saturating_sub(1),
            };
            let hint_row = Rect {
                x: local.x,
                y: local.y + local.height.saturating_sub(1),
                width: local.width,
                height: local.height.saturating_sub(1).min(1),
            };
            let names: Vec<String> = SAMPLES.iter().map(|s| s.to_string()).collect();
            Dial::new(&names, self.selected).render(dial_area, &mut buf);
            Text::new("Tab/Shift+Tab cycle * Enter launch * q quit").render(hint_row, &mut buf);
        }
        _ => {
            let name_row = Rect {
                x: local.x,
                y: local.y,
                width: local.width,
                height: local.height.min(1),
            };
            let placeholder_row = Rect {
                x: local.x,
                y: local.y + 1,
                width: local.width,
                height: local.height.saturating_sub(2),
            };
            let hint_row = Rect {
                x: local.x,
                y: local.y + local.height.saturating_sub(1),
                width: local.width,
                height: local.height.saturating_sub(1).min(1),
            };
            Text::new(mode.name()).render(name_row, &mut buf);
            Text::new("(not yet built)").render(placeholder_row, &mut buf);
            Text::new("Esc back * q quit").render(hint_row, &mut buf);
        }
    }
    buf
}

fn render_transition(&self, old_mode: AppMode, area: Rect, progress: f32, buf: &mut Buffer) {
    if progress < 0.2 {
        for y in 0..area.height {
            for x in 0..area.width {
                buf.set(
                    area.x + x,
                    area.y + y,
                    Cell {
                        symbol: ' ',
                        fg: Color::Reset,
                        bg: Color::Yellow,
                        ..Default::default()
                    },
                );
            }
        }
        return;
    }

    let wave = (progress - 0.2) / 0.8;
    let wave_row = (wave * area.height as f32) as u16;
    let old_content = self.render_mode_content(old_mode, area);
    let new_content = self.render_mode_content(self.mode, area);

    for y in 0..area.height {
        for x in 0..area.width {
            let cell = match y.cmp(&wave_row) {
                std::cmp::Ordering::Less => new_content.get(x, y).clone(),
                std::cmp::Ordering::Equal => Cell {
                    symbol: braille_noise(x, y, self.tick_count),
                    fg: Color::Reset,
                    bg: Color::Yellow,
                    ..Default::default()
                },
                std::cmp::Ordering::Greater => old_content.get(x, y).clone(),
            };
            buf.set(area.x + x, area.y + y, cell);
        }
    }
}
```

  and this free function at module scope (below `Omnitrix`'s `impl`
  blocks, above `fn main`):

```rust
fn braille_noise(x: u16, y: u16, tick: u64) -> char {
    let h = (x as u64)
        .wrapping_mul(374_761_393)
        ^ (y as u64).wrapping_mul(668_265_263)
        ^ tick.wrapping_mul(2_246_822_519);
    let dot_pattern = (h % 256) as u32;
    char::from_u32(0x2800 + dot_pattern).unwrap_or('\u{2800}')
}
```

  and a small blit helper alongside `braille_noise`:

```rust
fn blit(scratch: &Buffer, area: Rect, buf: &mut Buffer) {
    for y in 0..scratch.height {
        for x in 0..scratch.width {
            buf.set(area.x + x, area.y + y, scratch.get(x, y).clone());
        }
    }
}
```

- [ ] **Step 6: Rewrite `view()` to branch on transition state** —
  replace the method body:

```rust
fn view(&self, area: Rect, buf: &mut LayerStack) {
    let theme = self.theme();
    let inner = Block::new()
        .title("Omnitrix")
        .theme(&theme)
        .render(area, buf);

    match &self.transitioning_from {
        None => {
            let content = self.render_mode_content(self.mode, inner);
            blit(&content, inner, buf);
        }
        Some((old_mode, transition)) => {
            self.render_transition(*old_mode, inner, transition.progress(), buf);
        }
    }
}
```

- [ ] **Step 7: Build**

Run: `cargo build --example omnitrix`
Expected: compiles cleanly, no warnings.

- [ ] **Step 8: `cargo fmt && cargo clippy --all-targets -- -D warnings`**

Expected: clean, no warnings.

- [ ] **Step 9: Manual verification** (real-terminal check, not
  automatable — per this project's TDD exceptions for example code):

Run: `cargo run --example omnitrix`

Confirm, for both directions (Faceplate → a mode via Enter, and back via
Esc):
- Pressing Enter/Esc immediately flashes the inner area solid yellow for
  roughly the first fifth of the ~500ms transition.
- The flash gives way to a Braille-noise band that sweeps top-to-bottom
  over the remaining duration, with already-revealed rows showing the
  destination mode's content and not-yet-revealed rows still showing the
  source mode's content.
- Tab/Shift+Tab/Enter/Esc are all ignored while a transition is playing
  (try pressing Enter again mid-transition — nothing happens until it
  finishes).
- `q` still quits immediately even mid-transition.
- Once a transition completes, the destination mode is fully visible and
  interaction resumes normally.
- The border (double-thickness aside — Task 2/3 shipped no consumer for
  it, so the border looks the same as before this task) keeps
  pulsing/bolding throughout, unaffected by the transition.
- `q` quits cleanly from any state, no panic, no leftover terminal
  attributes in the shell prompt after exit.

- [ ] **Step 10: Commit**

```bash
git add examples/omnitrix.rs
git commit -m "feat(omnitrix): add corruption transition between modes (#44)"
```

---

## Self-Review

**Spec coverage:** Slice 1 (Dial geometry: center/radius formula, item
placement, dotted ring, outward-flowing clipped labels, selection
highlight + pointer glyph) — Task 1, with 5 tests deriving concrete
expected cell positions from the spec's exact formulas. Slice 2 (`Theme.
border_thick` field + mechanical exhaustive-literal fixes across every
site) — Task 2. Slice 2's `Block::render` extension (second ring drawn
outward, byte-for-byte unchanged when `false`, clips instead of panicking
with no caller margin) — Task 3. Slice 3 (`AppMode` enum, `SAMPLES`
const, Tab/Shift+Tab/Enter/Esc/`q` interaction contract carried over) —
Task 4. Slice 4 (List → Dial swap in Faceplate rendering only) — Task 5.
Slice 5 (`transitioning_from` field, flash phase, Braille wave phase,
input-ignored-mid-transition, deterministic non-RNG hash, scratch-buffer
compositing helper) — Task 6. Verification section (`cargo test`,
`cargo fmt --check`... note: this plan uses `cargo fmt` not
`--check` in intermediate steps per this repo's existing plan
convention, `cargo clippy --all-targets -- -D warnings`, manual
`cargo run --example omnitrix` walkthrough) — covered across every
task's final steps.

**Placeholder scan:** no TBD/TODO; every step has literal code or an
exact command. Task 2's `border_thick: false,` placeholders in
`examples/omnitrix.rs` and `examples/smash_crabs.rs` are explicitly
flagged as intentionally unwired (no task in this plan changes that),
not an unresolved placeholder in the plan itself.

**Type consistency:** `Dial::new(items: &[String], selected: usize)` /
`Dial::render(&self, area: Rect, buf: &mut Buffer)` (Task 1) match
exactly how Task 5 and Task 6 call it. `Theme.border_thick: bool`
(Task 2) is the exact field Task 3's `Block::render` reads. `AppMode`,
`SAMPLES`, `AppMode::from_selected`, `AppMode::name` (Task 4) are used
identically (same names, same signatures) in Tasks 5 and 6 — no renames
across tasks. `Omnitrix.transitioning_from: Option<(AppMode, Transition)>`
and `Omnitrix.tick_count: u64` (Task 6) are used consistently within
that task's `switch_mode`, `on_tick`, `render_transition`, and `view`.
