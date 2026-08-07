# TARDIS Remaining Sub-Apps — Design

**Status:** draft, pending your review.
**Date:** 2026-08-06
**Relationship to prior specs:** completes Arc 3 (TARDIS,
`2026-08-06-example-apps-roadmap-design.md`, issues #77-78) on top of
`2026-08-06-tardis-console-arc-design.md` (Camera/`GlitchBuffer`/hex
console/boot/Artron Energy, all shipped). No new core modules or
widgets this round — the smallest of the arcs this session, entirely
reusing machinery already built: `GlitchBuffer` (shared with Artron
Energy's lag state), `Transition`-driven flash/reveal timing (the same
shape as every other busy-state in this codebase), the deterministic
hash-noise technique (`braille_noise`/`GlitchBuffer`'s glyph choice),
`Roundel` (reused for Star Charts' pulsing present node), and the
existing `RodioAudioSink` (gains one more event). Full creative source:
`TTUI-Ideas/vision/UI/idea-3-TardisTUI.md`.

## Problem

Of TARDIS's three named console faces, only Artron Energy has real
content — Psychic Paper and Star Charts are still `render_placeholder`
stubs. This spec builds both, completing TARDIS the way the other two
example apps were completed earlier this session.

## Scope

Two slices, both `examples/tardis.rs`, independent of each other (both
only touch their own `Screen` arms and add their own fields — neither
reads or writes the other's state):

1. **Psychic Paper (Agent Interface)** (#77) — canned-prompt send loop
   on a reversed "paper" palette, replies that genuinely ink-bleed in
   via color fade, and a deterministic "Perception Filter" glitch break
   on every 3rd send.
2. **Star Charts (Productivity)** (#78) — a 5-slot circular timeline
   (2 past / 1 present / 2 future), past nodes fixed amber, the present
   node pulsing via a reused `Roundel`, future nodes obscured by a
   deterministic "probability cloud" of scattered glyphs, and a
   "Temporal Shift" flash + reindex on completing the present task.

**Explicitly out of scope:** real LLM integration for Psychic Paper
(canned prompts/replies, same posture as Omnitrix's Brainstorm); a
persistent/growing task backlog for Star Charts (a fixed 5-slot
circular timeline, not an open list); any new dependency or core
module — both slices are pure `examples/tardis.rs` additions.

## Design

### Slice 1: Psychic Paper (`examples/tardis.rs`, #77)

```rust
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
const PAPER_COLOR: Color = Color::Rgb { r: 230, g: 225, b: 210 };
const INK_COLOR: Color = Color::Rgb { r: 20, g: 20, b: 40 };
```

`Tardis` gains `psychic_log: Vec<(RelaySpeaker, String)>` (starts
empty), `psychic_prompt_index: usize` (starts `0`), `psychic_send_count:
u32` (starts `0`), `psychic_pending: Option<(bool, Transition)>` (starts
`None` — the `bool`, fixed at send time, is whether *this* pending
reply will glitch), `psychic_reveal: Option<Transition>` (starts
`None`).

**Interaction:** `Tab`/`Shift+Tab` cycle `psychic_prompt_index` (only
when `psychic_pending.is_none()`). `Enter`/`Space` (only when
`psychic_pending.is_none()`) push `(User,
PSYCHIC_PROMPTS[psychic_prompt_index])` onto `psychic_log`, increment
`psychic_send_count`, and start `psychic_pending = Some((psychic_send_
count % PSYCHIC_GLITCH_EVERY == 0, Transition::start(800ms)))` — every
3rd send is decided right here, deterministically, not by chance.
`Esc` returns to the Hub, only when `psychic_pending.is_none()`.
`on_tick` ticks `psychic_pending` and, on completion: if the stored
`bool` is `true`, pushes `(Agent, "...signal lost...")` and triggers
`self.glitch` (the *same* `GlitchBuffer` field Artron Energy already
uses for its lag state — mutually exclusive in time since only one
screen is ever active, so sharing it is safe, not a new field) for
`600ms`; otherwise pushes `(Agent, format!("{prompt} — relay
confirmed."))` and starts `psychic_reveal = Some(Transition::start
(800ms))`. Separately ticks `psychic_reveal` to `None` on its own
completion.

**Rendering — reversed palette:** unlike every other TARDIS screen
(deep space black), Psychic Paper fills its background with
`PAPER_COLOR` — "a shimmering, slightly translucent white" per the
vision doc, a deliberate one-screen reversal.

**Rendering — ink bleed:** the log shows the last 5 `psychic_log`
entries. `User` lines render directly in `INK_COLOR` (your own thought
appears instantly). The *most recent* `Agent` line, while
`psychic_reveal` is active, renders in `lerp_color(PAPER_COLOR,
INK_COLOR, psychic_reveal.progress())` — starting invisible against the
paper and resolving to sharp dark text over 800ms, literally "bleeding
through" as the vision doc describes; once `psychic_reveal` completes
(or for any older `Agent` line), it renders fully in `INK_COLOR`. A new
small local free function, `lerp_color(from, to, t) -> Color`,
Rgb-only (same width/color-model constraints as every other brightness-
driven effect in this codebase) — distinct from `camera::dim`, which
only fades toward black, not an arbitrary target color, so it doesn't
fit this reversed-palette use.

**Rendering — Perception Filter break:** if the most recent log entry
is the glitch reply, `self.glitch` (still decaying from its 600ms
trigger) overlays red noise across that line — the *same* call shape
Artron Energy already uses (`self.glitch.render(area, Color::Red,
tick_count, buf)`), just a different area and a different trigger
condition. No ink-bleed reveal plays for a glitch reply — the broken
connection never resolves to clean text, matching "the text glitches...
mimicking a failed psychic connection."

**Testing:** example code, no `src/` changes — verified by running.

### Slice 2: Star Charts (`examples/tardis.rs`, #78)

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

`Tardis` gains `present_index: usize` (starts `2` — so slots `0`/`1`
start as "past" and `3`/`4` as "future," matching the vision doc's 2/1/2
split immediately on first entry) and `temporal_shift: Option<Transition>`
(starts `None`).

**Circular status, not a growing/shrinking list:** `TIMELINE` is a
fixed 5-entry array; the timeline never gains or loses tasks. Each
slot's status is computed from its circular distance behind
`present_index`: `let diff = (index + 5 - self.present_index) % 5;`
— `diff == 0` is present, `diff` in `{1, 2}` is future, `diff` in
`{3, 4}` is past. As `present_index` advances, the same 5 slots
continuously cycle through past → present → future → ... → past again,
giving an unending demo loop rather than a one-shot list that runs out
(consistent with Artron Energy's `energy` also being a persistent,
never-reset value).

**Interaction:** `Enter`/`Space` (only when `temporal_shift.is_none()`)
advance `present_index = (present_index + 1) % 5` *immediately* (state
updates right away, same "update state now, animate the visual
separately" approach Fasttrack used in the Omnitrix arc) and start
`temporal_shift = Some(Transition::start(400ms))`. `Esc` returns to the
Hub, only when `temporal_shift.is_none()`. `on_tick` ticks
`temporal_shift` to `None` on completion — a pure visual-flash timer,
nothing else depends on its value once it clears.

**Rendering — three node types:**
- **Past** (`diff` 3 or 4): `"◆ {name}"` in `theme.accent` (Amber,
  already TARDIS's palette for "architecture/caution" elements) —
  fixed, no animation, "past tasks are fixed, glowing amber nodes."
- **Present** (`diff == 0`): a reused `Roundel` widget rendered just
  before the name, `intensity` driven by the same ambient sine pulse
  the Hub's decorative Roundels already use
  (`((tick_count as f32 * 0.1).sin() + 1.0) / 2.0`), color
  `theme.primary` (Temporal Green) — "present tasks pulse green,"
  achieved by reusing an existing widget rather than inventing new
  pulse-glyph logic.
- **Future** (`diff` 1 or 2): the task's real name is *not* shown — a
  12-glyph "probability cloud" renders instead, each glyph chosen from
  `CLOUD_GLYPHS` via `hash(x, row, tick_count) % 4` (same deterministic-
  hash shape as `braille_noise`/`GlitchBuffer`, no RNG), in a dimmed
  (`theme.secondary`) color — "the future isn't decided," represented
  by literally obscuring the future task's name, not just styling it
  differently.

**Rendering — Temporal Shift:** while `temporal_shift.is_some()` and
its `progress() < 0.3`, the whole area renders a solid `theme.accent`
flash instead of the timeline (matching the vision doc's "a screen
flash"); once `progress() >= 0.3`, the timeline renders normally — by
this point `present_index` has already advanced (it updated
synchronously at the `Enter` press), so what's revealed after the flash
is already the shifted state, reading as "a visual snap of the timeline
moving forward."

**Testing:** example code, no `src/` changes — verified by running,
including watching several Temporal Shifts cycle a task all the way
from future through present into past and back around.

### Audio: one more event

`RodioAudioSink::play` gains a fourth `event_id` match arm,
`"glitch"`, at a distinct frequency from the existing `"boot"`/
`"flight"`/`"vent"` tones — played whenever `self.glitch.trigger(...)`
fires for Psychic Paper's Perception Filter break (Artron Energy's own
lag-triggered `self.glitch.trigger` calls stay silent, matching its
existing behavior — this is additive, not a change to when Artron
Energy's glitch fires or sounds).

## Verification

- `cargo test --lib`, `cargo fmt --check`, `cargo clippy --all-targets
  -- -D warnings` all green (no `src/` changes this arc — both slices
  are example code).
- `cargo run --example tardis`: confirm Psychic Paper's paper-white
  background, a sent prompt's reply visibly bleeding from invisible to
  sharp dark ink over ~800ms; confirm every 3rd send instead shows
  "...signal lost..." with red glitch noise over it and an audible
  distinct tone; confirm Star Charts shows 2 amber past nodes, 1
  pulsing-green present node, and 2 obscured future nodes on entry;
  confirm Enter flashes and advances the present task, and repeating it
  cycles every task through future → present → past → future again;
  confirm `Esc` returns to the Hub from both screens (blocked mid-
  pending/mid-shift where applicable) and `q` quits cleanly from every
  state.
