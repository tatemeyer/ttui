# GitOps: GitHub Workflow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Structure note:** This plan is organized as **Arcs → Slices → Tasks**
> per `docs/design/README.md`, not the flat "Task N" list the
> `writing-plans` skill defaults to. Arc/Slice headings are pure
> grouping; tasks still follow the skill's bite-sized step structure,
> adapted for config/git-adjacent work (no unit-testable code here, so
> "test" steps are verification commands instead).

**Goal:** Stand up TTUI's GitHub presence — a new public repo, required
CI, branch protection, issue intake, and label/PR-template fixes — so
the core framework plan (`docs/design/plans/2026-08-04-ttui-core-framework-plan.md`)
has somewhere to land its first PR.

**Architecture:** No application architecture — this plan produces repo
configuration and `.github/` files. Ordering matters: all bootstrap
content (CI workflow, issue templates, PR template, labels) is pushed
directly to `main` *before* branch protection is enabled in the final
task, so no chicken-and-egg PR ceremony is needed to bootstrap the repo
that will later require PRs.

**Tech Stack:** GitHub, GitHub Actions, `gh` CLI (already authenticated
to the user's account, verified working).

## Global Constraints

- Repo name: `ttui`, public visibility, created fresh (verified via
  `gh repo view tatemeyer/ttui` returning "could not resolve" — name is
  free).
- Local branch `master` is renamed to `main` before the first push —
  branch protection and CI both target `main`.
- CI file `.github/workflows/ci.yml` triggers on `pull_request` and
  `push` to `main`; exactly four required jobs: `build` (`cargo build`),
  `test` (`cargo test`), `clippy` (`cargo clippy -- -D warnings`), `fmt`
  (`cargo fmt --check`).
- No `Cargo.toml` exists yet in this repo (the core framework plan is
  still blocked, pending this plan). The CI jobs above are **expected to
  fail** the first time they run, since there's nothing to build yet —
  this is fine and doesn't block anything, because branch protection
  isn't enabled until the last task, after all bootstrap files are
  already on `main`.
- Issue intake is two structured forms only — `bug_report.yml` and
  `feature_request.yml` under `.github/ISSUE_TEMPLATE/` — with blank
  issues disabled. No freeform fallback template.
- Labels: exactly `bug` (`d73a4a`), `enhancement` (`a2eeef`), `docs`
  (`0075ca`), `needs-design` (`fbca04`), matching
  `templates/github/labels.yml`. No new labels added (autonomy-tier
  scheme is out of scope, deferred to the `code-forge.md` brainstorm).
- PR template sourced from `templates/github/PULL_REQUEST_TEMPLATE.md`
  with its stale `docs/superpowers/{specs,plans}/` paths corrected to
  `docs/design/{specs,plans}/`.
- Branch protection on `main`: PRs required, the four CI checks required
  to pass, repo admin retains standard GitHub bypass (no separate
  override process to build).

---

## Arc 1: Repository Creation

### Slice 1.1: Rename branch and push to a new public GitHub repo

**Tags:** git-adjacent, admin

#### Task 1: Create `tatemeyer/ttui` and push the existing history

**Files:**
- None created/modified — this task operates on git/GitHub state, not
  repo files.

**Interfaces:**
- Consumes: the existing local repo on branch `master` (4 commits, no
  remote configured — verified via `git remote -v` returning empty).
- Produces: a public GitHub repo `tatemeyer/ttui` with a `main` branch
  matching current local history, and a `origin` remote in the local
  repo pointing at it. Every later task in this plan pushes commits to
  this remote.

- [ ] **Step 1: Rename the local branch from `master` to `main`**

Run: `git branch -m master main`

- [ ] **Step 2: Verify the rename**

Run: `git branch --show-current`
Expected: `main`

- [ ] **Step 3: Create the GitHub repo from the current source and push**

Run: `gh repo create ttui --public --source=. --remote=origin --push`

This single command creates a public repo named `ttui` under the
authenticated account, adds it as the `origin` remote, and pushes the
current branch (`main`) to it.

- [ ] **Step 4: Verify the repo exists and `main` is pushed**

Run: `gh repo view --web=false --json name,visibility,defaultBranchRef`
Expected: JSON showing `"name": "ttui"`, `"visibility": "PUBLIC"`,
`"defaultBranchRef": {"name": "main", ...}`.

Run: `git log origin/main --oneline -1`
Expected: shows the same commit as `git log main --oneline -1` (local
and remote `main` match).

---

## Arc 2: Continuous Integration

### Slice 2.1: CI workflow

**Tags:** git-adjacent, admin

#### Task 2: Add `.github/workflows/ci.yml`

**Files:**
- Create: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: the `origin` remote from Task 1.
- Produces: four named GitHub Actions checks (`build`, `test`, `clippy`,
  `fmt`) that Task 7's branch protection will require by name.

- [ ] **Step 1: Write the workflow file**

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main]
  pull_request:

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build --verbose

  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --verbose

  clippy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - run: cargo clippy --all-targets -- -D warnings

  fmt:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
      - run: cargo fmt --check
```

- [ ] **Step 2: Commit and push directly to `main`**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add build/test/clippy/fmt workflow"
git push origin main
```

Branch protection isn't enabled yet (that's Task 7), so a direct push to
`main` is expected to work here.

- [ ] **Step 3: Verify the workflow triggered**

Run: `gh run list --workflow=ci.yml --limit=1`
Expected: one run listed, triggered by the push in Step 2.

- [ ] **Step 4: Confirm the four jobs ran (pass/fail doesn't matter yet)**

Run: `gh run view --workflow=ci.yml`
Expected: four jobs named `build`, `test`, `clippy`, `fmt` are listed.
They are expected to **fail** — there's no `Cargo.toml` in the repo yet
(per Global Constraints). This step is only confirming the workflow
wiring is correct, not that the (nonexistent) crate builds.

---

## Arc 3: Issue Intake

### Slice 3.1: Structured issue forms, no freeform fallback

**Tags:** git-adjacent, admin

#### Task 3: Add `bug_report.yml`

**Files:**
- Create: `.github/ISSUE_TEMPLATE/bug_report.yml`

**Interfaces:**
- Consumes: the `origin` remote from Task 1.
- Produces: a "Bug Report" issue form selectable when filing a new
  issue.

- [ ] **Step 1: Write the form**

```yaml
# .github/ISSUE_TEMPLATE/bug_report.yml
name: Bug Report
description: Report something that isn't working as expected
title: "[Bug]: "
labels: ["bug"]
body:
  - type: textarea
    id: description
    attributes:
      label: What happened?
      description: A clear description of the bug.
    validations:
      required: true
  - type: textarea
    id: repro
    attributes:
      label: Steps to reproduce
      placeholder: |
        1. ...
        2. ...
    validations:
      required: true
  - type: textarea
    id: expected
    attributes:
      label: Expected behavior
    validations:
      required: true
  - type: textarea
    id: actual
    attributes:
      label: Actual behavior
    validations:
      required: true
  - type: input
    id: environment
    attributes:
      label: Environment
      placeholder: "e.g. Windows 11, Windows Terminal, cargo 1.79"
    validations:
      required: false
```

- [ ] **Step 2: Commit**

```bash
git add .github/ISSUE_TEMPLATE/bug_report.yml
git commit -m "chore: add bug report issue form"
```

(Push happens once with Task 4 in Step 2 of that task, to avoid two
single-file pushes back to back — see Task 4.)

#### Task 4: Add `feature_request.yml` and disable blank issues

**Files:**
- Create: `.github/ISSUE_TEMPLATE/feature_request.yml`
- Create: `.github/ISSUE_TEMPLATE/config.yml`

**Interfaces:**
- Consumes: `bug_report.yml` from Task 3 (pushed together).
- Produces: a "Feature Request" issue form, and `blank_issues_enabled:
  false` so the two forms are the only way to file an issue (per the
  spec's "no freeform fallback template").

- [ ] **Step 1: Write the feature request form**

```yaml
# .github/ISSUE_TEMPLATE/feature_request.yml
name: Feature Request
description: Propose a new feature or enhancement
title: "[Feature]: "
labels: ["enhancement"]
body:
  - type: textarea
    id: problem
    attributes:
      label: What problem does this solve?
      description: What's missing or painful today?
    validations:
      required: true
  - type: textarea
    id: proposal
    attributes:
      label: Proposed solution
    validations:
      required: true
  - type: textarea
    id: alternatives
    attributes:
      label: Alternatives considered
    validations:
      required: false
```

- [ ] **Step 2: Disable the blank-issue fallback**

```yaml
# .github/ISSUE_TEMPLATE/config.yml
blank_issues_enabled: false
```

- [ ] **Step 3: Commit both files and push**

```bash
git add .github/ISSUE_TEMPLATE/feature_request.yml .github/ISSUE_TEMPLATE/config.yml
git commit -m "chore: add feature request issue form, disable blank issues"
git push origin main
```

- [ ] **Step 4: Verify both forms are live and blank issues are off**

Run: `gh issue create --web` (opens the browser to the "new issue"
picker without submitting anything — close the tab without submitting).
Expected: two options shown, "Bug Report" and "Feature Request", with no
"Open a blank issue" link.

---

## Arc 4: PR Template and Labels

### Slice 4.1: Fix and apply the PR template

**Tags:** git-adjacent, admin

#### Task 5: Correct stale paths and publish the PR template

**Files:**
- Modify: `templates/github/PULL_REQUEST_TEMPLATE.md`
- Create: `.github/PULL_REQUEST_TEMPLATE.md`

**Interfaces:**
- Consumes: the existing `templates/github/PULL_REQUEST_TEMPLATE.md`
  (currently references the stale `docs/superpowers/{specs,plans}/`
  paths).
- Produces: `.github/PULL_REQUEST_TEMPLATE.md`, which GitHub
  auto-populates into every new PR's description.

- [ ] **Step 1: Fix the paths in the source template**

In `templates/github/PULL_REQUEST_TEMPLATE.md`, replace:

```markdown
- Design: `docs/superpowers/specs/YYYY-MM-DD-<topic>-design.md`
- Plan: `docs/superpowers/plans/YYYY-MM-DD-<feature-name>.md`
```

with:

```markdown
- Design: `docs/design/specs/YYYY-MM-DD-<topic>-design.md`
- Plan: `docs/design/plans/YYYY-MM-DD-<feature-name>.md`
```

- [ ] **Step 2: Copy the corrected template into `.github/`**

```bash
cp templates/github/PULL_REQUEST_TEMPLATE.md .github/PULL_REQUEST_TEMPLATE.md
```

- [ ] **Step 3: Commit and push**

```bash
git add templates/github/PULL_REQUEST_TEMPLATE.md .github/PULL_REQUEST_TEMPLATE.md
git commit -m "fix: correct stale docs paths in PR template, publish to .github/"
git push origin main
```

- [ ] **Step 4: Verify GitHub picks it up**

Run: `gh api repos/tatemeyer/ttui/contents/.github/PULL_REQUEST_TEMPLATE.md --jq .name`
Expected: `PULL_REQUEST_TEMPLATE.md`

### Slice 4.2: Apply labels

**Tags:** git-adjacent, admin

#### Task 6: Apply the label set from `templates/github/labels.yml`

**Files:**
- None created/modified — this task operates on live GitHub label
  state, not repo files. `templates/github/labels.yml` is read, not
  changed.

**Interfaces:**
- Consumes: `templates/github/labels.yml` (already exists, unchanged by
  this task) and the repo created in Task 1.
- Produces: exactly four labels on `tatemeyer/ttui`: `bug`,
  `enhancement`, `docs`, `needs-design`, with colors/descriptions
  matching the source file. (GitHub seeds new repos with its own
  default label set on creation, some of which — `bug`, `enhancement` —
  already happen to match this file's color/description; `--force`
  below makes all four idempotent regardless.)

- [ ] **Step 1: Apply each label**

```bash
gh label create bug --color d73a4a --description "Something isn't working" --force
gh label create enhancement --color a2eeef --description "New feature or request" --force
gh label create docs --color 0075ca --description "Documentation only" --force
gh label create needs-design --color fbca04 --description "Requires a /superpowers:brainstorm pass before implementation" --force
```

- [ ] **Step 2: Verify**

Run: `gh label list`
Expected: `bug`, `enhancement`, `docs`, `needs-design` all present with
the colors/descriptions above. (GitHub's other seeded defaults —
`documentation`, `duplicate`, `good first issue`, `help wanted`,
`invalid`, `question`, `wontfix` — are untouched; this spec doesn't call
for deleting them.)

---

## Arc 5: Branch Protection

### Slice 5.1: Require PRs and the four CI checks on `main`

**Tags:** git-adjacent, admin

#### Task 7: Enable branch protection on `main`

**Files:**
- None created/modified — this task operates on GitHub repo settings.

**Interfaces:**
- Consumes: the four named checks from Task 2 (`build`, `test`,
  `clippy`, `fmt`) and the `main` branch from Task 1. This is the last
  task — every other task in this plan pushes directly to `main`
  specifically because this task hasn't run yet.
- Produces: branch protection on `main` requiring PRs and the four CI
  checks. After this task, direct pushes to `main` (without admin
  bypass) are rejected.

- [ ] **Step 1: Apply branch protection via the API**

```bash
gh api repos/tatemeyer/ttui/branches/main/protection \
  --method PUT \
  --input - <<'EOF'
{
  "required_status_checks": {
    "strict": false,
    "checks": [
      {"context": "build"},
      {"context": "test"},
      {"context": "clippy"},
      {"context": "fmt"}
    ]
  },
  "enforce_admins": false,
  "required_pull_request_reviews": {
    "dismiss_stale_reviews": false,
    "require_code_owner_reviews": false,
    "required_approving_review_count": 0
  },
  "restrictions": null
}
EOF
```

Two fields do the real work here: `required_pull_request_reviews` must
be a non-null object — that's what actually forces changes through a
PR instead of a direct push (a common mistake is setting this to
`null`, which disables the PR requirement entirely, even with status
checks configured). `required_approving_review_count: 0` keeps that PR
requirement without demanding a second human reviewer, matching the
spec's "solo repo, no second reviewer" decision (0 is documented as a
valid value specifically to mean "require a PR, but don't require
reviewers"). Separately, `enforce_admins: false` preserves the repo
admin's standard GitHub bypass for the exceptions noted in the spec —
not a separate override mechanism, just leaving GitHub's default admin
behavior in place.

`required_status_checks` uses `checks` only, not both `checks` and the
legacy `contexts` array — confirmed against GitHub's live API during
execution (2026-08-04): sending both fields at once returns HTTP 422
("More than one subschema in oneOf matched"), because GitHub's schema
treats `contexts` and `checks` as mutually exclusive alternatives, not
fields that coexist. `checks` is the modern, more precise form (each
entry can optionally pin an `app_id`) and is what's used here.

- [ ] **Step 2: Verify protection is active**

Run: `gh api repos/tatemeyer/ttui/branches/main/protection --jq '.required_status_checks.checks, .required_pull_request_reviews.required_approving_review_count'`
Expected: an array of 4 objects with `context` values `build`, `test`,
`clippy`, `fmt`, followed by `0` — status checks required and
PR-required-with-zero-reviewers both active. (GitHub's GET response may
also include a derived `contexts` array alongside `checks` for backward
compatibility — that's fine, only `checks` was sent in the PUT.)

- [ ] **Step 3: Confirm a direct push is now rejected**

```bash
git commit --allow-empty -m "test: verify branch protection blocks direct push"
git push origin main
```

Expected: push is rejected (`protected branch hook declined` or
similar). Then undo the test commit locally so it doesn't linger:

```bash
git reset --hard HEAD~1
```

This confirms protection is live without leaving a stray commit in
history — from this point on, the core framework plan's first task
lands via a PR, not a direct push.
