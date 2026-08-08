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
