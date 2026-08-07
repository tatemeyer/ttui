# TARDIS Console + Boot + Artron Energy Arc — Design

**Status:** draft, pending your review.
**Date:** 2026-08-06
**Relationship to prior specs:** an Arc-level spec (per `docs/design/README.md`'s
Arc/Slice/Task structure) bundling the Core, Structural, Architectural,
and a slice of the Features+Polish tickets from Arc 3 (TARDIS,
`2026-08-06-example-apps-roadmap-design.md`) into one design and one
plan. Resolves the deferred "camera/viewport" section of
`2026-08-05-ttui-rev-b-vision-alignment-design.md` — that spec's
recorded direction (app-space, not core framework machinery; a
`Buffer`-sized virtual canvas needs no core change; per-cell rotation is
likely permanently out of scope, "walking around" should be *simulated*)
is treated as binding here, not re-litigated. Builds on Arc 0's shared
capabilities (`easing.rs`, `particles.rs`, `effects.rs::shake`,
`transition.rs`, `audio.rs`) and reuses the `rodio` dev-dependency and
`AudioSink`-implementation pattern established in the Smash Crabs arc.
Full creative source: `TTUI-Ideas/vision/UI/idea-3-TardisTUI.md`.

## Problem

Of the three example apps the roadmap names, TARDIS is the only one
that doesn't exist as a file yet — greenfield. It's also the most
architecturally novel: it wants a "bigger on the inside" hexagonal
console you pan and rotate around, a decaying glitch overlay for error/
lag states, and camera-driven transitions, none of which map onto
anything this framework currently has. The prior Rev B spec deliberately
deferred designing this rather than guess at it before Omnitrix proved
the tick/animation model out. That prototype has since shipped (twice —
Omnitrix and Smash Crabs), so this spec now does the deferred design
work, staying inside the boundary Rev B already recorded.

## Scope

Ten slices, ordered by dependency (core-adjacent modules and widgets
first, then example integration in dependency order — Boot deliberately
comes *after* the Hub exists, since its final phase needs the Hub's own
rendering to reveal):

1. **Camera + viewport + dim** (`src/camera.rs`, new) — pan/zoom blit
   and perspective-dimming helpers. Library-helper level per Rev B, not
   core framework machinery.
2. **`GlitchBuffer`** (`src/glitch.rs`, new) — decaying noise overlay.
3. **`Roundel` widget** (`src/widgets/roundel.rs`, new).
4. **`AnalogToggle` widget** (`src/widgets/analog_toggle.rs`, new).
5. **`TimeRotor` widget** (`src/widgets/time_rotor.rs`, new).
6. **Hexagonal console Hub** (`examples/tardis.rs`) — 6 simulated
   faces, short pan+dim rotation, instant switching (no flight
   transition yet). Depends on 1, 3, 5.
7. **Artron Energy sub-app** (`examples/tardis.rs`) — real interactive
   content for one of the three named faces. Depends on 2, 3, 4, 5.
8. **Flight transition** (`examples/tardis.rs`) — upgrades the instant
   Enter-switch into the big camera-flight transition. Depends on 6, 7.
9. **Boot sequence** (`examples/tardis.rs`) — materialization intro,
   gates initial entry. Depends on 1, 6 (its final phase reveals the
   Hub).
10. **Audio** (`examples/tardis.rs`, `Cargo.toml` already has `rodio`
    from the Smash Crabs arc) — looping hum + one-shot cues. Depends on
    6, 7, 8, 9 for its call sites.

**Explicitly out of scope:** Psychic Paper and Star Charts stay named
placeholder screens (same "3 slots, 1 real" pattern as both prior
arcs); literal per-cell rotation of the console (Rev B already flagged
this as likely permanently out of scope — the hexagon is a *navigation
model*, not a rendered 3D shape, see Slice 6); real system-metrics
integration for Artron Energy (no new dependency for this — the energy
value is a synthetic, player-driven simulation, not real CPU/RAM); any
bundled audio asset files (procedural tones only, matching Smash
Crabs); Perception Filter glitch content for Psychic Paper (that's
real sub-app work, out of scope along with the rest of that screen).

## Design

### Global constraint: brightness-driven widgets need RGB colors

`camera::dim()` and `Roundel`'s intensity both work by scaling a
`Color::Rgb`'s channels toward black — they're no-ops on named ANSI
colors (`Color::Red`, `Color::Reset`, etc.), the same kind of hard
constraint as `ScuttleCursor`'s single-width-glyph requirement in the
Smash Crabs arc. `examples/tardis.rs`'s theme must use `Color::Rgb`
throughout for anything that needs to visibly dim or pulse.

### Slice 1: Camera + viewport + dim (`src/camera.rs`, #TBD)

```rust
pub struct Camera {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
}

impl Camera {
    pub fn new(x: f32, y: f32, zoom: f32) -> Self { ... }
}

pub fn viewport(source: &Buffer, camera: &Camera, width: u16, height: u16) -> Buffer { ... }
pub fn dim(buf: &Buffer, factor: f32) -> Buffer { ... }
```

Per Rev B's recorded direction: the "virtual buffer" is just a large
`Buffer::new(w, h)` — no new buffer type. `viewport` samples a `width x
height` window out of `source`: for output cell `(x, y)`, the sampled
source position is `(camera.x + x / camera.zoom, camera.y + y /
camera.zoom)`, rounded to the nearest cell. `zoom > 1.0` magnifies
(blocky nearest-neighbor duplication — there's no sub-cell rendering in
a terminal, so this is deliberately coarse, not smooth); `zoom < 1.0`
shows more of the source compressed into the same output area;
`zoom == 1.0` is a plain crop. Sampled positions outside `source`'s
bounds are skipped (left default), matching every other widget's
out-of-bounds handling in this codebase.

`dim` returns a new `Buffer` with every cell's `fg`/`bg` scaled toward
black by `factor` (clamped to `[0, 1]`) — `factor: 0.0` is a no-op,
`1.0` is fully black. Only `Color::Rgb` cells are affected (see the
Global Constraint above); other `Color` variants pass through
unchanged.

**Testing:** `coding`-tagged, TDD applies. `viewport`: a `zoom: 1.0`
crop matches the expected source cells at a known camera position; a
`zoom: 2.0` magnification samples the same source cell for adjacent
output cells; an out-of-bounds camera position doesn't panic and
leaves those cells default. `dim`: `factor: 0.0` leaves an `Rgb` cell
unchanged; `factor: 1.0` drives it to `(0, 0, 0)`; `factor: 0.5` halves
each channel; a non-`Rgb` color is unaffected regardless of `factor`.

### Slice 2: `GlitchBuffer` (`src/glitch.rs`, #TBD)

```rust
pub struct GlitchBuffer {
    transition: Option<Transition>,
}

impl GlitchBuffer {
    pub fn new() -> Self { ... }
    pub fn trigger(&mut self, duration: Duration) { ... }
    pub fn tick(&mut self, elapsed: Duration) { ... }
    pub fn is_active(&self) -> bool { ... }
    pub fn render(&self, area: Rect, color: Color, tick_count: u64, buf: &mut Buffer) { ... }
}
```

Reuses `Transition` for the decay curve (the fourth reuse of that Arc 0
primitive across this project) rather than inventing a new decay
tracker. `trigger` starts (or restarts) the transition; while active,
`render` computes `intensity = 1.0 - transition.progress()` (starts at
`1.0`, decays to `0.0`) and, for each cell in `area`, uses a
deterministic hash of `(x, y, tick_count)` — same no-RNG approach as
Omnitrix's Braille-noise transition — to decide whether that cell gets
overwritten with one of 4 block-shade glyphs (`░▒▓█`, light to heavy)
in `color`. At `intensity: 1.0` every cell in `area` is glitched; as it
decays toward `0.0`, progressively fewer cells are. When inactive,
`render` is a no-op (leaves the buffer untouched — matches this
codebase's "default cell is transparent" convention from `LayerStack`).

**Testing:** `coding`-tagged, TDD applies. A fresh `GlitchBuffer` is
inactive and `render` leaves the buffer untouched. `trigger` makes
`is_active()` true immediately. Ticking past the triggered duration
makes `is_active()` false again and `render` goes back to a no-op. At
`intensity: 1.0` (immediately after `trigger`, before any tick), every
cell in a test area is non-default and carries the requested `color` —
this is a clean, hash-independent assertion (100% density doesn't
depend on what the hash actually returns, only on whether it's used at
all), avoiding the need to hand-derive exact hash outputs.

### Slice 3: `Roundel` widget (`src/widgets/roundel.rs`, #TBD)

```rust
pub struct Roundel {
    intensity: f32, // clamped 0.0-1.0
    color: Color,
}

impl Roundel {
    pub fn new(intensity: f32, color: Color) -> Self { ... }
    pub fn render(&self, area: Rect, buf: &mut Buffer) { ... }
}
```

Renders a single glyph (`'O'` — plain ASCII, not a Unicode circle,
matching `ScuttleCursor`'s established caution about glyph width)
centered in `area`, colored by scaling `color`'s channels by
`intensity` (same `Rgb`-only limitation as `camera::dim`, `intensity:
0.0` renders near-black, `1.0` renders full `color`). The vision doc's
"row of roundels" is achieved by the caller placing multiple `Roundel`
instances side by side — this widget draws exactly one node, matching
`List`/`Text`/`Dial`'s existing per-instance-simplicity precedent.

**Testing:** `coding`-tagged, TDD applies. `intensity: 0.0` renders
near-black; `intensity: 1.0` renders the exact input `color`
unchanged; `intensity: 0.5` renders each channel halved; renders at
`area`'s center and doesn't panic on a 1x1 area.

### Slice 4: `AnalogToggle` widget (`src/widgets/analog_toggle.rs`, #TBD)

```rust
pub struct AnalogToggle {
    on: bool,
}

impl AnalogToggle {
    pub fn new(on: bool) -> Self { ... }
    pub fn render(&self, area: Rect, buf: &mut Buffer) { ... }
}
```

Renders a fixed 5-character lever glyph, left-aligned in `area`:
`"[ \ ]"` when `on: false`, `"[ / ]"` when `on: true` — a physical
lever tilting between two positions, per the vision doc's "physical-
looking levers." Clips like every other text-rendering widget if
`area.width < 5`.

**Testing:** `coding`-tagged, TDD applies. `on: false` renders exactly
`"[ \ ]"`; `on: true` renders exactly `"[ / ]"`; a narrower-than-5
area clips without panicking.

### Slice 5: `TimeRotor` widget (`src/widgets/time_rotor.rs`, #TBD)

```rust
pub struct TimeRotor {
    speed: f32, // clamped to a sane minimum so it never divides toward zero
}

impl TimeRotor {
    pub fn new(speed: f32) -> Self { ... }
    pub fn render(&self, area: Rect, tick_count: u64, buf: &mut Buffer) { ... }
}
```

Draws one Braille-pattern glyph (`U+2800`-`U+28FF`, same range and
same deterministic-hash technique as Omnitrix's corruption-transition
noise and `GlitchBuffer`'s block glyphs — the third reuse of that
specific trick) per row down `area`'s full height, at `area`'s
horizontal center column. Each row's glyph is derived from a hash of
`(row, (tick_count as f32 * self.speed) as u64)` — higher `speed`
advances the effective tick faster, so the column visibly "pulses"
faster, matching the vision doc's "speeds up when the system is under
load."

**Testing:** `coding`-tagged, TDD applies. Renders one glyph per row
down `area`'s height at the center column. The same `(area, tick_count,
speed)` always renders identically (determinism). Two `TimeRotor`s with
different `speed` values render *differently* at the same `tick_count`
for some row — asserted as "the two output buffers aren't equal," not
by hand-deriving exact Braille codepoints (same reasoning as
`GlitchBuffer`'s tests: avoid fragile hash-value arithmetic in test
code).

### Slice 6: Hexagonal console Hub (`examples/tardis.rs`)

```rust
#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Hub,
    PsychicPaper,
    StarCharts,
    ArtronEnergy,
}

const FACE_COUNT: usize = 6;
const FACE_NAMES: [&str; 6] = [
    "Psychic Paper",
    "Auxiliary Roundel Bay",
    "Star Charts",
    "Auxiliary Roundel Bay",
    "Artron Energy",
    "Auxiliary Roundel Bay",
];
```

Faces `0`, `2`, `4` are the three real destinations; faces `1`, `3`,
`5` are decorative "Auxiliary Roundel Bay" faces with no destination
(`Enter` is a no-op there) — giving the hexagon genuine 6-way variety
without inventing 3 more sub-apps. **Honesty about what's actually
rendered:** this is a navigation model, not a literal on-screen
hexagon. The 6 faces are laid out side by side in a virtual `Buffer`
(`FACE_COUNT * viewport_width` wide), each face's content rendered
into its own `viewport_width`-wide slice. `camera::dim()` is applied
per-face-slice, with `factor` driven by **hex distance** from the
selected face (circular: `distance(a, b) = min(|a - b|, FACE_COUNT -
|a - b|)`) — distance `0` (active): no dim; `1`: `0.35`; `2`: `0.65`;
`3` (directly opposite, the hexagon's max distance): `0.85`. The
visible window is extracted via `camera::viewport()`, with
`camera.x` eased between `old_selected * viewport_width` and
`new_selected * viewport_width` over a short (**200ms**) `Transition`
on `Left`/`Right` — the same eased-position-plus-jerk-free tween shape
as Smash Crabs' cursor tween, reused here for camera panning instead
of a cursor. `TimeRotor` (the console's persistent central column) and
ambient decorative `Roundel`s are drawn directly onto the *final*
blitted buffer, not into the per-face virtual content — the console's
heartbeat and wall lighting stay fixed to the viewport regardless of
which face is panned into view, not part of any one face's "room."

**Interaction (this slice):** `Left`/`Right` rotate `selected_face`
(wrapping `0..FACE_COUNT`) and start the 200ms pan; `Enter` — only
when `screen_for_face(selected_face)` is `Some` and no pan is mid-
flight — switches `screen` **instantly** (no flight transition yet;
Slice 8 upgrades this, same "instant switch first" sequencing lesson
learned from the Smash Crabs arc, where the flight-equivalent
transition needs real destination content to preview and that content
doesn't exist until Slice 7). `Esc` from any non-`Hub` screen returns
to `Screen::Hub` instantly (not a flight transition — matches both
prior arcs' "Enter is fancy, Esc is instant" convention). `q` quits
unconditionally.

**Testing:** example code, no `src/` changes in this slice — verified
by running, per the TDD exceptions in `development-conventions.md`.

### Slice 7: Artron Energy sub-app (`examples/tardis.rs`)

`Tardis` gains `energy: f32` (starts `0.0`, **not** reset on screen
entry/exit — this is meant to feel like a persistent ship system, a
deliberate difference from Smash Crabs' per-visit damage reset). While
on `Screen::ArtronEnergy`: `Space` increases `energy` by `12.0`
(channeling more energy); `v` instantly vents `energy` down by `35.0`
(manual relief valve, floored at `0.0`) and starts a `Tardis`-owned
~300ms `Transition` (`vent_flash`) that, while active, passes `on:
true` to the (still-stateless) `AnalogToggle` widget's render call
instead of the toggle's resting `false` — the same "widget stays a
pure `render(value)`, the app owns any timing" split as every other
widget in this codebase;
otherwise `energy` decays by `4.0` per second (applied as `4.0 *
elapsed.as_secs_f32()` each tick, floored at `0.0`) regardless of which
screen is active — a background ship system, ticking everywhere, which
is also what drives the console Hub's `TimeRotor` speed (see below).

Two thresholds: `energy >= 80.0` ("getting full") spawns a radial
particle burst in `Color::Red` per hit of the threshold (venting
plasma — reuses `ParticleSystem`, same deterministic-angle burst
pattern as Smash Crabs); `energy >= 90.0` ("lagging") does three
things together: (1) `GlitchBuffer::trigger` is (re-)called every tick
while lagging persists, keeping it continuously at full intensity
until `energy` drops back below the threshold, at which point it's
left to decay over its own ~500ms on its own (matching the vision
doc's "decays over 500ms" — the decay only actually plays out once lag
*stops*); (2) `TimeRotor`'s `speed` (both here and in the Hub) is
computed as `1.0 + energy / 50.0`, so it visibly speeds up as energy
climbs, matching the vision doc's "speeds up under load" — this is the
single shared computation both screens read); (3)
**`tick_rate()` itself returns a longer `Duration` while lagging** (a
real slowdown, not a visual trick — matching the vision doc's "the
framework intentionally drops its frame rate... making the user feel
the physical strain").

Rendering: 3 `Roundel`s in a row as "pipe" segments, lighting up in
sequence as `energy` climbs (not all three scaling identically —
segment `i`'s intensity is `((energy - i * 33.0) / 33.0).clamp(0.0,
1.0)`, so they fill left-to-right); the vent `AnalogToggle`; a
`GlitchBuffer` overlay when lagging; the shared `TimeRotor`.

**Testing:** example code, no `src/` changes — verified by running,
including watching `energy` climb, the plasma burst trigger past 80,
the glitch/frame-slowdown kick in past 90, and manual venting bring it
back down.

### Slice 8: Flight transition (`examples/tardis.rs`)

`Tardis` gains `transitioning_to: Option<(Screen, Transition)>` (starts
`None`). `Enter` on a real face now starts this (**900ms**, noticeably
longer than the 200ms rotation pan — matching the vision doc's
distinction between a quick console spin and a full "flight") instead
of switching `screen` immediately; `screen` updates only once the
transition completes. While transitioning, all navigation input is
ignored (`q` still quits). Rendering, phased by `progress`:
- **Shake+blur** (`[0.0, 0.3)`): the current (pre-switch) Hub view
  renders as normal, then gets `effects::shake`'d with an increasing
  magnitude as `progress` climbs — "the current UI begins to shake and
  blur."
- **Void streak** (`[0.3, 0.7)`): solid near-black background with a
  radial burst of `'-'`-symbol particles streaking outward from center
  (reusing `ParticleSystem` again — "streaks of temporal energy"),
  density scaling with `progress`.
- **Arrival** (`[0.7, 1.0]`): the destination screen's content (via a
  `render_destination_preview` helper, same shape as both prior arcs'
  transition-preview helpers) fades in — implemented as a straight
  cross-fade is out of scope for a terminal's discrete cells, so this
  is a **hard cut** at `progress: 0.85` from void-streak to full
  destination content, held for the remaining ~15% of the transition
  as a brief "locked into focus" beat rather than an animated blend.

**Testing:** example code, no `src/` changes — verified by running,
including a full flight from the Hub into Artron Energy.

### Slice 9: Boot sequence (`examples/tardis.rs`)

`Tardis` gains `booting: Option<Transition>`, started immediately in
`new()` (**3000ms** total) — the very first `view()` call already
shows boot phase 0, gating all interaction except `q` until it
completes. Five phases by `progress`:
- **Police Box** (`[0.0, 0.15)`): a small fixed ASCII Police Box
  graphic, centered on black.
- **Shake** (`[0.15, 0.35)`): the same graphic, `effects::shake`'d
  with a deterministic jitter pattern (same magnitude/direction shape
  as Smash Crabs' `shake_offset`) — "violent flight."
- **Doors open** (`[0.35, 0.5)`): the graphic's door glyphs swap from
  closed to open (a small fixed character delta in the ASCII art).
- **Flash** (`[0.5, 0.65)`): solid white background fill — "a
  blinding, warm white light."
- **Push-through** (`[0.65, 1.0]`): `Camera.zoom` eases from `1.0` to
  `2.2` (`easing::ease_out`) while blitting the **Hub's own
  rendering** (Slice 6's already-built pipeline — this is why Boot is
  sequenced after the Hub, not before) through `camera::viewport` —
  "the camera pushes through the doors... revealing the... Console
  Room." Once `booting` completes, the app is in the ordinary
  `Screen::Hub` state with `booting: None` thereafter (never re-
  triggered — this is a one-time intro, not a repeatable transition).

**Testing:** example code, no `src/` changes — verified by running,
watching the full sequence play once at startup and hand off cleanly
into normal Hub interaction.

### Slice 10: Audio (`examples/tardis.rs`)

Reuses the exact `RodioAudioSink`/`AudioSink` pattern from the Smash
Crabs arc (same struct shape, same graceful no-device fallback) —
copied into this file, not shared via `src/`, matching that arc's
precedent of keeping example-local audio code example-local. Two
additions beyond Smash Crabs' one-shot-only design:
- **Looping hum:** on successful device open, immediately mixes in a
  low-amplitude tone built as `SineWave::new(freq).take_duration(hum_
  cycle).amplify(vol).repeat_infinite()` (rodio 0.22's `Source::
  repeat_infinite`, confirmed against docs.rs during this design — it
  buffers the finite `take_duration`'d chunk and loops it, which is
  exactly the intended use against `SineWave`'s otherwise-infinite
  signal). Started once in `new()`, never explicitly stopped — it ends
  naturally when the process exits and the `MixerDeviceSink` drops.
- **One-shot cues:** distinct tones at boot-phase-zero, the start of
  each flight transition, and the vent-toggle keypress — same `play
  (event_id: &str)` call shape as Smash Crabs.

**Testing:** example code, no `src/` changes — per the TDD exceptions,
verified by running; audio playback itself cannot be verified in this
environment (no audio device) — same caveat as the Smash Crabs arc,
stated here so it isn't re-litigated per task.

## Verification

- `cargo test --lib`, `cargo fmt --check`, `cargo clippy --all-targets
  -- -D warnings` all green (Slices 1-5 have real unit tests; Slices
  6-10 are example code).
- `cargo run --example tardis`: confirm the boot sequence plays once
  (Police Box → shake → doors → flash → push-through) and hands off to
  an interactive Hub; confirm Left/Right rotate the console with a
  visible pan and dimming of non-active faces; confirm Enter on a real
  face plays the full flight transition (shake → void streak → hard-
  cut arrival) into that screen, and Esc returns instantly; confirm
  Artron Energy's energy value responds to Space/`v`, the plasma burst
  and glitch/frame-slowdown trigger at their thresholds, and the same
  `TimeRotor` speed-up is visible back in the Hub afterward; confirm
  the looping hum starts at boot and one-shot cues fire at the
  documented call sites (audibly, on your machine — not verifiable by
  me); confirm `q` quits cleanly from every state, including mid-boot
  and mid-transition, with no leftover terminal attributes.
