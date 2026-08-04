# Code Forge

**Status:** stub — not yet defined. Needs a `/superpowers:brainstorm` pass.

Ported from the categories used on a prior project — the conventions
governing how work gets labeled, attributed, filed, and routed to a
model tier.

## Categories

- **Labels** — the taxonomy for classifying issues/PRs/work items
  (compare to Model-Experiments' `autonomy:safe` / `autonomy:review` /
  `autonomy:human` / `needs-intent` scheme — worth deciding whether TTUI
  adopts something similar or its own).
- **Agent authorship** — how work done by an agent is attributed/marked
  as such (commit trailers, PR metadata, etc.).
- **Filing work** — how new work items get created and scoped before
  they enter a `/superpowers:brainstorm` → plan cycle.
- **Model-tiered dispatch** — routing which tasks go to which model
  tier (e.g. cheap/fast model for mechanical work, stronger model for
  design-heavy work) — relevant once subagents are in active use via
  `subagent-driven-development`.

## Open questions to resolve via brainstorming

- Concrete label set and what triggers each one.
- Whether model-tiered dispatch is expressed as subagent `model:`
  frontmatter conventions, or something more automated.
