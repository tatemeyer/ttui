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
- **Does agent-authored work get a real merge gate, or is admin bypass
  accepted as the norm?** `main`'s branch protection
  (`docs/design/plans/core/2026-08-04-gitops-github-workflow-plan.md`, Task 7)
  sets `enforce_admins: false` — the repo owner (whose credentials an
  agent operates under) can push straight past required PRs and CI
  checks. This was a deliberate spec decision (the stated "escape valve
  for exceptions"), but in practice, within minutes of protection going
  live, three docs-only commits landed through that exact bypass rather
  than as PRs — bypass was the normal path, not the exception. The
  GitOps final review flagged this as worth resolving explicitly here:
  is that acceptable given "agentic-first development" means the agent
  *is* the admin, or does the autonomy-tier label scheme above need to
  gate this (e.g. only `autonomy:safe`-tier changes may bypass; anything
  else must go through a real PR with checks green)?
