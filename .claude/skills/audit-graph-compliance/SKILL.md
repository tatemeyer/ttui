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

1. From the same `git worktree list` enumeration, determine which
   worktree branches have already landed on `main`. A branch counts as
   **fully merged** if *either* of these holds:
   - it appears in `git branch --merged origin/main` (catches
     fast-forward and merge-commit merges via commit ancestry), **or**
   - `gh pr list --state merged --head <branch>` returns a merged PR
     for it.
   The second check is **required, not optional**: this repo
   squash-merges PRs (per `git-github-standards.md`), and a squash
   merge rewrites a branch's commits into one new commit on `main`, so
   the branch's own commits are never ancestors of `main` and
   `git branch --merged` will silently omit it. Concrete proof:
   `worktree-issue-41-omnitrix-glow-border` and
   `worktree-issue-42-omnitrix-faceplate` were squash-merged via PRs
   #85 and #86 but do **not** appear in
   `git branch --merged origin/main`. Because squash is the repo
   default, treat `gh pr list --state merged --head <branch>` as the
   authoritative signal and ancestry as the fallback. Any branch
   meeting *either* condition is **removable** — safe to
   `git worktree remove` and delete the branch, since its work already
   landed on `main`.
2. For worktree branches merged by *neither* check above, check commits
   ahead
   of `main` via `git rev-list --count origin/main..<branch>`. A branch
   with `0` commits ahead and no activity in the last 24 hours (reusing the
   Branch Freshness check's timestamp) is likely an **abandoned
   session artifact** — report as a candidate for removal, distinct
   from "stale" (which still has unshipped work) since there's nothing
   to lose by removing it.
3. Report the full list of worktrees in three buckets — removable
   (merged), abandoned (zero commits ahead, no PR, no recent
   activity), and active (everything else) — so the user can act on
   groups rather than one at a time.

This check *classifies*; the action to take on each bucket (and the
autonomy tier it falls under) is defined in the cleanup runbook,
`docs/design/specs/core/2026-08-08-worktree-cleanup-procedure-design.md`
— keep the two in sync but don't duplicate the classification there.

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
