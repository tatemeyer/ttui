# Omnitrix Widgets + Sub-Apps + Boot Arc Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `docs/design/specs/2026-08-06-omnitrix-sub-apps-boot-arc-design.md`:
two new core widgets (`EnergyCore`, `DNAConsole`) and real content for
all three of Omnitrix's Brainstorm/Fasttrack/Upgrade destinations (all
currently "(not yet built)" placeholders), plus a materialization boot
sequence — completing the Omnitrix example.

**Architecture:** Six tasks, same order as the spec's slices. Tasks 1-2
are core-framework (`src/widgets/`), TDD-mandatory, independent of each
other. Tasks 3-6 are all `examples/omnitrix.rs`, strictly sequential —
unlike the Smash Crabs/TARDIS arcs, there's no forward-dependency
reordering needed here: boot (Task 6) only reuses `Block`/`theme()`
(already present from the start) to render its trace-out phase, not any
sub-app's content, so it can safely stay last without needing anything
from Tasks 3-5 first.

**Tech Stack:** Rust, `crossterm`. Reuses `ttui::camera::dim` (built for
the TARDIS arc) — no new dependency, no `Cargo.toml` change.

## Global Constraints

- TDD mandatory for Tasks 1-2 (`coding`-tagged, no exception applies).
  Tasks 3-6 (`examples/omnitrix.rs`) are example code — per
  `.claude/rules/development-conventions.md`'s TDD exceptions, verified
  by running the example, not unit tested.
- Inline `#[cfg(test)] mod tests` per module — no new `tests/` directory.
- `cargo fmt` / `cargo clippy --all-targets -- -D warnings` clean after
  every task. Use `.is_multiple_of(2)` instead of `% 2 == 0` (the
  `clippy::manual_is_multiple_of` lint has hit this project's code
  twice already).
- No RNG anywhere: the border noise overlay, the lock-on ring, and the
  circuit chains are all deterministic, matching this codebase's
  established posture.
- Multi-row rendering in Tasks 3-4 (chat log, target lists) must bounds-
  check each row's `y` against the scratch buffer's declared height
  before writing — those buffers are exact-sized zero-origin `Buffer`s
  (`Buffer::new(area.width, area.height)`, no bounds-checking of its
  own), and both new sections render more rows than any prior example
  in this codebase, so an unguarded row offset can panic on small
  terminals in a way earlier examples' shorter fixed layouts never
  risked. Use the new `render_row` free function (Task 3) for all
  multi-row raw text in both tasks rather than ad hoc `Text::render`
  calls with unchecked `y`.
- Geometric-shape/Dingbat glyphs (`◉ ○ ● ✦`) are used deliberately in
  this arc (see the spec's Global Constraint) — Ambiguous Unicode
  width, safe in the overwhelming majority of terminals but not
  guaranteed narrow the way ASCII/Box-Drawing/Block-Elements are. Not
  re-litigated per task.

---

### Task 1: `EnergyCore` widget (`src/widgets/energy_core.rs`, #46)

**Files:**
- Create: `src/widgets/energy_core.rs`
- Modify: `src/widgets/mod.rs`

**Interfaces produced:**
```rust
pub struct EnergyCore { /* private */ }
impl EnergyCore {
    pub fn new(percent: u16, color: Color) -> Self;
    pub fn render(&self, area: Rect, buf: &mut Buffer);
}
```

- [ ] **Step 1: Write the failing tests** — create `src/widgets/energy_core.rs`:

```rust
use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;
use crossterm::style::Color;

pub struct EnergyCore {
    percent: u16,
    color: Color,
}

impl EnergyCore {
    pub fn new(percent: u16, color: Color) -> Self {
        EnergyCore { percent, color }
    }

    pub fn render(&self, _area: Rect, _buf: &mut Buffer) {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area10x1() -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 1,
        }
    }

    #[test]
    fn zero_percent_renders_all_empty_track() {
        let mut buf = Buffer::new(10, 1);
        EnergyCore::new(0, Color::Green).render(area10x1(), &mut buf);

        for x in 0..10 {
            assert_eq!(buf.get(x, 0).symbol, '░');
            assert_eq!(buf.get(x, 0).fg, Color::Green);
        }
    }

    #[test]
    fn fifty_percent_fills_half() {
        let mut buf = Buffer::new(10, 1);
        EnergyCore::new(50, Color::Green).render(area10x1(), &mut buf);

        for x in 0..5 {
            assert_eq!(buf.get(x, 0).symbol, '▓');
        }
        for x in 5..10 {
            assert_eq!(buf.get(x, 0).symbol, '░');
        }
    }

    #[test]
    fn full_percent_sparks_every_fourth_cell() {
        let mut buf = Buffer::new(8, 1);
        let area = Rect {
            x: 0,
            y: 0,
            width: 8,
            height: 1,
        };
        EnergyCore::new(100, Color::Green).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, '✦');
        assert_eq!(buf.get(0, 0).fg, Color::White);
        assert_eq!(buf.get(1, 0).symbol, '▓');
        assert_eq!(buf.get(4, 0).symbol, '✦');
        assert_eq!(buf.get(5, 0).symbol, '▓');
    }

    #[test]
    fn zero_width_area_does_not_panic() {
        let mut buf = Buffer::new(1, 1);
        let area = Rect {
            x: 0,
            y: 0,
            width: 0,
            height: 1,
        };
        EnergyCore::new(50, Color::Green).render(area, &mut buf);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib widgets::energy_core::tests`
Expected: the 3 render-dependent tests FAIL (`not implemented`); the
zero-width test also fails the same way (it still calls `render`).

- [ ] **Step 3: Implement** — replace the `render` method body:

```rust
impl EnergyCore {
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let filled_width = (area.width as u32 * self.percent.min(100) as u32 / 100) as u16;
        for x in 0..area.width {
            let filled = x < filled_width;
            let spark = self.percent >= 100 && filled && x % 4 == 0;
            let (symbol, fg) = if spark {
                ('✦', Color::White)
            } else if filled {
                ('▓', self.color)
            } else {
                ('░', self.color)
            };
            buf.set(
                area.x + x,
                area.y,
                Cell {
                    symbol,
                    fg,
                    bg: Color::Reset,
                    ..Default::default()
                },
            );
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib widgets::energy_core::tests`
Expected: all 4 PASS.

- [ ] **Step 5: Register the module** — add `pub mod energy_core;` to
  `src/widgets/mod.rs` (alphabetically, before `list`).

- [ ] **Step 6: Run full check**

Run: `cargo test --lib && cargo build --examples && cargo fmt && cargo clippy --all-targets -- -D warnings`
Expected: all pass, no warnings.

- [ ] **Step 7: Commit**

```bash
git add src/widgets/energy_core.rs src/widgets/mod.rs
git commit -m "feat(widgets): add EnergyCore widget"
```

---

### Task 2: `DNAConsole` widget (`src/widgets/dna_console.rs`, #47)

**Files:**
- Create: `src/widgets/dna_console.rs`
- Modify: `src/widgets/mod.rs`

**Interfaces produced:**
```rust
pub struct DNAConsole<'a> { /* private */ }
impl<'a> DNAConsole<'a> {
    pub fn new(content: &'a str, primary: Color, secondary: Color) -> Self;
    pub fn render(&self, area: Rect, buf: &mut Buffer);
}
```

- [ ] **Step 1: Write the failing tests** — create `src/widgets/dna_console.rs`:

```rust
use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;
use crossterm::style::Color;

pub struct DNAConsole<'a> {
    content: &'a str,
    primary: Color,
    secondary: Color,
}

impl<'a> DNAConsole<'a> {
    pub fn new(content: &'a str, primary: Color, secondary: Color) -> Self {
        DNAConsole {
            content,
            primary,
            secondary,
        }
    }

    pub fn render(&self, _area: Rect, _buf: &mut Buffer) {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alternates_colors_per_character_with_a_trailing_cursor() {
        let mut buf = Buffer::new(10, 1);
        let area = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 1,
        };

        DNAConsole::new("AB", Color::Red, Color::Blue).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, 'A');
        assert_eq!(buf.get(0, 0).fg, Color::Red);
        assert_eq!(buf.get(1, 0).symbol, 'B');
        assert_eq!(buf.get(1, 0).fg, Color::Blue);
        assert_eq!(buf.get(2, 0).symbol, '▌');
        assert_eq!(buf.get(2, 0).fg, Color::Red);
    }

    #[test]
    fn zero_width_area_renders_nothing_without_panic() {
        let mut buf = Buffer::new(1, 1);
        let area = Rect {
            x: 0,
            y: 0,
            width: 0,
            height: 1,
        };
        DNAConsole::new("A", Color::Red, Color::Blue).render(area, &mut buf);
    }

    #[test]
    fn one_wide_area_renders_only_the_cursor() {
        let mut buf = Buffer::new(1, 1);
        let area = Rect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        };

        DNAConsole::new("AB", Color::Red, Color::Blue).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, '▌');
        assert_eq!(buf.get(0, 0).fg, Color::Red);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib widgets::dna_console::tests`
Expected: `alternates_colors_per_character_with_a_trailing_cursor` and
`one_wide_area_renders_only_the_cursor` FAIL (`not implemented`);
`zero_width_area_renders_nothing_without_panic` also fails the same way.

- [ ] **Step 3: Implement** — replace the `render` method body:

```rust
impl<'a> DNAConsole<'a> {
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 {
            return;
        }
        let max_content = area.width.saturating_sub(1) as usize;
        let mut count: u16 = 0;
        for (i, ch) in self.content.chars().take(max_content).enumerate() {
            let color = if i % 2 == 0 {
                self.primary
            } else {
                self.secondary
            };
            buf.set(
                area.x + i as u16,
                area.y,
                Cell {
                    symbol: ch,
                    fg: color,
                    bg: Color::Reset,
                    ..Default::default()
                },
            );
            count = i as u16 + 1;
        }
        if count < area.width {
            buf.set(
                area.x + count,
                area.y,
                Cell {
                    symbol: '▌',
                    fg: self.primary,
                    bg: Color::Reset,
                    ..Default::default()
                },
            );
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib widgets::dna_console::tests`
Expected: all 3 PASS.

- [ ] **Step 5: Register the module** — add `pub mod dna_console;` to
  `src/widgets/mod.rs` (alphabetically, after `dial`).

- [ ] **Step 6: Run full check**

Run: `cargo test --lib && cargo build --examples && cargo fmt && cargo clippy --all-targets -- -D warnings`
Expected: all pass, no warnings.

- [ ] **Step 7: Commit**

```bash
git add src/widgets/dna_console.rs src/widgets/mod.rs
git commit -m "feat(widgets): add DNAConsole widget"
```

---

### Task 3: Brainstorm sub-app (`examples/omnitrix.rs`, #48)

**Files:**
- Modify: `examples/omnitrix.rs`

**Interfaces consumed:** `DNAConsole::new(content, primary, secondary)
.render(area, buf)` (Task 2); the existing `braille_noise(x, y, tick) ->
char` free function already in this file (from the dial-navigation
arc); the existing `Transition`/`easing` imports.

**Interfaces produced:** `render_row(buf, area, y, text, fg, bg)` free
function, reused by Task 4.

No new tests — example code, verified by running.

- [ ] **Step 1: Update imports** — add `dna_console::DNAConsole` to the
  existing widgets import line:

```rust
use ttui::widgets::{block::Block, dial::Dial, dna_console::DNAConsole, text::Text};
```

- [ ] **Step 2: Add the `ChatSpeaker` enum and Brainstorm constants** —
  insert above `struct Omnitrix`:

```rust
#[derive(Clone, Copy, PartialEq)]
enum ChatSpeaker {
    User,
    Agent,
}

const CANNED_PROMPTS: [&str; 3] = [
    "Summarize my inbox",
    "Draft a release note",
    "Explain this stack trace",
];
const BRAINSTORM_THINKING_MS: u64 = 1200;
const PREVIEW_REVEAL_MS: u64 = 400;
```

- [ ] **Step 3: Add fields to `Omnitrix`** — change the struct
  definition (insert after `tick_count`):

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
    chat_log: Vec<(ChatSpeaker, String)>,
    prompt_index: usize,
    thinking: Option<Transition>,
    preview_reveal: Transition,
}
```

  and in `new()`:

```rust
            tick_count: 0,
            chat_log: Vec::new(),
            prompt_index: 0,
            thinking: None,
            preview_reveal: Transition::start(Duration::from_millis(PREVIEW_REVEAL_MS)),
        }
    }
```

- [ ] **Step 4: Add `render_row` free function** — add near the
  existing `blit`/`braille_noise` free functions:

```rust
fn render_row(buf: &mut Buffer, area: Rect, y: u16, text: &str, fg: Color, bg: Color) {
    if y >= area.height {
        return;
    }
    for (i, ch) in text.chars().take(area.width as usize).enumerate() {
        buf.set(
            area.x + i as u16,
            area.y + y,
            Cell {
                symbol: ch,
                fg,
                bg,
                ..Default::default()
            },
        );
    }
}
```

  This guards `y` against `area.height` before writing — the scratch
  buffers `render_mode_content` builds are exact-sized zero-origin
  `Buffer`s with no bounds-checking of their own, and Brainstorm's chat
  log (up to 5 rows) and Fasttrack's target lists (Task 4) render more
  rows than any prior content in this file.

- [ ] **Step 5: Add the Brainstorm arm to `render_mode_content`** — in
  the `match mode` block, replace the `_ => { ...placeholder... }` arm
  with an explicit `AppMode::Brainstorm` arm, keeping `_` for the still-
  unbuilt Fasttrack/Upgrade:

```rust
            AppMode::Brainstorm => {
                let log_area = Rect {
                    x: local.x,
                    y: local.y,
                    width: local.width,
                    height: local.height.saturating_sub(2),
                };
                let start = self.chat_log.len().saturating_sub(5);
                for (i, (speaker, text)) in self.chat_log[start..].iter().enumerate() {
                    let prefix = match speaker {
                        ChatSpeaker::User => "You: ",
                        ChatSpeaker::Agent => "Agent: ",
                    };
                    render_row(
                        &mut buf,
                        log_area,
                        i as u16,
                        &format!("{prefix}{text}"),
                        Color::Reset,
                        Color::Reset,
                    );
                }

                let input_row = Rect {
                    x: local.x,
                    y: local.y + local.height.saturating_sub(2),
                    width: local.width,
                    height: 1,
                };
                let prompt = CANNED_PROMPTS[self.prompt_index];
                let reveal_len =
                    ((prompt.chars().count() as f32) * self.preview_reveal.progress()) as usize;
                let preview = &prompt[..reveal_len.min(prompt.len())];
                let theme = self.theme();
                DNAConsole::new(preview, theme.primary, theme.secondary).render(input_row, &mut buf);

                let hint_row = Rect {
                    x: local.x,
                    y: local.y + local.height.saturating_sub(1),
                    width: local.width,
                    height: local.height.saturating_sub(1).min(1),
                };
                Text::new("Tab cycle * Enter send * Esc back * q quit").render(hint_row, &mut buf);
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
```

  (`CANNED_PROMPTS` are plain ASCII, so `prompt.chars().count()` equals
  `prompt.len()` — the byte slice `&prompt[..reveal_len]` is always
  valid.)

- [ ] **Step 6: Add the Brainstorm arm to `update()`** — replace the
  `match self.mode` block's structure: keep `AppMode::Faceplate`'s arm
  unchanged, add a new `AppMode::Brainstorm` arm, and narrow the
  trailing catch-all to the two modes not yet built:

```rust
        match self.mode {
            AppMode::Faceplate => match k.code {
                KeyCode::Tab => self.selected = (self.selected + 1) % SAMPLES.len(),
                KeyCode::BackTab => {
                    self.selected = (self.selected + SAMPLES.len() - 1) % SAMPLES.len()
                }
                KeyCode::Enter => self.switch_mode(AppMode::from_selected(self.selected)),
                _ => {}
            },
            AppMode::Brainstorm => {
                if self.thinking.is_some() {
                    return;
                }
                match k.code {
                    KeyCode::Tab => {
                        self.prompt_index = (self.prompt_index + 1) % CANNED_PROMPTS.len();
                        self.preview_reveal =
                            Transition::start(Duration::from_millis(PREVIEW_REVEAL_MS));
                    }
                    KeyCode::BackTab => {
                        self.prompt_index =
                            (self.prompt_index + CANNED_PROMPTS.len() - 1) % CANNED_PROMPTS.len();
                        self.preview_reveal =
                            Transition::start(Duration::from_millis(PREVIEW_REVEAL_MS));
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        self.chat_log.push((
                            ChatSpeaker::User,
                            CANNED_PROMPTS[self.prompt_index].to_string(),
                        ));
                        self.thinking =
                            Some(Transition::start(Duration::from_millis(BRAINSTORM_THINKING_MS)));
                    }
                    KeyCode::Esc => self.switch_mode(AppMode::Faceplate),
                    _ => {}
                }
            }
            _ => {
                if k.code == KeyCode::Esc {
                    self.switch_mode(AppMode::Faceplate);
                }
            }
        }
    }
```

- [ ] **Step 7: Add `overlay_border_noise` and wire it into `view()`**
  — add the method to `impl Omnitrix`:

```rust
    fn overlay_border_noise(&self, area: Rect, buf: &mut LayerStack) {
        let theme = self.theme();
        for x in area.x..area.x + area.width {
            if (x as u64 + self.tick_count) % 5 == 0 {
                buf.set(
                    x,
                    area.y,
                    Cell {
                        symbol: braille_noise(x, area.y, self.tick_count),
                        fg: theme.primary,
                        bg: Color::Reset,
                        ..Default::default()
                    },
                );
                buf.set(
                    x,
                    area.y + area.height - 1,
                    Cell {
                        symbol: braille_noise(x, area.y + area.height - 1, self.tick_count),
                        fg: theme.primary,
                        bg: Color::Reset,
                        ..Default::default()
                    },
                );
            }
        }
        for y in area.y..area.y + area.height {
            if (y as u64 + self.tick_count) % 5 == 0 {
                buf.set(
                    area.x,
                    y,
                    Cell {
                        symbol: braille_noise(area.x, y, self.tick_count),
                        fg: theme.primary,
                        bg: Color::Reset,
                        ..Default::default()
                    },
                );
                buf.set(
                    area.x + area.width - 1,
                    y,
                    Cell {
                        symbol: braille_noise(area.x + area.width - 1, y, self.tick_count),
                        fg: theme.primary,
                        bg: Color::Reset,
                        ..Default::default()
                    },
                );
            }
        }
    }
```

  and in `view()`, right after `Block::render`:

```rust
    fn view(&self, area: Rect, buf: &mut LayerStack) {
        let theme = self.theme();
        let inner = Block::new()
            .title("Omnitrix")
            .theme(&theme)
            .render(area, buf);

        if self.mode == AppMode::Brainstorm && self.thinking.is_some() {
            self.overlay_border_noise(area, buf);
        }

        match &self.transitioning_from {
```

  (the rest of `view()` is unchanged).

- [ ] **Step 8: Tick `thinking`/`preview_reveal` and modulate the pulse
  rate in `on_tick`** — replace the existing single pulse-phase line:

```rust
        self.pulse_phase += elapsed.as_secs_f32() * std::f32::consts::PI;
```

  with:

```rust
        let pulse_rate = if self.mode == AppMode::Brainstorm && self.thinking.is_some() {
            3.0
        } else {
            1.0
        };
        self.pulse_phase += elapsed.as_secs_f32() * std::f32::consts::PI * pulse_rate;
```

  and append, after the existing `transitioning_from` tick block
  (before the closing brace of `on_tick`):

```rust
        if let Some(t) = &mut self.thinking {
            t.tick(elapsed);
            if t.is_complete() {
                let prompt = CANNED_PROMPTS[self.prompt_index];
                self.chat_log
                    .push((ChatSpeaker::Agent, format!("{prompt} ... complete.")));
                self.thinking = None;
            }
        }
        self.preview_reveal.tick(elapsed);
```

- [ ] **Step 9: Add the `Buffer`/`Cell` import needed by `render_row`**
  — confirm the top-of-file import already reads
  `use ttui::buffer::{Buffer, Cell, LayerStack};` (it does, from the
  dial-navigation arc's corruption transition — no change needed here,
  just confirming before building).

- [ ] **Step 10: Build**

Run: `cargo build --example omnitrix`
Expected: compiles cleanly, no warnings.

- [ ] **Step 11: `cargo fmt && cargo clippy --all-targets -- -D warnings`**

Expected: clean, no warnings.

- [ ] **Step 12: Manual verification**

Run: `cargo run --example omnitrix`

Navigate to Brainstorm and confirm:
- The first prompt types itself out over ~400ms on entry.
- `Tab`/`Shift+Tab` cycle prompts, each re-typing itself out.
- `Enter`/`Space` sends the prompt (appears in the log as "You: ..."),
  then for ~1.2s the border pulses visibly faster and shows sparse
  flickering Braille noise on top of its normal glyphs, then an
  "Agent: ..." reply appears.
- `Left`/`Right`/`Tab`/`Esc` are all ignored while "thinking" (only `q`
  works).
- `Esc` (when not thinking) returns to Faceplate; the existing
  corruption transition still plays.
- `q` quits cleanly.

- [ ] **Step 13: Commit**

```bash
git add examples/omnitrix.rs
git commit -m "feat(omnitrix): add Brainstorm sub-app (#48)"
```

---

### Task 4: Fasttrack sub-app (`examples/omnitrix.rs`, #49)

**Files:**
- Modify: `examples/omnitrix.rs`

**Interfaces consumed:** `EnergyCore::new(percent, color).render(area,
buf)` (Task 1); `render_row` (Task 3).

**Interfaces produced:** `render_lock_on_ring` method, reused nowhere
else in this arc but following the same "local decoration, not a core
widget" shape `render_circuit` (Task 5) will also use.

No new tests — example code, verified by running.

- [ ] **Step 1: Update imports** — add `energy_core::EnergyCore` to the
  widgets import line:

```rust
use ttui::widgets::{
    block::Block, dial::Dial, dna_console::DNAConsole, energy_core::EnergyCore, text::Text,
};
```

- [ ] **Step 2: Add Fasttrack constants** — alongside
  `PREVIEW_REVEAL_MS`:

```rust
const LOCK_ON_MS: u64 = 900;
const COMPLETE_FLASH_MS: u64 = 300;
const RING_POINTS: usize = 8;
```

- [ ] **Step 3: Add fields to `Omnitrix`** — insert after
  `preview_reveal`:

```rust
    preview_reveal: Transition,
    targets: Vec<(String, bool)>,
    target_selected: usize,
    lock_on: Option<(usize, Transition)>,
    complete_flash: Option<Transition>,
}
```

  and in `new()`:

```rust
            preview_reveal: Transition::start(Duration::from_millis(PREVIEW_REVEAL_MS)),
            targets: vec![
                ("Fix login bug".to_string(), false),
                ("Write tests".to_string(), false),
                ("Ship release".to_string(), false),
            ],
            target_selected: 0,
            lock_on: None,
            complete_flash: None,
        }
    }
```

- [ ] **Step 4: Add `active_target_indices` and `render_lock_on_ring`**
  — add to `impl Omnitrix`:

```rust
    fn active_target_indices(&self) -> Vec<usize> {
        self.targets
            .iter()
            .enumerate()
            .filter(|(_, (_, done))| !done)
            .map(|(i, _)| i)
            .collect()
    }

    fn render_lock_on_ring(&self, area: Rect, progress: f32, buf: &mut Buffer) {
        let cx = area.x as f32 + area.width as f32 / 2.0;
        let cy = area.y as f32 + area.height as f32 / 2.0;
        let radius_x = 4.0;
        let radius_y = 2.0;
        let lit_count = (progress * RING_POINTS as f32) as usize;
        let theme = self.theme();
        for i in 0..RING_POINTS {
            let angle = i as f32 * std::f32::consts::TAU / RING_POINTS as f32
                - std::f32::consts::FRAC_PI_2;
            let px = (cx + radius_x * angle.cos()).round();
            let py = (cy + radius_y * angle.sin()).round();
            if px >= area.x as f32
                && py >= area.y as f32
                && (px as u16) < area.x + area.width
                && (py as u16) < area.y + area.height
            {
                let (symbol, color) = if i < lit_count {
                    ('●', theme.primary)
                } else {
                    ('○', theme.secondary)
                };
                buf.set(
                    px as u16,
                    py as u16,
                    Cell {
                        symbol,
                        fg: color,
                        bg: Color::Reset,
                        ..Default::default()
                    },
                );
            }
        }
    }
```

  This reuses the same point-on-circle formula shape `Dial` uses
  internally (`angle = i * TAU / N - FRAC_PI_2`, aspect-corrected
  `radius_x`/`radius_y`) — written fresh here since `Dial`'s ring-
  drawing isn't exposed as a reusable function, not by modifying the
  shipped `Dial` widget.

- [ ] **Step 5: Add the Fasttrack arm to `render_mode_content`** —
  replace the `_ => { ...placeholder... }` arm again, this time
  splitting out `AppMode::Fasttrack` and narrowing `_` to just
  `AppMode::Upgrade`:

```rust
            AppMode::Fasttrack => {
                let active = self.active_target_indices();
                let mut y: u16 = 0;

                render_row(&mut buf, local, y, "Targets", Color::Reset, Color::Reset);
                y += 1;
                for (row, &idx) in active.iter().enumerate() {
                    let is_selected = row == self.target_selected;
                    let (fg, bg) = if is_selected {
                        (Color::Black, Color::White)
                    } else {
                        (Color::Reset, Color::Reset)
                    };
                    let line = format!("○ {}", self.targets[idx].0);
                    render_row(&mut buf, local, y, &line, fg, bg);
                    y += 1;
                }
                y += 1;

                if let Some((_, t)) = &self.lock_on {
                    if y < local.height {
                        let ring_area = Rect {
                            x: local.x,
                            y: local.y + y,
                            width: 9.min(local.width),
                            height: 5.min(local.height.saturating_sub(y)),
                        };
                        self.render_lock_on_ring(ring_area, t.progress(), &mut buf);
                    }
                }
                y += 6;

                render_row(&mut buf, local, y, "Completed", Color::Reset, Color::Reset);
                y += 1;
                let completed: Vec<&(String, bool)> =
                    self.targets.iter().filter(|(_, done)| *done).collect();
                let completed_len = completed.len();
                for (row, (name, _)) in completed.iter().enumerate() {
                    let flashing = self.complete_flash.is_some() && row + 1 == completed_len;
                    let bg = if flashing {
                        self.theme().accent
                    } else {
                        Color::Reset
                    };
                    let line = format!("◉ {name}");
                    render_row(&mut buf, local, y, &line, self.theme().secondary, bg);
                    y += 1;
                }
                y += 1;

                let percent = (completed_len as u32 * 100 / 3) as u16;
                if y < local.height {
                    let bar_area = Rect {
                        x: local.x,
                        y: local.y + y,
                        width: local.width,
                        height: 1,
                    };
                    EnergyCore::new(percent, self.theme().primary).render(bar_area, &mut buf);
                }

                let hint_row = Rect {
                    x: local.x,
                    y: local.y + local.height.saturating_sub(1),
                    width: local.width,
                    height: local.height.saturating_sub(1).min(1),
                };
                Text::new("Tab cycle * Enter lock-on * Esc back * q quit")
                    .render(hint_row, &mut buf);
            }
            _ => {
```

  (the existing placeholder body stays as the new, narrower `_` arm's
  content — no change to its own code, just which modes it now covers).

- [ ] **Step 6: Add the Fasttrack arm to `update()`** — insert a new
  arm between `AppMode::Brainstorm` and the trailing catch-all:

```rust
            AppMode::Fasttrack => {
                if self.lock_on.is_some() {
                    return;
                }
                let active = self.active_target_indices();
                match k.code {
                    KeyCode::Tab => {
                        if !active.is_empty() {
                            self.target_selected = (self.target_selected + 1) % active.len();
                        }
                    }
                    KeyCode::BackTab => {
                        if !active.is_empty() {
                            self.target_selected =
                                (self.target_selected + active.len() - 1) % active.len();
                        }
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        if let Some(&idx) = active.get(self.target_selected) {
                            self.lock_on =
                                Some((idx, Transition::start(Duration::from_millis(LOCK_ON_MS))));
                        }
                    }
                    KeyCode::Esc => self.switch_mode(AppMode::Faceplate),
                    _ => {}
                }
            }
            _ => {
                if k.code == KeyCode::Esc {
                    self.switch_mode(AppMode::Faceplate);
                }
            }
        }
    }
```

- [ ] **Step 7: Tick `lock_on`/`complete_flash` in `on_tick`** — append
  after the `preview_reveal.tick(elapsed);` line added in Task 3:

```rust
        if let Some((idx, t)) = &mut self.lock_on {
            t.tick(elapsed);
            if t.is_complete() {
                self.targets[*idx].1 = true;
                self.complete_flash = Some(Transition::start(Duration::from_millis(COMPLETE_FLASH_MS)));
                self.target_selected = 0;
                self.lock_on = None;
            }
        }
        if let Some(t) = &mut self.complete_flash {
            t.tick(elapsed);
            if t.is_complete() {
                self.complete_flash = None;
            }
        }
```

- [ ] **Step 8: Build**

Run: `cargo build --example omnitrix`
Expected: compiles cleanly.

- [ ] **Step 9: `cargo fmt && cargo clippy --all-targets -- -D warnings`**

Expected: clean, no warnings.

- [ ] **Step 10: Manual verification**

Run: `cargo run --example omnitrix`

Navigate to Fasttrack and confirm:
- 3 targets listed under "Targets," `Tab`/`Shift+Tab` cycle the
  highlight among them.
- `Enter`/`Space` on the selected target starts a ring below the list
  that visibly fills clockwise (`○` → `●` points, one at a time) over
  ~900ms.
- On completion, the target disappears from "Targets" and appears
  under "Completed" as `◉ name`, its row flashing briefly; the
  `EnergyCore` completion bar below advances by a third.
- Repeat until all 3 are complete — the bar reaches 100% and sparks
  (`✦`) appear.
- `Left`/`Right`/`Tab`/`Esc` are ignored while the ring is filling.
- `q` quits cleanly.

- [ ] **Step 11: Commit**

```bash
git add examples/omnitrix.rs
git commit -m "feat(omnitrix): add Fasttrack sub-app (#49)"
```

---

### Task 5: Upgrade sub-app (`examples/omnitrix.rs`, #50)

**Files:**
- Modify: `examples/omnitrix.rs`

**Interfaces consumed:** none new — this task adds no new widget calls,
only a local `render_circuit` helper (same shape as Task 4's
`render_lock_on_ring`).

No new tests — example code, verified by running.

- [ ] **Step 1: Add Upgrade constants** — alongside `RING_POINTS`:

```rust
const UPGRADE_LOAD_GAIN: f32 = 15.0;
const UPGRADE_LOAD_DECAY_PER_SEC: f32 = 3.0;
const OVERLOAD_THRESHOLD: f32 = 90.0;
const CIRCUIT_NODE_COUNT: u16 = 6;
```

- [ ] **Step 2: Add the `load` field** — insert after
  `complete_flash`:

```rust
    complete_flash: Option<Transition>,
    load: f32,
}
```

  and in `new()`:

```rust
            complete_flash: None,
            load: 0.0,
        }
    }
```

- [ ] **Step 3: Add `render_circuit`** — add to `impl Omnitrix`:

```rust
    fn render_circuit(&self, area: Rect, value: f32, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let lit = ((value.min(100.0) / 100.0) * CIRCUIT_NODE_COUNT as f32) as u16;
        let theme = self.theme();
        let mut x: u16 = 0;
        for i in 0..CIRCUIT_NODE_COUNT {
            if x >= area.width {
                break;
            }
            let (symbol, color) = if i < lit {
                ('●', theme.primary)
            } else {
                ('○', theme.secondary)
            };
            buf.set(
                area.x + x,
                area.y,
                Cell {
                    symbol,
                    fg: color,
                    bg: Color::Reset,
                    ..Default::default()
                },
            );
            x += 1;
            if i + 1 < CIRCUIT_NODE_COUNT && x < area.width {
                buf.set(
                    area.x + x,
                    area.y,
                    Cell {
                        symbol: '─',
                        fg: theme.secondary,
                        bg: Color::Reset,
                        ..Default::default()
                    },
                );
                x += 1;
            }
        }
    }
```

- [ ] **Step 4: Add the Upgrade arm to `render_mode_content`** —
  replace the now-single-mode `_` arm with an explicit
  `AppMode::Upgrade` arm (the placeholder body has no `_` catch-all
  left to fall into after this — every `AppMode` variant now has its
  own arm):

```rust
            AppMode::Upgrade => {
                let cpu_label = Rect {
                    x: local.x,
                    y: local.y,
                    width: local.width,
                    height: 1,
                };
                Text::new("CPU").render(cpu_label, &mut buf);
                let cpu_row = Rect {
                    x: local.x,
                    y: local.y + 1,
                    width: local.width,
                    height: 1,
                };
                self.render_circuit(cpu_row, self.load, &mut buf);

                let ram_value = (self.load * 0.6 + 10.0).min(100.0);
                let ram_label = Rect {
                    x: local.x,
                    y: local.y + 3,
                    width: local.width,
                    height: 1,
                };
                Text::new("RAM").render(ram_label, &mut buf);
                let ram_row = Rect {
                    x: local.x,
                    y: local.y + 4,
                    width: local.width,
                    height: 1,
                };
                self.render_circuit(ram_row, ram_value, &mut buf);

                let hint_row = Rect {
                    x: local.x,
                    y: local.y + local.height.saturating_sub(1),
                    width: local.width,
                    height: local.height.saturating_sub(1).min(1),
                };
                Text::new("Space overload * Esc back * q quit").render(hint_row, &mut buf);
            }
```

  Delete the old placeholder `_ => { ... }` arm entirely — it has no
  remaining variants to match. If the compiler still shows an unused-
  match-arm or non-exhaustive-match error, that means a variant was
  missed; there should be exactly four arms now (`Faceplate`,
  `Brainstorm`, `Fasttrack`, `Upgrade`).

- [ ] **Step 5: Add the Upgrade arm to `update()`** — replace the
  trailing catch-all (which by now only ever matched `AppMode::Upgrade`)
  with an explicit arm:

```rust
            AppMode::Upgrade => match k.code {
                KeyCode::Char(' ') => self.load += UPGRADE_LOAD_GAIN,
                KeyCode::Esc => self.switch_mode(AppMode::Faceplate),
                _ => {}
            },
        }
    }
```

- [ ] **Step 6: Decay `load` in `on_tick`** — append after the
  `complete_flash` tick block:

```rust
        self.load = (self.load - UPGRADE_LOAD_DECAY_PER_SEC * elapsed.as_secs_f32()).max(0.0);
```

- [ ] **Step 7: Wire the overload flash into `theme()`** — replace the
  `Theme { .. }` construction's `primary` computation. Currently:

```rust
        let brightness = (self.pulse_phase.sin() + 1.0) / 2.0;
        let primary = Color::Rgb {
            r: 0,
            g: (120.0 + brightness * 135.0) as u8,
            b: (32.0 + brightness * 33.0) as u8,
        };
```

  becomes:

```rust
        let brightness = (self.pulse_phase.sin() + 1.0) / 2.0;
        let mut primary = Color::Rgb {
            r: 0,
            g: (120.0 + brightness * 135.0) as u8,
            b: (32.0 + brightness * 33.0) as u8,
        };
        if self.mode == AppMode::Upgrade
            && self.load >= OVERLOAD_THRESHOLD
            && self.tick_count.is_multiple_of(2)
        {
            primary = Color::Red;
        }
```

  (everything below, including the `Theme { ... primary, ... }`
  literal, is unchanged — it already reads the `primary` local.)

- [ ] **Step 8: Build**

Run: `cargo build --example omnitrix`
Expected: compiles cleanly.

- [ ] **Step 9: `cargo fmt && cargo clippy --all-targets -- -D warnings`**

Expected: clean, no warnings.

- [ ] **Step 10: Manual verification**

Run: `cargo run --example omnitrix`

Navigate to Upgrade and confirm:
- Two circuit chains ("CPU"/"RAM"), each `"●─●─...─○─○"` style,
  starting fully unlit.
- Holding Space lights up nodes left-to-right on both chains (RAM
  slightly ahead of CPU, per its derived formula) as `load` climbs.
- Past 90%, the outer border and the circuits' own lit nodes both
  visibly flash between red and the normal green pulse.
- Releasing Space lets `load` decay and the chains dim back down over
  time.
- `Esc` returns to Faceplate at any time (no busy-state gate here);
  `q` quits cleanly.

- [ ] **Step 11: Commit**

```bash
git add examples/omnitrix.rs
git commit -m "feat(omnitrix): add Upgrade sub-app (#50)"
```

---

### Task 6: Boot/intro splash (`examples/omnitrix.rs`, #51)

**Files:**
- Modify: `examples/omnitrix.rs`

**Interfaces consumed:** `ttui::camera::dim(&Buffer, f32) -> Buffer`
(built for the TARDIS arc — first reuse outside `examples/tardis.rs`);
`easing::ease_out` (Arc 0); the existing `blit` free function already
in this file.

No new tests — example code, verified by running.

- [ ] **Step 1: Add the `camera` import**:

```rust
use ttui::camera;
```

- [ ] **Step 2: Add the boot constant and hourglass art** — alongside
  `CIRCUIT_NODE_COUNT`:

```rust
const BOOT_MS: u64 = 2500;

const HOURGLASS: [&str; 5] = [
    "┌───┐",
    " \\ / ",
    "  X  ",
    " / \\ ",
    "└───┘",
];
```

- [ ] **Step 3: Add the `booting` field** — insert after `load`:

```rust
    load: f32,
    booting: Option<Transition>,
}
```

  and in `new()`:

```rust
            load: 0.0,
            booting: Some(Transition::start(Duration::from_millis(BOOT_MS))),
        }
    }
```

- [ ] **Step 4: Add `render_boot`** — add to `impl Omnitrix`:

```rust
    fn render_boot(&self, area: Rect, progress: f32, buf: &mut LayerStack) {
        for y in 0..area.height {
            for x in 0..area.width {
                buf.set(
                    area.x + x,
                    area.y + y,
                    Cell {
                        symbol: ' ',
                        fg: Color::Reset,
                        bg: Color::Black,
                        ..Default::default()
                    },
                );
            }
        }

        if progress < 0.4 {
            let factor = (1.0 - progress / 0.4).clamp(0.0, 1.0);
            let mut scratch = Buffer::new(5, 5);
            let theme = self.theme();
            for (row, line) in HOURGLASS.iter().enumerate() {
                for (col, ch) in line.chars().enumerate() {
                    scratch.set(
                        col as u16,
                        row as u16,
                        Cell {
                            symbol: ch,
                            fg: theme.primary,
                            bg: Color::Reset,
                            ..Default::default()
                        },
                    );
                }
            }
            let dimmed = camera::dim(&scratch, factor);
            let x0 = area.x + (area.width.saturating_sub(5)) / 2;
            let y0 = area.y + (area.height.saturating_sub(5)) / 2;
            blit(
                &dimmed,
                Rect {
                    x: x0,
                    y: y0,
                    width: 5,
                    height: 5,
                },
                buf,
            );
            return;
        }

        if progress < 0.55 {
            for y in 0..area.height {
                for x in 0..area.width {
                    buf.set(
                        area.x + x,
                        area.y + y,
                        Cell {
                            symbol: ' ',
                            fg: Color::Reset,
                            bg: Color::Rgb { r: 0, g: 255, b: 65 },
                            ..Default::default()
                        },
                    );
                }
            }
            return;
        }

        let trace_progress = ((progress - 0.55) / 0.45).clamp(0.0, 1.0);
        let scale = easing::ease_out(0.2, 1.0, trace_progress);
        let w = (((area.width as f32) * scale) as u16).max(2).min(area.width);
        let h = (((area.height as f32) * scale) as u16).max(2).min(area.height);
        let x = area.x + (area.width.saturating_sub(w)) / 2;
        let y = area.y + (area.height.saturating_sub(h)) / 2;
        let theme = self.theme();
        Block::new()
            .title("Omnitrix")
            .theme(&theme)
            .render(Rect { x, y, width: w, height: h }, buf);
    }
```

- [ ] **Step 5: Check `booting` first in `view()`**:

```rust
    fn view(&self, area: Rect, buf: &mut LayerStack) {
        if let Some(t) = &self.booting {
            self.render_boot(area, t.progress(), buf);
            return;
        }
        let theme = self.theme();
        let inner = Block::new()
            .title("Omnitrix")
            .theme(&theme)
            .render(area, buf);
```

  (the rest of `view()` is unchanged).

- [ ] **Step 6: Gate `update()` on `booting`** — add a guard right
  after the `q` check, before the `transitioning_from` guard:

```rust
        if k.code == KeyCode::Char('q') {
            self.quit = true;
            return;
        }
        if self.booting.is_some() {
            return;
        }
        if self.transitioning_from.is_some() {
            return;
        }
```

- [ ] **Step 7: Tick and clear `booting`** — append to the end of
  `on_tick`:

```rust
        if let Some(t) = &mut self.booting {
            t.tick(elapsed);
            if t.is_complete() {
                self.booting = None;
            }
        }
```

- [ ] **Step 8: Build**

Run: `cargo build --example omnitrix`
Expected: compiles cleanly.

- [ ] **Step 9: `cargo fmt && cargo clippy --all-targets -- -D warnings`**

Expected: clean, no warnings.

- [ ] **Step 10: Manual verification**

Run: `cargo run --example omnitrix`

Confirm the ~2.5s sequence plays once at startup: a dim hourglass
silhouette (box-drawing frame, `X` pinch) fades up to full brightness,
then a bright green flash fills the screen, then the bordered frame
grows from a small centered box out to the full window. Confirm `q`
quits cleanly even mid-boot. Confirm once boot completes, Faceplate and
all three sub-apps behave exactly as in Tasks 3-5 (this task changes
nothing about post-boot interaction).

- [ ] **Step 11: Commit**

```bash
git add examples/omnitrix.rs
git commit -m "feat(omnitrix): add materialization boot sequence (#51)"
```

---

## Self-Review

**Spec coverage:** `EnergyCore` (fill/track/spark glyphs, over-100
handling) — Task 1. `DNAConsole` (alternating colors, cursor, clipping)
— Task 2. Brainstorm (typewriter reveal, send/thinking/reply loop,
tripled pulse + border noise while thinking) — Task 3. Fasttrack (real
point-on-circle lock-on ring, Targets/Completed section split,
`EnergyCore` aggregate) — Task 4. Upgrade (circuit-node chains, *not*
`EnergyCore` bars, red overload flash on both border and circuit) —
Task 5. Boot (hourglass fade via `camera::dim`, flash, border trace-
out) — Task 6. Verification section (`cargo test`/`fmt`/`clippy` + full
manual `cargo run --example omnitrix` walkthrough covering boot through
all three sub-apps) — covered across every task's final steps. The
spec's explicitly-out-of-scope list (real text editing, real system
metrics, new dependencies) — none added anywhere in this plan.

**Placeholder scan:** no TBD/TODO in code or commands. The bounds-
safety concern for multi-row rendering (flagged as a Global Constraint,
not present in the spec itself) is a legitimate implementation-level
detail filled in during plan-writing, same precedent as prior arcs'
plans adding concrete test values the spec didn't need to enumerate.

**Type consistency:** `EnergyCore::new(percent, color)` (Task 1) and
`DNAConsole::new(content, primary, secondary)` (Task 2) match every
call site in Tasks 3-5 exactly. `render_row` (Task 3) is reused
verbatim by Task 4 with no signature drift. `render_lock_on_ring`
(Task 4) and `render_circuit` (Task 5) follow the identical "local
helper method, not a core widget" shape, each self-contained to its own
task. `chat_log`/`prompt_index`/`thinking`/`preview_reveal` (Task 3),
`targets`/`target_selected`/`lock_on`/`complete_flash` (Task 4),
`load` (Task 5), and `booting` (Task 6) are each introduced once and
read/written consistently by every later task that touches them. The
`render_mode_content`/`update()` match arms are narrowed one variant at
a time across Tasks 3-5 (Brainstorm carved out first, then Fasttrack,
then Upgrade takes the last remaining arm) — verified each task's diff
leaves a compilable, exhaustive match, not a dangling catch-all.
