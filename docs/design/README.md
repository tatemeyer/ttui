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
- `parallax/` — the master Arc binding TTUI, Model-Experiments, and
  `plumb` into one verification-first agentic development platform,
  with a TTUI-built cockpit (**Panopticon**) as its frontend.
  Decomposes into five sub-projects, each of which gets its own
  spec/plan cycle; lives in its own repository.
- `plumb/` — a cross-project Claude Code plugin that gives the
  visual-review step eyes and opinions: portable capture adapters plus
  an adversarial multi-lens reviewer. Generalizes `tools/visual-
  snapshot` out of this repo; sub-project #1 of the Parallax Arc above,
  and the only one that ships standalone. Lives in its own repository,
  with TTUI as consumer #1 — the spec is filed here because this is
  where it was designed.
- `governance/` — release-process work (SemVer policy, label taxonomy,
  triage rules) rather than an example app or framework subsystem;
  first Arc under the TTUI v1.0.0 initiative.
- A new bucket is added here the first time a genuinely new Arc starts
  (e.g. a fourth example app) — this list, not the file count inside
  each bucket, is what stays small as the docs tree grows.

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
