# Control Panel (Mouse Support) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add real mouse support to TTUI (`Terminal` capture, `Rect::contains` hit-testing, `tools/visual-snapshot` click-scripting) and prove it with a new example where clicking is the primary interaction.

**Architecture:** `Rect::contains` is a plain bounds check apps use for hit-testing in their own `update()` — no new abstraction layer, matching how Falcon's WHACK handler already does ad hoc hit-testing today. `Terminal` gains mouse capture alongside its existing raw-mode/alt-screen setup. `tools/visual-snapshot` gains a `Click` script step so the mandatory visual-review convention can actually verify click-driven behavior.

**Tech Stack:** Rust, `crossterm::event` (`MouseEvent`, `MouseEventKind`, `MouseButton`, `EnableMouseCapture`/`DisableMouseCapture`).

## Global Constraints

- **`Rect::contains` and `tools/visual-snapshot`'s click-scripting support are `coding`-tagged with full TDD mandatory** — no exemption.
- **`Terminal`'s mouse-capture enable/disable is real-TTY exempt** per the existing "Real-TTY tests" exception in `.claude/rules/development-conventions.md` — bundled into the same `execute!` calls already covered by that exemption; `crossterm` exposes no queryable "is mouse capture enabled" state to assert against.
- **`examples/control_panel.rs` is TDD-exempt** (example code), but **`tools/visual-snapshot` capture is mandatory**, including at least one scripted `Click` step.
- **Click regions are hit-tested against each element's outer `Block` rect** (border included), not just inner content.
- **No drag, hover-highlight, or scroll-wheel handling** — click (press+release at the same position) only. **No keyboard equivalents** for the toggle/button/dial interactions — `q`-to-quit is the only keyboard handling in the example.

---

### Task 1: `Rect::contains`

**Files:**
- Modify: `src/layout.rs`

**Interfaces:**
- Produces: `impl Rect { pub fn contains(&self, x: u16, y: u16) -> bool }` — consumed by Task 4.

- [ ] **Step 1: Write the failing tests**

Add to `src/layout.rs`'s existing `#[cfg(test)] mod tests` block (or create one if none exists — check the file first):

```rust
    fn hit_test_rect() -> Rect {
        Rect {
            x: 5,
            y: 5,
            width: 10,
            height: 10,
        }
    }

    #[test]
    fn a_point_strictly_inside_is_contained() {
        assert!(hit_test_rect().contains(10, 10));
    }

    #[test]
    fn a_point_on_the_left_or_top_edge_is_contained() {
        assert!(hit_test_rect().contains(5, 10));
        assert!(hit_test_rect().contains(10, 5));
    }

    #[test]
    fn a_point_on_the_right_or_bottom_edge_is_not_contained() {
        assert!(!hit_test_rect().contains(15, 10));
        assert!(!hit_test_rect().contains(10, 15));
    }

    #[test]
    fn a_point_fully_outside_each_direction_is_not_contained() {
        assert!(!hit_test_rect().contains(0, 10));
        assert!(!hit_test_rect().contains(20, 10));
        assert!(!hit_test_rect().contains(10, 0));
        assert!(!hit_test_rect().contains(10, 20));
    }

    #[test]
    fn a_zero_width_or_zero_height_rect_contains_nothing() {
        let zero_w = Rect {
            x: 5,
            y: 5,
            width: 0,
            height: 10,
        };
        assert!(!zero_w.contains(5, 10));
        let zero_h = Rect {
            x: 5,
            y: 5,
            width: 10,
            height: 0,
        };
        assert!(!zero_h.contains(10, 5));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib layout::`
Expected: FAIL to compile — `contains` doesn't exist on `Rect` yet.

- [ ] **Step 3: Write the implementation**

Add to the `impl Rect` block in `src/layout.rs` (create the block if `Rect` doesn't already have one — check the file first):

```rust
    /// Whether `(x, y)` falls within this rect — inclusive of the
    /// left/top edge, exclusive of the right/bottom edge (matches how
    /// `width`/`height` are already used everywhere else in this crate).
    pub fn contains(&self, x: u16, y: u16) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib layout::`
Expected: all 5 new tests PASS.

- [ ] **Step 5: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings` — clean.
Run: `cargo fmt --check` — clean.

- [ ] **Step 6: Commit**

```bash
git add src/layout.rs
git commit -m "feat(core): add Rect::contains for mouse hit-testing"
```

---

### Task 2: Mouse capture in `Terminal`

**Files:**
- Modify: `src/terminal.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces: nothing new consumed by later tasks (Task 4's example relies on `Event::Mouse` being produced at all, which this task enables at the terminal level).

- [ ] **Step 1: Add `EnableMouseCapture` to `Terminal::new()`**

In `src/terminal.rs`, `Terminal::new()` currently has:

```rust
        execute!(out, terminal::EnterAlternateScreen, cursor::Hide)?;
```

Change it to:

```rust
        execute!(
            out,
            terminal::EnterAlternateScreen,
            cursor::Hide,
            event::EnableMouseCapture
        )?;
```

`crossterm::event` is already imported as `event` in this file (`use crossterm::event::{self, Event};`), so `event::EnableMouseCapture` resolves without a new import.

- [ ] **Step 2: Add `DisableMouseCapture` to `Drop for Terminal`**

`impl Drop for Terminal` currently has:

```rust
        let _ = execute!(self.out, terminal::LeaveAlternateScreen, cursor::Show);
```

Change it to:

```rust
        let _ = execute!(
            self.out,
            event::DisableMouseCapture,
            terminal::LeaveAlternateScreen,
            cursor::Show
        );
```

- [ ] **Step 3: Add `DisableMouseCapture` to `install_panic_hook`**

`install_panic_hook`'s closure currently has:

```rust
        let _ = execute!(stdout(), terminal::LeaveAlternateScreen, cursor::Show);
```

Change it to:

```rust
        let _ = execute!(
            stdout(),
            event::DisableMouseCapture,
            terminal::LeaveAlternateScreen,
            cursor::Show
        );
```

- [ ] **Step 4: Build, lint, format**

Run: `cargo build --all-targets` — succeeds.
Run: `cargo clippy --all-targets -- -D warnings` — clean.
Run: `cargo fmt --check` — clean.

- [ ] **Step 5: Confirm the existing real-TTY tests are unaffected**

Run: `cargo test --lib terminal:: -- --ignored`
Expected: this requires a real TTY and won't succeed in this sandboxed environment (same as before this task) — the point of this step is only to confirm the two existing ignored tests still *compile* and are unaffected by this change, not to make them pass here. Note in your report that you ran this and what it reported (even if "requires a real terminal" is the outcome, as expected).

- [ ] **Step 6: Commit**

```bash
git add src/terminal.rs
git commit -m "feat(core): enable mouse capture in Terminal

Real-TTY exempt per development-conventions.md's existing exception —
bundled into the same execute! calls already covered by it. No new
automated test: crossterm exposes no queryable mouse-capture state to
assert against, unlike is_raw_mode_enabled()."
```

---

### Task 3: `tools/visual-snapshot` click-scripting support

**Files:**
- Modify: `tools/visual-snapshot/src/script.rs`
- Modify: `tools/visual-snapshot/src/keys.rs`
- Modify: `tools/visual-snapshot/src/pty.rs`
- Modify: `tools/visual-snapshot/Cargo.toml`
- Create: `tools/visual-snapshot/examples/echo_mouse.rs`
- Modify: `tools/visual-snapshot/README.md`

**Interfaces:**
- Produces: `Step::Click { x: u16, y: u16 }` (added to the existing `Step` enum), `pub fn encode_click(x: u16, y: u16) -> Vec<u8>` (in `keys.rs`) — both consumed by Task 4 (indirectly, via the CLI) and by this task's own integration test.

- [ ] **Step 1: Write the failing tests — `Step::Click` parsing**

Add to `tools/visual-snapshot/src/script.rs`'s existing `#[cfg(test)] mod tests` block:

```rust
    #[test]
    fn parses_a_click_step() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("script.json");
        std::fs::write(&path, r#"[{"x":10,"y":5}]"#).unwrap();

        let steps = parse_script(&path).unwrap();

        assert_eq!(steps, vec![Step::Click { x: 10, y: 5 }]);
    }

    #[test]
    fn parses_a_mix_of_wait_key_and_click_steps() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("script.json");
        std::fs::write(
            &path,
            r#"[{"wait_ms":16},{"key":"Enter"},{"x":10,"y":5}]"#,
        )
        .unwrap();

        let steps = parse_script(&path).unwrap();

        assert_eq!(
            steps,
            vec![
                Step::Wait { wait_ms: 16 },
                Step::Key {
                    key: "Enter".to_string()
                },
                Step::Click { x: 10, y: 5 },
            ]
        );
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --package visual-snapshot --lib script::`
Expected: FAIL to compile — `Step::Click` doesn't exist yet.

- [ ] **Step 3: Add the `Click` variant to `Step`**

In `tools/visual-snapshot/src/script.rs`, add to the `Step` enum (alongside `Wait`/`Key`):

```rust
    /// Send a left-button click at the given cell coordinates to the
    /// spawned example.
    Click {
        /// Column (0-indexed) to click.
        x: u16,
        /// Row (0-indexed) to click.
        y: u16,
    },
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --package visual-snapshot --lib script::`
Expected: both new tests PASS, plus the 4 existing `script::` tests still pass.

- [ ] **Step 5: Write the failing tests — `encode_click`**

Add to `tools/visual-snapshot/src/keys.rs`'s existing `#[cfg(test)] mod tests` block:

```rust
    #[test]
    fn encode_click_produces_the_expected_sgr_press_and_release_sequence() {
        let bytes = encode_click(3, 7);
        // 1-indexed SGR coords: x=3->4, y=7->8. Button 0 = left.
        assert_eq!(bytes, b"\x1b[<0;4;8M\x1b[<0;4;8m".to_vec());
    }

    #[test]
    fn encode_click_at_the_origin_is_1_indexed_correctly() {
        let bytes = encode_click(0, 0);
        assert_eq!(bytes, b"\x1b[<0;1;1M\x1b[<0;1;1m".to_vec());
    }
```

- [ ] **Step 6: Run tests to verify they fail**

Run: `cargo test --package visual-snapshot --lib keys::`
Expected: FAIL to compile — `encode_click` doesn't exist yet.

- [ ] **Step 7: Write `encode_click`**

Add to `tools/visual-snapshot/src/keys.rs`, alongside `encode_key`:

```rust
/// Encodes a left-button click at cell `(x, y)` (0-indexed) into the
/// raw SGR mouse-protocol byte sequence a real terminal would send: a
/// press immediately followed by a release, both at the same
/// position. SGR coordinates are 1-indexed.
pub fn encode_click(x: u16, y: u16) -> Vec<u8> {
    let press = format!("\x1b[<0;{};{}M", x + 1, y + 1);
    let release = format!("\x1b[<0;{};{}m", x + 1, y + 1);
    let mut bytes = press.into_bytes();
    bytes.extend(release.into_bytes());
    bytes
}
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `cargo test --package visual-snapshot --lib keys::`
Expected: both new tests PASS, plus existing `keys::` tests still pass.

- [ ] **Step 9: Wire `Click` into `run_script`**

In `tools/visual-snapshot/src/pty.rs`, `run_script`'s `match step` currently has arms for `Step::Wait` and `Step::Key`. Add a third arm:

```rust
            Step::Click { x, y } => {
                session.send(&keys::encode_click(*x, *y))?;
                // Same "wait for the child's actual reaction" quiescence
                // strategy Key steps already use — a click should also
                // produce an observable reaction.
                frames.push((
                    session.capture_frame_after_key()?,
                    KEY_STEP_DISPLAY_DURATION,
                ));
            }
```

- [ ] **Step 10: Add the `echo_mouse` fixture**

Create `tools/visual-snapshot/examples/echo_mouse.rs`:

```rust
//! Minimal fixture binary for click-scripting integration tests:
//! echoes the debug representation of each mouse event it receives,
//! exits on Esc key.
use crossterm::event::{self, Event};
use crossterm::terminal;
use std::io::Write;

fn main() -> std::io::Result<()> {
    terminal::enable_raw_mode()?;
    crossterm::execute!(std::io::stdout(), event::EnableMouseCapture)?;
    let mut out = std::io::stdout();
    loop {
        match event::read()? {
            Event::Mouse(m) => {
                write!(out, "{:?}", m.kind)?;
                out.flush()?;
            }
            Event::Key(key) if key.code == event::KeyCode::Esc => break,
            _ => {}
        }
    }
    crossterm::execute!(std::io::stdout(), event::DisableMouseCapture)?;
    terminal::disable_raw_mode()?;
    Ok(())
}
```

Add a matching `[[example]]` entry to `tools/visual-snapshot/Cargo.toml`, next to the existing `echo_key` entry:

```toml
[[example]]
name = "echo_mouse"
```

- [ ] **Step 11: Write the failing integration test**

Create a new test in `tools/visual-snapshot/tests/pty_roundtrip.rs` (add alongside the existing tests — this exercises the same `run_script` path they do, so it belongs in the same file):

```rust
fn echo_mouse_binary() -> PathBuf {
    let mut path = examples_dir();
    path.push("echo_mouse");
    if cfg!(windows) {
        path.set_extension("exe");
    }
    path
}

#[test]
fn a_click_step_actually_reaches_the_child_process() {
    let steps = vec![
        Step::Click { x: 3, y: 2 },
        Step::Wait { wait_ms: 16 },
        Step::Key {
            key: "Esc".to_string(),
        },
    ];

    let frames = run_script(&echo_mouse_binary(), 5, 40, &steps).unwrap();

    // Initial frame + one per step.
    assert_eq!(frames.len(), 4);
    let after_click = &frames[1].0;
    let any_non_background = after_click
        .pixels()
        .any(|p| *p != image::Rgba([0, 0, 0, 255]));
    assert!(
        any_non_background,
        "expected the echoed mouse event text to draw something"
    );
}
```

- [ ] **Step 12: Run tests to verify they fail, then pass**

Run: `cargo test --package visual-snapshot --test pty_roundtrip`
Expected: FAIL first (binary doesn't exist / isn't built yet — `cargo test` builds examples referenced by tests automatically, so this should resolve once Step 10's fixture and Cargo.toml entry are in place; if it fails for a different reason, investigate before proceeding). Then PASS: the new test plus all existing `pty_roundtrip.rs` tests green.

- [ ] **Step 13: Update the README**

In `tools/visual-snapshot/README.md`, find the section documenting the script format (the `{"wait_ms": N}` / `{"key": "Name"}` step shapes). Add a third shape: `{"x": N, "y": N}` — sends a left-button click at cell `(x, y)` (0-indexed).

- [ ] **Step 14: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings` — clean.
Run: `cargo fmt --check` — clean.

- [ ] **Step 15: Full workspace test**

Run: `cargo test`
Expected: full workspace suite green — includes Task 1's `layout::` tests, this task's `script::`/`keys::`/`pty_roundtrip` tests, and everything else unchanged.

- [ ] **Step 16: Commit**

```bash
git add tools/visual-snapshot/src/script.rs tools/visual-snapshot/src/keys.rs \
        tools/visual-snapshot/src/pty.rs tools/visual-snapshot/Cargo.toml \
        tools/visual-snapshot/examples/echo_mouse.rs tools/visual-snapshot/tests/pty_roundtrip.rs \
        tools/visual-snapshot/README.md
git commit -m "feat(visual-snapshot): add Click step for scripting mouse events

Without this, the mandatory visual-review convention has no way to
verify click-driven behavior at all — needed before the Control Panel
example (mouse-click-primary) can be properly reviewed."
```

---

### Task 4: Control Panel example

**Files:**
- Create: `examples/control_panel.rs`
- Modify: `examples/README.md`

**Interfaces:**
- Consumes: `Rect::contains` (Task 1), `Terminal`'s mouse capture (Task 2, indirectly via `ttui::app::run`), `tools/visual-snapshot`'s `Click` step (Task 3, for verification). Existing: `ttui::widgets::{analog_toggle::AnalogToggle, block::Block, dial::Dial, text::Text}`, `ttui::particles::{Particle, ParticleSystem}`, `ttui::theme::{BorderSet, Theme}`.
- Produces: nothing consumed by later tasks (Task 5 is verification-only).

- [ ] **Step 1: Write the example**

Create `examples/control_panel.rs`:

```rust
use crossterm::event::{Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use crossterm::style::Color;
use std::time::Duration;
use ttui::app::{run, App};
use ttui::buffer::LayerStack;
use ttui::layout::{Constraint, Direction, Layout, Rect};
use ttui::particles::{Particle, ParticleSystem};
use ttui::theme::{BorderSet, Theme};
use ttui::widgets::{analog_toggle::AnalogToggle, block::Block, dial::Dial, text::Text};

const LAUNCH_SPARK_COUNT: usize = 8;
const LAUNCH_SPARK_LIFETIME_MS: u64 = 400;
const TOGGLE_LABELS: [&str; 3] = ["POWER", "SHIELDS", "COMMS"];

fn control_panel_theme() -> Theme {
    Theme {
        background: Color::Rgb {
            r: 10,
            g: 10,
            b: 10,
        },
        primary: Color::Rgb {
            r: 0,
            g: 255,
            b: 120,
        },
        secondary: Color::Rgb {
            r: 200,
            g: 200,
            b: 200,
        },
        tertiary: Color::Rgb { r: 255, g: 40, b: 40 },
        accent: Color::Rgb {
            r: 255,
            g: 200,
            b: 0,
        },
        primary_end: None,
        border: BorderSet::default(),
        border_bold: false,
        border_thick: false,
    }
}

struct ControlPanel {
    theme: Theme,
    toggles: [bool; 3],
    dial_items: Vec<String>,
    dial_selected: usize,
    particles: ParticleSystem,
    button_area: std::cell::Cell<Rect>,
    toggle_areas: std::cell::Cell<[Rect; 3]>,
    dial_area: std::cell::Cell<Rect>,
    quit: bool,
}

impl ControlPanel {
    fn new() -> Self {
        let zero_rect = Rect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        };
        ControlPanel {
            theme: control_panel_theme(),
            toggles: [false, false, false],
            dial_items: vec![
                "STANDBY".into(),
                "PATROL".into(),
                "COMBAT".into(),
                "STEALTH".into(),
            ],
            dial_selected: 0,
            particles: ParticleSystem::new(),
            button_area: std::cell::Cell::new(zero_rect),
            toggle_areas: std::cell::Cell::new([zero_rect; 3]),
            dial_area: std::cell::Cell::new(zero_rect),
            quit: false,
        }
    }

    fn spawn_launch_burst(&mut self, cx: f32, cy: f32) {
        for i in 0..LAUNCH_SPARK_COUNT {
            let angle = i as f32 * std::f32::consts::TAU / LAUNCH_SPARK_COUNT as f32;
            self.particles.spawn(Particle {
                x: cx,
                y: cy,
                vx: angle.cos() * 8.0,
                vy: angle.sin() * 4.0,
                symbol: '*',
                color: self.theme.accent,
                lifetime: Duration::from_millis(LAUNCH_SPARK_LIFETIME_MS),
                age: Duration::ZERO,
            });
        }
    }
}

impl App for ControlPanel {
    fn update(&mut self, event: &Event) {
        match event {
            Event::Key(k) if k.kind == KeyEventKind::Press && k.code == KeyCode::Char('q') => {
                self.quit = true;
            }
            Event::Mouse(m) if m.kind == MouseEventKind::Down(MouseButton::Left) => {
                let button = self.button_area.get();
                if button.contains(m.column, m.row) {
                    let cx = button.x as f32 + button.width as f32 / 2.0;
                    let cy = button.y as f32 + button.height as f32 / 2.0;
                    self.spawn_launch_burst(cx, cy);
                    return;
                }
                for (i, area) in self.toggle_areas.get().iter().enumerate() {
                    if area.contains(m.column, m.row) {
                        self.toggles[i] = !self.toggles[i];
                        return;
                    }
                }
                let dial = self.dial_area.get();
                if dial.contains(m.column, m.row) {
                    self.dial_selected = (self.dial_selected + 1) % self.dial_items.len();
                }
            }
            _ => {}
        }
    }

    fn view(&self, area: Rect, buf: &mut LayerStack) {
        let rows = Layout::new(
            Direction::Vertical,
            vec![
                Constraint::Percentage(40),
                Constraint::Percentage(30),
                Constraint::Fill(1),
            ],
        )
        .split(area);

        let button_inner = Block::new()
            .title("LAUNCH")
            .theme(&self.theme)
            .render(rows[0], buf);
        self.button_area.set(rows[0]);
        Text::new("Click to launch").render(button_inner, buf);

        let toggle_cols =
            Layout::new(Direction::Horizontal, vec![Constraint::Fill(1); 3]).split(rows[1]);
        let mut toggle_areas = [Rect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }; 3];
        for (i, col) in toggle_cols.iter().enumerate() {
            let inner = Block::new()
                .title(TOGGLE_LABELS[i])
                .theme(&self.theme)
                .render(*col, buf);
            toggle_areas[i] = *col;
            AnalogToggle::new(self.toggles[i]).render(inner, buf);
        }
        self.toggle_areas.set(toggle_areas);

        let dial_inner = Block::new()
            .title("MODE")
            .theme(&self.theme)
            .render(rows[2], buf);
        self.dial_area.set(rows[2]);
        Dial::new(&self.dial_items, self.dial_selected).render(dial_inner, buf);

        let overlay = buf.push_layer();
        self.particles.render(overlay);
    }

    fn should_quit(&self) -> bool {
        self.quit
    }

    fn tick_rate(&self) -> Option<Duration> {
        Some(Duration::from_millis(33))
    }

    fn on_tick(&mut self, elapsed: Duration) {
        self.particles.update(elapsed);
    }
}

fn main() -> std::io::Result<()> {
    let mut app = ControlPanel::new();
    run(&mut app)
}
```

- [ ] **Step 2: Add the `examples/README.md` entry**

Add an entry for `control_panel`, matching the existing entries' style (backticked name, one-sentence description, spec reference):

```markdown
- **`control_panel`** — a retro sci-fi console where clicking is the
  primary interaction: a LAUNCH button (spawns a particle burst),
  three clickable toggle switches, and a clickable mode dial. Built
  from `docs/design/specs/control-panel/2026-08-12-control-panel-design.md`.
```

- [ ] **Step 3: Build, lint, format**

Run: `cargo build --example control_panel` — succeeds, no warnings.
Run: `cargo clippy --example control_panel -- -D warnings` — clean.
Run: `cargo fmt --check` — clean.

- [ ] **Step 4: Capture and verify visually — initial layout**

Using `tools/visual-snapshot`, capture a single post-startup frame at a size like `100x30` (no script steps needed — this app has no boot sequence, it renders immediately):
```
cargo run -p visual-snapshot -- --example control_panel --size 100x30 --script <empty-array-script.json> --out <path>.png
```
`Read` it. Confirm: a LAUNCH button panel (top ~40%), three labeled toggle switches each showing `[ \ ]` (off) (middle ~30%), and a MODE dial showing 4 items with "STANDBY" highlighted (bottom, remainder).

- [ ] **Step 5: Determine click coordinates empirically**

From the frame captured in Step 4, determine the approximate on-screen `(x, y)` cell coordinates that fall inside the LAUNCH button's panel, inside the first toggle's panel, and inside the dial's panel — e.g. by inspecting the captured image's pixel dimensions (each cell is a fixed pixel size in the rasterized output — check `tools/visual-snapshot/src/render.rs` or the README for the exact glyph cell size) and converting to cell coordinates, or by reasoning from the known layout split (`Percentage(40)`/`Percentage(30)`/`Fill(1)` vertical, `Fill(1)`x3 horizontal for the toggle row) against the `100x30` capture size.

- [ ] **Step 6: Capture and verify visually — LAUNCH button click**

Script a `Click` at the coordinates determined in Step 5 for the LAUNCH button, e.g.:
```json
[{"x": 50, "y": 6}, {"wait_ms": 50}]
```
(adjust `x`/`y` to your Step 5 determination). `Read` the resulting frames. Confirm a particle burst (small `*` glyphs in `theme.accent` gold, fanned outward) is visible near the click location.

- [ ] **Step 7: Capture and verify visually — toggle click**

Script a `Click` at coordinates inside the first toggle's panel. `Read` the resulting frame. Confirm the toggle's glyph changed from `[ \ ]` to `[ / ]`.

- [ ] **Step 8: Capture and verify visually — dial click**

Script a `Click` at coordinates inside the dial's panel. `Read` the resulting frame. Confirm the highlighted dial item advanced from "STANDBY" to "PATROL".

- [ ] **Step 9: Commit**

```bash
git add examples/control_panel.rs examples/README.md
git commit -m "feat(control-panel): add mouse-click-primary console example

Proves Rect::contains, Terminal's mouse capture, and visual-snapshot's
Click scripting all together: a LAUNCH button, three toggle switches,
and a mode dial, all driven by clicking rather than keyboard."
```

---

### Task 5: Final verification

**Files:** none (verification only).

- [ ] **Step 1: Build every target**

Run: `cargo build --all-targets`
Expected: succeeds.

- [ ] **Step 2: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
Expected: both clean.

- [ ] **Step 3: Full test suite**

Run: `cargo test`
Expected: full workspace suite green — includes Task 1's `Rect::contains` tests, Task 3's `script::`/`keys::`/`pty_roundtrip` tests (including the new `echo_mouse` integration test), and everything else unchanged.

- [ ] **Step 4: One more full `tools/visual-snapshot` capture of the finished result**

Run a capture spanning all three interaction types in one script — a click on the LAUNCH button, a click on a toggle, a click on the dial, each separated by a short wait. `Read` it. This is the final, whole-Arc confirmation — a single artifact demonstrating all three clickable elements responding correctly in sequence. Reference this capture in the PR's Verification section.

## Final verification (whole plan)

- [ ] `cargo build --all-targets` succeeds.
- [ ] `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` — clean.
- [ ] `cargo test` — full suite green, including all new `Rect::contains`/`script::`/`keys::`/`pty_roundtrip` tests.
- [ ] At least one `tools/visual-snapshot` capture from Task 5 is referenced in the PR description, showing all three clickable elements responding to scripted clicks.
- [ ] Per `.claude/rules/git-github-standards.md`: open a PR from this Arc's worktree branch to `main`, wait for all four required checks green, squash-merge, then remove the worktree via `ExitWorktree` (per the documented squash-merge resolution: verify via `gh pr view --json state,mergedAt,mergeCommit`, then retry with `discard_changes: true` if the tool's own ancestry check false-positives).
