# Example-Apps Roadmap — Design

**Status:** draft, pending your review before we move to planning.
**Date:** 2026-08-06
**Relationship to prior specs:** this is a planning/backlog spec, not a
code-architecture one. It sits downstream of
`2026-08-05-ttui-rev-b-vision-alignment-design.md` (which validated the
tick/theme mechanism via Omnitrix) and
`2026-08-05-buffer-layering-compositing-design.md` +
`2026-08-05-buffer-layering-followups-design.md` (which shipped
`LayerStack`/`composite()` and the Smash Crabs example, PR #27/#32,
now merged to `main`). Those specs are unchanged and fully implemented.
This spec does not add or change any core capability itself — it
decomposes the remaining distance between "current example state" and
"full vision doc" into a wave-ordered ticket backlog, per example.

## Context / Motivation

Three independent vision documents
(`D:\Dev\Projects\TTUI-Ideas\vision\UI\idea-{1,2,3}-*.md` — Omnitrix,
Super Smash Crabs, TARDIS) each describe a full themed OS-shell: a
navigation hub, three themed sub-apps (an agent/chat interface, a
task-list app, a system dashboard), a boot sequence, and an app-switch
transition. Current implementation state, against that target:

- **`examples/omnitrix.rs`** — one screen: a themed, tick-driven
  pulsing border (the "breathing" glow), no layering, no sub-apps, no
  hub, no boot sequence.
- **`examples/smash_crabs.rs`** — one screen: three composited
  `LayerStack` layers (background arena, top-left HP panel, a
  center-screen hit-flash effect), no hub, no sub-apps, no tweening,
  no particles, no screen-shake, no audio.
- **`examples/tardis.rs`** — does not exist. TARDIS's two headline
  needs (a large virtual buffer + `Camera` viewport, and a decaying
  Glitch Buffer overlay) were both explicitly deferred by Rev B pending
  the Omnitrix tick-mechanism validation, which has since shipped —
  nothing has picked this up yet.
- **`examples/demo.rs`** — the plain, un-themed Rev A reference app
  (List/Table/Block/focus-cycling). Predates tick/theme/layering and
  uses none of it; retired as part of this backlog (see "Housekeeping"
  below) since the three themed examples now cover its demonstrative
  role and more.

Each vision doc invents its rendering assumptions independently (none
reference a shared architecture, per Rev B's own observation), so
turning "vision doc prose" directly into work risks each example
re-inventing capabilities (tweening, particles, audio, screen-shake)
the others also need. This spec's job is to (a) name and wave-order the
gap between current state and full vision scope for each example, (b)
pull out the capabilities 2+ examples need into a shared arc built
once, and (c) produce a concrete backlog ready to become filed GitHub
issues.

## Scope of this spec

**Committed and designed here:** the wave-ordered ticket backlog
itself — one arc per example plus a shared cross-cutting arc, one wave
per arc (core -> structural -> architectural -> features+polish, per
the user's requested ordering), one ticket per backlog item, and the
traceability mapping from each ticket back to the vision-doc language
it implements.

**Explicitly not designed here:** the actual technical design of any
individual ticket (e.g. the exact `Cell` style-field shape, the
particle system's API, the `Camera` struct's fields). Per this
project's TDD/design convention, each ticket gets its own
`/superpowers:brainstorm` -> spec -> plan cycle before any code is
written for it — this spec only establishes *that it exists, why, and
in what order*, not *how*.

**Full-vision-scope decision:** the target this backlog aims at is
each vision doc's full scope (all three themed sub-apps, hub
navigation, boot sequence, app-switch transition per example) — not
merely polish on the current single-scene implementations. This was an
explicit choice (over "deepen the current scene only," or "current
scene + one more screen") made to avoid re-scoping this backlog again
partway through. The resulting backlog is large (~40 issues) by design.

## Design

### Arc / Slice / Task shape

Per `docs/design/README.md`'s existing structure: one **Arc** per
example (plus one cross-cutting Arc for shared capabilities), one
**Slice** per wave, one **Task** per backlog ticket. Every Task in the
downstream plan doc is `git-adjacent`/`admin`-tagged — filing an issue
is the unit of work here, not writing code, so no TDD applies to *this*
plan. TDD applies later, per `.claude/rules/development-conventions.md`,
when an individual ticket is picked up for implementation through its
own brainstorm/spec/plan cycle.

Each filed issue's body states: what it is, which wave/arc it's in and
why (what it depends on), and which vision-doc passage motivates it —
enough to seed that future brainstorm without pre-designing it.

**Sequencing:** the Shared Core Capabilities arc lands first (the three
example arcs consume it); after that, the example arcs are independent
of each other and can proceed in any order. Recommended order —
Omnitrix, then Smash Crabs, then TARDIS — is least-to-most net-new work
(TARDIS has zero implementation today). Within an arc, waves are
strictly sequential: a later wave's tickets assume the earlier wave's
tickets are done (e.g. Smash Crabs' "screen-shake on impact" ticket
assumes the Shared Core arc's screen-shake helper already landed).

**Labeling:** `.claude/rules/code-forge.md` and
`.claude/rules/git-github-standards.md` are explicit stubs with no
label taxonomy decided. Issues from this backlog are filed **without
custom labels** (title + body only) rather than inventing a scheme as a
side effect of this spec; resolving a label taxonomy stays a separate,
explicitly flagged future brainstorm in `code-forge.md`.

**Tracking issues:** one lightweight checklist issue per arc (Shared
Core, Omnitrix, Smash Crabs, TARDIS) plus one for the Housekeeping
item — 5 tracking issues total, each body a checklist of its arc's
tickets, back-filled with real issue links once the child issues exist.

### Arc 0: Shared Core Capabilities

Capabilities needed by 2+ examples, designed once and consumed by
whichever example's wave needs them — mirrors how `Theme`/`tick_rate`
were built once for Omnitrix (Rev B) and reused unchanged by Smash
Crabs (buffer-layering follow-ups spec).

| Slice | Ticket | Needed by | Vision-doc source |
|---|---|---|---|
| Rendering primitives | `Cell` gains a `style`/attribute field (bold at minimum) + terminal-writer support | Omnitrix, Smash Crabs | Omnitrix: "glow effects achieved by layering ANSI bold text over bright color variants"; Smash Crabs: "Bold, heavy monospace... Text should feel loud" |
| Rendering primitives | Whole-buffer screen-shake helper (decaying random X/Y cell offset applied to a composited `Buffer` for N ticks) | Smash Crabs (required), TARDIS (usable) | Smash Crabs: "screen shake (intentionally offsetting the entire render buffer by 1-2 cells for a few frames on impact)" |
| Animation primitives | `Easing`/tween helper (linear + ease-out interpolation over a `Duration`) | Smash Crabs (required), TARDIS (usable for camera pans) | Smash Crabs: "coordinates are tweened over ~150ms," "ballooning hover effects" |
| Animation primitives | Lightweight particle system (spawn/update/render chars with velocity, lifespan, color decay) | Smash Crabs, TARDIS | Smash Crabs: "sweeping lens flares," impact bursts; TARDIS: "Particle System... sparks, temporal energy, and venting plasma" |
| App-switch transition hook | Generic tick-driven "transition" hook in `App`/`app.rs::run()` | Omnitrix, Smash Crabs, TARDIS | Omnitrix: buffer-corruption app-switch; Smash Crabs: VS-screen/wipe app-switch; TARDIS: camera-flight app-switch — three bespoke effects, one shared mechanism |
| Audio hook | Optional `rodio`-based sound-event hook (feature-gated; evaluated explicitly against Rev A's single-dependency posture since it's a second dependency) | Smash Crabs, TARDIS | Both vision docs: "Audio Engine (Highly Recommended/Optional)... integration via `rodio`" |

### Arc 1: Omnitrix

| Slice | Ticket |
|---|---|
| Core | Adopt the shared `Cell` style/bold primitive for the glow border once Arc 0 lands |
| Structural | Faceplate dial-navigation hub (scrollable DNA-sample list, "slam to launch") |
| Structural | `AppMode` state enum (Faceplate / Brainstorm / Fasttrack / Upgrade) driving which sub-app `view()` renders |
| Architectural | App-switch "corruption" transition (buffer glitch on switch) using Arc 0's transition hook |
| Architectural | Custom 2-char-thick Omnitrix border renderer extending `BorderSet` |
| Features+Polish | `EnergyCore` widget (fluid-fill progress bar + spark glyphs) |
| Features+Polish | `DNAConsole` widget (styled text input) |
| Features+Polish | "Brainstorm" (Agent Interface) sub-app screen — flickering "neural activity" border |
| Features+Polish | "Fasttrack" (Productivity) sub-app screen — lock-on completion animation + circular timer rings |
| Features+Polish | "Upgrade" (Dashboard) sub-app screen — circuit-lighting resource meters + red-flash overload state |
| Features+Polish | Boot/intro splash (hourglass fade-in -> flash -> circuit trace-out) |

### Arc 2: Smash Crabs

| Slice | Ticket |
|---|---|
| Core | Wire the current single-scene arena into Arc 0's tween/particle/screen-shake/audio primitives as they land (scaffolding, not new visuals yet) |
| Structural | Character-select grid hub (4x3/5x4 portrait grid + `ScuttleCursor` navigation) |
| Structural | State enum for Versus / Target Smash / Stage Hazards screens |
| Architectural | Generalize today's 3-layer (background/UI/effects) arena into reusable per-screen layer scaffolding for all three sub-apps |
| Architectural | `ScuttleCursor` jerky two-frame tween movement (Arc 0 easing primitive) |
| Architectural | Screen-shake on impact events (Arc 0 screen-shake helper) |
| Architectural | VS transition (2s VS screen + slam + circle-wipe) via Arc 0's transition hook |
| Features+Polish | `DamageMeter` widget (count-up %, white->yellow->red) |
| Features+Polish | `SmashBorder` widget (2-3 char thick beveled border) |
| Features+Polish | "Versus Mode" (Agent Interface) sub-app — token-hit flash + attack animation + `DamageMeter` token count |
| Features+Polish | "Target Smash" (Productivity) sub-app — KO stamp + fade-out on task completion |
| Features+Polish | "Stage Hazards" (Dashboard) sub-app — Bob-omb warning art at 90% RAM |
| Features+Polish | Boot/intro splash (claw-snap flash + logo slide + lens-flare sweep) |
| Features+Polish | Audio cues (cursor click / selection smack / impact) via Arc 0's audio hook |

### Arc 3: TARDIS

| Slice | Ticket |
|---|---|
| Core | Virtual buffer + `Camera` (x, y, zoom) blit-viewport helper — per Rev B's recorded direction, this lives at app/library-helper level, not core framework machinery |
| Core | Glitch Buffer overlay — a decaying overlay built on `LayerStack`, decay driven by app-space `on_tick`, per Rev B's app-space boundary |
| Structural | Hexagonal console hub — six-face navigation state, rotate on arrow keys, active-face foregrounding |
| Structural | Perspective/depth shading transform (dim+shrink toward viewport edges) as a per-cell transform during the camera blit |
| Architectural | Camera-pan "flight" transition between console faces/rooms (Arc 0 transition hook + camera target animation) |
| Architectural | Glitch Buffer wiring into error/lag states |
| Features+Polish | `Roundel` widget (pulsing circular data nodes) |
| Features+Polish | `AnalogToggle` widget (lever-style checkbox) |
| Features+Polish | `TimeRotor` widget (central pulsing Braille column, speeds up under load) |
| Features+Polish | "Psychic Paper" (Agent Interface) sub-app — ink-bleed token reveal + Perception Filter glitch on error |
| Features+Polish | "Star Charts" (Productivity) sub-app — timeline nodes (past/present/future) + Temporal Shift on completion |
| Features+Polish | "Artron Energy" (Dashboard) sub-app — pipe-flow resource meters + plasma-venting warning + frame-rate-drop-on-lag effect |
| Features+Polish | Boot/materialization sequence (Police Box glitch-shake -> doors open -> camera push-through) |
| Features+Polish | Audio cues (TARDIS hum loop + materialization sound) via Arc 0's audio hook |

### Housekeeping

| Ticket |
|---|
| Retire `examples/demo.rs` (remove file + any build/README references) |

### Traceability check

Every named component in each vision doc's "UI/UX Components &
Layouts" and "Framework Architecture" sections maps to at least one
ticket above:

- **Omnitrix:** EnergyCore, DNAConsole, Omnitrix Borders, Faceplate hub,
  Brainstorm/Fasttrack/Upgrade sub-apps, Recharge Pulse (already
  shipped via Rev B), Transition state/corruption, boot sequence — all
  present above.
- **Smash Crabs:** SmashBorder, DamageMeter, ScuttleCursor, Smash Menu
  hub (character-select grid), Versus/Target Smash/Stage Hazards
  sub-apps, layered buffers (already shipped), easing/tweening,
  screen-shake, audio, Z-indexing (already covered by `LayerStack`),
  boot sequence — all present above.
- **TARDIS:** Roundel, AnalogToggle, TimeRotor, Hexagonal Console hub,
  Psychic Paper/Star Charts/Artron Energy sub-apps, Camera/virtual
  buffer, Glitch Buffer, particle system (shared with Smash Crabs via
  Arc 0), perspective shading, boot/materialization sequence, audio —
  all present above.

Nothing from the three vision docs was found unmapped.

## Testing

Not applicable — this spec produces a ticket backlog (an `admin`/
`git-adjacent`-tagged planning artifact per
`docs/design/README.md`'s tag system), not code. TDD applies
individually to each ticket when it's picked up for implementation.

## Critical files

- `docs/design/specs/2026-08-06-example-apps-roadmap-design.md` — this
  file.
- `docs/design/plans/2026-08-06-example-apps-roadmap-plan.md` — the
  downstream Arc/Slice/Task plan (one task per ticket above), produced
  next via `superpowers:writing-plans`.
- No `src/` or `examples/` changes in this spec — planning only.

## Verification

- Self-review (below) confirms no placeholders, no contradictions
  between Arc 0 and the example arcs' "adopt shared primitive" tickets,
  and full vision-doc traceability (see "Traceability check" above).
- Downstream: the Arc/Slice/Task plan turns each ticket into a `gh
  issue create` call; `gh issue list --limit 50` after filing should
  show all ~40 issues (5 tracking + ~36 ticket issues).
