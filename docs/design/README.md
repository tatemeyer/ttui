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
