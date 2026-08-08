# GitOps: GitHub Workflow — Design

**Status:** Draft, pending your review before we move to planning.
**Date:** 2026-08-04

## Context / Motivation

TTUI has no git remote yet — `git remote -v` is empty, `master` is local
only. Before the core framework plan (`docs/design/plans/2026-08-04-ttui-core-framework-plan.md`)
unblocks, this project needs a settled answer for where the repo lives,
where issues/tickets get filed, and what runs CI.

"GitOps" here means git forge/host + issue tracking + CI, not the
declarative-infra/deployment sense of the term (ArgoCD/Flux-style) — TTUI
is a library with no deploy target, so that meaning doesn't apply.

Decision criteria, in priority order: agentic-first development (the
primary worker on this repo is Claude Code, not a human typing git
commands by hand), streamlined issues/tickets, and no money spent /
avoid enterprise tooling.

GitHub is the presumptive choice, not a genuine bake-off against
GitLab/Gitea/sourcehut: `gh` CLI is already authenticated to the user's
account, and `templates/github/*` and `templates/ci/github-ci.yml`
already assume GitHub's tooling and syntax. This spec covers configuring
GitHub well, not re-litigating the platform choice.

## Scope

**In scope:** repo visibility, branch protection on `main`, required CI
checks and their content, issue templates, label set, PR template
corrections.

**Explicitly out of scope, deferred elsewhere:**
- Autonomy label taxonomy (`autonomy:safe`/`autonomy:review`/etc.) and
  agent-commit-attribution conventions — both belong to the
  `code-forge.md` brainstorm, not this one.
- Per-PR merge-approval policy for agent-authored PRs (auto-merge on
  green CI vs. human click) — depends on the autonomy scheme above, so
  it's deferred to the same `code-forge.md` brainstorm rather than
  decided here.
- What `cargo test` actually covers, coverage expectations, integration
  test structure — this spec only fixes CI's entry point into test
  running (i.e., that `cargo test` runs as a required check), not test
  content. That's the next brainstorm in sequence (Testing/Verifying).

## Decision: Provider

**GitHub**, public repository.

Rationale: public repos get unlimited free GitHub Actions minutes
(private free tier caps at 2,000 min/month), which removes any billing
risk entirely — directly serving the "no money spent" criterion. TTUI is
also meant to be usable by others eventually, which a public repo serves
directly rather than deferring to some later "make it public" step.

## Repo configuration

- **Visibility:** public, from repo creation (not "private now, flip
  later").
- **Branch protection on `main`:** PRs required to merge; required
  status checks (see CI below) must pass. Repo admin (the user) retains
  GitHub's standard admin bypass — this is the escape valve for
  exceptions, not a separate documented override process. No additional
  approval-count requirement is configured (solo repo; a second human
  reviewer doesn't exist).

## CI

`.github/workflows/ci.yml`, triggered on `pull_request` and `push` to
`main`. Four required jobs, all gating merge via branch protection:

- `cargo build`
- `cargo test`
- `cargo clippy -- -D warnings`
- `cargo fmt --check`

This replaces `templates/ci/github-ci.yml` (deleted during
implementation once `.github/workflows/ci.yml` existed — it was a
placeholder, "TODO: add language toolchain setup... once the core
language is chosen," stale now that Rust is the confirmed core
language per the core framework design doc, and its duplicate `name:
CI` would have collided with the real workflow if left in place). The
template's `Makefile`
(`templates/ci/Makefile`) has the same staleness; whether CI calls
`cargo` directly or through `make` targets is an implementation-plan
detail, not a design decision — either satisfies this spec.

## Issues & Labels

- **Issue templates:** structured GitHub issue forms under
  `.github/ISSUE_TEMPLATE/` — separate `bug_report.yml` (repro steps,
  expected vs. actual) and `feature_request.yml` (problem, proposal)
  forms with required fields. No freeform fallback template; the two
  forms cover intake.
- **Scope of Issues:** unplanned work only. Anything already inside an
  Arc/Slice/Task plan doc (`docs/design/plans/`) is tracked via that
  doc's own checkboxes — no duplicate Issue gets filed per Slice or
  Task. Issues exist for bugs, ideas, and things that haven't gone
  through brainstorm → spec → plan yet.
- **Labels:** keep the existing minimal set in
  `templates/github/labels.yml` as-is (`bug`, `enhancement`, `docs`,
  `needs-design`). No new labels added by this spec — the autonomy-tier
  scheme is explicitly deferred (see Scope).

## PR template

Keep the structure of `templates/github/PULL_REQUEST_TEMPLATE.md`
(Summary / Verification / Stacked-PR-note sections) but fix its stale
path references: `docs/superpowers/specs/` → `docs/design/specs/`, and
`docs/superpowers/plans/` → `docs/design/plans/`, matching the actual
locations established in `docs/design/README.md`.

## Success criteria (this spec's "done")

- `.github/workflows/ci.yml` exists, runs the four required checks on
  PR and push to `main`.
- Branch protection on `main` requires those checks and requires PRs.
- `.github/ISSUE_TEMPLATE/bug_report.yml` and `feature_request.yml`
  exist with the required fields above.
- `templates/github/PULL_REQUEST_TEMPLATE.md`'s paths are corrected and
  copied to `.github/PULL_REQUEST_TEMPLATE.md`.
- `templates/github/labels.yml`'s labels are applied to the live repo.
- The local `master` branch is renamed to `main` and pushed to a new
  public GitHub repo — branch protection and CI triggers above both
  target `main`, so the rename happens before or as part of that push,
  not left as a follow-up.

## Explicitly deferred / open questions for future revisions

- Autonomy label taxonomy and what triggers each tier — `code-forge.md`
  brainstorm.
- Agent authorship/attribution convention (commit trailers, PR
  metadata) — `code-forge.md` brainstorm.
- Merge-approval policy for agent-authored PRs — `code-forge.md`
  brainstorm, downstream of the autonomy taxonomy above.
- Test content, coverage expectations, integration test structure —
  Testing/Verifying brainstorm (next in sequence).
