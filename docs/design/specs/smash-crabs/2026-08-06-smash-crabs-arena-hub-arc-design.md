# Smash Crabs Arena + Hub Arc — Design

**Status:** draft, pending your review.
**Date:** 2026-08-06
**Relationship to prior specs:** an Arc-level spec (per `docs/design/README.md`'s
Arc/Slice/Task structure) bundling several roadmap tickets from Arc 2
(Smash Crabs, `2026-08-06-example-apps-roadmap-design.md`) into one
design and one plan: the Core "wire Arc 0 primitives into the arena"
ticket, the Architectural `ScuttleCursor` tween/screen-shake/VS-transition
tickets, and the Features+Polish `DamageMeter`/`SmashBorder` widget
tickets and audio-cues ticket. Builds on Arc 0's shared capabilities
(`easing.rs`, `particles.rs`, `effects.rs::shake`, `transition.rs`,
`audio.rs` — all shipped, PR #84) and the existing
`examples/smash_crabs.rs` 3-layer arena (PR #27/#32). Full creative
source: `TTUI-Ideas/vision/UI/idea-2-SuperSmashCrabs.md`.

## Problem

`examples/smash_crabs.rs` today is a bare-bones demo: one arena screen,
pressing Space instantly subtracts 10 from a plain "P2: N HP" text row
and shows a static yellow flash for a few ticks. None of Arc 0's shipped
primitives (easing, particles, screen-shake, transition, audio hook) are
wired into it, and none of the vision doc's named components
(`ScuttleCursor`, `DamageMeter`, `SmashBorder`, character-select hub, VS
transition) exist yet. This spec turns that into the vertical slice the
roadmap calls for: a real two-screen app (character-select hub → arena)
with tweened, particle-bursting, screen-shaking, audio-cued combat
feedback.

## Scope

Seven slices, ordered by dependency (core widgets first, then example
integration in dependency order):

1. **`ScuttleCursor` widget** (`src/widgets/scuttle_cursor.rs`) — new
   core widget: jerky two-frame cursor rendering.
2. **`DamageMeter` widget** (`src/widgets/damage_meter.rs`) — new core
   widget: colored percent display.
3. **`SmashBorder` widget** (`src/widgets/smash_border.rs`) — new core
   widget: 3-ring beveled border, drawn inward (unlike Omnitrix's
   outward `border_thick`).
4. **Hub screen** (`examples/smash_crabs.rs`) — `Screen` enum,
   3-portrait grid, `ScuttleCursor`-driven navigation. Depends on 1.
5. **VS transition** (`examples/smash_crabs.rs`) — black+"VS" flash,
   then expanding-circle wipe into the destination screen, via Arc 0's
   `Transition`. Depends on 4.
6. **Arena polish** (`examples/smash_crabs.rs`) — damage-percent tween,
   particle burst, screen-shake, `DamageMeter`/`SmashBorder` wiring.
   Depends on 2, 3, 5.
7. **Audio cues** (`examples/smash_crabs.rs`, `Cargo.toml`) — a
   `rodio`-backed `AudioSink` impl, cursor/select/hit tones. Depends on
   4 and 6 for its call sites.

**Explicitly out of scope** (same YAGNI posture as the Omnitrix arc):
real Target Smash / Stage Hazards sub-app content (they stay named
placeholder screens — "3-slot grid, 1 real" per your call); a full
4x3/5x4 portrait grid (3 slots only — there are only 3 named
destinations); bundled audio asset files (procedural tones only, no
external `.wav`/`.ogg` sourcing); boot/intro splash sequence; P1 taking
damage (the existing Space-bar interaction is one-directional, P1 hits
P2 — expanding to real two-player input is a separate concern); any
change to `Theme`/`Block`/`border_thick` (this arc's border work is a
wholly separate `SmashBorder` widget, not a `Theme` extension).

## Design

### Slice 1: `ScuttleCursor` widget (`src/widgets/scuttle_cursor.rs`)

```rust
pub struct ScuttleCursor {
    symbol: char,
}

impl ScuttleCursor {
    pub fn new(symbol: char) -> Self { ... }
    pub fn render(&self, x: f32, y: f32, moving: bool, tick_count: u64, buf: &mut Buffer) { ... }
}
```

No `Theme` parameter, matching every widget except `Block`. Takes an
already-tweened float position — the caller owns the tween (via
Arc 0's `Transition` + `easing::ease_out`, same pattern the app already
proved out for Omnitrix's mode-switch and this arc's damage/cursor
tweens), the widget only renders a snapshot.

**Jerk rendering:** `symbol` must be a single-width character — real
crab emoji (🦀) are double-width in most terminals and would break the
single-cell coordinate math every widget in this codebase assumes, so
the vision doc's emoji cursor is deliberately **not** used; the example
substitutes a plain ASCII glyph (`'C'`, colored via `theme.accent`).
When `moving` is `true`, the rendered column is offset by `-1` on even
`tick_count` and `+1` on odd `tick_count` (the vision doc's "quick,
two-frame jerky animation... shifting left/right by one cell on
alternate ticks"); when `moving` is `false`, no offset. The base
position is `(x.round(), y.round())` before the jerk offset. Any cell
that lands outside `buf`'s bounds (including via the jerk push) is
silently skipped — no panic.

**Testing:** `coding`-tagged, TDD applies.
- idle (`moving: false`) at a fixed position renders at exactly the
  rounded position, unaffected by `tick_count`.
- `moving: true` at even `tick_count` renders one column left of the
  rounded position.
- `moving: true` at odd `tick_count` renders one column right of the
  rounded position.
- a position whose jerked column falls outside the buffer does not
  panic and does not corrupt an adjacent in-bounds cell.

### Slice 2: `DamageMeter` widget (`src/widgets/damage_meter.rs`)

```rust
pub struct DamageMeter {
    percent: u16,
}

impl DamageMeter {
    pub fn new(percent: u16) -> Self { ... }
    pub fn render(&self, area: Rect, buf: &mut Buffer) { ... }
}
```

Replaces the arena's plain "P2: N HP" text row with genuine Smash-Bros-
style damage: counts **up** from 0% (not down from 100), can exceed
100%, colored by fixed thresholds: `< 50` → `Color::White`, `50..100`
→ `Color::Yellow`, `>= 100` → `Color::Red` (the vision doc's "white to
yellow to red as limits are reached"). Renders `"{percent}%"`
left-aligned at `area`'s origin, clipped to `area.width` — same
truncation convention as `Text`. The widget renders a single already-
computed value; the caller owns the count-up animation (again via
`Transition` + `easing::ease_out`).

**Testing:** `coding`-tagged, TDD applies.
- `percent: 0` renders `"0%"` in `Color::White`.
- `percent: 50` renders in `Color::Yellow` (boundary is inclusive on
  the yellow side).
- `percent: 137` renders `"137%"` in `Color::Red`.
- a value whose text is wider than `area.width` clips instead of
  panicking (same pattern as `Text`'s `truncates_content_wider_than_the_area`
  test).

### Slice 3: `SmashBorder` widget (`src/widgets/smash_border.rs`)

```rust
pub struct SmashBorder;

impl SmashBorder {
    pub fn new() -> Self { ... }
    pub fn render(&self, area: Rect, theme: &Theme, buf: &mut Buffer) -> Rect { ... }
}
```

Takes `&Theme` (matching `Block`'s precedent — the only other widget
that does), since its 3 rings are colored from the theme rather than
hardcoded. Unlike Omnitrix's `border_thick` (draws outward from `area`,
requires caller-provided margin), `SmashBorder` draws all 3 rings
**inward** from `area` and returns the shrunk inner content `Rect` — a
drop-in `Block` replacement with no margin caveat, used for both the
Hub and Arena screens for consistent "toy-box" branding.

Three concentric rings, drawn outer to inner, each shrinking the next
ring's area by 1 cell per side (same per-edge/corner cell-setting shape
as `Block::render`'s ring loop, reused per ring instead of once):

| Ring | Glyphs (h / v / corner) | Color |
|---|---|---|
| outer | `'#'` / `'#'` / `'#'` | `theme.accent` |
| middle | `theme.border.horizontal` / `.vertical` / `.corner` | `theme.primary` |
| inner | `'-'` / `':'` / `'.'` | `theme.tertiary` |

If `area` is too small to fit all 3 rings (each ring needs the
remaining area to be at least 2x2, same guard as `Block::render`),
render stops after the last ring that fit — no panic, degrades
gracefully. The returned inner `Rect` reflects however many rings
actually got drawn (i.e. `area` shrunk by 1 per side per ring that fit,
up to 3).

**Testing:** `coding`-tagged, TDD applies.
- a large-enough area (e.g. 12x10): outer ring cell is `'#'` in
  `theme.accent`, the cell one step inward is `theme.border.horizontal`
  in `theme.primary`, the cell two steps inward is `'-'` in
  `theme.tertiary`.
- the returned inner `Rect` equals `area` shrunk by 3 on every side
  when all 3 rings fit.
- a too-small area (e.g. 3x3, fits 1 ring at most) does not panic and
  returns a sensible (possibly zero-size) inner `Rect`.

### Slice 4: Hub screen (`examples/smash_crabs.rs`)

```rust
#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Hub,
    Versus,
    TargetSmash,
    StageHazards,
}

const FIGHTERS: [&str; 3] = ["Versus Mode", "Target Smash", "Stage Hazards"];
```

`SmashCrabs` gains `screen: Screen` (starts `Screen::Hub`), `selected:
usize` (starts `0`), and `cursor_tween: Option<Transition>` (starts
`None`, `Duration::from_millis(150)` when started — the vision doc's
"coordinates are tweened over ~150ms").

**Layout:** the hub area splits into 3 equal-width panels via
`Layout::new(Direction::Horizontal, vec![Constraint::Fill(1); 3])`
(right-sized to the 3 real destinations, not the vision doc's literal
4x3/5x4 grid — an honestly-sized grid beats a mostly-empty one). Each
panel shows its fighter name (`Text`) and a placeholder portrait glyph.
`ScuttleCursor` renders near the bottom of the currently-tweening-toward
panel.

**Interaction:** `Left`/`Right` (not Tab/Shift+Tab — this is spatial
grid navigation, not list cycling) change `selected` with wraparound and
start `cursor_tween`; `on_tick` advances it via `.tick(elapsed)` and
clears it to `None` on `.is_complete()`. The cursor's displayed x
position is `easing::ease_out(old_panel_center_x, new_panel_center_x,
cursor_tween.progress())`; `moving` (passed to `ScuttleCursor::render`)
is `cursor_tween.is_some()`. `Enter` — only when not moving — starts the
VS transition (Slice 5) into `FIGHTERS[selected]`'s screen. `Esc` from
any non-`Hub` screen returns to `Screen::Hub` with `selected` preserved.
`q` quits unconditionally, same contract as every prior example.

**Testing:** example code, no `src/` changes in this slice — verified
by running, per the TDD exceptions in `development-conventions.md`
(same as every prior `examples/*.rs` slice in this project).

### Slice 5: VS transition (`examples/smash_crabs.rs`)

`SmashCrabs` gains `transitioning_to: Option<(Screen, Transition)>`
(starts `None`). On `Enter` from the Hub, `transitioning_to =
Some((FIGHTERS_screen, Transition::start(Duration::from_millis(700))))`;
`screen` does **not** change yet (unlike Omnitrix's transition, where
`mode` updates immediately) — here the destination only becomes the
active `screen` once the transition completes, since phase 1 shows
neither the old nor the new screen, just a black VS card. `on_tick`
ticks it and, on `.is_complete()`, sets `screen` to the stored
destination and clears `transitioning_to` to `None`. While
`transitioning_to.is_some()`, `Left`/`Right`/`Enter`/`Esc` are ignored
(`q` still quits).

**Rendering, two phases by `Transition::progress()`:**
- **VS card** (`progress` in `[0.0, 0.4)`): the whole inner area
  renders solid black, with bold `"VS"` text centered (`CellStyle.bold`
  — Arc 0's shipped Cell attribute, same mechanism Omnitrix's glow
  border already uses).
- **Circle wipe** (`progress` in `[0.4, 1.0]`): remap to `wipe =
  (progress - 0.4) / 0.6` in `[0, 1]`. Render the destination screen's
  content into a scratch buffer (a `render_screen_content(&self,
  screen: Screen, area: Rect) -> Buffer` helper, same shape as
  Omnitrix's `render_mode_content`). For each cell, compute distance
  from the inner area's center, with the x-distance halved to
  compensate for terminal cells being roughly twice as tall as wide
  (same aspect-ratio correction the Dial widget's radius_x/radius_y
  split already established) — this keeps the wipe reading as a circle,
  not an ellipse. `max_radius` = that same corrected distance from
  center to a corner. `radius = wipe * max_radius`. Cells within
  `radius` show the destination content; cells outside show black —
  no old-screen content is ever shown during the transition, matching
  the vision doc's "screen cuts to black... screen wipes to reveal."

**Testing:** example code, no `src/` changes — verified by running,
including watching the transition play from the Hub into Versus Mode.

### Slice 6: Arena polish (`examples/smash_crabs.rs`)

The existing `Screen::Versus` arena keeps its 3 named `LayerStack`
layers (`BACKGROUND`/`UI`/`EFFECTS` — preserves the layering demo PR
#27/#32 established) but each `paint_*` method changes from writing
directly into `buf.layer_mut(N)` to building and returning a local
scratch `Buffer` (`fn paint_background(&self, area: Rect) -> Buffer`,
etc.) sized to the full arena area. `view()` then, per layer: builds the
scratch buffer, applies `effects::shake(&scratch, dx, dy)` when
`shake_ticks_remaining > 0` (uniformly across all 3 layers, so the
*whole* composed scene shakes together, not each layer independently —
the vision doc's "offsetting the entire render buffer by 1-2 cells"),
then blits the result into the real `buf.layer_mut(N)`. This reuses
Arc 0's already-tested `effects::shake` exactly as designed (a
`&Buffer -> Buffer` remap with its own bounds-safe clipping) rather than
threading a shake offset through every individual `Rect`/`buf.set()`
call.

**On hit** (Space bar, unchanged trigger), all of the following start
together (a single keypress fans out to 5 simultaneous effects — this
is the "impressive" payoff of wiring Arc 0's primitives in):
1. `p2_damage_target` increases by `17` (an irregular Smash-style
   number rather than a round `10`, replacing the current code's
   `saturating_sub(10)` HP-down amount).
2. `damage_tween = Some(Transition::start(Duration::from_millis(250)))`
   starts (or restarts, if already mid-tween) from the currently
   *displayed* value to `p2_damage_target` — `on_tick` advances it, and
   the displayed value each frame is `easing::ease_out(display_start,
   p2_damage_target, damage_tween.progress())`, fed into
   `DamageMeter::new(displayed as u16)`.
3. `flash_ticks_remaining = FLASH_TICKS` (existing effect, unchanged).
4. `shake_ticks_remaining = SHAKE_TICKS` (new, `= 6`, matching
   `FLASH_TICKS`'s existing value — same ~200ms feel). Each tick,
   `dx`/`dy` are computed deterministically from
   `shake_ticks_remaining` (no RNG, matching this project's established
   no-new-dependency-for-randomness posture from the Omnitrix arc's
   Braille noise): magnitude `= ((shake_ticks_remaining + 1) / 2).min(2)`
   (decays from 2 to 1 as ticks run out, matching the vision doc's
   "1-2 cells"), `dx = if shake_ticks_remaining % 2 == 0 { magnitude }
   else { -magnitude }`, `dy = if (shake_ticks_remaining / 2) % 2 == 0
   { magnitude } else { -magnitude }`.
5. A radial particle burst spawns at the `DamageMeter`'s panel
   position: 8 particles, `angle = i as f32 * TAU / 8.0` for `i in
   0..8` (symmetric, deterministic — no RNG), fixed speed, `'*'` glyph,
   `theme.accent` color, ~400ms lifetime, via the existing
   `ParticleSystem` (Arc 0, already shipped).

`SmashBorder` (Slice 3) replaces the plain `Block` for both the Hub's
and Arena's outer frame — consistent branding across the whole app, per
the vision doc listing it as a "Global/Shared Component."

**Testing:** example code, no `src/` changes — verified by running,
including triggering several hits and watching the shake/particles/
tween/flash/meter all play together.

### Slice 7: Audio cues (`examples/smash_crabs.rs`, `Cargo.toml`)

Adds `rodio = "0.22"` to a new `[dev-dependencies]` table in
`Cargo.toml` — a dev-dependency only affects examples/tests, not
consumers of the `ttui` library, so this does not violate the core
crate's single-dependency (`crossterm`-only) posture from
`2026-08-04-ttui-core-framework-design.md`.

```rust
struct RodioAudioSink {
    sink: Option<rodio::stream::MixerDeviceSink>,
}

impl RodioAudioSink {
    fn new() -> Self {
        match rodio::stream::DeviceSinkBuilder::open_default_sink() {
            Ok(sink) => RodioAudioSink { sink: Some(sink) },
            Err(_) => RodioAudioSink { sink: None },
        }
    }
}

impl ttui::audio::AudioSink for RodioAudioSink {
    fn play(&mut self, event_id: &str) {
        let Some(sink) = &self.sink else { return };
        let freq = match event_id {
            "cursor" => 440.0,
            "select" => 660.0,
            "hit" => 220.0,
            _ => return,
        };
        let source = rodio::source::SineWave::new(freq)
            .take_duration(std::time::Duration::from_millis(120))
            .amplify(0.2);
        sink.mixer().add(source);
    }
}
```

If no output device is available (`open_default_sink()` errors —
expected in this headless dev environment, possibly in CI), `sink` is
`None` and `play()` silently no-ops; the app never panics or degrades
just because audio hardware isn't present. `MixerDeviceSink` must
outlive playback (documented rodio behavior: dropping it ends
playback), so it's stored as a long-lived field on `SmashCrabs`, not a
temporary.

Call sites: `"cursor"` on every `Left`/`Right` hub navigation (vision
doc: "a soft 'click'"), `"select"` on `Enter` (vision doc: "a 'smack'"),
`"hit"` on the Space-bar arena hit (vision doc: "an 'impact'").

**API caveat:** the exact `rodio` 0.22 method names above
(`DeviceSinkBuilder::open_default_sink`, `MixerDeviceSink::mixer`,
`Mixer::add`, `source::SineWave::new`) were confirmed against published
docs.rs pages during this design, not compiled locally — if the
implementing task hits a compile error against the actual resolved
crate version, that's an expected minor fixup against real compiler
output, not a sign the design is wrong.

**Testing:** example code, no `src/` changes — per the TDD exceptions,
verified by running. This one has a **stronger** manual-only caveat
than usual: this environment has no audio output to verify against, so
"verified by running" here means "compiles, doesn't panic, and the
`play()` call sites fire at the right game events" — whether the tones
actually sound right can only be confirmed by you, on your machine,
with speakers.

## Verification

- `cargo test --lib`, `cargo fmt --check`, `cargo clippy --all-targets
  -- -D warnings` all green (Slices 1-3 have real unit tests; Slices
  4-7 are example code).
- `cargo run --example smash_crabs`: confirm the Hub renders 3 fighter
  panels with a jerky-tweening `ScuttleCursor`; confirm Left/Right/
  Enter/Esc/q behave per the interaction contract above; confirm the VS
  transition plays (black card → circle wipe) on selecting Versus Mode;
  confirm hitting Space in the arena triggers the full combo (damage
  count-up, particle burst, screen shake, flash, and — separately,
  since I can't verify this myself — audible tones) together; confirm
  `q` quits cleanly with no leftover terminal attributes from any
  screen.
