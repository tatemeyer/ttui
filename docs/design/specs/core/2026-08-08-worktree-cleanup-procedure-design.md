# Worktree Cleanup Procedure — Design

**Status:** draft, pending your review before we move to planning.
**Date:** 2026-08-08
**Relationship to prior specs:** operational addendum to
`docs/design/specs/core/2026-08-07-git-github-standards-design.md` (the
worktree/branch lifecycle and autonomy tiers) and to the
`audit-graph-compliance` skill (`.claude/skills/audit-graph-compliance/
SKILL.md`), whose worktree-hygiene check was just corrected to detect
squash-merged branches. Neither of those is changed here; this spec
defines the *action* half of cleanup that they leave unspecified.

## Context / Motivation

`git-github-standards.md` says a worktree is removed via `ExitWorktree`
"at the end of the Arc," and `audit-graph-compliance` *reports* which
worktrees are removable/abandoned/active — but nothing writes down what
to actually do when that end-of-Arc removal was skipped and worktrees
accumulate. The concrete drift that motivated `git-github-standards.md`
(a 56-commit unmerged branch, ~13 orphaned worktrees) is exactly this:
the reporting existed, the *procedure* for acting on it did not, so the
report was never acted on.

A second, sharper trigger: the audit skill's hygiene check keyed
removability off `git branch --merged`, which relies on commit
ancestry. This repo **squash-merges** PRs, which rewrites a branch's
commits into one new commit on `main`, so a fully-merged branch never
appears in that list. Proof: `worktree-issue-41-omnitrix-glow-border`
and `worktree-issue-42-omnitrix-faceplate` were squash-merged (PRs #85,
#86) yet were invisible to `git branch --merged origin/main`, so a
naive cleanup would have preserved them as "unmerged" indefinitely. The
skill is now fixed to cross-reference `gh pr list --state merged --head
<branch>`; this procedure assumes that corrected classification and
never re-derives it.

## Scope of this spec

A repeatable, safe cleanup runbook that:

1. Consumes `audit-graph-compliance`'s three-bucket report (removable /
   abandoned / active) rather than re-implementing merged-detection.
2. Defines the concrete action per bucket, including the autonomy tier
   each action falls under.
3. Handles the two edge cases the raw buckets don't: a `locked`
   worktree, and a branch whose deletion is *Human*-tier because the
   current session didn't create it.

Explicitly **out of scope:** changing the classification logic (that
lives in the skill), automating the removal (this stays a human-invoked
or human-confirmed runbook, matching the tier rules), and pruning
remote branches (`delete_branch_on_merge` already handles that
server-side).

## The procedure

### Step 0 — Run the audit, from a safe cwd

Run `audit-graph-compliance` from the main checkout (or any directory
**not inside** a worktree being considered for removal — `git worktree
remove` refuses to remove the worktree you are standing in). Take its
three buckets as the input to every step below.

### Step 1 — Removable (merged): remove

For each worktree the audit reports as **removable** (merged by PR
*or* ancestry):

```sh
git worktree remove <path>
git branch -d <branch>      # -d is safe; it refuses if genuinely unmerged
```

`git branch -d` will refuse a squash-merged branch (ancestry says it's
unmerged even though its PR merged). That refusal is expected for this
repo; when the audit's PR-based check already classified the branch as
removable, `git branch -D` is the correct follow-up — the work is on
`main` under a rewritten commit. This is the one place the squash-merge
model forces `-D` over `-d`, and doing so is **not** a Human-tier
force-operation: it deletes a local ref whose content already landed.

Tier: **Direct** when the current session created the worktree;
**Human** otherwise (Step 3).

**If cleaning up via the `ExitWorktree` tool instead of raw `git`
commands** (the normal path for a session that entered the worktree
with `EnterWorktree`, and the one actually hit in practice — this case
recurred across roughly 30 PRs before being written down here): the
tool runs its own ancestry check, independent of `git branch -d`, and
produces the identical squash-merge false positive — it refuses
`action: "remove"` with an error naming "N commits ... not on the base
branch," even though the PR is genuinely merged, because the squashed
commit on `main` isn't a git-ancestry descendant of the worktree
branch's original commits. Resolution, mirroring Step 1's `-d`-vs-`-D`
reasoning exactly:

1. Independently confirm the merge — don't trust a paraphrase like
   "merged" at face value; verify directly:
   ```sh
   gh pr view <N> --json state,mergedAt,mergeCommit
   ```
   Confirm `state: "MERGED"` and a real `mergedAt` timestamp.
2. Once confirmed, call `ExitWorktree` again with
   `discard_changes: true`. This is **not** a data-loss risk the same
   way it would be for a genuinely unmerged branch — the tool's error
   message is doing exactly what Step 1 already established: refusing
   on ancestry grounds for a branch whose content already landed under
   a rewritten commit hash.
3. Tier: **Direct** when the current session's own worktree (same rule
   as Step 1) — the `gh pr view` check *is* the verification step; no
   separate human confirmation is needed each time this exact pattern
   recurs. (A first occurrence, or any case where the `gh pr view`
   check itself is inconclusive — no merge commit, unexpected base
   branch, etc. — still warrants asking, per Step 3's ownership gate or
   general judgment; it's the well-worn, independently-verified case
   that doesn't need to re-ask.)

### Step 2 — Abandoned (zero commits ahead, no PR, no recent activity)

For each **abandoned** worktree the audit reports, re-confirm it is
genuinely empty before removing — the audit already computed
`git rev-list --count origin/main..<branch>`, but re-print it so the
decision is visible:

```sh
git rev-list --count origin/main..<branch>   # expect 0
```

`0` → remove as in Step 1 (`git worktree remove` + `git branch -D`).
`>0` → it has unshipped commits despite no PR; **stop** and treat it as
**active** (Step 4), not abandoned. Do not delete a branch with commits
ahead of `main` on the strength of "no PR" alone.

### Step 3 — Ownership gate (Human tier)

Removing a worktree or branch **the current session did not create** is
a *Human*-tier action under `git-github-standards.md`, regardless of
merge state. For those, the runbook **stops and surfaces the exact
commands for the user to run or approve** rather than executing them.
Green merge state is necessary but not sufficient here — the tier is
triggered by *who created it*, not by whether it merged.

### Step 4 — Active: leave, or flag stale

**Active** worktrees (open PR, or recent commits, or commits ahead) are
left alone. If the audit's freshness check additionally flagged one as
**stale** (no activity in 24h *and* no open PR but still has unshipped
work), report it for the user to decide resume-vs-abandon — never
auto-remove a stale worktree, since "stale" by definition still has
work to lose.

### Step 5 — Edge case: `locked` worktrees

A worktree shown as `locked` in `git worktree list` was locked by a
prior session (often an interrupted one) to prevent accidental removal.
A lock does **not** override merge state — a locked worktree whose PR
merged is still removable — but the lock must be cleared first:

```sh
git worktree unlock <path>
git worktree remove <path>          # add --force only if remove still refuses
```

Investigate *why* it is locked before unlocking only when the lock is
recent (< 24h) and the branch is **not** classified removable —
otherwise a stale lock on a merged branch (e.g.
`worktree-issue-27-buffer-layering-design`, PR #32 merged 2026-08-06)
is just leftover bookkeeping and safe to clear.

### Step 6 — Prune bookkeeping, re-audit

```sh
git worktree prune          # drop stale administrative entries
git fetch origin --prune    # drop remote-tracking refs for deleted branches
```

Re-run `audit-graph-compliance` to confirm the removable and abandoned
buckets are now empty. A clean re-audit is the procedure's exit
criterion.

## Autonomy tier summary

| Bucket / action                         | Tier   |
|-----------------------------------------|--------|
| Remove merged worktree this session made | Direct |
| Remove abandoned (0-ahead) artifact      | Direct |
| `git branch -D` a squash-merged branch   | Direct |
| Remove any worktree another session made | Human  |
| Unlock + remove a locked worktree        | Human unless stale-lock-on-merged (then Direct) |
| Touch a stale/active worktree with work  | Human  |

## Testing

Per `.claude/rules/development-conventions.md`, this is `git-adjacent`/
`admin` work — pure procedure documentation with no application logic,
so it falls under the "pure config/git-adjacent work" TDD exception.
Correctness is verified by walking the current worktree situation
through the runbook and confirming a clean re-audit, not by unit tests.

## Critical files

- `docs/design/specs/core/2026-08-08-worktree-cleanup-procedure-design.md`
  — this spec.
- `docs/design/plans/core/2026-08-08-worktree-cleanup-procedure-plan.md`
  — the implementation plan (doc authoring only).
- Consumes, does not modify: `.claude/skills/audit-graph-compliance/
  SKILL.md`, `.claude/rules/git-github-standards.md`.

## Verification

- The runbook, applied to the current ~18-worktree situation, produces
  the same classification the last manual pass reached (5 named
  PR-merged worktrees + 7 ancestry-merged agent worktrees removable; 6
  agent branches triaged by commits-ahead; the `locked` #27 worktree
  cleared as stale-lock-on-merged).
- No step deletes a branch with commits ahead of `origin/main` on the
  strength of "no PR" alone.
- A post-cleanup `audit-graph-compliance` run reports empty removable
  and abandoned buckets.
