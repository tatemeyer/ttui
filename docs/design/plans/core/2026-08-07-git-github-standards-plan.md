# Git/GitHub Standards Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `docs/design/specs/core/2026-08-07-git-github-standards-design.md`:
resolve `git-github-standards.md`'s worktree-lifecycle and autonomy-
tier questions, give `audit-graph-compliance` real check logic for
three of its five responsibilities, and fix two small pieces of
concrete drift (a stale path reference, `delete_branch_on_merge`).

**Architecture:** Four small, independent tasks — a rules-file
rewrite, a skill-file rewrite, a one-line path fix in a third file, and
one `gh api` repo-setting change. No `src/`/`examples/` changes, so no
TDD; all four tasks are `admin`/`git-adjacent`-tagged under the very
tier scheme Task 1 defines.

**Tech Stack:** Markdown (rules/skill files), GitHub CLI (`gh api`).

## Global Constraints

- No `src/`/`examples/` changes this round — every task is doc/skill-
  file authoring or a single repo-setting API call.
- Each task's content is copied from the approved spec's Design
  section verbatim where the spec already gives exact wording;
  nothing here is newly invented beyond what the spec already settled.

---

### Task 1: Resolve `.claude/rules/git-github-standards.md`

**Files:**
- Modify: `.claude/rules/git-github-standards.md`

Doc-only. No TDD. Tagged `admin` under this task's own new tier
scheme (Direct tier — zero `src/`/`examples/` change).

- [ ] **Step 1: Replace the file's `Status` line and open-questions
  section.** Read the current file first (still has "Status: stub"
  and a trailing "## Open questions to resolve via brainstorming"
  section listing the two items this plan resolves). Replace the
  whole file with:

```markdown
# Git/GitHub Standards

**Status:** fully defined. See
`docs/design/specs/core/2026-08-07-git-github-standards-design.md` for
the full rationale, including the concrete drift (a 56-commit unmerged
branch, ~13 orphaned worktrees) that motivated these rules.

## Worktree/branch lifecycle

One worktree per Arc, created via `superpowers:using-git-worktrees`
(the `EnterWorktree` tool) at the start of an Arc's implementation
plan. Branch naming stays the tool's existing `worktree-<descriptor>`
convention.

**Ship at the end of the Arc, not later.** Once every task in the
Arc's plan is checked off and final verification is green: open a PR
from the worktree branch to `main` using the existing PR template,
wait for the four required status checks (`build`/`test`/`clippy`/
`fmt`) to pass, **squash-merge**, then remove the worktree via
`ExitWorktree`. Squash merge specifically — keeps `main`'s history one
commit per Arc and avoids `main` accumulating every intermediate
task-commit from inside the worktree.

`enforce_admins` stays `false` — the autonomy-tier scheme below is
what actually gates which work may use that bypass, not the
branch-protection setting itself. `delete_branch_on_merge` is `true`
(flipped from `false` — a merged PR's branch no longer needs manual
deletion).

## Autonomy tiers

Three tiers, reusing the existing Slice/Task tags (`coding`,
`research`, `admin`, `git-adjacent` — see `docs/design/README.md`) as
the primary signal rather than a second, parallel label vocabulary.
Deliberately smaller than the four-tier scheme this was compared
against (`autonomy:safe`/`review`/`human`/`needs-intent`) — TTUI
already has a structural equivalent of `needs-intent` in the
brainstorming skill's mandatory clarifying-questions gate before any
implementation starts.

- **Direct** — `admin`/`git-adjacent`-tagged work with zero `src/`/
  `examples/` behavior change (a rules-file edit, a `.github/`
  workflow tweak, a docs-only commit with no code touched). May push
  straight to `main`; no PR required.
- **Gated** (the default) — all `coding`- and `research`-tagged work.
  Requires a PR with all four required checks green before merge. No
  separate human-approval wait beyond that (`required_approving_
  review_count` is already `0`) — this tier means "agent-completable
  unattended, gated on objective checks," not "needs a human to look
  at it."
- **Human** — branch-protection or CI-workflow-config changes,
  force-pushes, deleting a branch/worktree the current session didn't
  create itself, or anything the user explicitly flags as sensitive in
  the moment. Requires explicit human sign-off before merge — green
  checks alone aren't sufficient.

A Slice/Task's existing tag(s) determine its default tier
(`admin`/`git-adjacent` → Direct, `coding`/`research` → Gated); Human
is never inferred from a tag, only from the specific trigger list
above.

## `audit-graph-compliance`

See `.claude/skills/audit-graph-compliance/SKILL.md` for the actual
check logic. Three of its five named responsibilities (artifact
completeness, branch freshness, worktree hygiene) are implemented;
interrupt handling and the label/diagram-dependent half of structural
enforcement stay stubbed, blocked on `diagram-standards.md` and
`code-forge.md`.

## Open questions to resolve via brainstorming

- **Agent system for anticipating intentions, usage, and
  expectations** — the most novel item in this file's original scope;
  still not designed. Likely related to, but distinct from, the
  deferred "agent state as graph nodes" scheduling idea noted in the
  root project context.
```

- [ ] **Step 2: Commit**

```bash
git add .claude/rules/git-github-standards.md
git commit -m "$(cat <<'EOF'
docs(git-github): resolve worktree lifecycle and autonomy tiers

Formalizes per-Arc worktree lifecycle (PR + squash-merge + remove) and
a 3-tier Direct/Gated/Human autonomy scheme, driven by the approved
design spec's findings of real branch/worktree drift in this repo.
EOF
)"
```

---

### Task 2: Give `audit-graph-compliance` real check logic

**Files:**
- Modify: `.claude/skills/audit-graph-compliance/SKILL.md`

Doc-only (skill instructions, not compiled code). No TDD. `admin`-
tagged, Direct tier.

- [ ] **Step 1: Replace the file's stub status and intended-
  responsibilities list with real check procedures.** Read the current
  file first. Replace the whole file with:

```markdown
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
```

- [ ] **Step 2: Commit**

```bash
git add .claude/skills/audit-graph-compliance/SKILL.md
git commit -m "$(cat <<'EOF'
feat(skills): give audit-graph-compliance real check logic

Implements artifact-completeness, branch-freshness, and worktree-
hygiene checks per the approved git/GitHub standards design spec.
Interrupt handling and label/diagram-dependent checks stay stubbed,
blocked on code-forge.md and diagram-standards.md.
EOF
)"
```

---

### Task 3: Fix `code-forge.md`'s stale path reference

**Files:**
- Modify: `.claude/rules/code-forge.md`

Doc-only, one-line fix. No TDD. `admin`-tagged, Direct tier.

- [ ] **Step 1: Fix the path** — replace:

```markdown
  (`docs/design/plans/2026-08-04-gitops-github-workflow-plan.md`, Task 7)
```

  with:

```markdown
  (`docs/design/plans/core/2026-08-04-gitops-github-workflow-plan.md`, Task 7)
```

- [ ] **Step 2: Confirm the path now resolves**

Run: `test -f docs/design/plans/core/2026-08-04-gitops-github-workflow-plan.md && echo exists`
Expected: `exists`

- [ ] **Step 3: Commit**

```bash
git add .claude/rules/code-forge.md
git commit -m "fix(docs): correct stale plan path in code-forge.md"
```

---

### Task 4: Flip `delete_branch_on_merge` to `true`

**Files:** none (repo setting, not a file)

`git-adjacent`-tagged, Direct tier, per this round's own new autonomy
scheme.

- [ ] **Step 1: Confirm current value**

Run: `gh api repos/tatemeyer/ttui --jq '.delete_branch_on_merge'`
Expected: `false`

- [ ] **Step 2: Flip it**

Run: `gh api -X PATCH repos/tatemeyer/ttui -f delete_branch_on_merge=true`
Expected: `200` response (no error).

- [ ] **Step 3: Confirm the new value**

Run: `gh api repos/tatemeyer/ttui --jq '.delete_branch_on_merge'`
Expected: `true`

No commit — this step has no file to add, it's a live GitHub repo
setting.

---

## Self-Review

**Spec coverage:** Slice 1 (worktree/branch lifecycle, squash-merge,
the two repo-setting fixes) — Task 1 (policy text) + Task 4 (the
actual API flip). Slice 2 (autonomy tiers) — Task 1. Slice 3
(`audit-graph-compliance` real logic) — Task 2. The drive-by path fix
— Task 3. Verification section (`gh api` check, path resolves, files
read accurately) — covered across Tasks 1-4's own steps.

**Placeholder scan:** no TBD/TODO. Every task's replacement content is
copied verbatim from the approved spec's Design section, not
paraphrased or deferred.

**Type/reference consistency:** the tier names (Direct/Gated/Human)
and their trigger conditions are identical between Task 1's rules-file
text and the spec. Task 2's `audit-graph-compliance` checks reference
exactly the file paths/tag names Task 1 establishes
(`docs/design/specs/<arc>/`, `docs/design/plans/<arc>/`,
`admin`/`git-adjacent`/`coding`/`research` tags) — no drift between
the two files' terminology.

**Task ordering:** Tasks 1-4 have no interdependencies (each touches a
different file or the API, none reads output the others produce) and
can run in any order; listed in spec-slice order for readability.
