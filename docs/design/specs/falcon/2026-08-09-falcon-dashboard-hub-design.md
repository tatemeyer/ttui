# Falcon Dashboard Hub — Design

**Status:** draft, pending your review before we move to planning.
**Date:** 2026-08-09
**Relationship to prior specs:** the first of two specs for Arc E (a
fourth themed example app), following the same hub-first-then-sub-apps
split used by all three existing apps (`docs/design/specs/omnitrix/
2026-08-06-omnitrix-dial-navigation-arc-design.md`, `docs/design/specs/
tardis/2026-08-06-tardis-console-arc-design.md`, `docs/design/specs/
smash-crabs/2026-08-06-smash-crabs-arena-hub-arc-design.md`). Built from
`TTUI-Ideas/vision/UI/idea-4-Falcon.md`. A follow-up spec covers
Hyperdrive/Sensors/Weapons' actual content once this hub ships.
Deliberately reuses only existing framework primitives — `GlitchBuffer`
(`src/glitch.rs`), `Transition` (`src/transition.rs`), the particle
system (`src/particles.rs`), and `Theme` (`src/theme.rs`) — no new
core rendering primitive is introduced by this Arc.

## Problem

The vision doc describes a smuggler-cockpit aesthetic and a
Panel-Cycle + Percussive Maintenance interaction paradigm, but nothing
maps it onto TTUI's actual primitives yet. This spec does that
translation for the hub only: the `CockpitPanel` border widget, the
3-panel dashboard layout with focus-driven enlarging, the percussive
maintenance mechanic, and the boot sequence. Sub-app content is
explicitly deferred (see Non-goals).

## Scope

**Tag: `coding`** for Slice 1 (new `src/` widget — full TDD applies).
**Tag: `coding`, TDD-exempt** for Slices 2-4 (example code — the
"Examples/demos" exception in `.claude/rules/development-conventions.md`:
correctness is checked by running the example, not asserting on it).

Four slices, in dependency order:

1. **`CockpitPanel` widget** (`src/widgets/cockpit_panel.rs`, new)
2. **Falcon app skeleton** (`examples/falcon/{main.rs,falcon.rs}`, new)
   — depends on 1.
3. **Percussive maintenance** (`examples/falcon/falcon.rs`) — depends
   on 2.
4. **Boot sequence** (`examples/falcon/boot.rs`, new) — depends on 2.

## Design

### Slice 1: `CockpitPanel` widget (`src/widgets/cockpit_panel.rs`)

```rust
pub struct CockpitPanel {
    pub focused: bool,
}

impl CockpitPanel {
    pub fn new(focused: bool) -> Self {
        CockpitPanel { focused }
    }

    /// Draws a thick, riveted, deliberately-asymmetric double-line
    /// border and returns the shrunk inner content area.
    pub fn render(&self, area: Rect, theme: &Theme, buf: &mut Buffer) -> Rect {
        ...
    }
}
```

No lifetime parameter, no `Theme` stored on the struct — matches
`SmashBorder`'s existing precedent (`theme` passed at render time, not
held).

**Geometry — two rings, drawn explicitly (not through `BorderSet`,**
**same as `SmashBorder`):**
- **Outer ring** at `area`'s bounding edges: horizontal glyph `=`,
  vertical glyph `#`, corner glyph `+` at the top-left, top-right, and
  bottom-left corners. The **bottom-right corner** deliberately uses a
  different glyph, `¤` — the one asymmetry in an otherwise-regular
  rectangle, standing in for "this thing was bolted together by hand."
- **Rivets:** along the outer ring's top and bottom edges, a rivet
  glyph `o` replaces the horizontal glyph every 3rd cell starting at
  offset 1 from each corner (deterministic position, not random — same
  no-RNG-dependency posture as every prior Arc). Along the left/right
  edges, a rivet replaces the vertical glyph every 2nd row, same
  offset rule.
- **Inner ring**, one cell further in: horizontal glyph `-`, vertical
  glyph `|`, corner glyph `+` at all four corners (no asymmetry on this
  ring — the asymmetry is the outer ring's signature, not repeated).
- **Color:** `self.focused` selects `theme.primary` (bright, focused)
  vs `theme.secondary` (dimmed, idle) for every glyph in both rings —
  a single color switch, no per-glyph variation.
- **Returned inner `Rect`:** `area` shrunk by 2 in each direction (both
  rings consumed), matching `Block`/`SmashBorder`'s existing
  shrink-and-return convention. If `area` is too small for both rings
  (width/height `< 4`), clamp to a zero-size `Rect` at `area`'s
  position rather than underflowing — same defensive convention
  `Block::render` already uses at its own small-area boundary.

**Testing (`coding`, full TDD):**
- Outer ring cells at `area`'s exact edges carry the outer glyphs
  (`=`/`#`/`+`), except the bottom-right corner, which carries `¤`
  specifically (pins the one intentional asymmetry).
- Rivet glyphs (`o`) appear at the expected deterministic offsets
  along all four edges, and nowhere else on the outer ring.
- Inner ring cells (one step in from the outer ring) carry `-`/`|`/`+`
  uniformly, no asymmetry, no rivets.
- `focused: true` renders both rings in `theme.primary`; `focused:
  false` renders both in `theme.secondary` — assert on a couple of
  sampled cells per ring.
- Returned inner `Rect` is `area` shrunk by 2 on every side for a
  normal-sized `area`, and degrades to a zero-size `Rect` without
  panicking for `area.width < 4` or `area.height < 4`.

### Slice 2: Falcon app skeleton (`examples/falcon/{main.rs,falcon.rs}`)

`examples/falcon/main.rs` is the thin standalone entry point, same
shape as every other app's `main.rs` (`#[path]`-includes `falcon.rs`,
runs it via the existing `App`/terminal-driver loop).

`falcon_theme() -> Theme`:

```rust
Theme {
    background: Color::Rgb { r: 10, g: 10, b: 8 },   // #0A0A08
    primary: Color::Rgb { r: 255, g: 176, b: 0 },     // #FFB000 amber
    secondary: Color::Rgb { r: 76, g: 187, b: 23 },   // #4CBB17 sickly green
    tertiary: Color::Rgb { r: 255, g: 49, b: 49 },    // #FF3131 warning red
    accent: Color::Rgb { r: 255, g: 215, b: 0 },      // #FFD700 hazard yellow
    primary_end: None,
    border: BorderSet::default(), // unused by CockpitPanel; set for Theme completeness
    border_bold: false,
    border_thick: false,
}
```

```rust
#[derive(Clone, Copy, PartialEq)]
enum PanelKind {
    Hyperdrive,
    Sensors,
    Weapons,
}

const PANELS: [PanelKind; 3] = [PanelKind::Hyperdrive, PanelKind::Sensors, PanelKind::Weapons];

struct Falcon {
    theme: Theme,
    focused: usize, // index into PANELS, wraps 0..3
    // Slice 3 adds glitch state here; Slice 4 adds boot state here.
}
```

**Layout:** the screen area splits into 3 equal-width vertical slots
left-to-right (`area.width / 3`, remainder cells absorbed into the
rightmost slot — same even-division-with-remainder convention used
elsewhere in this codebase for fixed N-way splits). Each slot computes
its own box: the focused slot's box is `base_w + 4` wide and
`base_h + 2` tall (clamped to the slot's available bounds, same
`focus_w`/`focus_h` clamping technique as `examples/launcher/nexus.rs`
lines 109-110), centered within its slot; non-focused slots use
`base_w`/`base_h` centered in theirs. Each box renders via
`CockpitPanel::new(i == self.focused).render(box_rect, &self.theme, buf)`,
and the returned inner area gets the panel's placeholder content:
`Text::new(name).render(...)` plus `Text::new("(not yet built)")` one
row below — same placeholder convention Omnitrix's hub used for its
unbuilt sub-apps.

**Interaction:** Tab cycles `focused = (focused + 1) % 3`; Shift+Tab
cycles `focused = (focused + 2) % 3` (backward, same wraparound
technique used everywhere else in this codebase). `q` quits
unconditionally, from anywhere — same contract as every other app.
(Space is reserved for Slice 3.)

**Testing:** example code, no `src/` changes in this slice — verified
by running.

### Slice 3: Percussive maintenance (`examples/falcon/falcon.rs`)

`Falcon` gains `glitches: [GlitchBuffer; 3]` (one per `PanelKind`,
all starting inactive via `GlitchBuffer::new()`).

**Spontaneous idle flicker:** on `on_tick`, for each panel index `i`,
if `glitches[i]` is not already active and `tick_count % 90 ==
i as u64 * 30` (a deterministic, staggered-by-panel-index trigger —
no RNG dependency, same posture as every prior Arc's noise/timing
logic; at a nominal ~30 ticks/second this fires roughly once every 3
seconds per panel, staggered so all three don't glitch in sync), call
`glitches[i].trigger(Duration::from_millis(600))`. Every panel's
`GlitchBuffer` also gets `.tick(elapsed)` called unconditionally each
frame (existing `GlitchBuffer` API — decays and deactivates on its own
once its duration elapses, whether or not the player intervenes).

**Rendering:** after a panel's normal content is drawn into its box,
if `glitches[i]` is active, its overlay renders on top via the
existing `GlitchBuffer` render call, composited over the panel's
content using `LayerStack`/alpha (Arc C) rather than fully replacing
it — the glitch reads as static laid over the readout, not a blank
panel.

**WHACK (early clear):** pressing Space while `glitches[self.focused]`
is active calls a new early-clear path — `GlitchBuffer` doesn't
currently expose one (`trigger`/`tick` only), so this slice adds:

```rust
impl GlitchBuffer {
    /// Ends the glitch immediately, regardless of remaining duration.
    pub fn clear(&mut self) {
        self.transition = None;
    }
}
```

This is a `src/` change (`coding`-tagged, TDD applies to this one
method): a test triggering a `GlitchBuffer`, confirming it reports
active, calling `clear()`, and confirming it reports inactive
immediately (not just eventually, after its own decay). Pressing Space
while the focused panel's glitch is active also spawns a small
particle-spark burst (existing `ParticleSystem::spawn`, a handful of
particles at the panel's border) as the "thunk" feedback. Space while
the focused panel is NOT glitching does nothing (no-op, not an error).

**Testing:** the `GlitchBuffer::clear()` addition is `coding`-tagged,
TDD applies (see above). The trigger-scheduling and rendering-overlay
logic in `falcon.rs` itself is example code, verified by running.

### Slice 4: Boot sequence (`examples/falcon/boot.rs`)

Same file-per-app-boot convention as `omnitrix/boot.rs`/
`tardis/boot.rs`/`smash_crabs/boot.rs`. `Falcon` gains `booting:
Option<Transition>` (starts `Some(Transition::start(Duration::from_
millis(1400)))`; `None` once complete — while `Some`, normal
Tab/Shift+Tab/Space input is ignored, `q` still quits unconditionally,
same "ignore input mid-transition" contract as Omnitrix's corruption
transition).

**Rendering, by `Transition::progress()`:**
- **`[0.0, 0.1)`:** solid black, except a single amber pilot-light
  glyph (`•`) at screen center — a static pre-boot moment.
- **`[0.1, 0.7)`:** remap to `wave = (progress - 0.1) / 0.6` in
  `[0, 1]`. `panels_shown = (wave * 3.0) as usize` (0, 1, 2, or 3).
  Panels `0..panels_shown` render their full `CockpitPanel` border
  (unfocused coloring) with their `GlitchBuffer` triggered for a brief
  burst (reuses Slice 3's overlay rendering) at the moment they first
  appear; panels not yet shown stay blank. This is the "rivet by
  rivet" snap-in — one panel's border resolves at a time rather than
  all three fading in together.
- **`[0.7, 1.0]`:** remap to `fade = (progress - 0.7) / 0.3` in
  `[0, 1]`. All three panels render normally (Slice 2's placeholder
  content, panel 0 focused per `Falcon`'s default `focused: 0`), with
  the whole frame's brightness ramped from dim to full via
  `easing::lerp_color(theme.background, <that cell's real color>,
  fade)` applied per-cell after normal rendering — same dim-to-bright
  technique already established for focus transitions elsewhere in
  this codebase, applied here across the whole frame instead of one
  widget.

On `Transition::is_complete()`, `booting` is set to `None` and normal
input handling resumes.

**Testing:** example code, no `src/` changes — verified by running,
including watching the full boot sequence play once at startup.

## Non-goals

- **Hyperdrive/Sensors/Weapons' actual content** — this spec ships
  placeholder screens only (Slice 2). A follow-up spec covers the
  jump-calculation console, radar sweep, and targeting reticle
  described in the vision doc.
- **No new core rendering primitive.** Every mechanic here (glitch
  overlay, particle sparks, dim/brighten transitions, alpha
  compositing) reuses what already exists in `src/`. `CockpitPanel` is
  a new *widget*, not a new primitive — same category as
  `SmashBorder`/`Dial`/`Roundel`.
- **No audio** — `smash_crabs` shows TTUI apps *can* opt into sound via
  `ttui::audio::AudioSink`, but the vision doc never called for it and
  this Arc doesn't add it; a purely deliberate exclusion, not a
  framework limitation.
- **No new dependency.**

## Verification

- `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D
  warnings` all green (Slice 1's widget and the `GlitchBuffer::clear()`
  addition have real unit tests; Slices 2-4 are example code).
- `cargo run --example falcon`: confirm the boot sequence plays once at
  startup (pilot light → panels snap in one at a time with a glitch
  burst each → whole frame brightens to full); confirm the dashboard
  settles with panel 0 (Hyperdrive) focused and enlarged; confirm
  Tab/Shift+Tab cycle focus with the enlarge/dim animation reading
  correctly; confirm idle panels occasionally flicker on their own and
  resolve without input; confirm Space clears a glitch on the focused
  panel with a visible spark burst, and does nothing when the focused
  panel isn't glitching; confirm `q` quits cleanly from any state,
  including mid-boot.
