# Smash Crabs Remaining Sub-Apps + Boot — Design

**Status:** draft, pending your review.
**Date:** 2026-08-07
**Relationship to prior specs:** completes Arc 2 (Smash Crabs,
`2026-08-06-example-apps-roadmap-design.md`, issues #63-65) on top of
`2026-08-06-smash-crabs-arena-hub-arc-design.md` (Hub navigation, Versus
Mode, `ScuttleCursor`/`DamageMeter`/`SmashBorder`, audio hook — all
shipped). No new core widgets this round; one small core addition
(`easing::lerp_color`, promoted from a pattern TARDIS's Psychic Paper
already needed once as a local fn — see Slice 1). Full creative source:
`TTUI-Ideas/vision/UI/idea-2-SuperSmashCrabs.md`.

## Problem

Of Smash Crabs' three named fighters, only Versus Mode has real content —
Target Smash and Stage Hazards are still a shared `render_placeholder`
stub, and the app currently starts directly on the Hub with no intro.
This spec builds both sub-apps and the boot/intro splash, completing
Smash Crabs the way Omnitrix and TARDIS were completed earlier this
session.

## Scope

Three slices, all `examples/smash_crabs.rs`, independent of each other
(each only touches its own `Screen` arm/fields, except the boot slice
which wraps entry into the existing Hub):

1. **Target Smash (Productivity)** (#63) — fixed 5-target list; smashing
   a target shakes the screen, overlays a literal `💥` impact + "KO"
   stamp over its text, then fades it out.
2. **Stage Hazards (System Dashboard)** (#64) — CPU/RAM `DamageMeter`s;
   RAM spikes on user input and decays on its own; at ≥90% RAM a
   flashing-red Bob-omb ASCII art appears in the corner.
3. **Boot/intro splash** (#65) — pitch-black → white flash resolving
   into a snapping claw → spaced title sliding in from both edges → a
   sweeping lens-flare band that reveals the Hub underneath.

**Explicitly out of scope:** a persistent/growing target backlog for
Target Smash (fixed 5 slots, same restraint TARDIS's Star Charts already
established for its timeline — not a new pattern); a CPU-side hazard
creature (the vision doc only specifies the Bob-omb for RAM; CPU stays a
plain ambient meter); real system-stat integration for CPU/RAM (mocked
values, same posture as every other app's non-real backend this
session); a skippable boot sequence (plays out fully, `q` still quits
unconditionally since that check already runs before the screen match).

## Design

### Slice 1: Target Smash (`examples/smash_crabs.rs`, #63)

```rust
const TS_TARGETS: [&str; 5] = [
    "Refactor auth module",
    "Fix flaky test",
    "Write release notes",
    "Review PR #42",
    "Update dependencies",
];
const TS_IMPACT_GLYPH: char = '💥'; // vision-literal; double-width in most
                                     // terminals, same documented risk as
                                     // Omnitrix's vision-doc glyphs — accepted,
                                     // not silently substituted
const KO_HOLD_MS: u64 = 600;
const TS_FADE_MS: u64 = 400;

enum TsPhase {
    Impact(Transition),
    Fade(Transition),
}
```

`SmashCrabs` gains `ts_smashed: [bool; 5]` (starts all `false`),
`ts_selected: usize` (starts `0`, indexes into the *visible* — i.e.
unsmashed — targets, not the fixed array directly), `ts_smashing:
Option<(usize, TsPhase)>` (starts `None`, the `usize` is the real index
into `TS_TARGETS`/`ts_smashed`). Reuses the existing `shake_ticks_
remaining` field for the shake (already mutually exclusive in time with
Versus Mode's use of it, same sharing precedent as TARDIS's shared
`GlitchBuffer`) — no new shake field.

**Visible-list navigation:** each frame/input, compute `visible: Vec<usize>
= (0..5).filter(|&i| !ts_smashed[i]).collect()`. `Up`/`Down` move `ts_
selected` modulo `visible.len()` (only when `ts_smashing.is_none()` and
`visible` is non-empty). `Enter`/`Space` (same guards) smashes `visible
[ts_selected]`: sets `shake_ticks_remaining = SHAKE_TICKS`, starts `ts_
smashing = Some((real_index, TsPhase::Impact(Transition::start
(600ms))))`. `Esc` returns to Hub, blocked while `ts_smashing.is_some()`.

**`on_tick` phase advance:** ticks the active `Transition` inside `ts_
smashing`; on `Impact`'s completion, replaces it with `TsPhase::Fade
(Transition::start(400ms))` (leaves the target's `ts_smashed` flag
untouched — it's still visible, now fading); on `Fade`'s completion, sets
`ts_smashed[real_index] = true` and clears `ts_smashing = None`, then
clamps `ts_selected` to `0` if it now exceeds the shrunk `visible.len() -
1`.

**Rendering:** each visible target renders as one row, `"{name}"` in
`theme.tertiary`, with the `ScuttleCursor`-style selection highlight
(reuse the existing hub-cursor color convention, `theme.accent`) on the
row at `ts_selected`. While `ts_smashing` is `Some((i, phase))`:
- `TsPhase::Impact`: overlay `TS_IMPACT_GLYPH` at 3 positions across
  target `i`'s row (start, middle, end of the text span) plus a 2-cell
  `"KO"` stamp in bold `theme.tertiary` on `theme.primary` background,
  positioned just past the end of the row — directly "over the text,"
  not a separate particle burst (Versus Mode already owns the
  particle-burst look; reusing it here would blur the two apps).
- `TsPhase::Fade`: target `i`'s row renders in `lerp_color(theme.
  tertiary, theme.background, phase_transition.progress())` — see
  `easing::lerp_color` below.

Uses the same 3-layer scratch-buffer + `effects::shake` compositing
pattern Versus Mode already established (`paint_background`/`paint_ui`/
`paint_effects` methods, `blit`) — Target Smash gets its own `paint_*`
trio reusing that exact shape, not a new compositing mechanism.

**`easing::lerp_color` (new core fn, TDD):** TARDIS's Psychic Paper
already implemented an Rgb-only color lerp as a private local fn
(`lerp_color(from, to, t) -> Color`) for its ink-bleed effect. Target
Smash's fade needs the identical primitive. Rather than duplicate it a
second time, promote one copy to `src/easing.rs`:

```rust
pub fn lerp_color(from: Color, to: Color, t: f32) -> Color {
    match (from, to) {
        (Color::Rgb { r: r1, g: g1, b: b1 }, Color::Rgb { r: r2, g: g2, b: b2 }) => {
            Color::Rgb {
                r: lerp(r1 as f32, r2 as f32, t) as u8,
                g: lerp(g1 as f32, g2 as f32, t) as u8,
                b: lerp(b1 as f32, b2 as f32, t) as u8,
            }
        }
        _ => to,
    }
}
```

TARDIS's existing local `lerp_color` in `examples/tardis.rs` is left
untouched — refactoring already-shipped, tested example code is out of
scope for this round.

**Testing:** `lerp_color` gets `src/` TDD tests (endpoints, midpoint,
non-Rgb fallback — same shape as `camera::dim`'s existing tests). The
rest is example code, verified by running.

### Slice 2: Stage Hazards (`examples/smash_crabs.rs`, #64)

```rust
const RAM_STRESS_AMOUNT: f32 = 22.0;
const RAM_DECAY_PER_SEC: f32 = 6.0;
const RAM_THRESHOLD: f32 = 90.0;
const BOBOMB_FLASH_TICKS: u64 = 6; // ~200ms per blink half, matches FLASH_TICKS's cadence

const BOBOMB_ART: [&str; 5] = [
    "  .  ",
    " /   ",
    "( o )",
    "(o o)",
    " \\_/ ",
];
```

`SmashCrabs` gains `sh_ram: f32` (starts `20.0`). CPU is *not* stored
state — it's a pure ambient function of `tick_count`, rendered each frame
as `50.0 + 15.0 * ((tick_count as f32 * 0.03).sin())`, representing a
"boss acting on its own" rather than anything user-controlled.

**Interaction:** `Space` (any time on this screen) adds `RAM_STRESS_
AMOUNT` to `sh_ram`, clamped to `100.0`. `on_tick` decays `sh_ram -=
RAM_DECAY_PER_SEC * elapsed.as_secs_f32()`, clamped to `0.0` — same
gain/decay shape as Artron Energy's `energy` field. `Esc` returns to
Hub.

**Rendering:** a `SmashBorder`-framed panel with two `DamageMeter` rows
— `DamageMeter::new(cpu.round() as u16)` and `DamageMeter::new(sh_ram.
round() as u16)`, each with a text label ("CPU"/"RAM") rendered via
`Text` just before it. `DamageMeter`'s existing white→yellow→red
thresholds (50%/100%) already give RAM a visibly escalating color as it
climbs toward the Bob-omb threshold — no new coloring logic needed.

**Bob-omb:** while `sh_ram >= RAM_THRESHOLD`, `BOBOMB_ART` renders in a
fixed corner (top-right of the panel), flashing between `Color::Red` and
`theme.background` every `BOBOMB_FLASH_TICKS` ticks via `(tick_count /
BOBOMB_FLASH_TICKS).is_multiple_of(2)` — same tick-parity flash idiom
used throughout this session, no RNG.

**Testing:** example code, no `src/` changes beyond Slice 1's — verified
by running, including holding `Space` until RAM crosses 90% and watching
it decay back down and the Bob-omb disappear.

### Slice 3: Boot/intro splash (`examples/smash_crabs.rs`, #65)

```rust
const BOOT_FLASH_MS: u64 = 200;
const BOOT_CLAW_MS: u64 = 800;
const BOOT_TITLE_MS: u64 = 600;
const BOOT_FLARE_MS: u64 = 500;
const BOOT_TOTAL_MS: u64 = BOOT_FLASH_MS + BOOT_CLAW_MS + BOOT_TITLE_MS + BOOT_FLARE_MS; // 2100

const BOOT_TITLE: &str = "S U P E R S M A S H C L A W S"; // vision doc's
    // exact letter-spaced formatting, not "SUPER SMASH CLAWS" solid

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
```

`SmashCrabs` gains `booting: Option<Transition>`, initialized in `new()`
to `Some(Transition::start(Duration::from_millis(BOOT_TOTAL_MS)))` (the
app now always boots on start — `screen` still starts at `Screen::Hub`
so nothing else changes once boot clears). `view()` checks `booting`
before the existing `transitioning_to` check and, while `Some`, renders
`render_boot` instead of the normal screen dispatch. `update()`'s
existing unconditional `q`-to-quit check (already first in the match,
before any screen-specific logic) continues to work during boot
unchanged; no other input is handled while `booting.is_some()`.

**Phase boundaries**, computed from `booting`'s single `Transition`'s
`progress()` (same "one Transition, phase-by-progress-threshold" idiom
as Versus Mode's `render_transition` and Omnitrix's boot). Four
cumulative thresholds, each the running sum of ms-so-far divided by
`BOOT_TOTAL_MS`: `T1 = 200/2100 ≈ 0.095` (flash ends), `T2 =
1000/2100 ≈ 0.476` (claw ends), `T3 = 1600/2100 ≈ 0.762` (title ends),
`T4 = 1.0` (flare ends). Each phase's own sub-progress is `(progress -
T_prev) / (T_next - T_prev)`, clamped to `0..1`.

- **`progress < T1`** (flash): full-screen flash fading from white to
  black — reuse `camera::dim`, same direction Omnitrix's boot already
  uses it (`camera::dim(&white_buf, sub_progress)`), "resolves into" the
  claw as it darkens.
- **`T1 <= progress < T2`** (claw, sub-progress = `(progress - T1) /
  (T2 - T1)`): `CLAW_OPEN` renders centered while sub-progress `< 0.5`,
  `CLAW_CLOSED` once sub-progress `>= 0.5` — the frame flip *is* the
  snap, same 2-frame-swap idea as `ScuttleCursor`'s jerky animation.
  `self.audio.play("snap")` fires exactly once, on the tick where
  sub-progress first crosses `0.5` (tracked via a `bool` comparison
  against the previous tick, not re-fired every frame past the
  threshold).
- **`T2 <= progress < T3`** (title, sub-progress = `(progress - T2) /
  (T3 - T2)`): `CLAW_CLOSED` stays visible above the title; `BOOT_
  TITLE`'s characters split at the midpoint — the first half slides in
  from `x = -half_len` toward its final centered position, the second
  half from `x = area.width + half_len`, both via `easing::ease_out`
  driven by this phase's own sub-progress.
- **`progress >= T3`** (flare, sub-progress = `(progress - T3) / (T4 -
  T3)`): a 3-column-wide band of bright glyphs (alternating `theme.
  accent`/`theme.tertiary`, chosen by `hash(x, tick_count) % 2` — same
  deterministic-hash shape as `braille_noise`, no RNG) sweeps from `x =
  -3` to `x = area.width + 3` as sub-progress goes 0→1. For each screen
  column: if the column is left of the flare band, render the real
  `render_hub` content (composited via the same scratch-`Buffer` +
  `blit` pattern `render_destination_preview` already uses); if inside
  or right of the band, render the boot logo/title still in place. This
  is a left-to-right sweeping reveal, deliberately a different wipe
  shape from the existing circular Hub→Versus wipe (matches "sweeps
  across... burning away" rather than reusing that shape verbatim).

Once `booting`'s `Transition::is_complete()`, `on_tick` sets `booting =
None` and normal `Screen::Hub` rendering/input takes over.

**Audio:** `RodioAudioSink::play` gains one new `event_id` match arm,
`"snap"`, at `110.0` Hz — distinct from and lower than the existing
`"cursor"` (440), `"select"` (660), `"hit"` (220) tones, matching the
vision doc's "heavy SNAP sound." No second event for the lens flare —
the vision doc only calls for audio on the claw snap, so that's the only
new cue this slice adds.

**Testing:** example code, no `src/` changes — verified by running,
confirming the full ~2.1s sequence (flash → claw open→closed snap with
audible tone → title sliding in from both edges spelling `BOOT_TITLE`
exactly → flare band sweeping left to right revealing the real Hub
underneath) and that `q` still quits immediately during any boot phase.

## Verification

- `cargo test --lib` (covers the new `easing::lerp_color` tests; every
  other existing test stays green).
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`.
- `cargo run --example smash_crabs`: confirm the boot sequence plays in
  full on launch and lands on the Hub; confirm Target Smash shows all 5
  targets, smashing one shakes the screen, overlays `💥`×3 + a "KO"
  stamp over its row, then fades the row out and removes it, repeatable
  until "ALL TARGETS DOWN"-style empty state (no remaining rows, cursor
  simply has nothing to move over); confirm Stage Hazards' CPU meter
  ambiently wobbles on its own, `Space` spikes RAM and it decays back
  down over a few seconds, and the Bob-omb art appears flashing red at
  ≥90% RAM and disappears once it decays back under; confirm `Esc`
  returns to the Hub from both sub-apps (blocked mid-smash-animation)
  and `q` quits cleanly from every state including mid-boot.
