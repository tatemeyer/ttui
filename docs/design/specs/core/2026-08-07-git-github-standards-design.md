# Git/GitHub Standards — Design

**Status:** draft, pending your review.
**Date:** 2026-08-07
**Relationship to prior specs:** resolves the two open questions in
`.claude/rules/git-github-standards.md` (branch/worktree lifecycle +
what `audit-graph-compliance` enforces; the safe-to-automate/needs-a-
human line). Directly unblocks `code-forge.md`'s own flagged question
("does agent-authored work get a real merge gate") by supplying the
autonomy-tier scheme it references. Builds on
`docs/design/specs/core/2026-08-04-gitops-github-workflow-design.md`
(branch protection, PR template, CI checks — all already live) and
`superpowers:using-git-worktrees`.

## Problem

This spec isn't starting from a blank page — real drift already exists.
`worktree-arc-omnitrix-dial-navigation` (this session's own branch) is
56 commits ahead of `main` with no PR ever opened against it; `main`
hasn't moved since PR #86 merged. That's a sharp reversal from the
project's first two days, which used dozens of small, individually
merged PRs. Separately, `git worktree list` shows roughly a dozen
`worktree-agent-<hash>` entries that look like abandoned automated-test
artifacts, never cleaned up — concrete evidence of exactly the
"worktree hygiene" gap `audit-graph-compliance`'s stub already names as
a responsibility but has no logic to actually check. `code-forge.md`
separately flags that `main`'s branch protection has
`enforce_admins: false`, and in practice every commit this session
used that bypass rather than a PR.

## Scope

Three slices:

1. **Worktree/branch lifecycle** — one worktree per Arc, PR + merge
   when the Arc's plan completes, immediate worktree removal.
2. **Autonomy tiers** — a TTUI-scoped Direct/Gated/Human taxonomy
   reusing the existing Slice/Task tag system, answering both this
   file's "safe to automate vs. needs a human" question and
   `code-forge.md`'s merge-gate question.
3. **`audit-graph-compliance` real logic** — replacing the stub's
   "intended responsibilities" list with actually-runnable checks for
   three of its five named responsibilities (structural/artifact
   completeness, branch freshness, worktree hygiene); the other two
   (interrupt handling, and the parts of structural enforcement that
   depend on `code-forge.md`/`diagram-standards.md` labels/diagrams
   that don't exist yet) stay stubbed, unblocked by future rounds.

**Explicitly out of scope:** the "agent system for anticipating
intentions, usage, and expectations" category — flagged in the rules
file itself as "the most novel item here; not yet designed" and never
one of the two formal open questions; actually cleaning up the 13
existing orphaned worktrees or opening the PR for this session's
56-commit backlog — real, visible actions this spec's policy would
recommend, but distinct from writing the policy, done only with
separate explicit confirmation; a full re-litigation of the branch
protection settings from the GitOps spec (required checks, PR
template) — unchanged here except the two specific fixes named below.

## Design

### Slice 1: Worktree/branch lifecycle

**One worktree per Arc.** Created via `superpowers:using-git-worktrees`
(the `EnterWorktree` tool) at the start of an Arc's implementation
plan. Branch naming stays exactly the tool's existing `worktree-
<descriptor>` convention — already consistent across every branch in
the repo, nothing to change there.

**Ship at the end of the Arc, not later.** Once every task in the
Arc's plan is checked off and final verification (`cargo test`/`fmt`/
`clippy`, manual-verification checklist where applicable) is green:
open a PR from the worktree branch to `main` using the existing PR
template (`2026-08-04-gitops-github-workflow-design.md`), wait for the
four required status checks (`build`/`test`/`clippy`/`fmt`) to pass,
squash-merge, then remove the worktree via `ExitWorktree`. Squash
merge specifically (not merge-commit or rebase) — keeps `main`'s
history one commit per Arc, matching the granularity
`development-conventions.md` already established for regular commits,
and avoids `main` accumulating every intermediate task-commit from
inside the worktree.

**Two repo-setting fixes, bundled into this same round's implementation
since they're direct, named contributors to the drift above:**
- Flip `delete_branch_on_merge` from `false` to `true` — a merged PR's
  branch should not need manual deletion; this alone would have
  prevented several of the 13 orphaned `worktree-agent-*` branches
  from surviving past their merge.
- No change to `enforce_admins` (stays `false`) — the autonomy-tier
  scheme in Slice 2 is what actually gates which work may use that
  bypass, not the branch-protection setting itself.

**What `audit-graph-compliance` checks here** (see Slice 3 for the
skill's full logic): a worktree whose branch has no commits in the
last 24 hours and no open PR is flagged stale; a worktree whose Arc
plan shows every task checked off but has no open PR yet is flagged
"ready to ship."

### Slice 2: Autonomy tiers

Three tiers, deliberately smaller than the four-tier scheme
`code-forge.md` names for comparison (`autonomy:safe`/`review`/
`human`/`needs-intent`) — TTUI already has a structural equivalent of
`needs-intent` in the brainstorming skill's mandatory clarifying-
questions gate before any implementation starts, so a fourth label
tier for the same concept would be redundant. Reuses the existing
Slice/Task tags (`coding`, `research`, `admin`, `git-adjacent`) from
`docs/design/README.md` as the primary signal, rather than introducing
a second, parallel label vocabulary:

- **Direct** — `admin`/`git-adjacent`-tagged work with zero `src/`/
  `examples/` behavior change (a rules-file edit, a `.github/`
  workflow tweak, a docs-only commit with no code touched). May push
  straight to `main`; no PR required. This is `enforce_admins: false`'s
  actual intended use going forward, not an unbounded bypass.
- **Gated** — the default; covers all `coding`- and `research`-tagged
  work, which is nearly everything that touches `src/` or `examples/`.
  Requires a PR with all four required checks green before merge. No
  separate human-approval wait beyond that — `required_approving_
  review_count` is already `0`, and this tier's whole point is
  "agent-completable unattended, gated on objective checks passing,"
  not "needs a human to look at it."
- **Human** — branch-protection or CI-workflow-config changes,
  force-pushes, deleting a branch/worktree the current session didn't
  create itself, or anything the user explicitly flags as sensitive
  in the moment. Requires explicit human sign-off before merge — green
  checks alone aren't sufficient for this tier.

A Slice/Task's existing tag(s) determine its default tier
(`admin`/`git-adjacent` → Direct, `coding`/`research` → Gated); Human
is never inferred from a tag, only from the specific trigger list
above, since it's about blast radius (repo config, other sessions'
work) rather than ordinary work classification.

### Slice 3: `audit-graph-compliance` real logic

Replaces the stub's `### Intended responsibilities` list with concrete
checks for three of the five:

1. **Artifact completeness** — for the current branch, confirm a spec
   under `docs/design/specs/<arc>/` and a plan under `docs/design/
   plans/<arc>/` both exist, and that the plan file has zero
   remaining `- [ ]` unchecked task boxes. A branch failing this check
   is not "ready to ship" regardless of CI status.
2. **Branch freshness** — via `git worktree list` (branch + last-
   commit timestamp) cross-referenced with `gh pr list --state open`:
   a worktree branch with no commits in the last 24 hours and no
   matching open PR is flagged stale (candidate for either resuming
   work or cleanup — the skill flags, it doesn't decide which).
3. **Worktree hygiene** — same `git worktree list` enumeration,
   flagging any worktree whose branch is fully merged into `main`
   (i.e. `git branch --merged main` contains it) as removable, and any
   worktree with zero commits ahead of its base and no recent activity
   as likely an abandoned/failed session artifact.

Two of the five stay stubbed, unblocked by future rounds: **interrupt
handling** (defining "safely interrupted" for in-flight work) and the
**label/diagram-dependent half of structural enforcement** (verifying
required diagrams or `code-forge.md` labels are present) — both still
depend on `diagram-standards.md` and `code-forge.md`, neither of which
is resolved yet.

### Drive-by fix: stale path reference

`.claude/rules/code-forge.md` still points at
`docs/design/plans/2026-08-04-gitops-github-workflow-plan.md`, which
the development-conventions round's docs reorg moved to
`docs/design/plans/core/2026-08-04-gitops-github-workflow-plan.md`.
Corrected as part of this round since it's directly in the area this
spec touches.

## Verification

- `.claude/rules/git-github-standards.md` and `.claude/skills/
  audit-graph-compliance/SKILL.md` both read accurately against this
  spec once updated.
- `gh api repos/tatemeyer/ttui --jq '.delete_branch_on_merge'` returns
  `true` after the repo-setting change.
- `.claude/rules/code-forge.md`'s path reference resolves to a real
  file.
- No code/test changes this round — this is `admin`/`git-adjacent`-
  tagged work under the tier scheme it itself defines (Slice 2), fully
  consistent with staying TDD-exempt per `development-conventions.md`.
