# Worktree Cleanup Procedure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Implement `docs/design/specs/core/2026-08-08-worktree-cleanup-procedure-design.md`:
capture the corrected, squash-merge-aware worktree cleanup as a
repeatable runbook that consumes `audit-graph-compliance`'s buckets and
defines the action + autonomy tier per bucket, so the "orphaned
worktrees" drift `git-github-standards.md` was written against does not
recur.

**Architecture:** Two doc-authoring tasks — write the procedure into a
durable location, and cross-link it from the two files that reference
cleanup but don't describe it. No `src/`/`examples/` changes, so no
TDD; both tasks are `git-adjacent`/`admin`-tagged (Direct tier).

**Tech Stack:** Markdown (spec + rules/skill cross-links).

## Global Constraints

- No `src/`/`examples/` changes — doc authoring and cross-linking only.
- Do **not** duplicate merged-detection logic: the runbook references
  `audit-graph-compliance`'s three buckets, it never re-implements the
  `gh pr list` / ancestry classification.
- The spec is the source of truth for the procedure's wording; these
  tasks copy from it rather than reinventing.

---

### Task 1: Land the procedure spec as the runbook of record

**Files:**
- Add: `docs/design/specs/core/2026-08-08-worktree-cleanup-procedure-design.md` (done)
- Add: `docs/design/plans/core/2026-08-08-worktree-cleanup-procedure-plan.md` (this file)

Doc-only. No TDD. Tagged `git-adjacent`/`admin` (Direct tier — zero
`src/`/`examples/` change).

- [x] **Step 1: Confirm the spec captures the six-step procedure,** the
  autonomy-tier table, the `locked`-worktree edge case, and the
  ownership (Human-tier) gate. The spec is the runbook — no separate
  runbook file is created; keeping it in `docs/design/specs/core/`
  matches the docs-organization rule and avoids a second source of
  truth.
- [x] **Step 2: Verify the spec's Verification section** matches the
  concrete current-state classification (5 named PR-merged + 7
  ancestry-merged agent worktrees removable; 6 agent branches triaged
  by commits-ahead; locked #27 cleared as stale-lock-on-merged).

### Task 2: Cross-link the procedure from the files that reference cleanup

**Files:**
- Modify: `.claude/rules/git-github-standards.md`
- Modify: `.claude/skills/audit-graph-compliance/SKILL.md`

Doc-only. No TDD. Tagged `git-adjacent`/`admin` (Direct tier).

- [x] **Step 1: Add a forward pointer in `git-github-standards.md`'s
  "Worktree/branch lifecycle" section** to the cleanup procedure spec,
  framed as "when end-of-Arc removal was skipped and worktrees
  accumulate, follow …" — one line, leaving the lifecycle rules
  themselves unchanged.
- [x] **Step 2: Add a closing pointer in the audit skill's "Worktree
  hygiene" section** noting that the *action* to take on each reported
  bucket is defined in the cleanup procedure spec — reinforcing that the
  skill classifies and the procedure acts, with no overlap.

---

## Verification (whole plan)

- Both docs exist under the correct per-Arc `core/` subdirectories with
  the `YYYY-MM-DD-<topic>` filename convention.
- `git-github-standards.md` and the audit skill each point to the
  procedure spec; neither duplicates the classification or the action
  steps.
- Applying the runbook to the current worktree situation yields a clean
  re-audit (empty removable/abandoned buckets), per the spec's
  Verification section.
