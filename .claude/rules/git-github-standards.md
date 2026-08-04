# Git/GitHub Standards

**Status:** stub — not yet defined. Needs a `/superpowers:brainstorm` pass.

Ported from the categories used on a prior project.

## Categories

- **Agent loop/graph architecture** — how agent work maps onto git
  structure (branches, worktrees — `superpowers`' own
  `using-git-worktrees` skill already provides a baseline here — and
  how that relates to the codebase graph tracked by
  `code-review-graph`).
- **Human in/out-of-loop workflows** — which git/GitHub operations
  require a human checkpoint versus which an agent can complete
  unattended.
- **Agent system for anticipating intentions, usage, and expectations**
  — the most novel item here; not yet designed. Likely related to, but
  distinct from, the deferred "agent state as graph nodes" scheduling
  idea noted in the root project context.

## Open questions to resolve via brainstorming

- Concrete branch/worktree lifecycle rules, including what
  `audit-graph-compliance` (see `.claude/skills/`) should actually
  enforce.
- Where the line sits between "safe to automate" and "needs a human" —
  this is the same question Model-Experiments answers with its
  `autonomy:*` label scheme; decide whether to reuse that pattern.
