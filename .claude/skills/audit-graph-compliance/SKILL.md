---
name: audit-graph-compliance
description: Enforce branch structural standards, freshness, worktree hygiene, and artifact completeness before work is considered done
---

## Audit Graph Compliance

**Status: stub.** This skill is not yet implemented — its actual checks
depend on `.claude/rules/diagram-standards.md` and
`.claude/rules/code-forge.md` being filled in first via
`/superpowers:brainstorm`. This file exists to hold the slot and record
scope, not to be invoked yet.

### Intended responsibilities

1. **Structural standards enforcement** — verify a branch's changes
   comply with whatever `code-forge.md` and `diagram-standards.md` end
   up specifying (e.g. required diagrams present for new components,
   labels applied correctly).
2. **Branch freshness** — detect whether a branch is stale relative to
   its base and flag it before further work continues.
3. **Worktree management** — audit `.claude/worktrees/` (or wherever
   `using-git-worktrees` places them) for orphaned or stale worktrees.
4. **Interrupt handling** — define and check what "safely interrupted"
   means for in-flight agent work (partial commits, dangling branches).
5. **Artifact completeness** — confirm that a unit of work has both its
   `docs/superpowers/specs/...` design doc and
   `docs/superpowers/plans/...` plan present and linked before the
   branch is considered mergeable.

### Likely dependencies once implemented

- `code-review-graph`'s MCP tools (`detect_changes_tool`,
  `get_impact_radius_tool`) for the structural side, the same way the
  other four skills in this directory use them.
- Git/GitHub CLI (`git worktree list`, `gh pr view`) for the
  branch/worktree/artifact checks, which are outside `code-review-
  graph`'s scope (it only understands source code structure, not
  git/GitHub state).
