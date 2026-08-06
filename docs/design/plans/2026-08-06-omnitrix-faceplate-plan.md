# Omnitrix Faceplate Dial-Navigation Hub (Issue #42) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `docs/design/specs/2026-08-06-omnitrix-faceplate-design.md`
(GitHub issue #42, tracking #52): add a Faceplate navigation screen
(3-item selectable DNA-sample list, Tab/Shift+Tab/Enter interaction) and a
placeholder `Launched` screen to `examples/omnitrix.rs`.

**Architecture:** One self-contained task, entirely within
`examples/omnitrix.rs`. State (two new enums, two new struct fields),
interaction (`update()`), and rendering (`view()`) are added together —
splitting them would leave an intermediate compile state where the new
fields are written but never read, which fails this repo's
`cargo clippy --all-targets -- -D warnings` CI gate on unused/dead code.
No `src/` changes; `List`, `Block`, `Text`, and `Theme` are reused
unmodified.

**Tech Stack:** Rust, `crossterm`, `ttui` (this crate). No new dependencies.

## Global Constraints

- `examples/omnitrix.rs` is example code — per
  `.claude/rules/development-conventions.md`'s TDD exceptions, verified
  by running the example, not unit tested. No new automated test.
- `cargo fmt` / `cargo clippy --all-targets` clean.
- No new dependencies. No changes to `src/`.
- No real scrolling logic — `List` (`src/widgets/list.rs`) is used exactly
  as it exists today; the 3-item DNA-sample list always fits.
- `Screen::Launched` selection index is preserved on return to
  `Screen::Faceplate` via `Esc` — the user lands back on the sample they
  launched, not reset to index 0.

---

### Task 1: Faceplate + Launched screens

**Files:**
- Modify: `examples/omnitrix.rs`

**Interfaces produced:** none public — `Omnitrix`, `DnaSample`, and
`Screen` are all private to this example binary; no other file consumes
them.

- [ ] **Step 1: Add the `DnaSample` and `Screen` types**

Add above the `struct Omnitrix` definition (currently at line 15):

```rust
#[derive(Clone, Copy, PartialEq)]
enum DnaSample {
    Brainstorm,
    Fasttrack,
    Upgrade,
}

impl DnaSample {
    const ALL: [DnaSample; 3] = [DnaSample::Brainstorm, DnaSample::Fasttrack, DnaSample::Upgrade];

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

- [ ] **Step 2: Add `selected` and `screen` fields to `Omnitrix`, and
  initialize them in `Omnitrix::new()`**

Change the struct definition (currently lines 15-20):

```rust
struct Omnitrix {
    pulse_phase: f32,
    quit: bool,
    last_tick_started: Instant,
    perf_log: std::fs::File,
    selected: usize,
    screen: Screen,
}
```

And the two new fields in the `Omnitrix { .. }` literal inside `new()`
(currently lines 29-34), appended after `perf_log`:

```rust
Omnitrix {
    pulse_phase: 0.0,
    quit: false,
    last_tick_started: Instant::now(),
    perf_log,
    selected: 0,
    screen: Screen::Faceplate,
}
```

- [ ] **Step 3: Add the `List` import**

Change the existing widgets import (currently line 11) from:

```rust
use ttui::widgets::{block::Block, text::Text};
```

to:

```rust
use ttui::widgets::{block::Block, list::List, text::Text};
```

- [ ] **Step 4: Add Tab/Shift+Tab/Enter/Esc handling to `update()`**

Replace the `update()` method body (currently lines 64-72):

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
    match self.screen {
        Screen::Faceplate => match k.code {
            KeyCode::Tab => self.selected = (self.selected + 1) % DnaSample::ALL.len(),
            KeyCode::BackTab => {
                self.selected = (self.selected + DnaSample::ALL.len() - 1) % DnaSample::ALL.len()
            }
            KeyCode::Enter => self.screen = Screen::Launched(DnaSample::ALL[self.selected]),
            _ => {}
        },
        Screen::Launched(_) => {
            if k.code == KeyCode::Esc {
                self.screen = Screen::Faceplate;
            }
        }
    }
}
```

  Note the added `return` after setting `self.quit = true` — without it,
  execution would fall into the `match self.screen` block on the same
  keypress, which is harmless here (no arm matches `Char('q')`) but the
  early return makes the quit-takes-priority intent explicit.

- [ ] **Step 5: Branch `view()` on `self.screen`**

Replace the `view()` method body (currently lines 74-81):

```rust
fn view(&self, area: Rect, buf: &mut LayerStack) {
    let theme = self.theme();
    let inner = Block::new()
        .title("Omnitrix")
        .theme(&theme)
        .render(area, buf);

    match &self.screen {
        Screen::Faceplate => {
            let names: Vec<String> = DnaSample::ALL.iter().map(|s| s.name().to_string()).collect();
            List::new(&names, self.selected).render(inner, buf);
        }
        Screen::Launched(sample) => {
            let name_row = Rect {
                x: inner.x,
                y: inner.y,
                width: inner.width,
                height: 1,
            };
            let placeholder_row = Rect {
                x: inner.x,
                y: inner.y + 1,
                width: inner.width,
                height: inner.height.saturating_sub(1),
            };
            Text::new(sample.name()).render(name_row, buf);
            Text::new("(not yet built)").render(placeholder_row, buf);
        }
    }
}
```

- [ ] **Step 6: Build**

Run: `cargo build --example omnitrix`
Expected: compiles cleanly, no warnings.

- [ ] **Step 7: `cargo fmt && cargo clippy --all-targets -- -D warnings`**

Expected: clean, no warnings (matches CI's exact clippy invocation).

- [ ] **Step 8: Manual verification** (real-terminal check, not
  automatable — per this project's TDD exceptions for example code):

Run: `cargo run --example omnitrix`

Confirm:
- App opens on the Faceplate screen showing 3 rows: "Brainstorm",
  "Fasttrack", "Upgrade", with "Brainstorm" highlighted (row 0 selected).
- `Tab` moves the highlight down (Brainstorm → Fasttrack → Upgrade), and
  wraps back to Brainstorm after Upgrade.
- `Shift+Tab` moves the highlight up, and wraps to Upgrade from
  Brainstorm.
- Pressing `Enter` on each of the 3 samples shows a screen with that
  sample's name and "(not yet built)" beneath it.
- `Esc` from a `Launched` screen returns to the Faceplate with the same
  sample still highlighted (not reset to Brainstorm) — e.g. launch
  Upgrade, press Esc, confirm Upgrade (not Brainstorm) is highlighted.
- The border keeps pulsing/bolding (from #41) on both screens, unchanged.
- `q` quits cleanly from both the Faceplate and a `Launched` screen, no
  panic, no leftover terminal attributes in the shell prompt after exit.

- [ ] **Step 9: Commit**

```bash
git add examples/omnitrix.rs
git commit -m "feat(omnitrix): add Faceplate dial-navigation hub (#42)"
```

---

## Self-Review

**Spec coverage:** `DnaSample`/`Screen` state — Step 1-2. Tab/Shift+Tab
cycling with wraparound, Enter launch — Step 4. Esc return with preserved
selection — Step 4. Faceplate rendering via unmodified `List` — Step 5.
`Launched` placeholder rendering (name + "(not yet built)") — Step 5.
q-quits-from-either-screen — Step 4 (unconditional check before the
screen match). No scrolling, no `src/` changes, no corruption transition
— none added anywhere in this plan. Manual verification of every
interaction path — Step 8.

**Placeholder scan:** no TBD/TODO; every step has literal code or an
exact command.

**Type consistency:** `DnaSample`, `DnaSample::ALL`, `DnaSample::name()`,
and `Screen::{Faceplate, Launched}` are defined once in Step 1 and used
identically (same variant names, same method name) in Steps 2, 4, and 5 —
no renames across steps.
