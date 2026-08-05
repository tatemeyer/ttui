# `.claude` Audit & Templates Relocation — Design

**Status:** Draft, pending your review before we move to planning.
**Date:** 2026-08-05

## Context / Motivation

The third and last of the original three open areas (`.claude` rules/
skills/templates housekeeping, following GitOps and Testing &
Verification conventions). Root `CLAUDE.md` asks that `.claude/rules/`
be read before proposing any design; this pass cleans up staleness and
structure in that directory and in `.claude/skills/`, and relocates
`templates/` (currently at repo root) into `.claude/`.

Also folds in a small, unrelated fix surfaced by the Testing &
Verification conventions' final review: the core framework plan's
Task 1 would clobber the repo's existing `.gitignore` if executed
literally.

## Scope

**In scope:**
- Move `templates/` → `.claude/templates/`.
- Fix two stale-reference bugs found during exploration (same class as
  the PR template path bug fixed during the GitOps plan).
- Delete `.claude/settings.json.bak` (untracked cruft).
- Fix the core framework plan's Task 1 `.gitignore` step.
- Document the cross-machine/user-level audit finding (see below) —
  no action required there.

**Explicitly out of scope:**
- Resolving the actual content of the three still-stub rules files
  (`code-forge.md`, `diagram-standards.md`, `git-github-standards.md`).
  This pass only touches structure/staleness, never their open
  questions — those stay deferred to their own future
  `/superpowers:brainstorm` passes, unchanged from today.
- Any change to `.claude/skills/`'s four implemented skills
  (`explore-codebase`, `debug-issue`, `refactor-safely`,
  `review-changes`) — reviewed during exploration, found consistent
  and current, nothing to fix.
- Any change to `.claude/settings.json` or `.claude/settings.local.json`
  themselves (only the `.bak` file is touched).

## Decision: Cross-machine/user audit finding

**No action required.** `~/.claude` was inspected during exploration:
no user-level `CLAUDE.md`, no user-level `agents/`, `~/.claude/skills/`
is empty, and everything else there (`plugins/`, `settings.json`,
session/cache/daemon state) is generic Claude Code machine
infrastructure, not project-specific rules or skills content. There is
nothing to reconcile, migrate, or dedupe against the project-level
`.claude/`. This is recorded here as the audit's conclusion for that
scope, not as a task — nothing to implement.

## Decision: Templates relocation

`templates/` moves to `.claude/templates/`, subfolder structure
unchanged (`ci/`, `github/`, `repo/`) — a straight move, no
reorganization. `templates/ci/` currently holds only `Makefile` and
`pre-commit-config.yaml` (the former `github-ci.yml` was deleted during
the GitOps plan's final review).

One currently-live path reference needs updating alongside the move:
`docs/tooling/submodule-upgrade.md` references
`templates/repo/pyproject.toml.template`. Every other reference to the
old `templates/...` path lives inside already-completed, historical
GitOps spec/plan documents (`docs/design/specs/2026-08-04-gitops-
github-workflow-design.md`, `docs/design/plans/2026-08-04-gitops-
github-workflow-plan.md`) — those describe what was decided/done at
the time and are not rewritten to track the new location, same
principle as not rewriting git history.

## Decision: Stale-reference fixes

Two fixes, both structural (path/fact corrections, not new content):

1. **`.claude/skills/audit-graph-compliance/SKILL.md`** — currently a
   stub (its actual checks depend on `code-forge.md` and
   `diagram-standards.md`, both still stubs, per its own "Status:
   stub" note — unaffected by this decision, still deferred). Its
   "Intended responsibilities" section references
   `docs/superpowers/specs/...` and `docs/superpowers/plans/...` — the
   same stale path this project already fixed once in the PR template
   during the GitOps plan. Corrected to `docs/design/specs/...` and
   `docs/design/plans/...`.
2. **`docs/tooling/submodule-upgrade.md`** — states TTUI's core
   language is "undecided." Rust was decided in the core framework
   design doc well before this document was last touched. Corrected to
   say so, alongside the path update from the templates move above.

## Decision: `.claude/settings.json.bak` cleanup

Deleted. Confirmed untracked by git (`git ls-files` returns nothing for
it — it already matches the `*.bak` pattern in the root `.gitignore`),
so this is a plain filesystem delete with no commit required for the
deletion itself; it simply stops existing in the working tree.

## Decision: Core framework plan `.gitignore` fix

The core framework plan's Task 1 (`docs/design/plans/2026-08-04-ttui-
core-framework-plan.md`) currently lists `- Create: \`.gitignore\`
(Rust section: \`/target\`)` and a Step 3 that would overwrite the file
with just `/target`. The repo's actual `.gitignore` already has 5
sections (code-review-graph, OS, editors, Python tooling, installer
backups). Changed to `- Modify: \`.gitignore\` (append Rust section:
\`/target\`)`, with Step 3 appending a `# Rust` / `/target` block to
the end of the existing file rather than replacing it.

## Success criteria (this spec's "done")

- `templates/` no longer exists at repo root; `.claude/templates/`
  exists with the same `ci/`/`github/`/`repo/` subfolder contents.
- `docs/tooling/submodule-upgrade.md` references
  `.claude/templates/repo/pyproject.toml.template` and no longer
  claims the core language is undecided.
- `.claude/skills/audit-graph-compliance/SKILL.md` references
  `docs/design/{specs,plans}/`, not `docs/superpowers/{specs,plans}/`.
- `.claude/settings.json.bak` no longer exists.
- The core framework plan's Task 1 `.gitignore` step is `Modify`
  (append), not `Create` (overwrite).

## Explicitly deferred / open questions for future revisions

- Content of `code-forge.md`, `diagram-standards.md`,
  `git-github-standards.md` — each still needs its own
  `/superpowers:brainstorm` pass, unchanged by this spec.
