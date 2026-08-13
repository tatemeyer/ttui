# Release Governance — Design

**Status:** draft, pending review before we move to planning.
**Date:** 2026-08-13
**Relationship to prior work:** resolves `.claude/rules/code-forge.md`,
a stub since early in the project with four open categories (labels,
agent authorship, filing work, model-tiered dispatch). Sub-project #1
of the TTUI v1.0.0 initiative — the sweep audit (sub-project #2)
depends on the triage process defined here to know how to route what
it finds.

## Problem

`code-forge.md` has sat as an unresolved stub since the project's
early GitOps work, explicitly blocking half of `audit-graph-
compliance`'s structural-enforcement checks. Separately, shipping
v1.0.0 means deciding, for the first time, what "breaking" means for
this crate and what happens when stabilization work turns up small
gaps across the many Arcs built so far — without a defined process,
"sweep the codebase for issues" has no way to decide what's urgent,
what's a public promise, and what can wait.

## Scope

**Tag: `admin`/`git-adjacent`** — this is process/documentation work
with no `src/`/`examples/` behavior change, so it qualifies for the
**Direct** autonomy tier already defined in `git-github-standards.md`
(no PR required, may commit straight to `main`) once approved. No TDD
applies (matches the existing tag conventions in
`docs/design/README.md`).

Four slices, matching `code-forge.md`'s four open categories:

1. **SemVer policy** — scope, what counts as breaking, how it relates
   to the pre-v1.0.0 version number.
2. **Label taxonomy** — the concrete GitHub labels the sweep audit
   files findings under.
3. **Filing-work rule** — the decision procedure for routing a finding
   to a direct fix vs. the full brainstorm→spec→plan→SDD cycle.
4. **Agent authorship + model-tiered dispatch** — formalize existing,
   already-working conventions; no new mechanism for either.

## Design

### Slice 1: SemVer policy

Applies to the root `ttui` library crate only — `tools/visual-
snapshot` is internal dev tooling with no external consumers and stays
unversioned in the SemVer sense (its own `Cargo.toml` version is
cosmetic).

**"Breaking" is defined as:** any change to `ttui`'s public API
surface — the same surface already gated by `src/lib.rs`'s
`#![warn(missing_docs)]` (every `pub` item under `src/`). Concretely:
removing/renaming a `pub` item, changing a `pub fn`'s signature,
adding a required field to a `pub struct`, changing a `pub` trait's
required methods, or adding a variant to an existing `pub enum` —
none of `ttui`'s current public enums are marked `#[non_exhaustive]`,
so a new variant breaks any exhaustive `match` a consumer wrote and
counts as `major`. Adding a wholly new `pub` item (function, struct,
enum, module) or a new optional builder-pattern method on an existing
type is additive (`minor`), not breaking.

**Version number stays where it is until the actual 1.0.0 tag.** This
policy takes effect immediately (every finding from here forward gets
sized against it), but `Cargo.toml`'s `version` field is not bumped by
this sub-project — that happens at the "cut the v1.0.0 tag" step,
the last sub-project in the v1.0.0 plan.

**`CHANGELOG.md`** (new, repo root) is added alongside this policy,
in the standard [Keep a Changelog](https://keepachangelog.com) format
(`## [Unreleased]` at the top, `### Added`/`### Changed`/`### Fixed`/
`### Removed` subsections). Entries start accruing under `[Unreleased]`
from this sub-project onward; the first real version header gets
written when the v1.0.0 tag cuts.

### Slice 2: Label taxonomy

Four labels, created via `gh label create`:

| Label | Color | Description |
|---|---|---|
| `semver:patch` | `#cccccc` (grey) | Fix/change with no public API impact |
| `semver:minor` | `#0e8a16` (green) | Adds public API surface, backward compatible |
| `semver:major` | `#d93f0b` (red) | Breaking change to `ttui`'s public API surface |
| `v1-blocking` | `#fbca04` (gold) | Must land before the v1.0.0 tag |

Deliberately minimal — four labels, not a parallel severity scale.
Severity (Critical/Important/Minor) already exists as the review-
finding vocabulary this project's task/final reviews use; these four
labels answer a different question (semver impact + release-queue
membership), not "how bad is it."

### Slice 3: Filing-work rule

Every sweep-audit finding (or any newly-discovered gap going forward)
gets filed as a GitHub issue first — even one that's about to be fixed
immediately — so there's always a record.

**Triage, in order:**

1. **Does it touch `ttui`'s public API surface** (per Slice 1's
   definition)? If yes, label `semver:minor` or `semver:major`
   (whichever applies) **and** `v1-blocking` — it's free to fix now,
   costly after 1.0, so it goes in the pre-v1 queue.
2. If no (internal-only, `patch`-sized), label `semver:patch`. No
   `v1-blocking` label — post-v1 by default, **unless** the fix is
   trivial enough to just do along the way (see below), in which case
   it's fixed immediately and the issue is closed referencing the
   commit, without ever needing the `v1-blocking` label at all.

**Routing the actual fix** — formalizes the pattern this project has
already been using successfully:

- **Trivial, mechanical, well-understood** (a stale doc comment, a
  duplicated helper superseded by a newer primitive, a one-line
  ordering bug with a clear root cause, a rename) — fixed directly,
  no subagent dispatch, referencing the filed issue in the commit and
  closing it.
- **Needs real design judgment** (a new abstraction, a behavior
  change with tradeoffs, anything ambiguous enough to warrant
  clarifying questions) — goes through the full
  `superpowers:brainstorming` → spec → plan →
  `subagent-driven-development` cycle, same as every other Arc this
  project has built.

No third bucket, no separate approval gate beyond that judgment call —
this mirrors the "Direct vs. Gated" autonomy-tier split already in
`git-github-standards.md`, applied to triage instead of merge policy.

### Slice 4: Agent authorship + model-tiered dispatch

**Agent authorship:** formalizes the existing convention as-is — no
new mechanism. Commits carry the `Co-Authored-By`/`Claude-Session`
trailers already established by the harness (per
`development-conventions.md`'s commit-conventions section). No PR
label is added; the trailers are the record.

**Model-tiered dispatch:** `code-forge.md` references
`subagent-driven-development`'s existing "Model Selection" section
(cheap model for mechanical/transcription work, standard model for
integration/judgment tasks, most capable model for architecture and
final review) as TTUI's answer — no separate, TTUI-specific scheme.

**Resolving `code-forge.md`'s remaining open question** ("does agent-
authored work get a real merge gate, or is admin bypass the norm?"):
this was already answered by `git-github-standards.md`'s Direct/
Gated/Human autonomy tiers, written after `code-forge.md`'s stub but
never linked back. The rewritten `code-forge.md` cross-references it
directly instead of re-litigating.

### `code-forge.md` rewrite

The file's `**Status:** stub` line becomes `**Status:** fully defined.`,
matching the convention `development-conventions.md`/
`git-github-standards.md` already use. Its four `## Categories` bullets
become four resolved sections (SemVer + labels + triage, filing work,
agent authorship, model-tiered dispatch) per the slices above, in the
same prose style as the project's other fully-defined rules files. Its
`## Open questions` section is removed — replaced by an explicit
cross-reference to `git-github-standards.md`'s autonomy tiers for the
bypass question, since nothing here is left open.

### `README.md` update

The existing "## Workflow" section gains one sentence pointing to
`code-forge.md` alongside its existing `.claude/rules/` reference —
additive, no rewrite of the existing paragraph.

## Non-goals

- **Bumping `Cargo.toml`'s version to 1.0.0.** That's the v1.0.0
  initiative's final sub-project, not this one.
- **A new severity/priority label scale.** The existing Critical/
  Important/Minor review vocabulary stays as-is; these labels answer
  a different question.
- **A PR-level "agent-authored" label or any new attribution
  mechanism.** The existing commit-trailer convention is formalized
  as-is, not extended.
- **A new, TTUI-specific model-dispatch scheme.** References the
  existing SDD skill guidance instead of inventing a parallel one.
- **Actually running the sweep audit.** This sub-project defines the
  process the audit (sub-project #2) will use; it doesn't perform any
  audit itself.
- **Versioning `tools/visual-snapshot`.** Stays outside the SemVer
  policy — internal dev tooling, no external consumers.

## Testing

`admin`/`git-adjacent`-tagged, no TDD — this is documentation and
GitHub label configuration, not application code. Verification is
read-through (does `code-forge.md` actually answer all four questions
it originally posed, unambiguously) plus confirming the four labels
exist in the repo after creation.

## Critical files

- `.claude/rules/code-forge.md` — full rewrite, stub → fully defined.
- `CHANGELOG.md` — new file, repo root.
- `README.md` — one-sentence additive update to "## Workflow".
- GitHub labels: `semver:patch`, `semver:minor`, `semver:major`,
  `v1-blocking` (created via `gh label create`, not a file).

## Verification

- `code-forge.md` no longer contains "stub" or "open question" —
  every one of its four original categories has a concrete, resolved
  answer in prose.
- All four labels exist: `gh label list` shows `semver:patch`,
  `semver:minor`, `semver:major`, `v1-blocking` with the colors/
  descriptions specified above.
- `CHANGELOG.md` exists with a `## [Unreleased]` section using the
  Keep a Changelog format.
- `README.md`'s Workflow section links to `code-forge.md`.
- Per `.claude/rules/git-github-standards.md`'s Direct tier: since
  this is `admin`-tagged with zero `src/`/`examples/` behavior change,
  it may commit straight to `main` — no PR required, once this design
  and its plan are approved.
