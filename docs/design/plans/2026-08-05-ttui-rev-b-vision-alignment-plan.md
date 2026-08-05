# TTUI Rev B (Vision Alignment) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Structure note:** This plan is organized as **Arcs → Slices → Tasks**
> per `docs/design/README.md`, not the flat "Task N" list the
> `writing-plans` skill defaults to. Tasks still follow the skill's
> bite-sized TDD step structure; Arc/Slice headings are pure grouping.

**Goal:** Implement the two decisions committed in Rev B — an opt-in
tick subscription on `App`, and a minimal semantic `Theme` — then
validate both together with a working Omnitrix-themed example, proving
the tick mechanism doesn't compromise Rev A's tactile-responsiveness
guarantee and producing real numbers on the `Terminal::draw_diff`
performance question the spec leaves open.

**Architecture:** `App` gains two new trait methods with default no-op
implementations (`tick_rate() -> Option<Duration>`, `on_tick(&mut self,
elapsed: Duration)`) so every existing `App` impl is unaffected. The
event loop's already-present-but-unused poll-timeout branch becomes the
tick trigger, reusing the exact same view→layout→paint→diff→flush
pipeline input events already use. A new `theme` module adds a
plain-data `Theme` + `BorderSet`, threaded into `Block` via a new
`.theme(&theme)` builder method — `Block`'s default (no `.theme()` call)
behavior is provably unchanged. Full detail:
`docs/design/specs/2026-08-05-ttui-rev-b-vision-alignment-design.md`
(Rev B); underlying architecture:
`docs/design/specs/2026-08-04-ttui-core-framework-design.md` (Rev A).

**Tech Stack:** Rust (stable, 2021 edition), `crossterm` (existing
dependency only — no new dependencies added by this plan).

## Global Constraints

- `App::tick_rate()` and `App::on_tick()` must have default
  implementations (`None` / no-op) — every existing `App` implementor,
  including `examples/demo.rs`, must compile and behave identically
  without being touched by this plan.
- The event loop must still do nothing on a timeout when `tick_rate()`
  is `None` — Rev A's "input-driven redraw, not tick-based" guarantee
  holds unchanged for every app that doesn't opt in.
- Widgets remain stateless `(data, area) -> paint` functions. `Theme` is
  passed in via the existing builder pattern (an extra `&Theme`
  reference), never read from a global/singleton, and animation state
  (e.g. `pulse_phase`) lives in app state, not in any widget.
- `Block`'s behavior with no `.theme()` call must remain byte-for-byte
  identical to today's hardcoded `'-'`/`'|'`/`'+'` glyphs and
  `Color::Reset` — the existing `draws_border_and_returns_inner_area`
  and `title_is_drawn_on_the_top_border` tests must keep passing
  unmodified.
- No buffer layering/compositing and no camera/viewport work — both are
  explicitly out of scope per the Rev B spec, deferred to a future plan.
- No new external dependencies; `crossterm = "0.27"` remains the only
  one.
- Every task's commit must pass this repo's required CI checks locally
  before committing: `cargo build`, `cargo test`, `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`.
- The Omnitrix example's performance-log file (`omnitrix_perf.log`) is a
  local diagnostic artifact only — it must be added to `.gitignore` and
  never committed.

---

## Arc 1: Opt-in Tick Subscription

### Slice 1.1: `App` trait tick hooks

**Tags:** coding

#### Task 1: Add `tick_rate`/`on_tick` default methods to `App`

**Files:**
- Modify: `src/app.rs`
- Test: `src/app.rs` (inline `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `std::time::Duration` (already imported in `src/app.rs`).
- Produces: `App::tick_rate(&self) -> Option<Duration>` (default `None`),
  `App::on_tick(&mut self, elapsed: Duration)` (default no-op). Both are
  provided-body trait methods — no existing `App` implementor needs to
  change.

- [ ] **Step 1: Create and check out the implementation branch**

```bash
git checkout -b core/ttui-rev-b-tick-theme-impl
```

- [ ] **Step 2: Write the failing test**

```rust
// src/app.rs, new module at the bottom of the file
#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::Buffer;
    use crate::layout::Rect;

    struct Dummy;

    impl App for Dummy {
        fn update(&mut self, _event: &Event) {}
        fn view(&self, _area: Rect, _buf: &mut Buffer) {}
        fn should_quit(&self) -> bool {
            false
        }
    }

    #[test]
    fn tick_rate_defaults_to_none() {
        let dummy = Dummy;
        assert_eq!(dummy.tick_rate(), None);
    }

    #[test]
    fn on_tick_default_is_a_no_op() {
        let mut dummy = Dummy;
        dummy.on_tick(Duration::from_millis(16));
        assert!(!dummy.should_quit());
    }
}
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test app::`
Expected: FAIL to compile — `tick_rate`/`on_tick` aren't methods on
`App` yet, so `dummy.tick_rate()` and `dummy.on_tick(...)` don't
resolve ("no method named `tick_rate` found for struct `Dummy`").

- [ ] **Step 4: Add the default methods to the `App` trait**

```rust
// src/app.rs
pub trait App {
    fn update(&mut self, event: &Event);
    fn view(&self, area: Rect, buf: &mut Buffer);
    fn should_quit(&self) -> bool;

    fn tick_rate(&self) -> Option<Duration> {
        None
    }

    fn on_tick(&mut self, _elapsed: Duration) {}
}
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test app::`
Expected: PASS (2 tests)

Run: `cargo fmt --check`
Expected: clean.

Run: `cargo clippy --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add src/app.rs
git commit -m "feat: add opt-in tick_rate/on_tick hooks to App trait"
```

### Slice 1.2: Event loop tick wiring

**Tags:** coding

#### Task 2: Wire the poll-timeout branch to trigger ticks

**Files:**
- Modify: `src/app.rs`

**Interfaces:**
- Consumes: `App::tick_rate`, `App::on_tick` (Task 1).
- Produces: `run<A: App>` (signature unchanged) now redraws after
  `on_tick` when `tick_rate()` is `Some`, and continues to do nothing on
  timeout when it's `None` — same as today.

This task has no automated test — real-terminal event-loop behavior is
outside the accepted testing gap (per
`docs/design/specs/2026-08-04-testing-verification-conventions-design.md`),
same as Rev A's `run` implementation. It's verified manually in Arc 3,
Task 6.

- [ ] **Step 1: Rewrite the loop body**

```rust
// src/app.rs, replace the `loop { ... }` block inside `run`
loop {
    let poll_timeout = app.tick_rate().unwrap_or(Duration::from_millis(250));
    let mut should_redraw = false;

    match term.next_event(poll_timeout)? {
        Some(event) => {
            app.update(&event);
            if app.should_quit() {
                break;
            }
            should_redraw = true;
        }
        None => {
            // Poll timed out with no input. If the app has opted into a
            // tick rate, this timeout IS the tick — call on_tick and
            // redraw. If it hasn't (tick_rate() is None), do nothing,
            // exactly like today: redraw only ever happens as a direct
            // consequence of an input event.
            if let Some(tick_rate) = app.tick_rate() {
                app.on_tick(tick_rate);
                should_redraw = true;
            }
        }
    }

    if should_redraw {
        let (w, h) = term.size()?;
        if (w, h) != (prev.width, prev.height) {
            prev = Buffer::new(w, h); // force full redraw on resize
        }
        let mut next = Buffer::new(w, h);
        app.view(
            Rect {
                x: 0,
                y: 0,
                width: w,
                height: h,
            },
            &mut next,
        );
        term.draw_diff(&diff(&prev, &next))?;
        prev = next;
    }
}
```

Note on `on_tick(tick_rate)`: the elapsed duration passed is the
*configured* tick interval, not a measured wall-clock delta — `poll`
returning `None` after `tick_rate` elapsed is already a close
approximation, and adding real elapsed-time instrumentation to core for
this is unnecessary precision for what `on_tick` callers need (smooth
animation progression), consistent with YAGNI.

- [ ] **Step 2: Verify it compiles and existing tests still pass**

Run: `cargo build`
Expected: builds successfully.

Run: `cargo test`
Expected: PASS, same test count as before this task (this task adds no
new tests — see the "no automated test" note above).

Run: `cargo fmt --check`
Expected: clean.

Run: `cargo clippy --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add src/app.rs
git commit -m "feat: trigger on_tick from the event loop's poll timeout"
```

---

## Arc 2: Minimal Semantic Theme

### Slice 2.1: `Theme` and `BorderSet` types

**Tags:** coding

#### Task 3: `Theme` and `BorderSet` structs

**Files:**
- Create: `src/theme.rs`
- Modify: `src/lib.rs`
- Test: `src/theme.rs` (inline `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `crossterm::style::Color` (external crate type).
- Produces: `BorderSet { horizontal: char, vertical: char, corner: char }`
  (derives `Clone, Copy, Debug, PartialEq`, `Default` glyphs `'-'`/`'|'`/
  `'+'`), `Theme { background: Color, primary: Color, secondary: Color,
  tertiary: Color, accent: Color, border: BorderSet }` (derives `Clone,
  Copy, Debug, PartialEq`, `Default` all-`Color::Reset` colors +
  `BorderSet::default()`).

- [ ] **Step 1: Write the failing test**

```rust
// src/theme.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_border_set_matches_todays_hardcoded_glyphs() {
        let b = BorderSet::default();
        assert_eq!(b.horizontal, '-');
        assert_eq!(b.vertical, '|');
        assert_eq!(b.corner, '+');
    }

    #[test]
    fn default_theme_uses_reset_colors_and_default_border() {
        let t = Theme::default();
        assert_eq!(t.background, Color::Reset);
        assert_eq!(t.primary, Color::Reset);
        assert_eq!(t.secondary, Color::Reset);
        assert_eq!(t.tertiary, Color::Reset);
        assert_eq!(t.accent, Color::Reset);
        assert_eq!(t.border, BorderSet::default());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test theme::`
Expected: FAIL to compile — `Theme`, `BorderSet` not defined yet.

- [ ] **Step 3: Write the implementation**

```rust
// src/theme.rs, above the tests module
use crossterm::style::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderSet {
    pub horizontal: char,
    pub vertical: char,
    pub corner: char,
}

impl Default for BorderSet {
    fn default() -> Self {
        BorderSet {
            horizontal: '-',
            vertical: '|',
            corner: '+',
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub background: Color,
    pub primary: Color,
    pub secondary: Color,
    pub tertiary: Color,
    pub accent: Color,
    pub border: BorderSet,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            background: Color::Reset,
            primary: Color::Reset,
            secondary: Color::Reset,
            tertiary: Color::Reset,
            accent: Color::Reset,
            border: BorderSet::default(),
        }
    }
}
```

- [ ] **Step 4: Add the module to `src/lib.rs`**

```rust
pub mod theme;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test theme::`
Expected: PASS (2 tests)

Run: `cargo fmt --check`
Expected: clean.

Run: `cargo clippy --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add src/theme.rs src/lib.rs
git commit -m "feat: add Theme and BorderSet types"
```

### Slice 2.2: Thread `Theme` through `Block`

**Tags:** coding

#### Task 4: `Block::theme()` builder method

**Files:**
- Modify: `src/widgets/block.rs`
- Test: `src/widgets/block.rs` (inline `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `Theme`, `BorderSet` (Task 3), existing `Block`/`Buffer`/
  `Cell`/`Rect`.
- Produces: `Block::theme(self, theme: &'a Theme) -> Block<'a>` (new
  builder method). `Block::render`'s signature is unchanged
  (`fn render(&self, area: Rect, buf: &mut Buffer) -> Rect`); its
  no-theme behavior is unchanged.

- [ ] **Step 1: Write the failing test**

```rust
// src/widgets/block.rs, add to the existing tests module
use crate::theme::{BorderSet, Theme};

#[test]
fn without_theme_border_colors_are_reset() {
    let mut buf = Buffer::new(4, 3);
    let area = Rect {
        x: 0,
        y: 0,
        width: 4,
        height: 3,
    };

    Block::new().render(area, &mut buf);

    assert_eq!(buf.get(0, 0).symbol, '+');
    assert_eq!(buf.get(0, 0).fg, Color::Reset);
    assert_eq!(buf.get(0, 0).bg, Color::Reset);
}

#[test]
fn with_theme_border_uses_theme_glyphs_and_colors() {
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
    };
    let mut buf = Buffer::new(4, 3);
    let area = Rect {
        x: 0,
        y: 0,
        width: 4,
        height: 3,
    };

    Block::new().theme(&theme).render(area, &mut buf);

    assert_eq!(buf.get(0, 0).symbol, '*'); // corner
    assert_eq!(buf.get(1, 0).symbol, '='); // horizontal
    assert_eq!(buf.get(0, 1).symbol, '#'); // vertical
    assert_eq!(buf.get(0, 0).fg, Color::Green);
    assert_eq!(buf.get(0, 0).bg, Color::Black);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test widgets::block::`
Expected: FAIL to compile — `Block::theme` doesn't exist yet.

- [ ] **Step 3: Update the implementation**

```rust
// src/widgets/block.rs, replace the struct/impl above the tests module
use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;
use crate::theme::{BorderSet, Theme};
use crossterm::style::Color;

pub struct Block<'a> {
    title: Option<&'a str>,
    theme: Option<&'a Theme>,
}

impl<'a> Block<'a> {
    pub fn new() -> Self {
        Block {
            title: None,
            theme: None,
        }
    }

    pub fn title(mut self, t: &'a str) -> Self {
        self.title = Some(t);
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) -> Rect {
        if area.width < 2 || area.height < 2 {
            return area;
        }
        let (border, fg, bg) = match self.theme {
            Some(t) => (t.border, t.primary, t.background),
            None => (BorderSet::default(), Color::Reset, Color::Reset),
        };
        let plain = || Cell {
            symbol: ' ',
            fg,
            bg,
        };
        for x in area.x..area.x + area.width {
            buf.set(
                x,
                area.y,
                Cell {
                    symbol: border.horizontal,
                    ..plain()
                },
            );
            buf.set(
                x,
                area.y + area.height - 1,
                Cell {
                    symbol: border.horizontal,
                    ..plain()
                },
            );
        }
        for y in area.y..area.y + area.height {
            buf.set(
                area.x,
                y,
                Cell {
                    symbol: border.vertical,
                    ..plain()
                },
            );
            buf.set(
                area.x + area.width - 1,
                y,
                Cell {
                    symbol: border.vertical,
                    ..plain()
                },
            );
        }
        buf.set(
            area.x,
            area.y,
            Cell {
                symbol: border.corner,
                ..plain()
            },
        );
        buf.set(
            area.x + area.width - 1,
            area.y,
            Cell {
                symbol: border.corner,
                ..plain()
            },
        );
        buf.set(
            area.x,
            area.y + area.height - 1,
            Cell {
                symbol: border.corner,
                ..plain()
            },
        );
        buf.set(
            area.x + area.width - 1,
            area.y + area.height - 1,
            Cell {
                symbol: border.corner,
                ..plain()
            },
        );

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
}

impl<'a> Default for Block<'a> {
    fn default() -> Self {
        Self::new()
    }
}
```

The two pre-existing tests (`draws_border_and_returns_inner_area`,
`title_is_drawn_on_the_top_border`) are untouched by this change — they
call `Block::new()` with no `.theme()`, so they still exercise the
`None` branch and must keep passing unmodified, per the Global
Constraints above.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test widgets::block::`
Expected: PASS (4 tests: the 2 pre-existing plus the 2 new ones)

Run: `cargo fmt --check`
Expected: clean.

Run: `cargo clippy --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add src/widgets/block.rs
git commit -m "feat: add Block::theme() to customize border glyphs and colors"
```

---

## Arc 3: Omnitrix Validation Prototype

### Slice 3.1: Omnitrix example

**Tags:** coding

#### Task 5: `examples/omnitrix.rs` — tick-driven pulsing themed border

**Files:**
- Create: `examples/omnitrix.rs`
- Modify: `.gitignore` (append `omnitrix_perf.log`)

**Interfaces:**
- Consumes: `App`, `run` (Arc 1), `Theme`, `BorderSet` (Task 3),
  `Block::theme` (Task 4), `Text`, `Buffer`, `Rect`.
- Produces: nothing consumed by later tasks — this is Rev B's
  validation vehicle, the example/demo TDD exception applies (per
  `.claude/rules/development-conventions.md`: correctness is checked by
  running the example, not asserting on it).

- [ ] **Step 1: Append the perf-log entry to `.gitignore`**

```
# Rev B validation artifacts
omnitrix_perf.log
```

- [ ] **Step 2: Write the example**

```rust
// examples/omnitrix.rs
use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::style::Color;
use std::fs::OpenOptions;
use std::io::Write;
use std::time::{Duration, Instant};
use ttui::app::{run, App};
use ttui::buffer::Buffer;
use ttui::layout::Rect;
use ttui::theme::{BorderSet, Theme};
use ttui::widgets::{block::Block, text::Text};

const TICK_INTERVAL: Duration = Duration::from_millis(33); // ~30 FPS

struct Omnitrix {
    pulse_phase: f32,
    quit: bool,
    last_tick_started: Instant,
    perf_log: std::fs::File,
}

impl Omnitrix {
    fn new() -> Self {
        let perf_log = OpenOptions::new()
            .create(true)
            .append(true)
            .open("omnitrix_perf.log")
            .expect("failed to open omnitrix_perf.log");
        Omnitrix {
            pulse_phase: 0.0,
            quit: false,
            last_tick_started: Instant::now(),
            perf_log,
        }
    }

    fn theme(&self) -> Theme {
        // Breathing pulse: sine wave brightness between a dim and a
        // bright green, matching the Omnitrix vision doc's "Recharge
        // Pulse" description.
        let brightness = (self.pulse_phase.sin() + 1.0) / 2.0;
        let primary = if brightness > 0.5 {
            Color::Rgb { r: 0, g: 255, b: 65 }
        } else {
            Color::Rgb { r: 0, g: 120, b: 32 }
        };
        Theme {
            background: Color::Black,
            primary,
            secondary: Color::DarkGreen,
            tertiary: Color::Red,
            accent: Color::White,
            border: BorderSet {
                horizontal: '=',
                vertical: '#',
                corner: '+',
            },
        }
    }
}

impl App for Omnitrix {
    fn update(&mut self, event: &Event) {
        let Event::Key(k) = event else { return };
        if k.kind != KeyEventKind::Press {
            return;
        }
        if k.code == KeyCode::Char('q') {
            self.quit = true;
        }
    }

    fn view(&self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme();
        let inner = Block::new()
            .title("Omnitrix")
            .theme(&theme)
            .render(area, buf);
        Text::new("The Omnitrix breathes... press q to quit").render(inner, buf);
    }

    fn should_quit(&self) -> bool {
        self.quit
    }

    fn tick_rate(&self) -> Option<Duration> {
        Some(TICK_INTERVAL)
    }

    fn on_tick(&mut self, elapsed: Duration) {
        // Measures wall-clock time since the previous tick STARTED,
        // which includes this loop iteration's poll wait plus the
        // PREVIOUS iteration's full render+flush. If Terminal::draw_diff's
        // per-cell execute! pattern (the Rev B spec's open performance
        // risk) is expensive, this value will consistently exceed
        // TICK_INTERVAL by more than the previous frame's render cost
        // should account for. This is a deliberately simple, core-code-free
        // way to get real numbers for a prototype, not a permanent
        // profiling mechanism.
        let now = Instant::now();
        let since_last_tick = now.duration_since(self.last_tick_started);
        self.last_tick_started = now;
        let _ = writeln!(
            self.perf_log,
            "nominal_tick={elapsed:?} actual_time_since_last_tick_start={since_last_tick:?}"
        );

        self.pulse_phase += elapsed.as_secs_f32() * std::f32::consts::PI;
    }
}

fn main() -> std::io::Result<()> {
    let mut app = Omnitrix::new();
    run(&mut app)
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build --examples`
Expected: builds successfully.

Run: `cargo fmt --check`
Expected: clean.

Run: `cargo clippy --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add examples/omnitrix.rs .gitignore
git commit -m "feat: add Omnitrix validation example (tick-driven pulse + theme)"
```

### Slice 3.2: Manual verification and performance measurement

**Tags:** coding, admin

#### Task 6: Run the example, verify responsiveness, and record perf numbers

**Files:**
- None (manual verification task, no code changes).

**Interfaces:**
- Consumes: the whole crate plus `examples/omnitrix.rs` (Task 5).
- Produces: nothing — this is the plan's acceptance check and the Rev B
  spec's required validation step.

- [ ] **Step 1: Run the example in a real terminal**

Run: `cargo run --example omnitrix`

- [ ] **Step 2: Check the pulse animates and input stays responsive**

  - [ ] The border color visibly pulses (dim green ↔ bright green)
    continuously, with no key presses, for at least 10 seconds.
  - [ ] Pressing `q` quits promptly — no perceptible lag between the
    keypress and the app exiting (this is Rev A's tactile-responsiveness
    commitment; it must hold for a ticking app too, not just static
    ones).
  - [ ] After quitting, the terminal is restored to normal (non-raw,
    non-alternate-screen) state — the shell prompt is immediately
    usable, matching the same check as the Rev A plan's Task 17.

- [ ] **Step 3: Inspect the performance log**

Run (after quitting the example): open `omnitrix_perf.log` in the crate
root and compute, across the logged lines, the average and maximum
`actual_time_since_last_tick_start`.

  - [ ] If the average stays close to the nominal 33ms (roughly within
    10-20%, i.e. under ~40ms) and the maximum doesn't spike far above
    that, `Terminal::draw_diff`'s per-cell `execute!` pattern is cheap
    enough at this scale (a themed border + one text line, ~2×(w+h)
    cells changing per tick) — the Rev B spec's open performance risk is
    resolved for this scope, and buffer layering / heavier tick-driven
    apps remain a fair question for a future spec, not a blocked one.
  - [ ] If the average or maximum is substantially higher (e.g.
    consistently 2x+ the nominal interval), that's a real finding: record
    it as a known limitation in a follow-up note (or a new "Explicitly
    deferred" entry in a future spec revision) rather than silently
    proceeding — do not mark this task done with an unrecorded
    regression.

- [ ] **Step 4: Clean up the log file**

`omnitrix_perf.log` is already gitignored (Task 5, Step 1); delete the
local file so a stale log doesn't confuse a future run:

```bash
rm -f omnitrix_perf.log
```

- [ ] **Step 5: Push the branch and open a PR**

Only after Steps 2-3 record a pass (or an explicitly recorded, accepted
finding per Step 3's second bullet):

```bash
git push -u origin core/ttui-rev-b-tick-theme-impl
gh pr create --title "feat: TTUI Rev B — opt-in tick subscription + minimal Theme" --body "$(cat <<'EOF'
## Summary
- Design: docs/design/specs/2026-08-05-ttui-rev-b-vision-alignment-design.md
- Plan: docs/design/plans/2026-08-05-ttui-rev-b-vision-alignment-plan.md

Implements Rev B's two committed decisions: an opt-in tick subscription
on `App` (`tick_rate`/`on_tick`, default no-op — existing apps
unaffected) and a minimal `Theme`/`BorderSet` threaded through `Block`
via a new `.theme()` builder method. Validated end-to-end with a new
Omnitrix-themed example exercising both: a continuously pulsing themed
border driven by the tick loop.

## Verification
- `cargo build`, `cargo test`, `cargo fmt --check`, and `cargo clippy
  --all-targets -- -D warnings` pass clean at every task (see commit
  history).
- Manual verification (Task 6): pulse animates smoothly, `q` quits
  responsively, terminal restores cleanly, and
  actual_time_since_last_tick_start stayed close to the nominal 33ms tick
  — see PR discussion for the recorded numbers.
EOF
)"
```

Expected: PR opens against `main` and the `build`/`test`/`clippy`/`fmt`
required status checks (`.github/workflows/ci.yml`) run and pass.
