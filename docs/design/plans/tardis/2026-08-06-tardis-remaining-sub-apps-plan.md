# TARDIS Remaining Sub-Apps Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `docs/design/specs/2026-08-06-tardis-remaining-sub-apps-design.md`:
real content for Psychic Paper (canned-prompt send loop on a reversed
paper palette, ink-bleed replies, deterministic Perception Filter
glitch) and Star Charts (circular 5-slot timeline with past/present/
future nodes, Temporal Shift on completion) — completing TARDIS.

**Architecture:** Two tasks, both `examples/tardis.rs`, no `src/`
changes at all this arc — everything reuses machinery already shipped
(`GlitchBuffer`, `Roundel`, `Transition`-driven flash timing, the
deterministic hash-noise technique, `RodioAudioSink`). Sequential only
because both narrow the same `match self.screen` blocks one variant at
a time (same pattern as every other sub-app task this session) — Task 1
carves `Screen::PsychicPaper` out of the combined
`PsychicPaper | StarCharts` arm, Task 2 carves `Screen::StarCharts` out
of what's left, and removes `render_placeholder` once nothing calls it.

**Tech Stack:** Rust, `crossterm`, `rodio` (already present, no
`Cargo.toml` change).

## Global Constraints

- Example code — per `.claude/rules/development-conventions.md`'s TDD
  exceptions, verified by running, not unit tested. No `src/` changes,
  so no TDD tasks this arc.
- `cargo fmt` / `cargo clippy --all-targets -- -D warnings` clean after
  every task. Use `.is_multiple_of(N)` instead of `% N == 0` (hit
  repeatedly this session).
- No RNG: the probability-cloud glyphs reuse the exact deterministic-
  hash shape already used by `braille_noise`/`GlitchBuffer`.
- `self.glitch` (the `GlitchBuffer` field already on `Tardis` for
  Artron Energy's lag state) is reused as-is for Psychic Paper's
  Perception Filter break — not a new field. The two triggers are
  mutually exclusive in time (only one screen renders at once) and
  `GlitchBuffer` has no per-trigger identity to confuse, so sharing it
  is safe.
- `render_placeholder` becomes genuinely dead code once Task 2 lands
  (nothing calls it anymore) — delete it in Task 2 rather than leaving
  unused code behind, same precedent as removing `AppMode::name()` in
  the Omnitrix arc when its last caller went away.

---

### Task 1: Psychic Paper (`examples/tardis.rs`, #77)

**Files:**
- Modify: `examples/tardis.rs`

**Interfaces consumed:** `self.glitch: GlitchBuffer` (already on
`Tardis`, from the console arc); `RodioAudioSink::play` (already on
`Tardis`, gains one more `event_id`).

**Interfaces produced:** `lerp_color(from, to, t) -> Color` and
`render_ink_row(buf, area, y, text, fg)` free functions — the latter is
reused by Task 2 for nothing (Star Charts has its own distinct
per-node-type rendering, not a plain text row), but kept as a free
function rather than a method for consistency with this file's other
free helpers (`blit`).

No new tests — example code, verified by running.

- [ ] **Step 1: Add the `RelaySpeaker` enum and Psychic Paper
  constants** — insert above `struct Tardis`:

```rust
#[derive(Clone, Copy, PartialEq)]
enum RelaySpeaker {
    User,
    Agent,
}

const PSYCHIC_PROMPTS: [&str; 3] = [
    "Status of the away team",
    "Translate this inscription",
    "Locate the temporal anomaly",
];
const PSYCHIC_THINKING_MS: u64 = 800;
const PSYCHIC_REVEAL_MS: u64 = 800;
const PSYCHIC_GLITCH_EVERY: u32 = 3;
const PSYCHIC_GLITCH_DURATION_MS: u64 = 600;
const PAPER_COLOR: Color = Color::Rgb {
    r: 230,
    g: 225,
    b: 210,
};
const INK_COLOR: Color = Color::Rgb {
    r: 20,
    g: 20,
    b: 40,
};
```

- [ ] **Step 2: Add fields to `Tardis`** — insert after `audio`:

```rust
    audio: RodioAudioSink,
    psychic_log: Vec<(RelaySpeaker, String)>,
    psychic_prompt_index: usize,
    psychic_send_count: u32,
    psychic_pending: Option<(bool, Transition)>,
    psychic_reveal: Option<Transition>,
    quit: bool,
}
```

  and in `new()` (inside the struct literal, before `quit: false,`):

```rust
            audio: RodioAudioSink::new(),
            psychic_log: Vec::new(),
            psychic_prompt_index: 0,
            psychic_send_count: 0,
            psychic_pending: None,
            psychic_reveal: None,
            quit: false,
        };
        tardis.audio.play("boot");
        tardis
    }
```

- [ ] **Step 3: Add the `"glitch"` audio event** — in
  `RodioAudioSink::play`, add a match arm:

```rust
        let freq: f32 = match event_id {
            "boot" => 100.0,
            "flight" => 300.0,
            "vent" => 500.0,
            "glitch" => 700.0,
            _ => return,
        };
```

- [ ] **Step 4: Add `lerp_color` and `render_ink_row` free functions**
  — add near `blit`:

```rust
fn lerp_color(from: Color, to: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
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
            r: (r1 as f32 + (r2 as f32 - r1 as f32) * t) as u8,
            g: (g1 as f32 + (g2 as f32 - g1 as f32) * t) as u8,
            b: (b1 as f32 + (b2 as f32 - b1 as f32) * t) as u8,
        },
        _ => to,
    }
}

fn render_ink_row(buf: &mut LayerStack, area: Rect, y: u16, text: &str, fg: Color) {
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
                bg: PAPER_COLOR,
                ..Default::default()
            },
        );
    }
}
```

- [ ] **Step 5: Add `render_psychic_paper`** — add to `impl Tardis`:

```rust
    fn render_psychic_paper(&self, area: Rect, buf: &mut LayerStack) {
        for y in 0..area.height {
            for x in 0..area.width {
                buf.set(
                    area.x + x,
                    area.y + y,
                    Cell {
                        symbol: ' ',
                        fg: Color::Reset,
                        bg: PAPER_COLOR,
                        ..Default::default()
                    },
                );
            }
        }

        let start = self.psychic_log.len().saturating_sub(5);
        let last_index = self.psychic_log.len().saturating_sub(1);
        for (i, (speaker, text)) in self.psychic_log[start..].iter().enumerate() {
            let absolute_index = start + i;
            let prefix = match speaker {
                RelaySpeaker::User => "You: ",
                RelaySpeaker::Agent => "Relay: ",
            };
            let is_latest_agent = *speaker == RelaySpeaker::Agent
                && !self.psychic_log.is_empty()
                && absolute_index == last_index;
            let fg = if is_latest_agent {
                match &self.psychic_reveal {
                    Some(t) => lerp_color(PAPER_COLOR, INK_COLOR, t.progress()),
                    None => INK_COLOR,
                }
            } else {
                INK_COLOR
            };
            render_ink_row(buf, area, i as u16, &format!("{prefix}{text}"), fg);

            if is_latest_agent && self.glitch.is_active() && (i as u16) < area.height {
                let glitch_row = Rect {
                    x: area.x,
                    y: area.y + i as u16,
                    width: area.width,
                    height: 1,
                };
                self.glitch.render(glitch_row, Color::Red, self.tick_count, buf);
            }
        }

        render_ink_row(
            buf,
            area,
            area.height.saturating_sub(2),
            PSYCHIC_PROMPTS[self.psychic_prompt_index],
            INK_COLOR,
        );
        render_ink_row(
            buf,
            area,
            area.height.saturating_sub(1),
            "Tab cycle * Enter send * Esc back * q quit",
            INK_COLOR,
        );
    }
```

- [ ] **Step 6: Route `Screen::PsychicPaper` to it** — in
  `render_destination_preview`, replace:

```rust
            Screen::PsychicPaper | Screen::StarCharts => {
                self.render_placeholder(screen, local, &mut stack)
            }
```

  with:

```rust
            Screen::PsychicPaper => self.render_psychic_paper(local, &mut stack),
            Screen::StarCharts => self.render_placeholder(screen, local, &mut stack),
```

  and in `view()`, replace:

```rust
            Screen::PsychicPaper | Screen::StarCharts => {
                self.render_placeholder(self.screen, area, buf)
            }
```

  with:

```rust
            Screen::PsychicPaper => self.render_psychic_paper(area, buf),
            Screen::StarCharts => self.render_placeholder(self.screen, area, buf),
```

- [ ] **Step 7: Add the Psychic Paper arm to `update()`** — replace:

```rust
            Screen::PsychicPaper | Screen::StarCharts => {
                if k.code == KeyCode::Esc {
                    self.screen = Screen::Hub;
                }
            }
```

  with:

```rust
            Screen::PsychicPaper => {
                if self.psychic_pending.is_some() {
                    return;
                }
                match k.code {
                    KeyCode::Tab => {
                        self.psychic_prompt_index =
                            (self.psychic_prompt_index + 1) % PSYCHIC_PROMPTS.len();
                    }
                    KeyCode::BackTab => {
                        self.psychic_prompt_index = (self.psychic_prompt_index
                            + PSYCHIC_PROMPTS.len()
                            - 1)
                            % PSYCHIC_PROMPTS.len();
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        self.psychic_log.push((
                            RelaySpeaker::User,
                            PSYCHIC_PROMPTS[self.psychic_prompt_index].to_string(),
                        ));
                        self.psychic_send_count += 1;
                        let will_glitch = self.psychic_send_count.is_multiple_of(PSYCHIC_GLITCH_EVERY);
                        self.psychic_pending = Some((
                            will_glitch,
                            Transition::start(Duration::from_millis(PSYCHIC_THINKING_MS)),
                        ));
                    }
                    KeyCode::Esc => self.screen = Screen::Hub,
                    _ => {}
                }
            }
            Screen::StarCharts => {
                if k.code == KeyCode::Esc {
                    self.screen = Screen::Hub;
                }
            }
```

- [ ] **Step 8: Tick `psychic_pending`/`psychic_reveal` in `on_tick`**
  — append after the existing `vent_flash` tick block:

```rust
        if let Some((will_glitch, t)) = &mut self.psychic_pending {
            t.tick(elapsed);
            if t.is_complete() {
                if *will_glitch {
                    self.psychic_log
                        .push((RelaySpeaker::Agent, "...signal lost...".to_string()));
                    self.glitch
                        .trigger(Duration::from_millis(PSYCHIC_GLITCH_DURATION_MS));
                    self.audio.play("glitch");
                } else {
                    let prompt = PSYCHIC_PROMPTS[self.psychic_prompt_index];
                    self.psychic_log
                        .push((RelaySpeaker::Agent, format!("{prompt} — relay confirmed.")));
                    self.psychic_reveal =
                        Some(Transition::start(Duration::from_millis(PSYCHIC_REVEAL_MS)));
                }
                self.psychic_pending = None;
            }
        }
        if let Some(t) = &mut self.psychic_reveal {
            t.tick(elapsed);
            if t.is_complete() {
                self.psychic_reveal = None;
            }
        }
```

- [ ] **Step 9: Build**

Run: `cargo build --example tardis`
Expected: compiles cleanly, no warnings.

- [ ] **Step 10: `cargo fmt && cargo clippy --all-targets -- -D warnings`**

Expected: clean, no warnings.

- [ ] **Step 11: Manual verification**

Run: `cargo run --example tardis`

Navigate to Psychic Paper and confirm:
- The screen background is a warm off-white "paper" color, not the
  usual deep-space black.
- `Tab`/`Shift+Tab` cycle the 3 canned prompts; `Enter`/`Space` sends
  one — it appears instantly as "You: ...".
- On the 1st and 2nd sends, after a brief pause a "Relay: ..." reply
  fades in from invisible-against-the-paper to sharp dark ink over
  ~800ms.
- On the 3rd send (and every 3rd after), instead the reply is
  "...signal lost..." with red glitch noise flickering over it, and a
  distinct audible tone plays.
- Navigation is ignored while a reply is pending.
- `Esc` (when not pending) returns to the Hub; `q` quits cleanly.

- [ ] **Step 12: Commit**

```bash
git add examples/tardis.rs
git commit -m "feat(tardis): add Psychic Paper sub-app (#77)"
```

---

### Task 2: Star Charts (`examples/tardis.rs`, #78)

**Files:**
- Modify: `examples/tardis.rs`

**Interfaces consumed:** `Roundel::new(intensity, color).render(area,
buf)` (already imported, from the console arc).

**Interfaces produced:** none new — this task's rendering is entirely
inline in `render_star_charts`, no new free functions.

No new tests — example code, verified by running.

- [ ] **Step 1: Add Star Charts constants** — alongside
  `PSYCHIC_GLITCH_DURATION_MS`:

```rust
const TIMELINE: [&str; 5] = [
    "Draft proposal",
    "Review PR",
    "Deploy hotfix",
    "Write docs",
    "Plan sprint",
];
const TEMPORAL_SHIFT_MS: u64 = 400;
const CLOUD_GLYPHS: [char; 4] = ['?', '~', '·', '#'];
```

- [ ] **Step 2: Add fields to `Tardis`** — insert after
  `psychic_reveal`:

```rust
    psychic_reveal: Option<Transition>,
    present_index: usize,
    temporal_shift: Option<Transition>,
    quit: bool,
}
```

  and in `new()`:

```rust
            psychic_reveal: None,
            present_index: 2,
            temporal_shift: None,
            quit: false,
```

- [ ] **Step 3: Add `render_star_charts`** — add to `impl Tardis`:

```rust
    fn render_star_charts(&self, area: Rect, buf: &mut LayerStack) {
        if let Some(t) = &self.temporal_shift {
            if t.progress() < 0.3 {
                for y in 0..area.height {
                    for x in 0..area.width {
                        buf.set(
                            area.x + x,
                            area.y + y,
                            Cell {
                                symbol: ' ',
                                fg: Color::Reset,
                                bg: self.theme.accent,
                                ..Default::default()
                            },
                        );
                    }
                }
                return;
            }
        }

        for index in 0..TIMELINE.len() {
            let diff = (index + TIMELINE.len() - self.present_index) % TIMELINE.len();
            let row = index as u16;
            if row >= area.height {
                continue;
            }
            if diff == 0 {
                let pulse = ((self.tick_count as f32 * 0.1).sin() + 1.0) / 2.0;
                Roundel::new(pulse, self.theme.primary).render(
                    Rect {
                        x: area.x,
                        y: area.y + row,
                        width: 1,
                        height: 1,
                    },
                    buf,
                );
                let name_area = Rect {
                    x: area.x + 2,
                    y: area.y + row,
                    width: area.width.saturating_sub(2),
                    height: 1,
                };
                Text::new(TIMELINE[index]).render(name_area, buf);
            } else if diff == 3 || diff == 4 {
                let line = format!("◆ {}", TIMELINE[index]);
                for (i, ch) in line.chars().take(area.width as usize).enumerate() {
                    buf.set(
                        area.x + i as u16,
                        area.y + row,
                        Cell {
                            symbol: ch,
                            fg: self.theme.accent,
                            bg: Color::Reset,
                            ..Default::default()
                        },
                    );
                }
            } else {
                for col in 0..12u16.min(area.width) {
                    let h = (col as u64)
                        .wrapping_mul(374_761_393)
                        ^ (row as u64).wrapping_mul(668_265_263)
                        ^ self.tick_count.wrapping_mul(2_246_822_519);
                    let glyph = CLOUD_GLYPHS[(h % 4) as usize];
                    buf.set(
                        area.x + col,
                        area.y + row,
                        Cell {
                            symbol: glyph,
                            fg: self.theme.secondary,
                            bg: Color::Reset,
                            ..Default::default()
                        },
                    );
                }
            }
        }

        let hint_row = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(1),
            width: area.width,
            height: area.height.saturating_sub(1).min(1),
        };
        Text::new("Enter shift * Esc back * q quit").render(hint_row, buf);
    }
```

- [ ] **Step 4: Route `Screen::StarCharts` to it and delete
  `render_placeholder`** — in `render_destination_preview`, replace:

```rust
            Screen::StarCharts => self.render_placeholder(screen, local, &mut stack),
```

  with:

```rust
            Screen::StarCharts => self.render_star_charts(local, &mut stack),
```

  and in `view()`, replace:

```rust
            Screen::StarCharts => self.render_placeholder(self.screen, area, buf),
```

  with:

```rust
            Screen::StarCharts => self.render_star_charts(area, buf),
```

  Then delete the entire `render_placeholder` method — after this
  change nothing calls it. Run `cargo build --example tardis 2>&1` and
  confirm the only diagnostic (if any) is the expected
  `warning: method \`render_placeholder\` is never used` *before* you
  delete it; after deleting, the build should be warning-free.

- [ ] **Step 5: Add the Star Charts arm to `update()`** — replace:

```rust
            Screen::StarCharts => {
                if k.code == KeyCode::Esc {
                    self.screen = Screen::Hub;
                }
            }
```

  with:

```rust
            Screen::StarCharts => {
                if self.temporal_shift.is_some() {
                    return;
                }
                match k.code {
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        self.present_index = (self.present_index + 1) % TIMELINE.len();
                        self.temporal_shift =
                            Some(Transition::start(Duration::from_millis(TEMPORAL_SHIFT_MS)));
                    }
                    KeyCode::Esc => self.screen = Screen::Hub,
                    _ => {}
                }
            }
```

- [ ] **Step 6: Tick `temporal_shift` in `on_tick`** — append after the
  `psychic_reveal` tick block added in Task 1:

```rust
        if let Some(t) = &mut self.temporal_shift {
            t.tick(elapsed);
            if t.is_complete() {
                self.temporal_shift = None;
            }
        }
```

- [ ] **Step 7: Build**

Run: `cargo build --example tardis`
Expected: compiles cleanly, no warnings (confirms `render_placeholder`
was fully unused before deletion, not silently still needed somewhere).

- [ ] **Step 8: `cargo fmt && cargo clippy --all-targets -- -D warnings`**

Expected: clean, no warnings.

- [ ] **Step 9: Manual verification**

Run: `cargo run --example tardis`

Navigate to Star Charts and confirm:
- On entry: 2 amber `◆` past nodes, 1 pulsing-green present node
  (via `Roundel`), 2 rows of scattered `?`/`~`/`·`/`#` glyphs
  obscuring the future task names.
- `Enter`/`Space` flashes amber briefly, then the timeline has visibly
  advanced — the old present node is now amber, the next future node
  is now the pulsing present one, a new future node's name is now
  obscured.
- Repeating this several times cycles every one of the 5 tasks all the
  way around (future → present → past → future again), since the
  timeline is circular, not a one-shot list.
- Navigation is ignored during the flash.
- `Esc` returns to the Hub; `q` quits cleanly.

Also re-confirm Psychic Paper (Task 1) and Artron Energy still work
exactly as before — this task only touches `Screen::StarCharts`'s own
arms plus the now-fully-exhaustive dispatch matches.

- [ ] **Step 10: Commit**

```bash
git add examples/tardis.rs
git commit -m "feat(tardis): add Star Charts sub-app (#78)"
```

---

## Self-Review

**Spec coverage:** Psychic Paper (reversed paper palette, canned-prompt
send loop, ink-bleed reveal via `lerp_color`, deterministic every-3rd-
send Perception Filter break reusing `self.glitch`, new `"glitch"`
audio event) — Task 1. Star Charts (circular 5-slot timeline, past/
present/future rendering including the reused `Roundel` pulse and the
deterministic probability-cloud noise, Temporal Shift flash + reindex)
— Task 2. Verification section (`cargo test`/`fmt`/`clippy` + full
manual `cargo run --example tardis` walkthrough of both new screens
plus a regression check on the two screens built in the prior arc) —
covered across both tasks' final steps. The spec's explicitly-out-of-
scope list (real LLM integration, a growing task backlog, new
dependencies/core modules) — none added anywhere in this plan.

**Placeholder scan:** no TBD/TODO in code or commands. Step 4 of Task 2
includes an explicit instruction to confirm `render_placeholder`'s
unused-method warning appears *before* deletion (proving it's really
dead, not a copy-paste assumption) rather than asserting this as
certain.

**Type consistency:** `RelaySpeaker`, `PSYCHIC_PROMPTS`, `lerp_color`,
`render_ink_row` (Task 1) are used consistently within Task 1 and never
touched by Task 2. `TIMELINE`, `CLOUD_GLYPHS`, `present_index`,
`temporal_shift` (Task 2) are self-contained to Task 2. Both tasks'
`Screen::PsychicPaper`/`Screen::StarCharts` match-arm edits target
exactly the arm the *other* task leaves untouched — verified by reading
each `old_string` against the other task's final state before writing
these steps, not assumed.
