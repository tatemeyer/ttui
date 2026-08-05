# `.claude` Audit & Templates Relocation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Structure note:** This plan is organized as **Arcs → Slices → Tasks**
> per `docs/design/README.md`, not the flat "Task N" list the
> `writing-plans` skill defaults to. Arc/Slice headings are pure
> grouping; tasks still follow the skill's bite-sized step structure,
> adapted for this plan's documentation/file-move work — all four
> tasks are tagged `admin`/`git-adjacent`, not `coding`, so the
> TDD-mandatory policy in `.claude/rules/development-conventions.md`
> doesn't apply to them.

**Goal:** Move `templates/` into `.claude/templates/`, fix two stale
path/fact references, remove an untracked backup file, and fix the
core framework plan's Task 1 `.gitignore` step so it stops overwriting
the repo's real `.gitignore`, per
`docs/design/specs/2026-08-05-claude-audit-templates-design.md`.

**Architecture:** No application architecture — this plan moves one
directory and edits three existing documents. No code, no CI changes.

**Tech Stack:** N/A (file moves and documentation only).

## Global Constraints

- `templates/` moves to `.claude/templates/` with its subfolder
  structure (`ci/`, `github/`, `repo/`) unchanged — a straight move,
  no reorganization.
- Historical GitOps spec/plan documents that reference the old
  `templates/...` path are NOT rewritten — they document what was
  decided/done at the time. Only the one currently-live reference
  (`docs/tooling/submodule-upgrade.md`) is updated.
- Content of `.claude/rules/code-forge.md`,
  `.claude/rules/diagram-standards.md`, and
  `.claude/rules/git-github-standards.md` is untouched — each stays
  deferred to its own future `/superpowers:brainstorm` pass.
- `.claude/settings.json.bak` is untracked by git (confirmed via
  `git ls-files` returning nothing for it) — its deletion needs no
  commit.

---

## Arc 1: Templates Relocation

### Slice 1.1: Move `templates/` and update its one live reference

**Tags:** admin, git-adjacent

#### Task 1: `git mv templates .claude/templates`, fix `submodule-upgrade.md`

**Files:**
- Move: `templates/` → `.claude/templates/` (all contents, `git mv`)
- Modify: `docs/tooling/submodule-upgrade.md`

**Interfaces:**
- Consumes: nothing (first task).
- Produces: `.claude/templates/{ci,github,repo}/...` at the new
  location; `templates/` no longer exists at repo root.

- [ ] **Step 1: Move the directory**

```bash
git mv templates .claude/templates
```

- [ ] **Step 2: Verify the move**

Run: `ls templates 2>&1`
Expected: `ls: templates: No such file or directory` (or platform
equivalent — the point is the old path is gone)

Run: `ls .claude/templates`
Expected: `ci`, `github`, `repo` subdirectories listed

- [ ] **Step 3: Update `docs/tooling/submodule-upgrade.md`'s opening paragraph**

The file's opening paragraph (the four lines after the `# Python
Tooling Upgrade Procedure` heading, before `## Standard procedure`)
currently reads:

```markdown
Scoped to Python-based tooling in this repo (`code-review-graph`, and
any future scripts scaffolded from
`templates/repo/pyproject.toml.template`) — not TTUI's own core
language, which is undecided.
```

Replace it with:

```markdown
Scoped to Python-based tooling in this repo (`code-review-graph`, and
any future scripts scaffolded from
`.claude/templates/repo/pyproject.toml.template`) — not TTUI's own
core language, which is Rust (see root `CLAUDE.md`).
```

This both updates the path for the move and fixes the stale
"undecided" claim — Rust was decided in the core framework design doc.
Nothing else in the file changes.

- [ ] **Step 4: Verify the edit**

The new path text (`.claude/templates/repo/pyproject.toml.template`)
contains the old bare path as a substring, so a plain substring grep
for the old path would false-positive-match the new text too. Anchor
on the backtick immediately preceding the path to tell them apart (the
old text has `` `templates/repo/... `` directly; the new text has
`` `.claude/templates/repo/... ``, so a backtick directly followed by
`templates` — no `.claude/` in between — only matches the old,
unfixed version):

```
grep -c '`templates/repo/pyproject.toml.template`' docs/tooling/submodule-upgrade.md
```

Expected: `0`

Run: `grep -c ".claude/templates/repo/pyproject.toml.template" docs/tooling/submodule-upgrade.md`
Expected: `1`

Run: `grep -c "which is undecided" docs/tooling/submodule-upgrade.md`
Expected: `0`

- [ ] **Step 5: Commit**

```bash
git add -A templates .claude/templates docs/tooling/submodule-upgrade.md
git commit -m "chore: move templates/ into .claude/templates/"
```

(`git add -A` on both old and new paths ensures the rename is recorded
as a rename, not a delete+add, if `git mv` didn't already stage it —
harmless either way since `git mv` already staged the move.)

---

## Arc 2: Stale Reference Fix

### Slice 2.1: Fix `audit-graph-compliance.md`'s stale doc paths

**Tags:** admin, git-adjacent

#### Task 2: Correct `docs/superpowers/` → `docs/design/` in the skill stub

**Files:**
- Modify: `.claude/skills/audit-graph-compliance/SKILL.md`

**Interfaces:**
- Consumes: nothing from Task 1 (independent edit, different file).
- Produces: a corrected reference in a still-stub skill file — no
  change to the skill's stub status or its "not yet implemented"
  framing.

- [ ] **Step 1: Locate and replace the stale reference**

Item 5 of the "Intended responsibilities" list currently reads:

```markdown
5. **Artifact completeness** — confirm that a unit of work has both its
   `docs/superpowers/specs/...` design doc and
   `docs/superpowers/plans/...` plan present and linked before the
   branch is considered mergeable.
```

Replace it with:

```markdown
5. **Artifact completeness** — confirm that a unit of work has both its
   `docs/design/specs/...` design doc and
   `docs/design/plans/...` plan present and linked before the
   branch is considered mergeable.
```

Nothing else in the file changes — it remains a stub with the same
"Status: stub" note and the same dependencies on `code-forge.md` and
`diagram-standards.md`.

- [ ] **Step 2: Verify the edit**

Run: `grep -c "docs/superpowers" .claude/skills/audit-graph-compliance/SKILL.md`
Expected: `0`

Run: `grep -c "docs/design/specs\|docs/design/plans" .claude/skills/audit-graph-compliance/SKILL.md`
Expected: `2`

- [ ] **Step 3: Commit**

```bash
git add .claude/skills/audit-graph-compliance/SKILL.md
git commit -m "fix(skills): correct stale docs/superpowers path in audit-graph-compliance stub"
```

---

## Arc 3: Repository Cleanup

### Slice 3.1: Remove untracked backup file

**Tags:** admin, git-adjacent

#### Task 3: Delete `.claude/settings.json.bak`

**Files:**
- Delete: `.claude/settings.json.bak` (untracked — this is a plain
  filesystem delete, not a git operation)

**Interfaces:**
- Consumes: nothing.
- Produces: nothing — this task removes a file, it doesn't produce
  anything later tasks depend on.

- [ ] **Step 1: Confirm the file is untracked before deleting**

Run: `git ls-files .claude/settings.json.bak`
Expected: empty output (confirms git isn't tracking it, so no commit
will be needed for its removal)

- [ ] **Step 2: Delete the file**

```bash
rm .claude/settings.json.bak
```

- [ ] **Step 3: Verify**

Run: `ls .claude/settings.json.bak 2>&1`
Expected: `ls: .claude/settings.json.bak: No such file or directory` (or
platform equivalent)

Run: `git status --short`
Expected: no output related to `.claude/settings.json.bak` (since it
was never tracked, its deletion produces no git status change — this
step is a sanity check that nothing unexpected got staged, not a
commit step).

---

## Arc 4: Core Framework Plan Fix

### Slice 4.1: Stop Task 1's `.gitignore` step from overwriting the real file

**Tags:** admin, git-adjacent

#### Task 4: Change Task 1's `.gitignore` step from `Create` to `Modify` (append)

**Files:**
- Modify: `docs/design/plans/2026-08-04-ttui-core-framework-plan.md`

**Interfaces:**
- Consumes: nothing from Tasks 1-3 (independent edit, different file).
- Produces: a corrected core framework plan whose Task 1, once
  executed, appends to the repo's existing `.gitignore` instead of
  overwriting it. The repo's actual `.gitignore` today has 5 sections
  (code-review-graph, OS, editors, Python tooling, installer backups)
  that Task 1 as currently written would destroy.

- [ ] **Step 1: Update the Files list**

In `docs/design/plans/2026-08-04-ttui-core-framework-plan.md`, find
`#### Task 1: Initialize the Cargo project`. Its **Files** list
currently reads:

```markdown
**Files:**
- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `.gitignore` (Rust section: `/target`)
- Create: `tests/README.md`
```

Replace the `.gitignore` line only:

```markdown
**Files:**
- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Modify: `.gitignore` (append Rust section: `/target`)
- Create: `tests/README.md`
```

- [ ] **Step 2: Replace Step 3 (currently "Create `.gitignore`")**

Change Step 3's heading from "Create `.gitignore`" to "Append a Rust
section to `.gitignore`" (keep the same `- [ ] **Step 3: ...**`
checkbox format every other step in this document uses). Directly
below the new heading, before the fenced block, add this sentence:
"The repo's `.gitignore` already exists with code-review-graph/OS/
editor/Python-tooling/installer-backup sections — append to it, do not
overwrite (the code-review-graph section in particular is
installer-managed and must not be lost):"

Then change the fenced block's content from just `/target` to:

```
# Rust
/target
```

- [ ] **Step 3: Verify the edit**

Run (the search pattern itself contains backticks, so it's shown as a
command block rather than inline code):

```
grep -c "Modify: \`.gitignore\`" docs/design/plans/2026-08-04-ttui-core-framework-plan.md
```

Expected: `1`

```
grep -c "^- Create: \`.gitignore\`" docs/design/plans/2026-08-04-ttui-core-framework-plan.md
```

Expected: `0`

Run: `grep -c "^#### Task 1:" docs/design/plans/2026-08-04-ttui-core-framework-plan.md`
Expected: `1` (confirms no duplicate heading was introduced)

- [ ] **Step 4: Commit**

```bash
git add docs/design/plans/2026-08-04-ttui-core-framework-plan.md
git commit -m "fix(core-framework-plan): stop Task 1 from overwriting the real .gitignore"
```
