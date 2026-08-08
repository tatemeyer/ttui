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
   send/thinking/reply loop, `DNAConsole` typewriter-revealing the
   selected prompt, and a border-noise overlay (reusing the existing
   `braille_noise` free function already in this file) during
   "thinking."
4. **Fasttrack sub-app** (`examples/omnitrix.rs`, #49) — fixed target
   list split into active/Completed sections, a real point-on-circle
   lock-on ring animation (not a bracket-glyph placeholder), and
   `EnergyCore` as the overall completion summary.
5. **Upgrade sub-app** (`examples/omnitrix.rs`, #50) — CPU/RAM rendered
   as literal lit/unlit circuit-node chains (**not** `EnergyCore` bars
   — the vision doc is explicit that this screen "isn't a bar, it's a
   circuit"), outer border flashes red past 90%.
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
TARDIS's Artron Energy); any new dependency (this arc adds no audio, no
new crates — `rodio` stays scoped to Smash Crabs/TARDIS unless a future
ticket asks for Omnitrix audio cues, which the roadmap doesn't list).

## Design

### Global constraint: `DNAConsole` stays a pure `render(value)` widget — Brainstorm owns the "typing" animation

The vision doc's "Styled to look like alien DNA sequences are being
typed" reads as a live-typing text input. This framework has no
text-editing widget at all yet (no cursor position, no insert/delete)
— building one is out of scope for a single widget in this arc.
Instead, `DNAConsole` keeps the same "dumb, caller-owns-timing" contract
every widget in this codebase already follows (`DamageMeter`,
`Roundel`, etc.): it renders whatever string it's given, alternating
colors per character, plus a trailing cursor. Brainstorm gets the
*feel* of typing by computing a growing substring itself (see Slice 3)
and re-rendering `DNAConsole` with more of the prompt visible each
frame — the widget never needs to know about time at all.

### Global constraint: geometric-shape and Dingbat glyphs are used deliberately here, with their width caveat noted once

This arc uses `◉`/`○`/`●` (Geometric Shapes block) and `✦` (Dingbats)
for the lock-on ring, the circuit chains, and `EnergyCore`'s sparks —
closer to the vision doc's actual glyphs than this codebase's earlier
default-to-ASCII caution (`ScuttleCursor`'s `'C'`, `Roundel`'s `'O'`).
The risk category is different: crab/emoji-presentation glyphs are
*definitionally* double-width per Unicode's East Asian Width tables,
which is why `ScuttleCursor` avoided one outright; these are classified
"Ambiguous" width — narrow in the overwhelming majority of terminal
configurations (this is the same glyph family widely used by real
terminal UIs like `lazygit`/`k9s`), but not narrow by *guarantee* the
way ASCII or the Block Elements/Box Drawing ranges are. Noted once
here rather than re-litigated per widget.

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
renders as `'✦'` (the vision doc's literal spark glyph) in
`Color::White` — deterministic, not RNG-driven, matching this
codebase's established posture.

**Testing:** `coding`-tagged, TDD applies. `percent: 0` on a 10-wide
area renders all `'░'`. `percent: 50` on a 10-wide area renders 5
`'▓'` cells then 5 `'░'` cells. `percent: 100` on an 8-wide area
renders `'✦'` at columns 0 and 4, `'▓'` elsewhere. A zero-width area
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
(starts `None`), `preview_reveal: Transition` (starts already-running,
`Transition::start(400ms)`, so the first prompt types itself out
immediately on entering Brainstorm), where:

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
const PREVIEW_REVEAL_MS: u64 = 400;
```

All three `CANNED_PROMPTS` are plain ASCII (1 byte per character), so
slicing them by character count is also a valid byte-slice — no
`char_indices` bookkeeping needed for the reveal math below.

**Interaction:** `Tab`/`Shift+Tab` cycle `prompt_index` (only when
`thinking.is_none()`, matching Faceplate's own wraparound cycling) and
restart `preview_reveal = Transition::start(400ms)` — each newly-
selected prompt types itself out again. `Enter`/`Space` (only when
`thinking.is_none()`) push `(User, CANNED_PROMPTS[prompt_index])` onto
`chat_log` and start `thinking = Some(Transition::start(1200ms))`.
`Esc` returns to Faceplate, only when `thinking.is_none()` — you can't
back out mid-"processing," consistent with every other busy-state gate
in this codebase (Smash Crabs' cursor tween, TARDIS's flight
transition, etc.). `on_tick` ticks `thinking` and, on completion,
pushes `(Agent, format!("{prompt} ... complete."))` and clears it; also
ticks `preview_reveal` (it simply stays complete/inert once finished
until the next `Tab` restarts it).

**Rendering:** the conversation pane shows the last 5 `chat_log`
entries as `"You: ..."`/`"Agent: ..."` lines; `DNAConsole` renders a
*prefix* of `CANNED_PROMPTS[prompt_index]` — `reveal_len =
(prompt.chars().count() as f32 * preview_reveal.progress()) as usize`,
sliced as `&prompt[..reveal_len]` — growing from nothing to the full
prompt over the 400ms reveal, the actual "being typed" motion the
vision doc describes, entirely driven by Brainstorm's own state, not
by any new capability inside `DNAConsole` itself; a hint row.
**Border effect while thinking:** two things happen together, both driven by the existing
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
release"`), `target_selected: usize` (starts `0`, indexes into the
*active* — not-yet-complete — subset, see Rendering below), `lock_on:
Option<(usize, Transition)>` (starts `None`, the `usize` is a
`targets` index), `complete_flash: Option<Transition>` (starts `None`),
where:

```rust
const LOCK_ON_MS: u64 = 900;
const COMPLETE_FLASH_MS: u64 = 300;
const RING_POINTS: usize = 8;
```

**Interaction:** `Tab`/`Shift+Tab` cycle `target_selected` through the
*active* (incomplete) targets only, wrapping — a target that's already
complete has moved to the Completed section and is no longer part of
this cycle (see Rendering). `Enter`/`Space` (only when
`lock_on.is_none()`) starts `lock_on = Some((active_target_index,
Transition::start(900ms)))`. `Esc` returns to Faceplate, only when
`lock_on.is_none()`. `on_tick` ticks `lock_on` and, on completion,
marks that target's `bool` `true` (moving it into the Completed
section on the next render), starts `complete_flash =
Some(Transition::start(300ms))`, resets `target_selected` to `0`
(the active list just shrank), and clears `lock_on`; separately ticks
`complete_flash` to `None` on its own completion.

**Rendering — real lock-on ring, not a bracket-glyph placeholder:**
the vision doc's `[ ] → [◎] → [◉]` sequence and its separate "circular
loading rings" description are really the same idea, so this slice
builds one mechanic for both instead of two. A small local helper (not
a new core widget — this is one screen's decoration, same precedent as
Slice 6's hourglass and the existing `overlay_border_noise`/
`braille_noise` helpers already in this file) places `RING_POINTS = 8`
points around a center, reusing the same point-on-circle formula shape
`Dial` uses internally (`angle = i * TAU / 8 - FRAC_PI_2`, `radius_x =
4.0`, `radius_y = 2.0` for the same terminal-cell aspect correction
Dial applies) — written fresh in `examples/omnitrix.rs` since `Dial`'s
ring-drawing isn't exposed as a reusable function, not by modifying the
shipped `Dial` widget. While a target's `lock_on` is active, `lit_count
= (lock_on.1.progress() * 8.0) as usize` points (in ring order) render
as `'●'` in `theme().primary`; the rest render as `'○'` in
`theme().secondary` — the ring visibly fills clockwise from empty
(`progress: 0`, all `○`) toward a completed circle. The ring occupies
a dedicated area below the active-targets list (9 wide, 5 tall to fit
`radius_x: 4` / `radius_y: 2`), visible only while `lock_on.is_some()`.

**Rendering — two sections:** a "Targets" section lists the active
(incomplete) targets, each as `"{marker} {name}"` where `marker` is
`'○'` normally or highlighted (`List`'s black-on-white convention) for
the currently-selected one; a "Completed" section below it lists
completed targets as `"◉ {name}"` in a dimmed/secondary color — a
target visually *moves* from one section to the other the render right
after `lock_on` finishes, matching the vision doc's "moving the item to
a Completed/Archived list" literally rather than flipping a marker in
place. The just-completed target's row flashes (accent-colored
background) for the `complete_flash` window right after it lands in
the Completed section. Below both sections, `EnergyCore::new(completed_
count * 100 / 3, theme().primary)` renders the overall completion
summary (a generic aggregate-progress use, which the vision doc doesn't
single out the way it does Upgrade's CPU/RAM — see the Upgrade slice).

**Testing:** example code, no `src/` changes — verified by running,
including watching the ring fill clockwise to completion, the target
moving from Targets to Completed, and the completion bar advancing.

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

**Rendering — a circuit, not a bar:** the vision doc is explicit that
this screen's meters are "not a bar; it's a circuit lighting up," so
this slice does not reuse `EnergyCore` here (it's reserved for
Fasttrack's generic aggregate, which the vision doc doesn't call out
the same way). A local helper (same "one screen's decoration, not a
new core widget" precedent as Slice 4's ring) renders a horizontal
chain of `NODE_COUNT = 6` alternating node/trace glyphs — `'●'` (lit,
`theme().primary`) or `'○'` (unlit, `theme().secondary`) nodes joined
by `'─'` trace segments (`theme().secondary`), e.g. `"●─●─●─○─○─○"` at
50%. `lit_count = ((value.min(100.0) / 100.0) * 6.0) as u16` per row —
"CPU" using `load` directly, "RAM" using `(load * 0.6 + 10.0).min
(100.0)` (a derived second value for visual variety without tracking
two independent synthetic resources). **Overload effect:** when
`self.mode == AppMode::Upgrade && self.load >= 90.0`, `theme()`'s
computed `primary` color is overridden to alternate between
`Color::Red` and the normal breathing-pulse green by `tick_count`
parity (a flash, not a solid color swap) — since `theme()` already
governs the single outer `Block`'s border color for every mode, this
is the only place the "UI edges flash warning red" effect needs to be
wired, with no change to `Block` itself. Past the threshold, the
circuit's own lit nodes render in this same alternating red/green
color too (they read `theme().primary`), so the overload reads
consistently across the whole screen, not just the border.

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
- **Hourglass fade-in** (`[0.0, 0.4)`): a small fixed 5x5 hourglass
  silhouette — `["┌───┐", " \ / ", "  X  ", " / \ ", "└───┘"]` (box-
  drawing corners for the frame, plain ASCII for the diagonals and
  center pinch — a clearer recognizable hourglass shape than a bare
  ASCII blob, built entirely from glyphs with *guaranteed* single-cell
  width, unlike the geometric-shape/Dingbat glyphs used elsewhere in
  this arc) rendered into a scratch `Buffer` in `theme().primary`, then
  `camera::dim`'d by
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
  Faceplate becomes interactive; confirm Brainstorm's typewriter-
  revealed prompt preview, the send/thinking/reply loop, the tripled
  pulse rate, and the Braille noise on the border while thinking;
  confirm Fasttrack's lock-on ring fills clockwise to completion, the
  target moves from Targets to Completed with a flash, and the
  `EnergyCore` completion bar advances; confirm Upgrade's two circuit
  chains light up left-to-right as `load` climbs and both the border
  and the lit nodes flash red past 90%; confirm `Esc` returns to
  Faceplate from all three (blocked mid-"thinking"/mid-lock-on where
  applicable) and the existing corruption transition still plays on
  every mode switch; confirm `q` quits cleanly from every state,
  including mid-boot, with no leftover terminal attributes.
