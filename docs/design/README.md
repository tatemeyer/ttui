# Design docs

This directory holds:

- `specs/<arc>/` — approved design documents, one per major design
  decision or subsystem, produced by the `superpowers:brainstorming`
  skill. Filename convention: `specs/<arc>/YYYY-MM-DD-<topic>-design.md`.
- `plans/<arc>/` — implementation plans derived from specs, structured as
  **Arcs → Slices → Tasks** (below), produced by the
  `superpowers:writing-plans` skill. Same `<arc>` bucketing as `specs/`.

## Arcs

- `core/` — framework internals, tooling, process, and anything not
  scoped to one example app.
- `omnitrix/`, `tardis/`, `smash-crabs/` — one bucket per example app.
- `launcher/` — the cross-app portal-nexus shell that composes the
  three example apps into one launcher (a fourth example, not a themed
  app of its own).
- `falcon/` — a fifth themed example app (a scrappy smuggler-freighter
  cockpit), built from `TTUI-Ideas/vision/UI/idea-4-Falcon.md`.
- `mission-control/` — a sixth example app (a NASA-style ground-control
  telemetry console) and the `BarChart`/`Sparkline` data-viz widgets it
  was built to prove out — spec-derived rather than vision-doc-derived.
- `control-panel/` — a seventh example app (a physical-control-panel-
  style console: click-toggle switches, a click-advance dial, a launch
  button) built to prove out real mouse support (`Rect::contains`,
  `Terminal` mouse capture, `tools/visual-snapshot` click-scripting).
- `governance/` — release-process work (SemVer policy, label taxonomy,
  triage rules) rather than an example app or framework subsystem;
  first Arc under the TTUI v1.0.0 initiative.
- `showcase/` — the flagship demo reel: a mascot-hosted tile menu of
  five vignettes (mouse interaction, particles, camera+glitch, chord
  input, data-viz) pulled together into one polished entry point, run
  via its own `showcase` `[[bin]]` target rather than cataloged as
  another `examples/` vision-doc app; fifth sub-project of the TTUI
  v1.0.0 initiative.
- `showcase-polish/` — a follow-up Arc to `showcase/`: mascot idle
  animation and a redesigned eye/blink, plus a full Assembly Line
  rework (a real crate sprite, the mascot sliding to and reaching down
  for a caught crate) — deferred out of `showcase/`'s own final review
  rather than folded into it. Sub-project #5.1 of the TTUI v1.0.0
  initiative.
- A new bucket is added here the first time a genuinely new Arc starts
  (e.g. a fourth example app) — this list, not the file count inside
  each bucket, is what stays small as the docs tree grows.

## Arcs that live in another repository

Two Arcs were designed here and moved out, because the code they
describe is cross-project by definition and does not belong in TTUI.
Both live in the **Parallax** repository (`D:/Dev/Projects/Parallax`).
`specs/parallax/` and `specs/plumb/` in this tree are pointer stubs, not
the documents.

- `parallax/` — the master design binding TTUI, Model-Experiments, and
  Plumb into one platform: a verification-tier ladder, a three-axis
  autonomy model that TTUI's Direct/Gated/Human tiers project onto, the
  `parallax.yaml` manifest, and a five-sub-project roadmap. TTUI is a
  consumer, and its cockpit is `ttui`'s first genuine external consumer.
- `plumb/` — sub-project #1: perceptual verification. Generalizes
  `tools/visual-snapshot` into a portable Claude Code plugin and adds
  the judgment half — a blinded, adversarial multi-lens reviewer
  rendering GO / NO-GO / HOLD against `.plumb/taste.md`. TTUI is
  consumer #1 and adopts it via the `command` adapter, keeping
  `tools/visual-snapshot` verbatim and unmodified.

TTUI keeps its own Plumb *project state* — `.plumb/taste.md` today,
`.plumb/config.yaml` and `.plumb/scripts/` on adoption. Those are this
repo's declared aesthetic and scenarios, not the tool.

## Arc / Slice / Task structure

A spec's implementation plan is organized into three nested levels:

- **Arc** — a large body of related work within a spec (e.g. "rendering
  pipeline," "widget set").
- **Slice** — a coherent, independently-completable piece of work within
  an Arc (e.g. "buffer diffing algorithm").
- **Task** — the smallest unit of actual work within a Slice; what
  actually gets executed and checked off.

Arcs and Slices deliberately have no formally defined scope, capacity,
size, or duration. Experience on prior projects is that it's easier to
add structure later where it's actually needed than to work around
hard-enforced rules that turn out to be wrong for a given piece of work.

## Tags

Slices and Tasks can each be tagged with one or more of:

- `coding` — writing or modifying source code
- `research` — investigation, spikes, evaluating options
- `admin` — process/tooling/non-code housekeeping
- `git-adjacent` — branch/PR/commit/repo-structure work

This tag set is intentionally small and expected to grow — add a tag
when a new paradigm or tool (e.g. Docker) genuinely needs one, rather
than anticipating categories that don't have a concrete use yet.
