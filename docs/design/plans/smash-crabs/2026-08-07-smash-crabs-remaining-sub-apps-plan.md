# Smash Crabs Remaining Sub-Apps + Boot Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `docs/design/specs/2026-08-07-smash-crabs-remaining-sub-apps-design.md`:
Target Smash (#63), Stage Hazards (#64), and the boot/intro splash (#65)
— completing Smash Crabs.

**Architecture:** Four tasks. Task 1 is the only `src/` change (a small
`easing::lerp_color` addition, TDD). Tasks 2-4 are all
`examples/smash_crabs.rs`, sequential because each narrows the same
`match self.screen` / `view()` / `render_destination_preview` blocks one
variant at a time (same pattern as every other sub-app arc this
session): Task 2 carves `Screen::TargetSmash` out of the combined
`TargetSmash | StageHazards` arms, Task 3 carves `Screen::StageHazards`
out of what's left and removes `render_placeholder`, Task 4 wraps the
whole app's entry point in a boot sequence.

**Tech Stack:** Rust, `crossterm`, `rodio` (already present, no
`Cargo.toml` change).

## Global Constraints

- `cargo fmt` / `cargo clippy --all-targets -- -D warnings` clean after
  every task. Use `.is_multiple_of(N)` instead of `% N == 0` (hit
  repeatedly this session) for the Bob-omb flash and flare-column
  parity checks.
- No RNG: the flare-column glyph choice reuses the exact
  deterministic-hash shape already used by `braille_noise`/
  `GlitchBuffer`/Star Charts' probability cloud.
- `self.shake_ticks_remaining` (already on `SmashCrabs`, from the arena
  arc) is reused as-is for Target Smash's shake — not a new field. Its
  only two triggers (Versus Mode's hit, Target Smash's smash) are
  mutually exclusive in time (only one screen renders at once), same
  sharing precedent as TARDIS's shared `GlitchBuffer`.
- `render_placeholder` becomes genuinely dead code once Task 3 lands
  (nothing calls it anymore) — delete it in Task 3 rather than leaving
  unused code behind, same precedent as removing `AppMode::name()`
  (Omnitrix) and TARDIS's own `render_placeholder`.
- `💥` (Task 2) is double-width in most terminals — same documented,
  accepted risk as Omnitrix's vision-doc glyphs. Not silently
  substituted for a safer ASCII stand-in.

---

### Task 1: `easing::lerp_color` (`src/easing.rs`)

**Files:**
- Modify: `src/easing.rs`

**Interfaces produced:** `lerp_color(from: Color, to: Color, t: f32) ->
Color` — Rgb-only lerp, falls back to `to` for any non-Rgb input (same
shape as `camera::dim`'s existing Rgb-only handling). Consumed by Task 2
for Target Smash's fade-out.

TDD — this is a `src/` change, per `.claude/rules/development-
conventions.md`.

- [ ] **Step 1: Write the failing tests** — add to `src/easing.rs`'s
  `#[cfg(test)] mod tests`:

```rust
    #[test]
    fn test_lerp_color_endpoints() {
        let from = Color::Rgb { r: 0, g: 0, b: 0 };
        let to = Color::Rgb {
            r: 200,
            g: 100,
            b: 50,
        };
        assert_eq!(lerp_color(from, to, 0.0), from);
        assert_eq!(lerp_color(from, to, 1.0), to);
    }

    #[test]
    fn test_lerp_color_midpoint() {
        let from = Color::Rgb { r: 0, g: 0, b: 0 };
        let to = Color::Rgb {
            r: 200,
            g: 100,
            b: 50,
        };
        assert_eq!(
            lerp_color(from, to, 0.5),
            Color::Rgb {
                r: 100,
                g: 50,
                b: 25
            }
        );
    }

    #[test]
    fn test_lerp_color_non_rgb_falls_back_to_target() {
        let from = Color::Reset;
        let to = Color::Rgb {
            r: 10,
            g: 20,
            b: 30,
        };
        assert_eq!(lerp_color(from, to, 0.5), to);
    }
```

  and add the import at the top of `src/easing.rs`:

```rust
use std::time::Duration;
use crossterm::style::Color;
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib easing:: -- --include-ignored`
Expected: FAIL — `lerp_color` not found in this scope (3 compile
errors, one per new test).

- [ ] **Step 3: Implement `lerp_color`** — add above the `#[cfg(test)]`
  block in `src/easing.rs`:

```rust
pub fn lerp_color(from: Color, to: Color, t: f32) -> Color {
    match (from, to) {
        (
            Color::Rgb {
                r: r1,
                g: g1,
                b: b1,
            },
            Color::Rgb {
                r: r2,
                g: g2,
                b: b2,
            },
        ) => Color::Rgb {
            r: lerp(r1 as f32, r2 as f32, t) as u8,
            g: lerp(g1 as f32, g2 as f32, t) as u8,
            b: lerp(b1 as f32, b2 as f32, t) as u8,
        },
        _ => to,
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib easing::`
Expected: PASS, all `easing::` tests including the 3 new ones.

- [ ] **Step 5: `cargo fmt && cargo clippy --all-targets -- -D warnings`**

Expected: clean, no warnings.

- [ ] **Step 6: Commit**

```bash
git add src/easing.rs
git commit -m "feat(core): add easing::lerp_color"
```

---

### Task 2: Target Smash (`examples/smash_crabs.rs`, #63)

**Files:**
- Modify: `examples/smash_crabs.rs`

**Interfaces consumed:** `easing::lerp_color(from, to, t) -> Color`
(Task 1); `effects::shake`, `self.shake_ticks_remaining`,
`self.paint_background`, `blit` (all already in this file, from the
arena arc).

**Interfaces produced:** `render_row(buf, area, text, fg)` free
function — bounds-guarded single-color-row text helper, reused by
Task 3 for nothing (Stage Hazards uses `Text`/`DamageMeter` directly)
but kept as a free function for consistency with this file's other
free helper (`blit`), same precedent as TARDIS's `render_ink_row`.

No new tests — example code, verified by running.

- [ ] **Step 1: Add Target Smash constants and the `TsPhase` enum** —
  in `examples/smash_crabs.rs`, replace:

```rust
const VS_TRANSITION_MS: u64 = 700;

#[derive(Clone, Copy, PartialEq)]
enum Screen {
```

  with:

```rust
const VS_TRANSITION_MS: u64 = 700;

const TS_TARGETS: [&str; 5] = [
    "Refactor auth module",
    "Fix flaky test",
    "Write release notes",
    "Review PR #42",
    "Update dependencies",
];
const TS_IMPACT_GLYPH: char = '💥';
const KO_HOLD_MS: u64 = 600;
const TS_FADE_MS: u64 = 400;

enum TsPhase {
    Impact(Transition),
    Fade(Transition),
}

#[derive(Clone, Copy, PartialEq)]
enum Screen {
```

- [ ] **Step 2: Add fields to `SmashCrabs`** — replace:

```rust
    audio: RodioAudioSink,
    quit: bool,
}
```

  with:

```rust
    audio: RodioAudioSink,
    ts_smashed: [bool; 5],
    ts_selected: usize,
    ts_smashing: Option<(usize, TsPhase)>,
    quit: bool,
}
```

  and in `new()`, replace:

```rust
            audio: RodioAudioSink::new(),
            quit: false,
        }
    }
```

  with:

```rust
            audio: RodioAudioSink::new(),
            ts_smashed: [false; 5],
            ts_selected: 0,
            ts_smashing: None,
            quit: false,
        }
    }
```

- [ ] **Step 3: Add `render_row`** — replace:

```rust
fn blit(scratch: &Buffer, area: Rect, buf: &mut Buffer) {
    for y in 0..scratch.height {
        for x in 0..scratch.width {
            buf.set(area.x + x, area.y + y, scratch.get(x, y).clone());
        }
    }
}
```

  with:

```rust
fn blit(scratch: &Buffer, area: Rect, buf: &mut Buffer) {
    for y in 0..scratch.height {
        for x in 0..scratch.width {
            buf.set(area.x + x, area.y + y, scratch.get(x, y).clone());
        }
    }
}

fn render_row(buf: &mut Buffer, area: Rect, text: &str, fg: Color) {
    if area.height == 0 {
        return;
    }
    for (i, ch) in text.chars().take(area.width as usize).enumerate() {
        buf.set(
            area.x + i as u16,
            area.y,
            Cell {
                symbol: ch,
                fg,
                bg: Color::Reset,
                ..Default::default()
            },
        );
    }
}
```

- [ ] **Step 4: Add Target Smash rendering methods** — replace:

```rust
    fn render_destination_preview(&self, screen: Screen, area: Rect) -> Buffer {
```

  with:

```rust
    fn ts_visible(&self) -> Vec<usize> {
        (0..TS_TARGETS.len())
            .filter(|&i| !self.ts_smashed[i])
            .collect()
    }

    fn ts_smashing_is_impact(&self) -> bool {
        matches!(&self.ts_smashing, Some((_, TsPhase::Impact(_))))
    }

    fn paint_ts_ui(&self, area: Rect) -> Buffer {
        let mut buf = Buffer::new(area.width, area.height);
        let local = Rect {
            x: 0,
            y: 0,
            width: area.width,
            height: area.height,
        };
        let inner = SmashBorder::new().render(local, &self.theme, &mut buf);
        let visible = self.ts_visible();
        if visible.is_empty() {
            render_row(&mut buf, inner, "ALL TARGETS DOWN", self.theme.tertiary);
        } else {
            for (row, &real_index) in visible.iter().enumerate() {
                let y = inner.y + row as u16;
                if y >= inner.y + inner.height {
                    break;
                }
                let fg = match &self.ts_smashing {
                    Some((i, TsPhase::Fade(t))) if *i == real_index => {
                        easing::lerp_color(self.theme.tertiary, self.theme.background, t.progress())
                    }
                    _ if row == self.ts_selected => self.theme.accent,
                    _ => self.theme.tertiary,
                };
                let row_area = Rect {
                    x: inner.x,
                    y,
                    width: inner.width,
                    height: 1,
                };
                render_row(&mut buf, row_area, TS_TARGETS[real_index], fg);
            }
        }
        let hint_row = Rect {
            x: inner.x,
            y: inner.y + inner.height.saturating_sub(1),
            width: inner.width,
            height: inner.height.saturating_sub(1).min(1),
        };
        Text::new("Up/Down move * Enter smash * Esc back * q quit").render(hint_row, &mut buf);
        buf
    }

    fn paint_ts_effects(&self, area: Rect) -> Buffer {
        let mut buf = Buffer::new(area.width, area.height);
        if self.ts_smashing_is_impact() {
            let cx = area.width / 2;
            let cy = area.height / 2;
            for offset in [-4i32, 0, 4] {
                let x = cx as i32 + offset;
                if x >= 0 && (x as u16) < area.width && cy > 0 {
                    buf.set(
                        x as u16,
                        cy - 1,
                        Cell {
                            symbol: TS_IMPACT_GLYPH,
                            fg: self.theme.accent,
                            bg: Color::Reset,
                            ..Default::default()
                        },
                    );
                }
            }
            let ko_x = cx.saturating_sub(1);
            for (i, ch) in "KO".chars().enumerate() {
                let x = ko_x + i as u16;
                if x < area.width && cy + 1 < area.height {
                    buf.set(
                        x,
                        cy + 1,
                        Cell {
                            symbol: ch,
                            fg: self.theme.tertiary,
                            bg: self.theme.primary,
                            style: CellStyle { bold: true },
                        },
                    );
                }
            }
        }
        buf
    }

    fn render_target_smash(&self, area: Rect, buf: &mut LayerStack) {
        let (dx, dy) = self.shake_offset();
        let layers: [(usize, Buffer); 3] = [
            (BACKGROUND, self.paint_background(area)),
            (UI, self.paint_ts_ui(area)),
            (EFFECTS, self.paint_ts_effects(area)),
        ];
        for (index, scratch) in layers {
            let final_buf = if dx != 0 || dy != 0 {
                effects::shake(&scratch, dx, dy)
            } else {
                scratch
            };
            blit(&final_buf, area, buf.layer_mut(index));
        }
    }

    fn render_destination_preview(&self, screen: Screen, area: Rect) -> Buffer {
```

- [ ] **Step 5: Route `Screen::TargetSmash` in `view()`** — replace:

```rust
            Screen::TargetSmash | Screen::StageHazards => {
                self.render_placeholder(self.screen, area, buf)
            }
```

  with:

```rust
            Screen::TargetSmash => {
                buf.push_layer(); // index 1: UI
                buf.push_layer(); // index 2: EFFECTS
                self.render_target_smash(area, buf);
            }
            Screen::StageHazards => self.render_placeholder(self.screen, area, buf),
```

- [ ] **Step 6: Route `Screen::TargetSmash` in
  `render_destination_preview`** — replace:

```rust
            Screen::TargetSmash | Screen::StageHazards => {
                let mut stack = LayerStack::new(area.width, area.height);
                self.render_placeholder(screen, local, &mut stack);
                blit(&stack, local, &mut buf);
            }
```

  with:

```rust
            Screen::TargetSmash => {
                let background = self.paint_background(local);
                blit(&background, local, &mut buf);
                let ui = self.paint_ts_ui(local);
                blit(&ui, local, &mut buf);
            }
            Screen::StageHazards => {
                let mut stack = LayerStack::new(area.width, area.height);
                self.render_placeholder(screen, local, &mut stack);
                blit(&stack, local, &mut buf);
            }
```

- [ ] **Step 7: Add the Target Smash arm to `update()`** — replace:

```rust
            Screen::TargetSmash | Screen::StageHazards => {
                if k.code == KeyCode::Esc {
                    self.screen = Screen::Hub;
                }
            }
```

  with:

```rust
            Screen::TargetSmash => {
                if self.ts_smashing.is_some() {
                    return;
                }
                let visible = self.ts_visible();
                match k.code {
                    KeyCode::Up => {
                        if !visible.is_empty() {
                            self.ts_selected =
                                (self.ts_selected + visible.len() - 1) % visible.len();
                        }
                    }
                    KeyCode::Down => {
                        if !visible.is_empty() {
                            self.ts_selected = (self.ts_selected + 1) % visible.len();
                        }
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        if let Some(&real_index) = visible.get(self.ts_selected) {
                            self.shake_ticks_remaining = SHAKE_TICKS;
                            self.ts_smashing = Some((
                                real_index,
                                TsPhase::Impact(Transition::start(Duration::from_millis(KO_HOLD_MS))),
                            ));
                        }
                    }
                    KeyCode::Esc => self.screen = Screen::Hub,
                    _ => {}
                }
            }
            Screen::StageHazards => {
                if k.code == KeyCode::Esc {
                    self.screen = Screen::Hub;
                }
            }
```

- [ ] **Step 8: Tick `ts_smashing` in `on_tick`** — replace:

```rust
        self.particles.update(elapsed);

        if let Some((destination, t)) = &mut self.transitioning_to {
```

  with:

```rust
        self.particles.update(elapsed);

        if let Some((real_index, phase)) = &mut self.ts_smashing {
            let real_index = *real_index;
            match phase {
                TsPhase::Impact(t) => {
                    t.tick(elapsed);
                    if t.is_complete() {
                        *phase = TsPhase::Fade(Transition::start(Duration::from_millis(TS_FADE_MS)));
                    }
                }
                TsPhase::Fade(t) => {
                    t.tick(elapsed);
                    if t.is_complete() {
                        self.ts_smashed[real_index] = true;
                        self.ts_smashing = None;
                    }
                }
            }
        }
        if self.ts_smashing.is_none() {
            let visible_len = self.ts_visible().len();
            if visible_len == 0 {
                self.ts_selected = 0;
            } else if self.ts_selected >= visible_len {
                self.ts_selected = visible_len - 1;
            }
        }

        if let Some((destination, t)) = &mut self.transitioning_to {
```

- [ ] **Step 9: Build**

Run: `cargo build --example smash_crabs`
Expected: compiles cleanly, no warnings.

- [ ] **Step 10: `cargo fmt && cargo clippy --all-targets -- -D warnings`**

Expected: clean, no warnings.

- [ ] **Step 11: Manual verification**

Run: `cargo run --example smash_crabs`

Navigate to Target Smash and confirm:
- All 5 targets list, `Up`/`Down` moves the accent-colored selection.
- `Enter` on a target shakes the screen, shows `💥` glyphs and a "KO"
  stamp near the panel center, then the target's text fades from white
  toward the background color and disappears — the list shrinks by one.
- Repeating this for all 5 targets ends on an "ALL TARGETS DOWN" empty
  state with no cursor to move.
- Navigation and `Enter` are ignored while a smash animation is playing.
- `Esc` (when not mid-smash) returns to the Hub; `q` quits cleanly.

Also re-confirm the Hub and Versus Mode still work exactly as before —
this task only touches `Screen::TargetSmash`'s own arms plus the
now-partially-split dispatch matches.

- [ ] **Step 12: Commit**

```bash
git add examples/smash_crabs.rs
git commit -m "feat(smash_crabs): add Target Smash sub-app (#63)"
```

---

### Task 3: Stage Hazards (`examples/smash_crabs.rs`, #64)

**Files:**
- Modify: `examples/smash_crabs.rs`

**Interfaces consumed:** `DamageMeter::new(percent).render(area, buf)`,
`Text::new(content).render(area, buf)` (both already imported, from the
arena arc).

**Interfaces produced:** none new — this task's rendering is entirely
inline in `render_stage_hazards`/`sh_cpu`.

No new tests — example code, verified by running.

- [ ] **Step 1: Add Stage Hazards constants** — replace:

```rust
const TS_FADE_MS: u64 = 400;

enum TsPhase {
```

  with:

```rust
const TS_FADE_MS: u64 = 400;

const RAM_STRESS_AMOUNT: f32 = 22.0;
const RAM_DECAY_PER_SEC: f32 = 6.0;
const RAM_THRESHOLD: f32 = 90.0;
const BOBOMB_FLASH_TICKS: u64 = 6;
const BOBOMB_ART: [&str; 5] = ["  .  ", " /   ", "( o )", "(o o)", " \\_/ "];

enum TsPhase {
```

- [ ] **Step 2: Add `sh_ram` field to `SmashCrabs`** — replace:

```rust
    ts_smashing: Option<(usize, TsPhase)>,
    quit: bool,
}
```

  with:

```rust
    ts_smashing: Option<(usize, TsPhase)>,
    sh_ram: f32,
    quit: bool,
}
```

  and in `new()`, replace:

```rust
            ts_smashing: None,
            quit: false,
        }
    }
```

  with:

```rust
            ts_smashing: None,
            sh_ram: 20.0,
            quit: false,
        }
    }
```

- [ ] **Step 3: Add Stage Hazards rendering** — replace:

```rust
    fn render_destination_preview(&self, screen: Screen, area: Rect) -> Buffer {
```

  with:

```rust
    fn sh_cpu(&self) -> f32 {
        50.0 + 15.0 * (self.tick_count as f32 * 0.03).sin()
    }

    fn render_stage_hazards(&self, area: Rect, buf: &mut LayerStack) {
        let inner = SmashBorder::new().render(area, &self.theme, buf);
        let rows = Layout::new(
            Direction::Vertical,
            vec![Constraint::Fixed(1), Constraint::Fixed(1)],
        )
        .split(inner);

        Text::new("CPU").render(
            Rect {
                x: rows[0].x,
                y: rows[0].y,
                width: 4.min(rows[0].width),
                height: 1,
            },
            buf,
        );
        DamageMeter::new(self.sh_cpu().round() as u16).render(
            Rect {
                x: rows[0].x + 4,
                y: rows[0].y,
                width: rows[0].width.saturating_sub(4),
                height: 1,
            },
            buf,
        );
        Text::new("RAM").render(
            Rect {
                x: rows[1].x,
                y: rows[1].y,
                width: 4.min(rows[1].width),
                height: 1,
            },
            buf,
        );
        DamageMeter::new(self.sh_ram.round() as u16).render(
            Rect {
                x: rows[1].x + 4,
                y: rows[1].y,
                width: rows[1].width.saturating_sub(4),
                height: 1,
            },
            buf,
        );

        if self.sh_ram >= RAM_THRESHOLD {
            let flashing_on = (self.tick_count / BOBOMB_FLASH_TICKS).is_multiple_of(2);
            let color = if flashing_on {
                Color::Red
            } else {
                self.theme.background
            };
            let art_width = BOBOMB_ART[0].chars().count() as u16;
            let art_x = area.x + area.width.saturating_sub(art_width + 1);
            for (row, line) in BOBOMB_ART.iter().enumerate() {
                let y = area.y + 1 + row as u16;
                if y >= area.y + area.height {
                    break;
                }
                for (col, ch) in line.chars().enumerate() {
                    let x = art_x + col as u16;
                    if x < area.x + area.width {
                        buf.set(
                            x,
                            y,
                            Cell {
                                symbol: ch,
                                fg: color,
                                bg: Color::Reset,
                                ..Default::default()
                            },
                        );
                    }
                }
            }
        }

        let hint_row = Rect {
            x: inner.x,
            y: inner.y + inner.height.saturating_sub(1),
            width: inner.width,
            height: inner.height.saturating_sub(1).min(1),
        };
        Text::new("Space stress RAM * Esc back * q quit").render(hint_row, buf);
    }

    fn render_destination_preview(&self, screen: Screen, area: Rect) -> Buffer {
```

- [ ] **Step 4: Route `Screen::StageHazards` and delete
  `render_placeholder`** — in `view()`, replace:

```rust
            Screen::StageHazards => self.render_placeholder(self.screen, area, buf),
```

  with:

```rust
            Screen::StageHazards => self.render_stage_hazards(area, buf),
```

  in `render_destination_preview`, replace:

```rust
            Screen::StageHazards => {
                let mut stack = LayerStack::new(area.width, area.height);
                self.render_placeholder(screen, local, &mut stack);
                blit(&stack, local, &mut buf);
            }
```

  with:

```rust
            Screen::StageHazards => {
                let mut stack = LayerStack::new(area.width, area.height);
                self.render_stage_hazards(local, &mut stack);
                blit(&stack, local, &mut buf);
            }
```

  Then run `cargo build --example smash_crabs 2>&1` and confirm the
  only diagnostic (if any) is `warning: method \`render_placeholder\`
  is never used` — proving it before deleting, not assuming it. Then
  delete the entire `render_placeholder` method. Run the build again;
  it should now be warning-free.

- [ ] **Step 5: Add the Stage Hazards arm to `update()`** — replace:

```rust
            Screen::StageHazards => {
                if k.code == KeyCode::Esc {
                    self.screen = Screen::Hub;
                }
            }
```

  with:

```rust
            Screen::StageHazards => match k.code {
                KeyCode::Char(' ') => {
                    self.sh_ram = (self.sh_ram + RAM_STRESS_AMOUNT).min(100.0);
                }
                KeyCode::Esc => self.screen = Screen::Hub,
                _ => {}
            },
```

- [ ] **Step 6: Decay `sh_ram` in `on_tick`** — replace:

```rust
        self.particles.update(elapsed);

        if let Some((real_index, phase)) = &mut self.ts_smashing {
```

  with:

```rust
        self.particles.update(elapsed);

        self.sh_ram = (self.sh_ram - RAM_DECAY_PER_SEC * elapsed.as_secs_f32()).max(0.0);

        if let Some((real_index, phase)) = &mut self.ts_smashing {
```

- [ ] **Step 7: Build**

Run: `cargo build --example smash_crabs`
Expected: compiles cleanly, no warnings (confirms `render_placeholder`
was fully unused before deletion).

- [ ] **Step 8: `cargo fmt && cargo clippy --all-targets -- -D warnings`**

Expected: clean, no warnings.

- [ ] **Step 9: Manual verification**

Run: `cargo run --example smash_crabs`

Navigate to Stage Hazards and confirm:
- CPU's meter ambiently wobbles on its own without any input.
- `Space` spikes RAM's meter; releasing it, the meter decays back down
  over a few seconds.
- Holding `Space` until RAM reaches ~90%+ makes a small flashing-red
  Bob-omb ASCII art appear in a corner; letting RAM decay back under
  90% makes it disappear.
- `Esc` returns to the Hub; `q` quits cleanly.

Also re-confirm Target Smash (Task 2), the Hub, and Versus Mode still
work exactly as before.

- [ ] **Step 10: Commit**

```bash
git add examples/smash_crabs.rs
git commit -m "feat(smash_crabs): add Stage Hazards sub-app (#64)"
```

---

### Task 4: Boot/intro splash (`examples/smash_crabs.rs`, #65)

**Files:**
- Modify: `examples/smash_crabs.rs`

**Interfaces consumed:** `camera::dim(buf, factor) -> Buffer` (new
import this task); `self.render_destination_preview(Screen::Hub, area)`
(already exists, reused as-is to get the real Hub's content for the
flare-reveal phase); `easing::lerp`/`easing::ease_out` (already
imported).

**Interfaces produced:** `render_centered_art(buf, area, art, fg)` and
`render_boot_title(buf, area, sub, fg)` free functions, local to this
file (one-off boot decoration, same "local helper, not a core widget"
precedent as TARDIS's `render_ink_row`).

No new tests — example code, verified by running.

- [ ] **Step 1: Import `camera`** — replace:

```rust
use ttui::app::{run, App};
use ttui::audio::AudioSink;
```

  with:

```rust
use ttui::app::{run, App};
use ttui::audio::AudioSink;
use ttui::camera;
```

- [ ] **Step 2: Add boot constants** — replace:

```rust
const BOBOMB_ART: [&str; 5] = ["  .  ", " /   ", "( o )", "(o o)", " \\_/ "];

enum TsPhase {
```

  with:

```rust
const BOBOMB_ART: [&str; 5] = ["  .  ", " /   ", "( o )", "(o o)", " \\_/ "];

const BOOT_FLASH_MS: u64 = 200;
const BOOT_CLAW_MS: u64 = 800;
const BOOT_TITLE_MS: u64 = 600;
const BOOT_FLARE_MS: u64 = 500;
const BOOT_TOTAL_MS: u64 = BOOT_FLASH_MS + BOOT_CLAW_MS + BOOT_TITLE_MS + BOOT_FLARE_MS;
const BOOT_TITLE: &str = "S U P E R S M A S H C L A W S";
const CLAW_OPEN: [&str; 5] = [
    " \\           / ",
    "  \\         /  ",
    "   (         )  ",
    "    \\       /   ",
    "     \\_____/    ",
];
const CLAW_CLOSED: [&str; 5] = [
    "   \\       /    ",
    "    \\     /     ",
    "     (   )      ",
    "      \\ /       ",
    "       X        ",
];

enum TsPhase {
```

- [ ] **Step 3: Add fields to `SmashCrabs`** — replace:

```rust
    ts_smashing: Option<(usize, TsPhase)>,
    sh_ram: f32,
    quit: bool,
}
```

  with:

```rust
    ts_smashing: Option<(usize, TsPhase)>,
    sh_ram: f32,
    booting: Option<Transition>,
    boot_snap_played: bool,
    quit: bool,
}
```

  and in `new()`, replace:

```rust
            ts_smashing: None,
            sh_ram: 20.0,
            quit: false,
        }
    }
```

  with:

```rust
            ts_smashing: None,
            sh_ram: 20.0,
            booting: Some(Transition::start(Duration::from_millis(BOOT_TOTAL_MS))),
            boot_snap_played: false,
            quit: false,
        }
    }
```

- [ ] **Step 4: Add the `"snap"` audio event** — replace:

```rust
        let freq: f32 = match event_id {
            "cursor" => 440.0,
            "select" => 660.0,
            "hit" => 220.0,
            _ => return,
        };
```

  with:

```rust
        let freq: f32 = match event_id {
            "cursor" => 440.0,
            "select" => 660.0,
            "hit" => 220.0,
            "snap" => 110.0,
            _ => return,
        };
```

- [ ] **Step 5: Add `render_centered_art` and `render_boot_title`** —
  replace:

```rust
impl App for SmashCrabs {
```

  with:

```rust
fn render_centered_art(buf: &mut Buffer, area: Rect, art: &[&str], fg: Color) {
    let art_height = art.len() as u16;
    let art_width = art
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0) as u16;
    let y0 = area.height.saturating_sub(art_height) / 3;
    let x0 = area.width.saturating_sub(art_width) / 2;
    for (row, line) in art.iter().enumerate() {
        let y = y0 + row as u16;
        if y >= area.height {
            break;
        }
        for (col, ch) in line.chars().enumerate() {
            if ch == ' ' {
                continue;
            }
            let x = x0 + col as u16;
            if x < area.width {
                buf.set(
                    area.x + x,
                    area.y + y,
                    Cell {
                        symbol: ch,
                        fg,
                        bg: Color::Reset,
                        ..Default::default()
                    },
                );
            }
        }
    }
}

fn render_boot_title(buf: &mut Buffer, area: Rect, sub: f32, fg: Color) {
    let chars: Vec<char> = BOOT_TITLE.chars().collect();
    let half = chars.len() / 2;
    let total_width = chars.len() as u16;
    let start_x = area.width.saturating_sub(total_width) / 2;
    let y = area.height * 2 / 3;
    if y >= area.height {
        return;
    }
    for (i, &ch) in chars.iter().enumerate() {
        if ch == ' ' {
            continue;
        }
        let final_x = (start_x + i as u16) as f32;
        let x = if i < half {
            let from_x = -((half - i) as f32) - 2.0;
            easing::ease_out(from_x, final_x, sub)
        } else {
            let from_x = area.width as f32 + (i - half) as f32 + 2.0;
            easing::ease_out(from_x, final_x, sub)
        };
        let x = x.round();
        if x >= 0.0 && (x as u16) < area.width {
            buf.set(
                area.x + x as u16,
                area.y + y,
                Cell {
                    symbol: ch,
                    fg,
                    bg: Color::Reset,
                    ..Default::default()
                },
            );
        }
    }
}

impl App for SmashCrabs {
```

- [ ] **Step 6: Add `render_boot`** — replace:

```rust
    fn render_destination_preview(&self, screen: Screen, area: Rect) -> Buffer {
```

  with:

```rust
    fn render_boot(&self, area: Rect, progress: f32, buf: &mut LayerStack) {
        let t1 = BOOT_FLASH_MS as f32 / BOOT_TOTAL_MS as f32;
        let t2 = (BOOT_FLASH_MS + BOOT_CLAW_MS) as f32 / BOOT_TOTAL_MS as f32;
        let t3 = (BOOT_FLASH_MS + BOOT_CLAW_MS + BOOT_TITLE_MS) as f32 / BOOT_TOTAL_MS as f32;
        let local = Rect {
            x: 0,
            y: 0,
            width: area.width,
            height: area.height,
        };

        if progress < t1 {
            let sub = progress / t1;
            let mut white = Buffer::new(area.width, area.height);
            let cell = Cell {
                symbol: ' ',
                fg: Color::Reset,
                bg: Color::White,
                ..Default::default()
            };
            for y in 0..area.height {
                for x in 0..area.width {
                    white.set(x, y, cell.clone());
                }
            }
            let dimmed = camera::dim(&white, sub);
            blit(&dimmed, area, buf);
            return;
        }

        if progress < t2 {
            let sub = (progress - t1) / (t2 - t1);
            let art: &[&str] = if sub < 0.5 { &CLAW_OPEN } else { &CLAW_CLOSED };
            render_centered_art(buf, area, art, self.theme.tertiary);
            return;
        }

        if progress < t3 {
            let sub = (progress - t2) / (t3 - t2);
            render_centered_art(buf, area, &CLAW_CLOSED, self.theme.tertiary);
            render_boot_title(buf, area, sub, self.theme.accent);
            return;
        }

        let sub = ((progress - t3) / (1.0 - t3)).clamp(0.0, 1.0);
        let hub_content = self.render_destination_preview(Screen::Hub, area);
        let mut logo = Buffer::new(area.width, area.height);
        render_centered_art(&mut logo, local, &CLAW_CLOSED, self.theme.tertiary);
        render_boot_title(&mut logo, local, 1.0, self.theme.accent);

        let flare_x = -3.0 + sub * (area.width as f32 + 6.0);
        for y in 0..area.height {
            for x in 0..area.width {
                let fx = x as f32;
                let cell = if (fx - flare_x).abs() <= 1.5 {
                    let h = (x as u64).wrapping_mul(374_761_393)
                        ^ self.tick_count.wrapping_mul(2_246_822_519);
                    let fg = if h.is_multiple_of(2) {
                        self.theme.accent
                    } else {
                        self.theme.tertiary
                    };
                    Cell {
                        symbol: '|',
                        fg,
                        bg: Color::Reset,
                        ..Default::default()
                    }
                } else if fx < flare_x {
                    hub_content.get(x, y).clone()
                } else {
                    logo.get(x, y).clone()
                };
                buf.set(area.x + x, area.y + y, cell);
            }
        }
    }

    fn render_destination_preview(&self, screen: Screen, area: Rect) -> Buffer {
```

- [ ] **Step 7: Check `booting` first in `view()`** — replace:

```rust
    fn view(&self, area: Rect, buf: &mut LayerStack) {
        if let Some((destination, transition)) = &self.transitioning_to {
```

  with:

```rust
    fn view(&self, area: Rect, buf: &mut LayerStack) {
        if let Some(t) = &self.booting {
            self.render_boot(area, t.progress(), buf);
            return;
        }
        if let Some((destination, transition)) = &self.transitioning_to {
```

- [ ] **Step 8: Block input while booting** — replace:

```rust
        if self.transitioning_to.is_some() {
            return;
        }
        match self.screen {
```

  with:

```rust
        if self.transitioning_to.is_some() || self.booting.is_some() {
            return;
        }
        match self.screen {
```

- [ ] **Step 9: Tick `booting` in `on_tick`** — replace:

```rust
    fn on_tick(&mut self, elapsed: Duration) {
        if self.flash_ticks_remaining > 0 {
```

  with:

```rust
    fn on_tick(&mut self, elapsed: Duration) {
        if let Some(t) = &mut self.booting {
            t.tick(elapsed);
            let progress = t.progress();
            let t1 = BOOT_FLASH_MS as f32 / BOOT_TOTAL_MS as f32;
            let t2 = (BOOT_FLASH_MS + BOOT_CLAW_MS) as f32 / BOOT_TOTAL_MS as f32;
            if !self.boot_snap_played && progress >= t1 {
                let claw_sub = ((progress - t1) / (t2 - t1)).clamp(0.0, 1.0);
                if claw_sub >= 0.5 {
                    self.boot_snap_played = true;
                    self.audio.play("snap");
                }
            }
            if t.is_complete() {
                self.booting = None;
            }
        }

        if self.flash_ticks_remaining > 0 {
```

- [ ] **Step 10: Build**

Run: `cargo build --example smash_crabs`
Expected: compiles cleanly, no warnings.

- [ ] **Step 11: `cargo fmt && cargo clippy --all-targets -- -D warnings`**

Expected: clean, no warnings.

- [ ] **Step 12: Manual verification**

Run: `cargo run --example smash_crabs`

Confirm on launch:
- A brief white flash resolves to black.
- An ASCII claw appears, flips from an open to a closed pose partway
  through (a single frame swap), with an audible tone at the flip.
- `S U P E R S M A S H C L A W S` slides in — the left half of the
  letters from the left edge, the right half from the right edge —
  converging to a centered, fully-spelled title above the claw.
- A vertical band of bright yellow/white glyphs sweeps left to right,
  and the real Hub is visible underneath wherever the band has already
  passed.
- The app lands on the normal, interactive Hub once the sweep finishes.
- `q` quits immediately if pressed at any point during boot.

Also re-confirm Target Smash (Task 2), Stage Hazards (Task 3), and
Versus Mode all still work exactly as before — this task only wraps
entry into the existing Hub, it doesn't touch any other screen's logic.

- [ ] **Step 13: Commit**

```bash
git add examples/smash_crabs.rs
git commit -m "feat(smash_crabs): add boot/intro splash sequence (#65)"
```

---

## Self-Review

**Spec coverage:** Target Smash (fixed 5-target list, shake + `💥` + KO
stamp + fade-out via the promoted `easing::lerp_color`) — Task 2. Stage
Hazards (ambient CPU wobble, user-driven/decaying RAM, flashing Bob-omb
at ≥90%) — Task 3. Boot sequence (flash → claw open/closed snap with
audio → spaced title slide-in → sweeping flare reveal of the real Hub)
— Task 4. `easing::lerp_color` (TDD) — Task 1. Verification section
(`cargo test`/`fmt`/`clippy` plus a full manual `cargo run --example
smash_crabs` walkthrough of all three new pieces and a regression check
on the Hub/Versus Mode) — covered across every task's final steps. The
spec's out-of-scope list (growing target backlog, a CPU-side hazard
creature, real system-stat integration, a skippable boot) — none added
anywhere in this plan.

**Placeholder scan:** no TBD/TODO in code or commands. Task 3 Step 4
includes an explicit instruction to confirm `render_placeholder`'s
unused-method warning appears *before* deletion, not assumed.

**Type consistency:** `TsPhase`, `TS_TARGETS`, `ts_smashed`/
`ts_selected`/`ts_smashing`, `render_row`, `paint_ts_ui`/
`paint_ts_effects`/`render_target_smash` (Task 2) are used consistently
within Task 2 and never touched by Tasks 3-4. `sh_ram`, `sh_cpu`,
`render_stage_hazards`, `RAM_*`/`BOBOMB_*` constants (Task 3) are
self-contained to Task 3. `booting`/`boot_snap_played`,
`render_boot`/`render_centered_art`/`render_boot_title`, `BOOT_*`/
`CLAW_*` constants (Task 4) are self-contained to Task 4 and only
*read* Task 2/3 state indirectly via the pre-existing
`render_destination_preview(Screen::Hub, ...)` call, which itself is
untouched by this plan. Every task's `Screen::TargetSmash`/
`Screen::StageHazards` match-arm edits target exactly the arm the
*other* task leaves untouched — verified by reading each `old_string`
against the prior task's final state before writing these steps, not
assumed.
