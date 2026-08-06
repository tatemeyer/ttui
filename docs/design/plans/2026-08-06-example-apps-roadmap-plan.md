# Example-Apps Roadmap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** File the ~40-issue GitHub backlog derived from
`docs/design/specs/2026-08-06-example-apps-roadmap-design.md` — one
tracking issue per arc plus one issue per ticket — so future sessions
have a concrete, prioritized queue for growing Omnitrix, Smash Crabs,
and TARDIS toward their full vision-doc scope, and for retiring
`examples/demo.rs`.

**Architecture:** No source changes. Every task in this plan is
`git-adjacent`/`admin`-tagged: its "step" is a `gh issue create` (and,
for tracking issues, a follow-up `gh issue edit` to back-fill real
issue links) against `tatemeyer/ttui`, not a code change. No TDD
applies to this plan itself, per `.claude/rules/development-conventions.md`'s
TDD scope (this is pure git-adjacent/admin work, not `coding`-tagged).
Each ticket issue's body states its wave, its dependencies, its
vision-doc source, and a pointer back to the roadmap spec — enough to
seed that ticket's own future `/superpowers:brainstorm` pass, not a
full design.

**Tech Stack:** `gh` CLI (already authenticated as `tatemeyer`, repo
`tatemeyer/ttui` confirmed via `gh auth status` / `git remote -v`). No
new dependencies.

## Global Constraints

- Repo: `tatemeyer/ttui`. All `gh` commands below take `--repo
  tatemeyer/ttui` explicitly.
- No custom labels — `.claude/rules/code-forge.md` and
  `git-github-standards.md` are stubs with no label taxonomy decided;
  issues are filed title+body only.
- Arc order: Arc 0 (Shared Core) first, since the other arcs reference
  it; Arcs 1-3 (Omnitrix, Smash Crabs, TARDIS) and the Housekeeping
  item can then be filed in any order — this plan lists them in the
  recommended Omnitrix -> Smash Crabs -> TARDIS sequence.
- Within an arc, ticket issues are filed in wave order (core ->
  structural -> architectural -> features+polish) before that arc's
  tracking issue, so the tracking issue's checklist can reference real
  issue numbers rather than placeholders.
- Every ticket issue body ends with the same boilerplate pointer:
  `Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly.`
- After each arc's ticket issues are filed, record the returned issue
  numbers (from `gh issue create`'s output URL) to build that arc's
  tracking-issue checklist.

---

## Arc 0: Shared Core Capabilities

### Task 0.1 (Slice: Rendering primitives) — `Cell` style/attribute field

- [ ] **File the issue**

```
gh issue create --repo tatemeyer/ttui \
  --title "core: add style/attribute field to Cell (bold at minimum)" \
  --body "Wave: Core — Arc 0 (Shared Core Capabilities). Depends on: none.

Cell (src/buffer.rs) currently has only {symbol, fg, bg} — no style/attribute field. Add at least a bold flag (extensible to underline/dim later) plus terminal-writer support in src/terminal.rs's draw_diff.

Needed by: Omnitrix (glow via 'ANSI bold text over bright color variants', per idea-1-omnitrix.md) and Smash Crabs ('Bold, heavy monospace... Text should feel loud', per idea-2-SuperSmashCrabs.md).

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 0.2 (Slice: Rendering primitives) — screen-shake helper

- [ ] **File the issue**

```
gh issue create --repo tatemeyer/ttui \
  --title "core: add whole-buffer screen-shake helper" \
  --body "Wave: Core — Arc 0 (Shared Core Capabilities). Depends on: none.

Add a helper that applies a decaying random X/Y cell offset to a composited Buffer for N ticks (a render-time transform, not a Cell-level change).

Needed by: Smash Crabs (required — 'screen shake: intentionally offsetting the entire render buffer by 1-2 cells for a few frames on impact'), usable by TARDIS for turbulence-style effects.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 0.3 (Slice: Animation primitives) — easing/tween helper

- [ ] **File the issue**

```
gh issue create --repo tatemeyer/ttui \
  --title "core: add Easing/tween helper (linear + ease-out over a Duration)" \
  --body "Wave: Core — Arc 0 (Shared Core Capabilities). Depends on: none.

Add a small tweening helper: given a start value, end value, Duration, and elapsed time, return the interpolated value (linear and ease-out at minimum).

Needed by: Smash Crabs (required — 'coordinates are tweened over ~150ms', 'ballooning hover effects'), reusable by TARDIS for camera pans between console faces/rooms.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 0.4 (Slice: Animation primitives) — particle system

- [ ] **File the issue**

```
gh issue create --repo tatemeyer/ttui \
  --title "core: add lightweight particle system" \
  --body "Wave: Core — Arc 0 (Shared Core Capabilities). Depends on: none.

Add a lightweight particle system: spawn/update/render characters with velocity, lifespan, and color decay, driven by the existing on_tick hook.

Needed by: Smash Crabs ('sweeping lens flares', impact bursts) and TARDIS ('Particle System... essential for sparks, temporal energy, and venting plasma').

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 0.5 (Slice: App-switch transition hook) — generic transition hook

- [ ] **File the issue**

```
gh issue create --repo tatemeyer/ttui \
  --title "core: add generic tick-driven app-switch transition hook" \
  --body "Wave: Core — Arc 0 (Shared Core Capabilities). Depends on: none.

Add a generic tick-driven 'transition' hook in App/app.rs::run() so app-switch effects share one mechanism instead of three bespoke ones.

Needed by: Omnitrix (buffer-corruption app-switch), Smash Crabs (VS-screen/circle-wipe app-switch), TARDIS (camera-flight app-switch) — three different visual effects, one shared trigger mechanism.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 0.6 (Slice: Audio hook) — optional rodio-based sound-event hook

- [ ] **File the issue**

```
gh issue create --repo tatemeyer/ttui \
  --title "core: evaluate optional rodio-based audio hook" \
  --body "Wave: Core — Arc 0 (Shared Core Capabilities). Depends on: none.

Add an optional, feature-gated sound-event hook via rodio. Must explicitly weigh this against Rev A's single-dependency (crossterm-only) posture (docs/design/specs/2026-08-04-ttui-core-framework-design.md) since it's a second dependency — the brainstorm for this ticket should reach an explicit decision, not assume yes.

Needed by: Smash Crabs and TARDIS (both vision docs: 'Audio Engine (Highly Recommended/Optional)... integration via rodio').

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 0.7 — Arc 0 tracking issue

- [ ] **File the tracking issue, then edit it in with the real issue numbers from 0.1-0.6**

```
gh issue create --repo tatemeyer/ttui \
  --title "[Tracking] Shared Core Capabilities arc" \
  --body "Tracks the Shared Core Capabilities arc from docs/design/specs/2026-08-06-example-apps-roadmap-design.md — capabilities needed by 2+ example arcs, built once.

- [ ] core: add style/attribute field to Cell (bold at minimum)
- [ ] core: add whole-buffer screen-shake helper
- [ ] core: add Easing/tween helper (linear + ease-out over a Duration)
- [ ] core: add lightweight particle system
- [ ] core: add generic tick-driven app-switch transition hook
- [ ] core: evaluate optional rodio-based audio hook"
```

- [ ] Replace each checklist line above with `- [ ] #<issue-number> <title>` once 0.1-0.6 are filed, via `gh issue edit <tracking-issue-number> --body "..."`.

---

## Arc 1: Omnitrix

### Task 1.1 (Slice: Core)

```
gh issue create --repo tatemeyer/ttui \
  --title "omnitrix: adopt Cell style/bold for the glow border" \
  --body "Wave: Core — Arc 1 (Omnitrix). Depends on: Arc 0 Task 0.1 (Cell style field).

Once Cell gains a style/bold field, use it in Omnitrix's border/theme rendering to match the vision doc's 'glow via bold text over bright color variants' more closely than color-pulsing alone.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 1.2 (Slice: Structural)

```
gh issue create --repo tatemeyer/ttui \
  --title "omnitrix: build the Faceplate dial-navigation hub" \
  --body "Wave: Structural — Arc 1 (Omnitrix). Depends on: none.

Build the 'Faceplate': a scrollable list of DNA-sample apps (Brainstorm/Fasttrack/Upgrade), triggered by Tab/Space, with 'slam the dial down' to launch the selected sample. Per idea-1-omnitrix.md's Main Hub / Navigation section.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 1.3 (Slice: Structural)

```
gh issue create --repo tatemeyer/ttui \
  --title "omnitrix: add AppMode state enum for Faceplate/Brainstorm/Fasttrack/Upgrade" \
  --body "Wave: Structural — Arc 1 (Omnitrix). Depends on: Task 1.2 (Faceplate hub).

Add an AppMode enum (Faceplate/Brainstorm/Fasttrack/Upgrade) in app state driving which sub-app view() renders, following the existing Focus-in-app-state pattern (examples/demo.rs, no framework-side navigation manager per Rev B's 'Not a gap' section).

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 1.4 (Slice: Architectural)

```
gh issue create --repo tatemeyer/ttui \
  --title "omnitrix: app-switch 'corruption' transition" \
  --body "Wave: Architectural — Arc 1 (Omnitrix). Depends on: Arc 0 Task 0.5 (transition hook), Task 1.3 (AppMode).

Implement the app-switch transition: screen flashes Hazard Yellow, a cascade of Braille patterns rolls down the screen, new layout snaps in beneath the cleared wave. Per idea-1-omnitrix.md's 'App Switch Animation'.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 1.5 (Slice: Architectural)

```
gh issue create --repo tatemeyer/ttui \
  --title "omnitrix: custom 2-char-thick border renderer" \
  --body "Wave: Architectural — Arc 1 (Omnitrix). Depends on: none (extends existing Theme/BorderSet).

Extend BorderSet/Block rendering to support a 2-character-thick border for the 'heavy, plastic/metallic toy-box' Omnitrix border look, per idea-1-omnitrix.md's 'Omnitrix Borders' component.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 1.6 (Slice: Features+Polish)

```
gh issue create --repo tatemeyer/ttui \
  --title "omnitrix: EnergyCore widget" \
  --body "Wave: Features+Polish — Arc 1 (Omnitrix). Depends on: none.

Add the EnergyCore widget: replaces standard progress bars, fills with green fluid and spark glyphs when full. Per idea-1-omnitrix.md's Global/Shared Components.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 1.7 (Slice: Features+Polish)

```
gh issue create --repo tatemeyer/ttui \
  --title "omnitrix: DNAConsole widget" \
  --body "Wave: Features+Polish — Arc 1 (Omnitrix). Depends on: none.

Add the DNAConsole widget: replaces TextInput, styled to look like alien DNA sequences being typed. Per idea-1-omnitrix.md's Global/Shared Components.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 1.8 (Slice: Features+Polish)

```
gh issue create --repo tatemeyer/ttui \
  --title "omnitrix: 'Brainstorm' (Agent Interface) sub-app" \
  --body "Wave: Features+Polish — Arc 1 (Omnitrix). Depends on: Task 1.3 (AppMode).

Build the Brainstorm sub-app: a multi-pane agent/chat view whose borders show flickering Braille 'neural activity' that pulses rapidly while the LLM is processing. Per idea-1-omnitrix.md's 'App: Agent Interface (Brainstorm)'.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 1.9 (Slice: Features+Polish)

```
gh issue create --repo tatemeyer/ttui \
  --title "omnitrix: 'Fasttrack' (Productivity) sub-app" \
  --body "Wave: Features+Polish — Arc 1 (Omnitrix). Depends on: Task 1.3 (AppMode).

Build the Fasttrack sub-app: tasks as 'Targets/Missions', completion triggers a lock-on sequence + green flash moving the item to Completed, timers rendered as circular loading rings. Per idea-1-omnitrix.md's 'App: Super Productivity Clone (Fasttrack)'.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 1.10 (Slice: Features+Polish)

```
gh issue create --repo tatemeyer/ttui \
  --title "omnitrix: 'Upgrade' (System Dashboard) sub-app" \
  --body "Wave: Features+Polish — Arc 1 (Omnitrix). Depends on: Task 1.3 (AppMode).

Build the Upgrade sub-app: machine-resource visualization as a lighting 'circuit', UI edges flash warning red at 90%+ CPU. Per idea-1-omnitrix.md's 'App: System Dashboard (Upgrade)'.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 1.11 (Slice: Features+Polish)

```
gh issue create --repo tatemeyer/ttui \
  --title "omnitrix: boot/intro splash sequence" \
  --body "Wave: Features+Polish — Arc 1 (Omnitrix). Depends on: none.

Build the intro splash: pitch black, a dim green hourglass fades in, a bright green flash fills the screen, circuit-board lines trace outward from center to draw the borders. Per idea-1-omnitrix.md's 'Boot Sequence & Transitions'.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 1.12 — Arc 1 tracking issue

- [ ] **File after Tasks 1.1-1.11, then back-fill with real issue numbers**

```
gh issue create --repo tatemeyer/ttui \
  --title "[Tracking] Omnitrix arc" \
  --body "Tracks the Omnitrix arc from docs/design/specs/2026-08-06-example-apps-roadmap-design.md.

- [ ] omnitrix: adopt Cell style/bold for the glow border
- [ ] omnitrix: build the Faceplate dial-navigation hub
- [ ] omnitrix: add AppMode state enum for Faceplate/Brainstorm/Fasttrack/Upgrade
- [ ] omnitrix: app-switch 'corruption' transition
- [ ] omnitrix: custom 2-char-thick border renderer
- [ ] omnitrix: EnergyCore widget
- [ ] omnitrix: DNAConsole widget
- [ ] omnitrix: 'Brainstorm' (Agent Interface) sub-app
- [ ] omnitrix: 'Fasttrack' (Productivity) sub-app
- [ ] omnitrix: 'Upgrade' (System Dashboard) sub-app
- [ ] omnitrix: boot/intro splash sequence"
```

---

## Arc 2: Smash Crabs

### Task 2.1 (Slice: Core)

```
gh issue create --repo tatemeyer/ttui \
  --title "smash_crabs: wire the current arena scene into Arc 0 primitives" \
  --body "Wave: Core — Arc 2 (Smash Crabs). Depends on: Arc 0 (all tickets, as each lands).

Scaffolding-only ticket: as Arc 0's tween/particle/screen-shake/audio primitives land, wire examples/smash_crabs.rs's existing single scene to consume them (no new visuals yet — that's the Architectural/Features+Polish waves below).

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 2.2 (Slice: Structural)

```
gh issue create --repo tatemeyer/ttui \
  --title "smash_crabs: Character-select grid hub + ScuttleCursor" \
  --body "Wave: Structural — Arc 2 (Smash Crabs). Depends on: none.

Build the Character Select screen: a 4x3 or 5x4 grid of 'portrait' panels (Agent = Mewtwo Crab, Productivity = Captain Falcon Crab, Dashboard = Bowser Crab), navigated with the ScuttleCursor. Per idea-2-SuperSmashCrabs.md's 'App Selection (The Character Select Screen)'.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 2.3 (Slice: Structural)

```
gh issue create --repo tatemeyer/ttui \
  --title "smash_crabs: state enum for Versus/Target Smash/Stage Hazards screens" \
  --body "Wave: Structural — Arc 2 (Smash Crabs). Depends on: Task 2.2 (Character-select hub).

Add a screen-state enum (Versus/Target Smash/Stage Hazards) driving which sub-app view() renders, following the app-state navigation pattern from Rev B's 'Not a gap' section.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 2.4 (Slice: Architectural)

```
gh issue create --repo tatemeyer/ttui \
  --title "smash_crabs: generalize the 3-layer arena into reusable per-screen scaffolding" \
  --body "Wave: Architectural — Arc 2 (Smash Crabs). Depends on: Task 2.3 (screen state enum).

Generalize today's single-scene 3-layer (background/UI/effects) LayerStack usage into scaffolding reusable by all three sub-app screens (Versus/Target Smash/Stage Hazards), not just the current arena.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 2.5 (Slice: Architectural)

```
gh issue create --repo tatemeyer/ttui \
  --title "smash_crabs: ScuttleCursor jerky two-frame tween movement" \
  --body "Wave: Architectural — Arc 2 (Smash Crabs). Depends on: Arc 0 Task 0.3 (easing helper).

Implement ScuttleCursor's movement style: shifting left/right by one cell on alternate ticks (jerky, not smooth), using the shared easing/tween primitive. Per idea-2-SuperSmashCrabs.md's ScuttleCursor component.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 2.6 (Slice: Architectural)

```
gh issue create --repo tatemeyer/ttui \
  --title "smash_crabs: screen-shake on impact events" \
  --body "Wave: Architectural — Arc 2 (Smash Crabs). Depends on: Arc 0 Task 0.2 (screen-shake helper).

Wire the shared screen-shake helper into impact events (hits, KOs) per idea-2-SuperSmashCrabs.md's rendering-techniques section.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 2.7 (Slice: Architectural)

```
gh issue create --repo tatemeyer/ttui \
  --title "smash_crabs: VS transition (2s VS screen + slam + circle-wipe)" \
  --body "Wave: Architectural — Arc 2 (Smash Crabs). Depends on: Arc 0 Task 0.5 (transition hook), Task 2.3 (screen state enum).

Implement the app-switch transition: Player 1 token slams into the selected portrait, screen cuts to black, massive VS block-text flashes center-screen, then an expanding circle-of-light wipe reveals the app's Stage UI. Per idea-2-SuperSmashCrabs.md's 'App Switch Animation'.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 2.8 (Slice: Features+Polish)

```
gh issue create --repo tatemeyer/ttui \
  --title "smash_crabs: DamageMeter widget" \
  --body "Wave: Features+Polish — Arc 2 (Smash Crabs). Depends on: none.

Add the DamageMeter widget: replaces progress bars, numbers count up 0%-300%, color ramps white -> yellow -> red as limits are approached. Per idea-2-SuperSmashCrabs.md's Global/Shared Components (already partially prototyped as plain HP text in the current example — this formalizes it as a widget).

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 2.9 (Slice: Features+Polish)

```
gh issue create --repo tatemeyer/ttui \
  --title "smash_crabs: SmashBorder widget" \
  --body "Wave: Features+Polish — Arc 2 (Smash Crabs). Depends on: none.

Add the SmashBorder widget: 2-3 character thick beveled, chunky, plastic-look borders. Per idea-2-SuperSmashCrabs.md's Global/Shared Components.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 2.10 (Slice: Features+Polish)

```
gh issue create --repo tatemeyer/ttui \
  --title "smash_crabs: 'Versus Mode' (Agent Interface) sub-app" \
  --body "Wave: Features+Polish — Arc 2 (Smash Crabs). Depends on: Task 2.3 (screen state enum), Task 2.8 (DamageMeter).

Build Versus Mode: incoming LLM tokens 'hit' the screen with a brief yellow flash, the agent crab plays an attack animation, token count shown as a rising DamageMeter. Per idea-2-SuperSmashCrabs.md's 'App: Agent Interface (Versus Mode)'.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 2.11 (Slice: Features+Polish)

```
gh issue create --repo tatemeyer/ttui \
  --title "smash_crabs: 'Target Smash' (Productivity) sub-app" \
  --body "Wave: Features+Polish — Arc 2 (Smash Crabs). Depends on: Task 2.3 (screen state enum).

Build Target Smash: tasks as 'Targets', completing one triggers a screen shake, an impact effect over the text, and a 'KO' stamp before the task fades out. Per idea-2-SuperSmashCrabs.md's 'App: Super Productivity Clone (Target Smash)'.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 2.12 (Slice: Features+Polish)

```
gh issue create --repo tatemeyer/ttui \
  --title "smash_crabs: 'Stage Hazards' (System Dashboard) sub-app" \
  --body "Wave: Features+Polish — Arc 2 (Smash Crabs). Depends on: Task 2.3 (screen state enum).

Build Stage Hazards: CPU/RAM represented as stage bosses/hazards; at 90% RAM a Bob-omb ASCII art appears in the corner, flashing red. Per idea-2-SuperSmashCrabs.md's 'App: System Dashboard (Stage Hazards)'.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 2.13 (Slice: Features+Polish)

```
gh issue create --repo tatemeyer/ttui \
  --title "smash_crabs: boot/intro splash sequence" \
  --body "Wave: Features+Polish — Arc 2 (Smash Crabs). Depends on: none.

Build the intro splash: pitch black, a blinding white flash resolves into an ASCII-art crab claw snapping shut, the title slides in from the sides, a lens-flare sweep burns away the logo into the main menu. Per idea-2-SuperSmashCrabs.md's 'Boot Sequence & Transitions'.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 2.14 (Slice: Features+Polish)

```
gh issue create --repo tatemeyer/ttui \
  --title "smash_crabs: audio cues via Arc 0's audio hook" \
  --body "Wave: Features+Polish — Arc 2 (Smash Crabs). Depends on: Arc 0 Task 0.6 (audio hook).

Add sound effects: cursor movement (soft click), selection (smack), incoming messages (impact). Per idea-2-SuperSmashCrabs.md's Audio Engine section.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 2.15 — Arc 2 tracking issue

- [ ] **File after Tasks 2.1-2.14, then back-fill with real issue numbers**

```
gh issue create --repo tatemeyer/ttui \
  --title "[Tracking] Smash Crabs arc" \
  --body "Tracks the Smash Crabs arc from docs/design/specs/2026-08-06-example-apps-roadmap-design.md.

- [ ] smash_crabs: wire the current arena scene into Arc 0 primitives
- [ ] smash_crabs: Character-select grid hub + ScuttleCursor
- [ ] smash_crabs: state enum for Versus/Target Smash/Stage Hazards screens
- [ ] smash_crabs: generalize the 3-layer arena into reusable per-screen scaffolding
- [ ] smash_crabs: ScuttleCursor jerky two-frame tween movement
- [ ] smash_crabs: screen-shake on impact events
- [ ] smash_crabs: VS transition (2s VS screen + slam + circle-wipe)
- [ ] smash_crabs: DamageMeter widget
- [ ] smash_crabs: SmashBorder widget
- [ ] smash_crabs: 'Versus Mode' (Agent Interface) sub-app
- [ ] smash_crabs: 'Target Smash' (Productivity) sub-app
- [ ] smash_crabs: 'Stage Hazards' (System Dashboard) sub-app
- [ ] smash_crabs: boot/intro splash sequence
- [ ] smash_crabs: audio cues via Arc 0's audio hook"
```

---

## Arc 3: TARDIS

### Task 3.1 (Slice: Core)

```
gh issue create --repo tatemeyer/ttui \
  --title "tardis: virtual buffer + Camera blit-viewport helper" \
  --body "Wave: Core — Arc 3 (TARDIS). Depends on: none.

Build a large virtual Buffer (e.g. 500x500) plus a Camera (x, y, zoom) that extracts a terminal-sized viewport window each frame. Per Rev B's recorded direction (docs/design/specs/2026-08-05-ttui-rev-b-vision-alignment-design.md, 'Deferred: camera/viewport'): this lives at app/library-helper level, not core framework machinery. Per-cell rotation is flagged there as likely out of scope entirely.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 3.2 (Slice: Core)

```
gh issue create --repo tatemeyer/ttui \
  --title "tardis: Glitch Buffer decaying overlay" \
  --body "Wave: Core — Arc 3 (TARDIS). Depends on: none (builds on existing LayerStack).

Build the Glitch Buffer: a decaying overlay LayerStack layer for 'temporal distortions' — fills with random block characters on error, decays over ~500ms via the existing on_tick mechanism, per Rev B's app-space boundary (decay logic lives in app code, not core).

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 3.3 (Slice: Structural)

```
gh issue create --repo tatemeyer/ttui \
  --title "tardis: Hexagonal Console hub" \
  --body "Wave: Structural — Arc 3 (TARDIS). Depends on: Task 3.1 (Camera/viewport).

Build the Hexagonal Console hub: six faces (one per app subsystem), '<-'/'->' rotates the whole UI, the active face renders sharp/bright while receding faces dim in perspective. Per idea-3-TardisTUI.md's 'Main Hub / Navigation'.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 3.4 (Slice: Structural)

```
gh issue create --repo tatemeyer/ttui \
  --title "tardis: perspective/depth shading transform" \
  --body "Wave: Structural — Arc 3 (TARDIS). Depends on: Task 3.1 (Camera/viewport).

Add a per-cell color transform (dimming/shrinking toward the viewport edges) applied during the camera blit, per Rev B's recorded direction for perspective shading — an app/library-helper-level transform, not core machinery.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 3.5 (Slice: Architectural)

```
gh issue create --repo tatemeyer/ttui \
  --title "tardis: camera-pan 'flight' transition between console faces" \
  --body "Wave: Architectural — Arc 3 (TARDIS). Depends on: Arc 0 Task 0.5 (transition hook), Arc 0 Task 0.3 (easing helper), Task 3.3 (Hexagonal Console hub).

Implement the app-switch transition: current UI shakes/blurs, camera rapidly pans across the dark virtual buffer past streaks of temporal energy, then arrives and locks into sharp focus at the new room. Per idea-3-TardisTUI.md's 'App Switch Animation (Flight)'.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 3.6 (Slice: Architectural)

```
gh issue create --repo tatemeyer/ttui \
  --title "tardis: wire Glitch Buffer into error/lag states" \
  --body "Wave: Architectural — Arc 3 (TARDIS). Depends on: Task 3.2 (Glitch Buffer).

Trigger the Glitch Buffer overlay on error states (e.g. 'Perception Filter' failures) and lag states, per idea-3-TardisTUI.md's Key Systems section.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 3.7 (Slice: Features+Polish)

```
gh issue create --repo tatemeyer/ttui \
  --title "tardis: Roundel widget" \
  --body "Wave: Features+Polish — Arc 3 (TARDIS). Depends on: none.

Add the Roundel widget: circular UI nodes that pulse with data (e.g. CPU usage as glow brightness across a row of roundels). Per idea-3-TardisTUI.md's Global/Shared Components.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 3.8 (Slice: Features+Polish)

```
gh issue create --repo tatemeyer/ttui \
  --title "tardis: AnalogToggle widget" \
  --body "Wave: Features+Polish — Arc 3 (TARDIS). Depends on: none.

Add the AnalogToggle widget: replaces checkboxes, rendered as physical levers or bicycle-pump-style buttons. Per idea-3-TardisTUI.md's Global/Shared Components.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 3.9 (Slice: Features+Polish)

```
gh issue create --repo tatemeyer/ttui \
  --title "tardis: TimeRotor widget" \
  --body "Wave: Features+Polish — Arc 3 (TARDIS). Depends on: none.

Add the TimeRotor widget: a central, constantly pulsing vertical column of Braille patterns acting as the system heartbeat, speeding up under load. Per idea-3-TardisTUI.md's Global/Shared Components.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 3.10 (Slice: Features+Polish)

```
gh issue create --repo tatemeyer/ttui \
  --title "tardis: 'Psychic Paper' (Agent Interface) sub-app" \
  --body "Wave: Features+Polish — Arc 3 (TARDIS). Depends on: Task 3.3 (Hexagonal Console hub), Task 3.6 (Glitch Buffer wiring).

Build Psychic Paper: chat styled as a shimmering translucent-white paper; incoming tokens 'bleed' in like ink; on agent error, the 'Perception Filter' breaks — text glitches with red ASCII distortion (via the Glitch Buffer). Per idea-3-TardisTUI.md's 'App: Agent Interface (The Psychic Paper)'.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 3.11 (Slice: Features+Polish)

```
gh issue create --repo tatemeyer/ttui \
  --title "tardis: 'Star Charts' (Productivity) sub-app" \
  --body "Wave: Features+Polish — Arc 3 (TARDIS). Depends on: Task 3.3 (Hexagonal Console hub).

Build Star Charts: tasks as points on a timeline — past tasks fixed/amber, present tasks pulse green, future tasks are scattered 'probability cloud' characters. Completing a task triggers a Temporal Shift (screen flash + timeline snap forward). Per idea-3-TardisTUI.md's 'App: Super Productivity Clone (Star Charts)'.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 3.12 (Slice: Features+Polish)

```
gh issue create --repo tatemeyer/ttui \
  --title "tardis: 'Artron Energy' (System Dashboard) sub-app" \
  --body "Wave: Features+Polish — Arc 3 (TARDIS). Depends on: Task 3.3 (Hexagonal Console hub), Arc 0 Task 0.4 (particle system).

Build Artron Energy: resource usage as energy flowing through visible pipes; near-full RAM vents a stream of red particles across the screen edges; when lagging, Console Room lights dim to emergency red and the framework intentionally drops frame rate slightly. Per idea-3-TardisTUI.md's 'App: System Dashboard (Artron Energy)'.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 3.13 (Slice: Features+Polish)

```
gh issue create --repo tatemeyer/ttui \
  --title "tardis: boot/materialization sequence" \
  --body "Wave: Features+Polish — Arc 3 (TARDIS). Depends on: Task 3.1 (Camera/viewport).

Build the intro: a small ASCII Police Box in the center glitches and shakes (violent-flight simulation), doors swing open, a blinding warm white light floods the terminal, camera pushes through the doors revealing the Console Room. Per idea-3-TardisTUI.md's 'Boot Sequence & Transitions (Materialization)'.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 3.14 (Slice: Features+Polish)

```
gh issue create --repo tatemeyer/ttui \
  --title "tardis: audio cues via Arc 0's audio hook" \
  --body "Wave: Features+Polish — Arc 3 (TARDIS). Depends on: Arc 0 Task 0.6 (audio hook).

Add a low-volume looping TARDIS hum in the background, plus materialization/grinding sounds on app launches. Per idea-3-TardisTUI.md's Audio Engine section.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

### Task 3.15 — Arc 3 tracking issue

- [ ] **File after Tasks 3.1-3.14, then back-fill with real issue numbers**

```
gh issue create --repo tatemeyer/ttui \
  --title "[Tracking] TARDIS arc" \
  --body "Tracks the TARDIS arc from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. TARDIS has no implementation today (examples/tardis.rs does not exist) — this arc starts from zero.

- [ ] tardis: virtual buffer + Camera blit-viewport helper
- [ ] tardis: Glitch Buffer decaying overlay
- [ ] tardis: Hexagonal Console hub
- [ ] tardis: perspective/depth shading transform
- [ ] tardis: camera-pan 'flight' transition between console faces
- [ ] tardis: wire Glitch Buffer into error/lag states
- [ ] tardis: Roundel widget
- [ ] tardis: AnalogToggle widget
- [ ] tardis: TimeRotor widget
- [ ] tardis: 'Psychic Paper' (Agent Interface) sub-app
- [ ] tardis: 'Star Charts' (Productivity) sub-app
- [ ] tardis: 'Artron Energy' (System Dashboard) sub-app
- [ ] tardis: boot/materialization sequence
- [ ] tardis: audio cues via Arc 0's audio hook"
```

---

## Housekeeping

### Task H.1

```
gh issue create --repo tatemeyer/ttui \
  --title "housekeeping: retire examples/demo.rs" \
  --body "Wave: Housekeeping. Depends on: none.

Remove examples/demo.rs (the plain, un-themed Rev A reference app — List/Table/Block/focus-cycling, no ticks/theme/layering) and any build or README references to it. The three themed examples (omnitrix, smash_crabs, and eventually tardis) now cover its demonstrative role and more.

Backlog ticket from docs/design/specs/2026-08-06-example-apps-roadmap-design.md. Needs its own /superpowers:brainstorm -> spec -> plan cycle before implementation (per .claude/rules/development-conventions.md) — no code should land against this issue directly."
```

---

## Self-Review Notes

- **Spec coverage:** every ticket in `docs/design/specs/2026-08-06-example-apps-roadmap-design.md`'s tables has exactly one corresponding task above (Arc 0: 6 tickets + 1 tracking; Arc 1: 11 tickets + 1 tracking; Arc 2: 14 tickets + 1 tracking; Arc 3: 14 tickets + 1 tracking; Housekeeping: 1 ticket). Total: 41 issues (45 counting the 4 arc tracking issues... i.e. 36 ticket issues + 5 tracking issues = 41).
- **Placeholder scan:** no TBDs; every task has a literal `gh issue create` command with a complete title and body.
- **Dependency consistency:** every "Depends on" reference in a task body names either "none," an Arc 0 task (by title, since Arc 0 files first), or an earlier task within the same arc (since tickets file in wave order) — no task depends on a not-yet-filed task in a later wave.
- **Tracking-issue numbering:** tracking issues are deliberately the last task in each arc so their checklists can be back-filled with real issue numbers rather than shipped with dead placeholder text.
