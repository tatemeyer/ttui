# Buffer Layering Follow-ups Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Clean up `LayerStack`'s API (drop redundant fields, cheaper `composite()`, add `layer_count()`/`layer()`), fix stale language in the Rev B spec, and add a third example that exercises the multi-layer path end-to-end — three items PR #32's final review parked as deferred follow-ups.

**Architecture:** All three items land as additional commits on the existing open PR #32 branch. Task 1 refactors `LayerStack` in `src/buffer.rs` behind its existing tests (behavior-preserving) plus two new tests for the new accessors. Task 2 is a docs-only edit. Task 3 adds `examples/smash_crabs.rs`, a third `App` implementor that pushes two extra layers per frame (UI, Effects) on top of the base (Background), using only APIs that exist after Task 1.

**Tech Stack:** Rust, `crossterm` (unchanged, no new dependencies).

## Global Constraints

- TDD is mandatory for `coding`-tagged work (Task 1); Task 2 is pure docs (no TDD); Task 3 is an example/demo (explicit TDD exception per `.claude/rules/development-conventions.md` — correctness is checked by running it).
- Inline `#[cfg(test)] mod tests` is the test convention — no new `tests/` directory.
- `cargo fmt` / `cargo clippy --all-targets` must stay clean throughout.
- No new dependencies.
- Task 1's field removal and `composite()` rewrite must not change behavior — the 6 existing `LayerStack`/`composite` tests in `src/buffer.rs` (`new_layer_stack_has_one_default_filled_base_layer`, `push_layer_appends_a_same_dimension_default_filled_layer`, `layer_stack_derefs_to_the_base_layer`, `composite_of_a_single_layer_stack_matches_that_layer`, `composite_of_a_three_layer_stack_lets_topmost_non_default_cell_win`, `cloning_a_layer_stack_preserves_all_layers_not_just_the_base`) must keep passing unmodified.

---

### Task 1: `LayerStack` cleanup

**Files:**
- Modify: `src/buffer.rs` (struct definition, `new`/`push_layer`/`composite`, add `layer`/`layer_count`, two new tests)

**Interfaces:**
- Consumes: nothing new — this task only reshapes `LayerStack`'s existing internals.
- Produces: `LayerStack::layer(&self, index: usize) -> &Buffer` and `LayerStack::layer_count(&self) -> usize`, both used by Task 3's example.

- [ ] **Step 1: Write the failing tests**

Add to the existing `mod tests` block in `src/buffer.rs` (after `cloning_a_layer_stack_preserves_all_layers_not_just_the_base`):

```rust
    #[test]
    fn layer_count_reflects_pushed_layers() {
        let mut stack = LayerStack::new(2, 2);
        assert_eq!(stack.layer_count(), 1);
        stack.push_layer();
        stack.push_layer();
        assert_eq!(stack.layer_count(), 3);
    }

    #[test]
    fn layer_gives_read_only_access_to_a_pushed_layer() {
        let mut stack = LayerStack::new(2, 2);
        let cell = Cell {
            symbol: 'q',
            fg: Color::Reset,
            bg: Color::Reset,
        };
        stack.push_layer().set(0, 0, cell.clone());

        assert_eq!(*stack.layer(1).get(0, 0), cell);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib buffer::tests`
Expected: FAIL to compile — no method named `layer_count`/`layer` found for `LayerStack`.

- [ ] **Step 3: Replace the struct definition and its four existing methods**

Find this block in `src/buffer.rs` (currently the whole `LayerStack` struct + `impl LayerStack` block):

```rust
#[derive(Clone, Debug)]
pub struct LayerStack {
    width: u16,
    height: u16,
    // Invariant: always has length >= 1; layers[0] is the base layer. This
    // is what keeps Deref's `&self.layers[0]` from ever panicking.
    layers: Vec<Buffer>,
}

impl LayerStack {
    pub fn new(width: u16, height: u16) -> Self {
        LayerStack {
            width,
            height,
            layers: vec![Buffer::new(width, height)],
        }
    }

    pub fn push_layer(&mut self) -> &mut Buffer {
        self.layers.push(Buffer::new(self.width, self.height));
        self.layers.last_mut().unwrap()
    }

    // `index` must already exist via a prior `push_layer()` call — there is
    // no auto-grow; an out-of-range index panics (standard Vec indexing
    // panic).
    pub fn layer_mut(&mut self, index: usize) -> &mut Buffer {
        &mut self.layers[index]
    }

    // Depth-1 fast path: returns a clone of the base layer with no scan.
    // For depth > 1: bottom-to-top scan where the last (topmost) non-default
    // cell at each position wins (see transparency rule on `LayerStack`).
    pub fn composite(&self) -> Buffer {
        if self.layers.len() == 1 {
            return self.layers[0].clone();
        }
        let mut out = Buffer::new(self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                let mut cell = Cell::default();
                for layer in &self.layers {
                    let c = layer.get(x, y);
                    if *c != Cell::default() {
                        cell = c.clone();
                    }
                }
                out.set(x, y, cell);
            }
        }
        out
    }
}
```

Replace it with:

```rust
#[derive(Clone, Debug)]
pub struct LayerStack {
    // Invariant: always has length >= 1; layers[0] is the base layer. This
    // is what keeps Deref's `&self.layers[0]` from ever panicking.
    layers: Vec<Buffer>,
}

impl LayerStack {
    pub fn new(width: u16, height: u16) -> Self {
        LayerStack {
            layers: vec![Buffer::new(width, height)],
        }
    }

    pub fn push_layer(&mut self) -> &mut Buffer {
        let width = self.layers[0].width;
        let height = self.layers[0].height;
        self.layers.push(Buffer::new(width, height));
        self.layers.last_mut().unwrap()
    }

    // `index` must already exist via a prior `push_layer()` call — there is
    // no auto-grow; an out-of-range index panics (standard Vec indexing
    // panic).
    pub fn layer_mut(&mut self, index: usize) -> &mut Buffer {
        &mut self.layers[index]
    }

    // Read-only counterpart to `layer_mut` — same out-of-range panic
    // behavior (standard Vec indexing panic).
    pub fn layer(&self, index: usize) -> &Buffer {
        &self.layers[index]
    }

    // Number of layers currently in the stack (always >= 1).
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    // Depth-1 fast path: returns a clone of the base layer with no scan.
    // For depth > 1: top-to-bottom scan, stopping at the first (topmost)
    // non-default cell at each position (see transparency rule on
    // `LayerStack`).
    pub fn composite(&self) -> Buffer {
        if self.layers.len() == 1 {
            return self.layers[0].clone();
        }
        let width = self.layers[0].width;
        let height = self.layers[0].height;
        let mut out = Buffer::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let mut cell = Cell::default();
                for layer in self.layers.iter().rev() {
                    let c = layer.get(x, y);
                    if *c != Cell::default() {
                        cell = c.clone();
                        break;
                    }
                }
                out.set(x, y, cell);
            }
        }
        out
    }
}
```

- [ ] **Step 4: Run the full test suite to verify everything passes**

Run: `cargo test --lib`
Expected: PASS — all buffer tests (the 6 pre-existing `LayerStack`/`composite` tests plus the 2 new ones from Step 1), plus the rest of the library suite (app, layout, widgets, theme), all green.

- [ ] **Step 5: Commit**

```bash
git add src/buffer.rs
git commit -m "refactor(buffer): drop redundant LayerStack fields, add layer()/layer_count(), cheaper composite()"
```

---

### Task 2: Rev B spec forward pointer

**Files:**
- Modify: `docs/design/specs/2026-08-05-ttui-rev-b-vision-alignment-design.md`

**Interfaces:** none — docs only, no code interfaces produced or consumed.

- [ ] **Step 1: Insert a forward pointer under the "Deferred" heading**

Find this text (the `## Deferred (documented, not designed): buffer layering` heading and the start of its body):

```
## Deferred (documented, not designed): buffer layering

Smash Crabs wants three explicit, z-ordered buffers (background/UI/
```

Replace it with:

```
## Deferred (documented, not designed): buffer layering

**Update (2026-08-05):** this has since been designed and shipped — see
`2026-08-05-buffer-layering-compositing-design.md`. The rest of this
section is preserved as the original historical record.

Smash Crabs wants three explicit, z-ordered buffers (background/UI/
```

- [ ] **Step 2: Update the stale bullet under "Explicitly deferred / open questions"**

Find this bullet:

```
- Buffer layering/compositing for Smash Crabs (see "Deferred" above) —
  not designed, direction recorded only.
```

Replace it with:

```
- Buffer layering/compositing for Smash Crabs (see "Deferred" above) —
  not designed, direction recorded only — since designed and
  implemented, see `2026-08-05-buffer-layering-compositing-design.md`.
```

- [ ] **Step 3: Confirm nothing else changed**

Run: `git diff docs/design/specs/2026-08-05-ttui-rev-b-vision-alignment-design.md`
Expected: only the two insertions above show up in the diff — no other lines touched.

- [ ] **Step 4: Commit**

```bash
git add docs/design/specs/2026-08-05-ttui-rev-b-vision-alignment-design.md
git commit -m "docs(spec): point Rev B's buffer-layering section forward to the shipped design"
```

---

### Task 3: `examples/smash_crabs.rs` — multi-layer end-to-end example

**Files:**
- Create: `examples/smash_crabs.rs`

**Interfaces:**
- Consumes: `ttui::app::{run, App}` (existing), `ttui::buffer::{Cell, LayerStack}` (Task 1 adds `layer`/`layer_count`, not directly used by this file but available), `ttui::layout::{Constraint, Direction, Layout, Rect}` (existing), `ttui::theme::{BorderSet, Theme}` (existing), `ttui::widgets::{block::Block, text::Text}` (existing, unchanged `render(&self, area: Rect, buf: &mut Buffer)` signatures).
- Produces: nothing consumed by a later task — this is the last task in the plan.

- [ ] **Step 1: Confirm the baseline compiles before adding the new example**

Run: `cargo build --examples`
Expected: PASS (the two existing examples still build after Task 1's refactor).

- [ ] **Step 2: Create `examples/smash_crabs.rs`**

```rust
// examples/smash_crabs.rs
use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::style::Color;
use std::time::Duration;
use ttui::app::{run, App};
use ttui::buffer::{Cell, LayerStack};
use ttui::layout::{Constraint, Direction, Layout, Rect};
use ttui::theme::{BorderSet, Theme};
use ttui::widgets::{block::Block, text::Text};

const BACKGROUND: usize = 0;
const UI: usize = 1;
const EFFECTS: usize = 2;

const TICK_INTERVAL: Duration = Duration::from_millis(33); // ~30 FPS, matches omnitrix
const FLASH_TICKS: u8 = 6; // ~200ms flash at 33ms/tick

fn arena_theme() -> Theme {
    Theme {
        background: Color::Rgb {
            r: 92,
            g: 64,
            b: 20,
        }, // packed-sand arena floor
        primary: Color::Red,    // crab shell red
        secondary: Color::Cyan, // water
        tertiary: Color::White,
        accent: Color::Yellow,
        border: BorderSet {
            horizontal: '=',
            vertical: '|',
            corner: '+',
        },
    }
}

struct SmashCrabs {
    theme: Theme,
    p1_hp: u8,
    p2_hp: u8,
    flash_ticks_remaining: u8,
    quit: bool,
}

impl SmashCrabs {
    fn new() -> Self {
        SmashCrabs {
            theme: arena_theme(),
            p1_hp: 100,
            p2_hp: 100,
            flash_ticks_remaining: 0,
            quit: false,
        }
    }

    fn paint_background(&self, area: Rect, buf: &mut LayerStack) {
        let cell = Cell {
            symbol: ' ',
            fg: self.theme.primary,
            bg: self.theme.background,
        };
        let layer = buf.layer_mut(BACKGROUND);
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                layer.set(x, y, cell.clone());
            }
        }
    }

    fn paint_ui(&self, area: Rect, buf: &mut LayerStack) {
        let panel = Layout::new(Direction::Vertical, vec![Constraint::Fixed(4)]).split(area)[0];
        let panel = Rect {
            width: panel.width.min(20),
            ..panel
        };
        let inner = Block::new()
            .title("Fighters")
            .theme(&self.theme)
            .render(panel, buf.layer_mut(UI));
        let rows = Layout::new(
            Direction::Vertical,
            vec![Constraint::Fixed(1), Constraint::Fixed(1)],
        )
        .split(inner);
        Text::new(&format!("P1: {} HP", self.p1_hp)).render(rows[0], buf.layer_mut(UI));
        Text::new(&format!("P2: {} HP", self.p2_hp)).render(rows[1], buf.layer_mut(UI));
    }

    fn paint_effects(&self, area: Rect, buf: &mut LayerStack) {
        if self.flash_ticks_remaining == 0 {
            return;
        }
        let flash = Cell {
            symbol: '*',
            fg: Color::Black,
            bg: self.theme.accent,
        };
        let w = 7.min(area.width);
        let h = 3.min(area.height);
        let x0 = area.x + (area.width.saturating_sub(w)) / 2;
        let y0 = area.y + (area.height.saturating_sub(h)) / 2;
        let layer = buf.layer_mut(EFFECTS);
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                layer.set(x, y, flash.clone());
            }
        }
    }
}

impl App for SmashCrabs {
    fn update(&mut self, event: &Event) {
        let Event::Key(k) = event else { return };
        if k.kind != KeyEventKind::Press {
            return;
        }
        match k.code {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Char(' ') => {
                self.flash_ticks_remaining = FLASH_TICKS;
                self.p2_hp = self.p2_hp.saturating_sub(10);
            }
            _ => {}
        }
    }

    fn view(&self, area: Rect, buf: &mut LayerStack) {
        buf.push_layer(); // index 1: UI
        buf.push_layer(); // index 2: EFFECTS
        self.paint_background(area, buf);
        self.paint_ui(area, buf);
        self.paint_effects(area, buf);
    }

    fn should_quit(&self) -> bool {
        self.quit
    }

    fn tick_rate(&self) -> Option<Duration> {
        Some(TICK_INTERVAL)
    }

    fn on_tick(&mut self, _elapsed: Duration) {
        if self.flash_ticks_remaining > 0 {
            self.flash_ticks_remaining -= 1;
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut app = SmashCrabs::new();
    run(&mut app)
}
```

- [ ] **Step 3: Build and run the full test suite**

Run: `cargo build --examples && cargo test`
Expected: PASS — all three examples build, full library test suite stays green.

- [ ] **Step 4: `cargo fmt` / `cargo clippy`**

Run: `cargo fmt --check && cargo clippy --all-targets`
Expected: both clean, no warnings.

- [ ] **Step 5: Manual smoke check**

Run: `cargo run --example smash_crabs`
Expected: the arena background (sand-colored, red-accented text cells) fills the whole screen; a "Fighters" panel appears in the top-left corner (bordered with `=`/`|`/`+` glyphs) showing "P1: 100 HP" and "P2: 100 HP", with the arena background visible everywhere else on screen; pressing Space produces a brief yellow flash rectangle in the center of the screen that clears back to the arena/UI beneath it after roughly 200ms, and decrements P2's HP by 10; `q` quits.

- [ ] **Step 6: Commit**

```bash
git add examples/smash_crabs.rs
git commit -m "feat(examples): add Smash Crabs example exercising the multi-layer path end-to-end"
```

---

## Self-Review Notes

- **Spec coverage:** Task 1 covers the design spec's section 1 (`LayerStack` cleanup) in full — field removal, `composite()` rewrite, `layer_count()`/`layer()`. Task 2 covers section 2 (Rev B forward pointer) in full. Task 3 covers section 3 (the Smash Crabs example) in full, including the three-layer structure (background/UI/effects), the theme choice rationale, and the tick-driven effect lifecycle. No spec section lacks a task.
- **Placeholder scan:** no TBDs; every step has literal code, literal before/after text, or literal commands.
- **Type consistency:** `LayerStack::layer(&self, index: usize) -> &Buffer` and `layer_count(&self) -> usize` are introduced once in Task 1 and match the names used in this plan's own interface notes; Task 3 uses only `layer_mut`/`push_layer`, both pre-existing and unchanged by Task 1, so there is no cross-task signature drift to check beyond confirming Task 3 doesn't rely on `layer`/`layer_count` (it doesn't — they exist for future app authors following the spec's `const BACKGROUND/UI/EFFECTS` pattern to introspect the stack, not because this example needs them).
