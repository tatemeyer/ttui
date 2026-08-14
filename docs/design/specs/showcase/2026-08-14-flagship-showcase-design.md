# Flagship Showcase Design

**Status:** approved (brainstorming complete 2026-08-14)

Sub-project #5 of the TTUI v1.0.0 initiative — the last content Arc
before #6 (cutting the v1.0.0 tag). Builds a curated, mascot-hosted
showcase that pulls together five techniques already proven separately
across the example apps (mouse interaction, particles, camera+glitch,
chord input, data-viz) into one flagship piece, positioned as the
framework's primary demo reel rather than another cataloged example.

## Purpose

`examples/` already demonstrates every technique individually, spread
across `control_panel`, `falcon`, `mission_control`, and others. This
Arc doesn't add a new technique — it adds a single, polished entry
point that shows five of them together, hosted by a mascot, as
something you'd point someone to first. It is explicitly **not**
another vision-doc app and explicitly **not** absorbed into
`examples/launcher`'s cross-app portal system (that already does
whole-app switching between the six vision-doc apps; this is a
different, narrower thing).

## Entry point

A new top-level `showcase/` directory, sibling to `src/`, `examples/`,
and `tools/`, with its own Cargo `[[bin]]` target:

```toml
[[bin]]
name = "showcase"
path = "showcase/main.rs"
```

Run via `cargo run --bin showcase`, mechanically distinct from
`cargo run --example <name>`. `showcase/main.rs` is a thin entry point
matching the existing `examples/<app>/main.rs` pattern.

## Architecture

Follows the established single-`App`-struct-with-sub-screens pattern
already used by `examples/omnitrix` (see `examples/omnitrix/omnitrix.rs`)
rather than introducing a new trait-object plugin system:

```
showcase/
  main.rs           — thin entry, constructs ShowcaseApp and runs it
  showcase.rs        — ShowcaseApp struct, Screen enum, top-level
                       view()/on_tick()/input dispatch (implements
                       ttui::app::App)
  boot.rs            — startup materialization sequence
  menu.rs            — tile grid: rendering + hit-testing + navigation
  mascot.rs          — GripperMascot: pixel-grid rendering + state
  mouse_grab.rs       — vignette 1: Assembly Line
  particle_vent.rs    — vignette 2: Overload Vent
  camera_glitch.rs     — vignette 3: Diagnostic Scan
  chord_override.rs    — vignette 4: Override Sequence
  telemetry.rs         — vignette 5: Telemetry
```

```rust
enum Screen {
    Menu,
    Vignette(VignetteId),
}

#[derive(Clone, Copy, PartialEq)]
enum VignetteId {
    AssemblyLine,
    OverloadVent,
    DiagnosticScan,
    OverrideSequence,
    Telemetry,
}
```

`ShowcaseApp::view()`/`on_tick()` match on `self.screen` and delegate to
the boot/menu/vignette renderer, mirroring how `Omnitrix::view()`
matches on `AppMode`. Each vignette owns its own state struct
(constructed on entry, dropped on exit) rather than the top-level
struct carrying every vignette's fields permanently — cheaper to reason
about and matches how `omnitrix`'s sub-apps are scoped, modulo that
`omnitrix` happens to keep its sub-app state inline; either is
consistent with this codebase's existing patterns, and per-vignette
state structs are the better fit here since vignettes are mutually
exclusive and transient rather than persistent across the whole app
lifetime.

**Navigation:** `Esc` returns from any vignette to the menu early; every
vignette also auto-returns to the menu on completion (see per-vignette
durations below) with no user action required. `q` at the menu quits
the whole app. There is no `F12`/nexus-return mechanic — `showcase` is
not part of `examples/launcher`'s portal system.

**Boot sequence:** a short (~1200ms) materialization on startup, before
the menu appears — matching `falcon` (1400ms) and `tardis`'s existing
visual identity, giving `showcase` the same "arrival" feel as every
themed app rather than a bare instant menu.

## The mascot: `GripperMascot`

Renders as a fixed 12×12-cell region using solid-color `Cell`s — one
`Cell { symbol: ' ', bg: <color>, alpha: 1.0, ..Default::default() }`
per filled pixel, the same bg-fill technique `list.rs`/`block.rs`
already use for row highlighting (`bar_chart.rs`'s `'█'`-glyph
approach is the other proven option in this codebase; bg-fill is used
here because it doesn't depend on a specific glyph's rendered shape
matching a full cell). Palette-`0` entries are skipped entirely so the
background shows through — no cell is drawn for them.

Palette:

| Code | Hex | Role |
|---|---|---|
| 0 | — | transparent (skip) |
| 1 | `#2a2a2a` | dark trim / joints |
| 2 | `#8a8f98` | metallic body |
| 3 | `#ff8c42` | claw accent |
| 4 | `#5fd4ff` | LED visor |
| 6 | `#c7cbd1` | bolt highlight |

Three frames (12 cols × 12 rows each, row-major, values above):

```
idle:
[0,0,0,0,0,1,0,0,0,0,0,0]
[0,0,0,0,0,1,0,0,0,0,0,0]
[0,6,2,2,2,2,2,2,2,6,0,0]
[0,2,2,4,4,4,4,4,4,2,2,0]
[0,2,2,2,2,2,2,2,2,2,2,0]
[0,0,2,2,2,2,2,2,2,2,0,0]
[6,2,2,2,2,2,2,2,2,2,2,6]
[0,0,2,2,2,2,2,2,2,2,0,0]
[0,0,0,1,2,2,2,2,1,0,0,0]
[0,0,0,0,1,2,2,1,0,0,0,0]
[0,0,0,0,0,3,3,0,0,0,0,0]
[0,0,0,3,3,0,3,3,0,0,0,0]

reacting (visor band shifts inward):
[0,0,0,0,0,1,0,0,0,0,0,0]
[0,0,0,0,0,1,0,0,0,0,0,0]
[0,6,2,2,2,2,2,2,2,6,0,0]
[0,2,2,2,4,4,4,4,2,2,2,0]
[0,2,2,2,2,2,2,2,2,2,2,0]
[0,0,2,2,2,2,2,2,2,2,0,0]
[6,2,2,2,2,2,2,2,2,2,2,6]
[0,0,2,2,2,2,2,2,2,2,0,0]
[0,0,0,1,2,2,2,2,1,0,0,0]
[0,0,0,0,1,2,2,1,0,0,0,0]
[0,0,0,0,0,3,3,0,0,0,0,0]
[0,0,0,3,3,0,3,3,0,0,0,0]

grabbing (claw closes):
[0,0,0,0,0,1,0,0,0,0,0,0]
[0,0,0,0,0,1,0,0,0,0,0,0]
[0,6,2,2,2,2,2,2,2,6,0,0]
[0,2,2,4,4,4,4,4,4,2,2,0]
[0,2,2,2,2,2,2,2,2,2,2,0]
[0,0,2,2,2,2,2,2,2,2,0,0]
[6,2,2,2,2,2,2,2,2,2,2,6]
[0,0,2,2,2,2,2,2,2,2,0,0]
[0,0,0,1,2,2,2,2,1,0,0,0]
[0,0,0,0,1,2,2,1,0,0,0,0]
[0,0,0,0,0,3,3,0,0,0,0,0]
[0,0,0,0,3,3,3,3,0,0,0,0]
```

**Behavior:** idles beside the tile menu, playing `reacting` briefly
whenever the highlighted tile changes via the arrow keys, then
returning to `idle`. (Not hover-triggered — `Terminal::next_event`
filters out `Moved`/`Drag` mouse events before any app sees them, so
hover-to-select isn't reachable without a framework change; the Menu
section below already correctly scopes highlight-changing input to
arrow keys and a direct click.) `grabbing` plays once at the moment Assembly
Line's vignette successfully catches a crate (the mascot is rendered
both beside the menu and inside that specific vignette, reusing the
same frames — not two separate mascot implementations). Frame
transitions are instant (poses hold, not tweened) — none of the other
apps interpolate between discrete poses either, so this stays
consistent rather than introducing a new animation primitive for one
widget.

## Menu

A single row of 5 tiles at the `100x30` size this project's other apps
use, each showing the vignette's title and a one-line hint (e.g.
"Assembly Line — click"). Arrow keys move the highlight; Enter or a
direct mouse click on a tile launches its vignette. Hover alone never
launches — avoids accidental triggers, matching this project's existing
click-to-confirm conventions (`control_panel`'s buttons behave the same
way). The mascot sits beside the tile row, per above.

## The five vignettes

Each auto-plays once launched and auto-returns to the menu on
completion; `Esc` skips back early from any of them. None of the five
literally embeds an existing example's UI — each is new content built
on the same underlying toolkit (`particles.rs`, `glitch.rs`,
`camera.rs`/`perspective.rs`, `input.rs`'s chord binder, the
`sparkline`/`bar_chart` widgets) that those examples also use.

1. **Assembly Line** (mouse) — crates scroll across a conveyor row on a
   timer; clicking a crate before it exits triggers the mascot's
   `grabbing` frame plus a small particle puff (`particles.rs`) where
   it was caught. 6 crates spawn total; the vignette auto-completes
   when the last one exits or is caught, whichever comes last — no
   score/game-over state, catching is just the visual payoff. Reuses
   `control_panel`'s click hit-testing pattern.

2. **Overload Vent** (particles) — the mascot's shoulder/joint cells
   vent multiple simultaneous particle emitters (a heavier, multi-
   source burst pattern than `control_panel`'s single-button burst) for
   a fixed ~3.5s, then auto-returns. No interaction required —
   exercises `particles.rs` more fully than any existing example does.

3. **Diagnostic Scan** (camera + glitch) — a rotating 3D wireframe
   schematic of the gripper's arm (`camera.rs`/`perspective.rs`, same
   family as `falcon`'s canopy projection) auto-rotates continuously;
   a `GlitchBuffer` glitch triggers automatically twice during the
   sequence (percussive-maintenance style, matching `falcon`'s
   mechanic) — `Space` "whacks" it clear early if the user chooses to,
   otherwise it clears on its own after the glitch's duration. Auto-
   completes after the second glitch clears.

4. **Override Sequence** (chord input) — the vignette displays a target
   chord (`Left, Right, Left, Right` — deliberately distinct from
   `falcon`'s `Up,Up,Down,Down`, so this isn't just falcon's chord
   copy-pasted) and waits for it via `input.rs`'s chord binder. On
   success: a `GlitchBuffer::with_alpha`-driven power-up flash plays
   (exercising the pre-v1 fix wave's new builder method) and the
   mascot holds `reacting` briefly as a "triumphant" beat, then
   auto-returns after ~1.5s. No timeout on entry — the vignette simply
   waits (wrong keys reset the chord binder's own progress, same as
   `falcon`'s).

5. **Telemetry** (data-viz) — live sparklines (Grip Force, Servo Load)
   and a bar chart (Power Draw, Efficiency) animate via the same
   deterministic-random-walk approach `mission_control` uses, for a
   fixed ~5.5s, then auto-returns.

## Testing / verification

TDD stays mandatory for any genuinely new reusable `src/`-level logic
this Arc introduces; the vignette/app code itself (`showcase/*.rs`)
falls under `development-conventions.md`'s "Examples/demos" TDD
exemption by spirit — it's demo code verified by running and visually
reviewing it, not by assertion, the same as every `examples/*.rs` app,
even though it now lives in a new top-level bin target rather than
literally under `examples/`.

`tools/visual-snapshot` capture + `Read`-and-review is mandatory before
this Arc's PR merges, covering: the boot sequence, the menu (idle and
with a tile highlighted), and all 5 vignettes — following
`development-conventions.md`'s existing "Visual review" convention by
spirit (that section's file list predates this Arc; `showcase/`'s
`view()`/`on_tick()` is the same category of rendering-affecting code
its existing entries already cover).

The human-only real-TTY checklist (per `development-conventions.md`'s
real-TTY-tests convention) covers Assembly Line (mouse) and Override
Sequence (chord input) specifically, since those are the two vignettes
with genuine interactive input — same pattern already used for
`control_panel` and `falcon`.

## Out of scope

- No sound. `src/audio.rs` exists but none of the five vignettes
  require it; adding audio here would be new scope this Arc doesn't
  need.
- No persistence/config (e.g. "skip boot next time") — every other
  themed app starts fresh each run; `showcase` matches that.
- Not reachable from `examples/launcher`'s portal nexus, and
  `examples/README.md` is not updated to index it — both deliberate,
  per the "outside the examples/ catalog" framing this Arc was scoped
  under.
- `showcase/` is not part of `ttui`'s public API surface, so
  `code-forge.md`'s SemVer policy doesn't apply to it (same as
  `examples/` and `tools/visual-snapshot`).
