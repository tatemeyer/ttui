---
name: audit-graph-compliance
description: Enforce branch structural standards, freshness, worktree hygiene, and artifact completeness before work is considered done
---

## Audit Graph Compliance

Checks a branch/worktree's readiness against
`.claude/rules/git-github-standards.md`. Three of five originally-
scoped responsibilities are implemented below; two stay stubbed,
blocked on `.claude/rules/diagram-standards.md` and
`.claude/rules/code-forge.md` (see "Still stubbed" at the end).

### 1. Artifact completeness

For the current branch's Arc:

1. Identify the Arc's spec (`docs/design/specs/<arc>/YYYY-MM-DD-
   <topic>-design.md`) and plan (`docs/design/plans/<arc>/YYYY-MM-DD-
   <topic>-plan.md`). If either is missing, report **not ready** —
   "no spec" or "no plan" respectively.
2. Read the plan file and count remaining `- [ ]` (unchecked) task
   boxes. If any remain, report **not ready** — "N tasks unchecked."
3. If both a spec and plan exist and every task is checked, report
   **ready to ship** — the branch qualifies for a PR under
   `git-github-standards.md`'s "ship at the end of the Arc" rule.

### 2. Branch freshness

1. Run `git worktree list` to enumerate active worktrees and their
   branches.
2. For each worktree branch (excluding the main checkout), get its
   most recent commit timestamp via `git log -1 --format=%cI <branch>`.
3. Run `gh pr list --state open --head <branch>` for that branch.
4. If the branch has no commits in the last 24 hours **and** no open
   PR, report **stale** — flagged for the user to decide whether to
   resume it or clean it up. This check flags; it does not delete or
   merge anything itself.

### 3. Worktree hygiene

1. From the same `git worktree list` enumeration, run `git branch
   --merged main` and cross-reference: any worktree whose branch
   appears in that list is **fully merged** — report as **removable**
   (safe to `git worktree remove` and delete the branch, since its
   work already landed on `main`).
2. For worktree branches not merged into `main`, check commits ahead
   of `main` via `git rev-list --count main..<branch>`. A branch with
   `0` commits ahead and no activity in the last 24 hours (reusing the
   Branch Freshness check's timestamp) is likely an **abandoned
   session artifact** — report as a candidate for removal, distinct
   from "stale" (which still has unshipped work) since there's nothing
   to lose by removing it.
3. Report the full list of worktrees in three buckets — removable
   (merged), abandoned (zero commits ahead, no PR, no recent
   activity), and active (everything else) — so the user can act on
   groups rather than one at a time.

### Still stubbed

- **Interrupt handling** — defining and checking what "safely
  interrupted" means for in-flight agent work (partial commits,
  dangling branches). Not yet designed.
- **Label/diagram-dependent structural enforcement** — verifying
  required diagrams or `code-forge.md` labels are present on a branch.
  Blocked on `.claude/rules/diagram-standards.md` and
  `.claude/rules/code-forge.md`, neither resolved yet.

### Dependencies

- Git/GitHub CLI (`git worktree list`, `git log`, `git branch
  --merged`, `git rev-list`, `gh pr list`) for every check above —
  none of this needs `code-review-graph`'s MCP tools, since it's
  entirely git/GitHub state, not source-code structure.
