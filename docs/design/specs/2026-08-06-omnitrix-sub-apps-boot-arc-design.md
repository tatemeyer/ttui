# Omnitrix Widgets + Sub-Apps + Boot Arc — Design

**Status:** draft, pending your review.
**Date:** 2026-08-06
**Relationship to prior specs:** an Arc-level spec (per `docs/design/README.md`'s
Arc/Slice/Task structure) bundling the remaining Features+Polish tickets
from Arc 1 (Omnitrix, `2026-08-06-example-apps-roadmap-design.md`,
issues #46-51) into one design and one plan — the "widgets + sub-apps +
boot" completion of the Omnitrix example, on top of the dial-navigation
arc (`2026-08-06-omnitrix-dial-navigation-arc-design.md`, issues #43,
44, 45, 87, 88, all shipped). Reuses `camera::dim` (built for the TARDIS
arc, `2026-08-06-tardis-console-arc-design.md`) — the first cross-arc
reuse of that module outside `examples/tardis.rs`, confirming it really
is the generic "library-helper level" utility Rev B called for, not
TARDIS-specific machinery. Full creative source:
`TTUI-Ideas/vision/UI/idea-1-omnitrix.md`.

## Problem

`examples/omnitrix.rs` today has a working Faceplate hub (`Dial`
navigation, corruption transition) but its three destinations
(Brainstorm/Fasttrack/Upgrade) are still bare "(not yet built)"
placeholders, and two vision-doc widgets (`EnergyCore`, `DNAConsole`)
and the boot/intro splash don't exist. This spec builds all of it,
completing the Omnitrix example the way the Smash Crabs and TARDIS arcs
completed theirs.

## Scope

Six slices, ordered by dependency (widgets first, then example
integration — boot last, since its final phase reuses `Block`'s normal
rendering path which doesn't depend on anything else in this arc):

1. **`EnergyCore` widget** (`src/widgets/energy_core.rs`, #46) — new
   core widget: deterministic fill-bar with sparks at 100%.
2. **`DNAConsole` widget** (`src/widgets/dna_console.rs`, #47) — new
   core widget: alternating-color character display with a cursor
   glyph, *not* a real text editor (see Global Constraint below).
3. **Brainstorm sub-app** (`examples/omnitrix.rs`, #48) — canned-prompt
   send/thinking/reply loop, using `DNAConsole` for the prompt preview
   and a border-noise overlay (reusing the existing `braille_noise`
   free function already in this file) during "thinking."
4. **Fasttrack sub-app** (`examples/omnitrix.rs`, #49) — fixed target
   list, lock-on completion animation + flash, `EnergyCore` as the
   overall completion bar (substituting for the vision doc's "circular
   loading rings" — see Global Constraint).
5. **Upgrade sub-app** (`examples/omnitrix.rs`, #50) — two `EnergyCore`
   bars driven by a synthetic load value, outer border flashes red past
   90%.
6. **Boot/intro splash** (`examples/omnitrix.rs`, #51) — hourglass
   fade-in (via `camera::dim`) → bright flash → border trace-out (via
   `Block` rendered at a growing interpolated rect), gates initial
   entry. Depends on 1-5 only insofar as it reuses the file's existing
   `Block`/theme machinery — no new dependency on the sub-apps
   themselves.

**Explicitly out of scope:** real free-text input/editing for
`DNAConsole` (no cursor-movement/backspace capability exists anywhere
in this framework yet — building one is a separate, much larger
feature, not a widget-level concern); real system-metrics integration
for Upgrade's load value (synthetic and player-driven, same posture as
TARDIS's Artron Energy); a dedicated circular-ring widget for
Fasttrack's timers (reuses `EnergyCore` instead); any new dependency
(this arc adds no audio, no new crates — `rodio` stays scoped to
Smash Crabs/TARDIS unless a future ticket asks for Omnitrix audio
cues, which the roadmap doesn't list).

## Design

### Global constraint: `DNAConsole` is a styled preview, not a text editor

The vision doc's "Styled to look like alien DNA sequences are being
typed" reads as a live-typing text input. This framework has no
text-editing widget at all yet (no cursor position, no insert/delete)
— adding one is out of scope for a single widget in this arc.
`DNAConsole` instead renders a caller-supplied string with alternating
character colors and a trailing cursor glyph; Brainstorm uses it to
preview the *currently selected* canned prompt (cycled with `Tab`, the
same convention Faceplate already uses), not to accept live keystrokes.

### Global constraint: Fasttrack's "rings" are `EnergyCore`, not a new ring widget

The vision doc's circular loading rings would need their own point-on-
circle geometry (similar to `Dial`'s, but for a different purpose).
Since this arc already ships `EnergyCore` and the metaphor ("Omnitrix
recharging") fits a fill-bar just as well, Fasttrack's overall-progress
indicator reuses `EnergyCore` instead of adding a second geometry-heavy
widget for one screen.

### Slice 1: `EnergyCore` widget (`src/widgets/energy_core.rs`, #46)

```rust
pub struct EnergyCore {
    percent: u16,
    color: Color,
}

impl EnergyCore {
    pub fn new(percent: u16, color: Color) -> Self { ... }
    pub fn render(&self, area: Rect, buf: &mut Buffer) { ... }
}
```

`filled_width = area.width * percent.min(100) / 100` (integer math,
`percent` above 100 still renders a fully-filled bar, matching
`DamageMeter`'s precedent of accepting over-100 values without special
handling). Cells `0..filled_width` render `'▓'` in `color` (the fluid);
cells `filled_width..area.width` render `'░'` in `color` (the empty
track — dimmer glyph, not a color change, since `EnergyCore` has no
theme access to compute a genuinely dimmer shade). When `percent >=
100`, every 4th cell (`x % 4 == 0`) within the filled region additionally
renders as `'*'` in `Color::White` (sparks) — deterministic, not
RNG-driven, matching this codebase's established posture.

**Testing:** `coding`-tagged, TDD applies. `percent: 0` on a 10-wide
area renders all `'░'`. `percent: 50` on a 10-wide area renders 5
`'▓'` cells then 5 `'░'` cells. `percent: 100` on an 8-wide area
renders `'*'` at columns 0 and 4, `'▓'` elsewhere. A zero-width area
doesn't panic.

### Slice 2: `DNAConsole` widget (`src/widgets/dna_console.rs`, #47)

```rust
pub struct DNAConsole<'a> {
    content: &'a str,
    primary: Color,
    secondary: Color,
}

impl<'a> DNAConsole<'a> {
    pub fn new(content: &'a str, primary: Color, secondary: Color) -> Self { ... }
    pub fn render(&self, area: Rect, buf: &mut Buffer) { ... }
}
```

Renders up to `area.width - 1` characters of `content` (one column
reserved for the trailing cursor), alternating `fg` between `primary`
(even character index) and `secondary` (odd index) — the "double
helix" read. A cursor glyph (`'▌'`) in `primary` follows immediately
after the last rendered character, at column `content.chars().count()`
(clamped to fit).

**Testing:** `coding`-tagged, TDD applies. A short string on a wide
area alternates `primary`/`secondary` per character with the cursor
glyph immediately after. A zero-width area renders nothing without
panicking. A 1-wide area renders only the cursor glyph (no room for
content).

### Slice 3: Brainstorm sub-app (`examples/omnitrix.rs`, #48)

`Omnitrix` gains `chat_log: Vec<(ChatSpeaker, String)>` (starts empty),
`prompt_index: usize` (starts `0`), `thinking: Option<Transition>`
(starts `None`), where:

```rust
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
```

**Interaction:** `Tab`/`Shift+Tab` cycle `prompt_index` (only when
`thinking.is_none()`, matching Faceplate's own wraparound cycling).
`Enter`/`Space` (only when `thinking.is_none()`) push `(User,
CANNED_PROMPTS[prompt_index])` onto `chat_log` and start `thinking =
Some(Transition::start(1200ms))`. `Esc` returns to Faceplate, only when
`thinking.is_none()` — you can't back out mid-"processing," consistent
with every other busy-state gate in this codebase (Smash Crabs' cursor
tween, TARDIS's flight transition, etc.). `on_tick` ticks `thinking`
and, on completion, pushes `(Agent, format!("{prompt} ... complete."))`
and clears it.

**Rendering:** the conversation pane shows the last 5 `chat_log`
entries as `"You: ..."`/`"Agent: ..."` lines; `DNAConsole` previews
`CANNED_PROMPTS[prompt_index]`; a hint row. **Border effect while
thinking:** two things happen together, both driven by the existing
`pulse_phase`/`braille_noise` mechanisms already in this file (no new
animation primitive) — (1) `pulse_phase`'s increment rate triples
(`elapsed * PI * 3.0` instead of `* PI`) while `self.mode ==
AppMode::Brainstorm && self.thinking.is_some()`, making the border's
existing breathing pulse read as "rapid"; (2) a new
`overlay_border_noise` helper, called in `view()` right after
`Block::render()` (only under the same condition), sparsely overlays
`braille_noise` glyphs directly onto the border ring's cells (looping
the same edge positions `Block::render` draws, at roughly 1-in-5
density via `(x + tick_count) % 5 == 0`) — "flickering Braille patterns"
on top of the existing border, not a replacement for it.

**Testing:** example code, no `src/` changes in this slice — verified
by running, per the TDD exceptions in `development-conventions.md`.

### Slice 4: Fasttrack sub-app (`examples/omnitrix.rs`, #49)

`Omnitrix` gains `targets: Vec<(String, bool)>` (starts as 3 fixed
`(name, false)` pairs: `"Fix login bug"`, `"Write tests"`, `"Ship
release"`), `target_selected: usize` (starts `0`), `lock_on:
Option<(usize, Transition)>` (starts `None`), `complete_flash:
Option<Transition>` (starts `None`), where:

```rust
const LOCK_ON_MS: u64 = 600;
const COMPLETE_FLASH_MS: u64 = 300;
```

**Interaction:** `Tab`/`Shift+Tab` cycle `target_selected` through all
3 targets (wrapping, regardless of completion state). `Enter`/`Space`
(only when `lock_on.is_none()` and the selected target isn't already
complete) starts `lock_on = Some((target_selected, Transition::start
(600ms)))`. `Esc` returns to Faceplate, only when `lock_on.is_none()`.
`on_tick` ticks `lock_on` and, on completion, marks that target's
`bool` `true`, starts `complete_flash = Some(Transition::start(300ms))`,
and clears `lock_on`; separately ticks `complete_flash` to `None` on
its own completion.

**Rendering:** each target renders its bracket glyph by state —
incomplete and not locking: `"[ ]"`; mid-lock-on: `"[o]"` while
`lock_on`'s `progress() < 0.5`, `"[X]"` once `>= 0.5` (a 2-stage
lock-on sequence, ASCII-safe rather than the vision doc's Unicode
target-reticle glyphs, matching this codebase's established glyph-
width caution); already complete: `"[X]"` in a dimmed/secondary color.
The just-completed target's row background flashes (accent color) while
`complete_flash` is active. Below the list, `EnergyCore::new(completed_
count * 100 / 3, theme.primary)` renders the overall completion bar.

**Testing:** example code, no `src/` changes — verified by running,
including watching a full lock-on-to-completion cycle and the
completion bar advance.

### Slice 5: Upgrade sub-app (`examples/omnitrix.rs`, #50)

`Omnitrix` gains `load: f32` (starts `0.0`, not reset on mode
entry/exit — a persistent "system" value, same posture as TARDIS's
`energy`), where:

```rust
const UPGRADE_LOAD_GAIN: f32 = 15.0;
const UPGRADE_LOAD_DECAY_PER_SEC: f32 = 3.0;
const OVERLOAD_THRESHOLD: f32 = 90.0;
```

**Interaction:** `Space` increases `load` by `15.0` (uncapped — can
exceed 100, same as `EnergyCore`'s accepted range); `Esc` returns to
Faceplate (no busy-state gate needed here, there's no in-flight
animation to protect). `on_tick` decays `load` by `3.0` per second
(`3.0 * elapsed.as_secs_f32()`, floored at `0.0`) regardless of mode,
same "ticks everywhere" posture as TARDIS's `energy`.

**Rendering:** two `EnergyCore` bars — "CPU" using `load` directly,
"RAM" using `(load * 0.6 + 10.0).min(100.0)` (a derived second value
for visual variety without tracking two independent synthetic
resources). **Overload effect:** when `self.mode == AppMode::Upgrade &&
self.load >= 90.0`, `theme()`'s computed `primary` color is overridden
to alternate between `Color::Red` and the normal breathing-pulse green
by `tick_count` parity (a flash, not a solid color swap) — since
`theme()` already governs the single outer `Block`'s border color for
every mode, this is the only place the "UI edges flash warning red"
effect needs to be wired, with no change to `Block` itself.

**Testing:** example code, no `src/` changes — verified by running,
including holding Space until the border visibly starts flashing red.

### Slice 6: Boot/intro splash (`examples/omnitrix.rs`, #51)

`Omnitrix` gains `booting: Option<Transition>`, started immediately in
`new()` (**2500ms** — shorter than TARDIS's 3000ms, matching the vision
doc's snappier "sudden flash" versus TARDIS's more gradual
materialization). Gates
`update()` (all input ignored except `q`) and is checked first in
`view()`, same shape as TARDIS's `booting` field. Three phases by
`progress`:
- **Hourglass fade-in** (`[0.0, 0.4)`): a small fixed 4x4 ASCII
  hourglass (`["/--\\", "\\  /", "/  \\", "\\__/"]`) rendered into a
  scratch `Buffer` in `theme().primary`, then `camera::dim`'d by
  `factor = 1.0 - progress / 0.4` (starts fully dimmed/black at
  `progress: 0`, reaches full brightness at `progress: 0.4`) and
  blitted centered on an otherwise-black screen — the first reuse of
  `camera::dim` outside `examples/tardis.rs`.
- **Flash** (`[0.4, 0.55)`): solid fill in a fixed bright green
  (`Color::Rgb { r: 0, g: 255, b: 65 }`, the vision doc's `#00FF41`),
  not the breathing-pulse color — a punchy, non-pulsing flash.
- **Border trace-out** (`[0.55, 1.0]`): remap to `trace_progress =
  (progress - 0.55) / 0.45`; `scale = easing::ease_out(0.2, 1.0,
  trace_progress)`; render `Block` (title "Omnitrix", normal `theme()`)
  at a `Rect` centered in `area` and scaled to `scale` of `area`'s
  width/height (minimum `2x2`) — growing from a small centered box to
  the full frame approximates "circuit-board lines tracing outward
  from the center" without needing per-perimeter-cell sequencing.
  At `progress: 1.0`, the scaled rect equals `area` exactly, hand-off
  to normal post-boot rendering is seamless.

**Testing:** example code, no `src/` changes — verified by running,
watching the full ~2.5s sequence play once at startup before Faceplate
becomes interactive.

## Verification

- `cargo test --lib`, `cargo fmt --check`, `cargo clippy --all-targets
  -- -D warnings` all green (Slices 1-2 have real unit tests; Slices
  3-6 are example code).
- `cargo run --example omnitrix`: confirm the boot sequence plays once
  (hourglass fade-in → green flash → border growing outward) before
  Faceplate becomes interactive; confirm Brainstorm's send/thinking/
  reply loop, including the tripled pulse rate and Braille noise on the
  border while thinking; confirm Fasttrack's lock-on-to-completion
  animation, the row flash, and the `EnergyCore` completion bar
  advancing; confirm Upgrade's two `EnergyCore` bars respond to Space
  and the border flashes red past 90%; confirm `Esc` returns to
  Faceplate from all three (blocked mid-"thinking"/mid-"lock-on" where
  applicable) and the existing corruption transition still plays on
  every mode switch; confirm `q` quits cleanly from every state,
  including mid-boot, with no leftover terminal attributes.
