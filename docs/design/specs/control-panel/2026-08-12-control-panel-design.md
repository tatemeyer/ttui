# Control Panel (Mouse Support) — Design

**Status:** draft, pending review before we move to planning.
**Date:** 2026-08-12
**Relationship to prior specs:** a fresh, free-ranging Arc (not part of
the original 4-Arc brainstorm, which is now complete except this).
Builds on `src/particles.rs` (`ParticleSystem`/`Particle`, already
proven by Falcon's WHACK mechanic), `src/theme.rs`/`src/layout.rs`
(existing), and the existing `AnalogToggle`/`Dial`/`Block` widgets
(`src/widgets/`, already shipped) — this Arc gives them a second,
mouse-driven interaction path rather than inventing new widgets.
Explicitly deferred as a Non-goal when `InputBinder` was built
(`docs/design/specs/core/2026-08-12-input-binding-design.md`'s
Non-goals) — that deferral's reasoning (no example used pointer
interaction, doesn't compose with the chord-binding core idea) is
resolved here with a dedicated example rather than folded into
`InputBinder` itself.

## Problem

TTUI has no mouse support at all — `Terminal` never enables mouse
capture, so `crossterm::event::Event::Mouse` is never even produced,
and no widget or app has ever needed to hit-test a click against a
render region. This spec adds the minimum real capability (mouse
capture, a point-in-rect hit test) and proves it with a new example
where clicking is the *primary* interaction, not an alternative to
keyboard — plus the one enabling change needed for
`.claude/rules/development-conventions.md`'s mandatory visual-review
convention to actually verify click-driven behavior: `tools/
visual-snapshot` currently has no way to script a mouse event at all.

## Scope

**`Rect::contains` (`src/layout.rs`) and `tools/visual-snapshot`'s new
click-scripting support: tag `coding`, TDD mandatory** — no exemption.

**`Terminal`'s mouse-capture enable/disable (`src/terminal.rs`): tag
`coding`, real-TTY exempt** per the existing "Real-TTY tests" exception
in `development-conventions.md` — it's bundled into the same
`execute!` calls already covered by that exemption (raw-mode enter/
exit), and `crossterm` exposes no queryable "is mouse capture enabled"
state to assert against, unlike `is_raw_mode_enabled()`.

**`examples/control_panel.rs`: tag `coding`, TDD-exempt** per the
"Examples/demos" exception, but **`tools/visual-snapshot` capture is
mandatory** — including at least one scripted click, now that the
tool supports it.

Four slices, in dependency order:

1. **`Rect::contains`** (`src/layout.rs`)
2. **Mouse capture in `Terminal`** (`src/terminal.rs`) — independent of 1.
3. **`tools/visual-snapshot` click-scripting support** (`tools/
   visual-snapshot/src/{script.rs,keys.rs,pty.rs}`) — independent of
   1-2, but sequenced before 4 since the example's mandatory visual
   review needs it.
4. **Control Panel example** (`examples/control_panel.rs`) — depends
   on 1-3.

## Design

### Slice 1: `Rect::contains`

```rust
impl Rect {
    /// Whether `(x, y)` falls within this rect — inclusive of the
    /// left/top edge, exclusive of the right/bottom edge (matches how
    /// `width`/`height` are already used everywhere else in this crate).
    pub fn contains(&self, x: u16, y: u16) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
}
```

### Slice 2: Mouse capture in `Terminal`

`Terminal::new()` (`src/terminal.rs`) adds `event::EnableMouseCapture`
to its existing `execute!` call:

```rust
execute!(out, terminal::EnterAlternateScreen, cursor::Hide, event::EnableMouseCapture)?;
```

`Drop for Terminal` and `install_panic_hook` both add
`event::DisableMouseCapture` to their existing cleanup `execute!`
calls, before `LeaveAlternateScreen`. `crossterm::event` is already
imported as `event` in this file (`use crossterm::event::{self,
Event};`), so `event::EnableMouseCapture`/`event::DisableMouseCapture`
resolve without a new import.

### Slice 3: `tools/visual-snapshot` click-scripting support

Add a `Click` variant to `Step` (`tools/visual-snapshot/src/script.rs`)
— the enum's existing `#[serde(untagged)]` disambiguates by field
presence, same as `Wait`/`Key` already do:

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

Add an encoder, `encode_click` (new function in `tools/visual-snapshot/
src/keys.rs`, alongside the existing `encode_key`):

```rust
/// Encodes a left-button click at cell `(x, y)` (0-indexed) into the
/// raw SGR mouse-protocol byte sequence a real terminal would send:
/// a press immediately followed by a release, both at the same
/// position. SGR coordinates are 1-indexed.
pub fn encode_click(x: u16, y: u16) -> Vec<u8> {
    let press = format!("\x1b[<0;{};{}M", x + 1, y + 1);
    let release = format!("\x1b[<0;{};{}m", x + 1, y + 1);
    let mut bytes = press.into_bytes();
    bytes.extend(release.into_bytes());
    bytes
}
```

Wire it into `run_script` (`tools/visual-snapshot/src/pty.rs`), a new
match arm alongside the existing `Wait`/`Key` handling, reusing the
same "wait for the child's actual reaction" quiescence strategy `Key`
steps already use (a click should also produce an observable
reaction, same reasoning as a keypress):

```rust
            Step::Click { x, y } => {
                session.send(&keys::encode_click(*x, *y))?;
                frames.push((
                    session.capture_frame_after_key()?,
                    KEY_STEP_DISPLAY_DURATION,
                ));
            }
```

Update `tools/visual-snapshot/README.md`'s script-format documentation
to add `{"x": N, "y": N}` as a third step shape — matching the
untagged enum's actual field-presence disambiguation, the same flat
shape `{"wait_ms": N}`/`{"key": "Name"}` already use, no wrapper key.

### Slice 4: Control Panel example

A retro sci-fi console (`examples/control_panel.rs`) where clicking is
the primary interaction: one big LAUNCH button, three `AnalogToggle`
switches, one `Dial` mode selector — five clickable regions total, all
existing widgets except the button (a plain `Block`).

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
        background: Color::Rgb { r: 10, g: 10, b: 10 },
        primary: Color::Rgb { r: 0, g: 255, b: 120 },
        secondary: Color::Rgb { r: 200, g: 200, b: 200 },
        tertiary: Color::Rgb { r: 255, g: 40, b: 40 },
        accent: Color::Rgb { r: 255, g: 200, b: 0 },
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
        let zero_rect = Rect { x: 0, y: 0, width: 0, height: 0 };
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
        let mut toggle_areas = [Rect { x: 0, y: 0, width: 0, height: 0 }; 3];
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

Click regions are hit-tested against each element's *outer* `Block`
rect (border included), not just its inner content — clicking the
border still counts, matching how a real physical control's clickable
area usually extends past its visible label text. `q` still quits via
keyboard (the one universal convention every TTUI example shares) —
the "no keyboard fallback" decision from brainstorming is specifically
about not duplicating the *showcased* interactions (toggle/button/
dial) with keyboard equivalents, not about removing the ability to
quit.

## Non-goals

- **Drag, hover-highlight, scroll-wheel handling.** Click (press+release
  at the same position) only.
- **Keyboard equivalents for the toggle/button/dial interactions.**
  `q`-to-quit is the only keyboard handling.
- **Extending `InputBinder` to resolve mouse events.** Mouse hit-testing
  is handled directly in each app's `update()` via `Rect::contains`,
  matching how Falcon's WHACK handler already does hit-testing today —
  chords/sequences don't apply to mouse clicks, so there's no shared
  resolution problem `InputBinder` would solve here.
- **A generalized `Button`/clickable-region widget.** The LAUNCH
  button is a plain `Block` plus app-tracked `Rect` — introducing a
  new stateful widget would violate the existing "dumb widget, no
  internal state" convention every other widget follows.
- **`tools/visual-snapshot` support for drag/scroll/right-click
  scripting.** Only a single left-button click is added.

## Testing

Per `.claude/rules/development-conventions.md`:

**`Rect::contains`** (`coding`, TDD mandatory): a point strictly
inside returns `true`; a point on the left/top edge returns `true`
(inclusive); a point on the right/bottom edge returns `false`
(exclusive); a point fully outside (each of the four directions)
returns `false`; a zero-width or zero-height rect contains nothing.

**`tools/visual-snapshot`'s `encode_click`/`Step::Click`** (`coding`,
TDD mandatory): `encode_click` produces the exact expected SGR byte
sequence for a known `(x, y)`; `Step` deserializes `{"x": N, "y": N}`
correctly and doesn't collide with `Wait`/`Key`'s field shapes; an
integration test (mirroring `pty_roundtrip.rs`'s existing
`a_key_step_actually_reaches_the_child_process`) proves a scripted
click genuinely reaches a spawned fixture process — extend one of the
existing fixture examples (or add a new one) to echo received mouse
events, matching `echo_key.rs`'s existing pattern for keys.

**`Terminal`'s mouse capture**: real-TTY exempt, no new automated
test (no queryable state to assert). Verified manually alongside the
existing raw-mode manual verification, and indirectly by every
scripted `Click` step in the Control Panel example's mandatory visual
review actually producing a visible effect.

Control Panel example (TDD-exempt, `tools/visual-snapshot` mandatory):
capture the initial panel layout (five regions, correct labels/glyphs);
a scripted `Click` on the LAUNCH button confirming a particle burst
appears; a scripted `Click` on a toggle confirming its glyph flips
(`[ \ ]` → `[ / ]`); a scripted `Click` on the dial confirming the
highlighted item advances.

## Critical files

- `src/layout.rs` — `Rect::contains`.
- `src/terminal.rs` — mouse capture enable/disable.
- `tools/visual-snapshot/src/script.rs` — `Step::Click`.
- `tools/visual-snapshot/src/keys.rs` — `encode_click`.
- `tools/visual-snapshot/src/pty.rs` — `run_script`'s new match arm.
- `tools/visual-snapshot/README.md` — script-format doc update.
- `examples/control_panel.rs` — new example.
- `examples/README.md` — new entry.

## Verification

- `cargo build --all-targets` / `cargo clippy --all-targets -- -D
  warnings` / `cargo fmt --check` — clean.
- `cargo test` — all new `Rect::contains`/`encode_click`/`Step::Click`
  tests green, full existing suite (including `tools/visual-snapshot`'s
  own) unchanged elsewhere.
- `tools/visual-snapshot` captures per the Testing section above,
  `Read` and confirmed — including at least one scripted `Click` step,
  proving the new capability end-to-end.
