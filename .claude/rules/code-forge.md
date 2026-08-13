# Code Forge

**Status:** fully defined. See
`docs/design/specs/governance/2026-08-13-release-governance-design.md`
for the full rationale.

Governs how work gets labeled, attributed, filed, and routed to a
model tier.

## SemVer policy

Applies to the root `ttui` library crate only — `tools/visual-
snapshot` is internal dev tooling with no external consumers and stays
outside this policy.

**Breaking** means any change to `ttui`'s public API surface — the
same surface `src/lib.rs`'s `#![warn(missing_docs)]` already gates
(every `pub` item under `src/`): removing/renaming a `pub` item,
changing a `pub fn`'s signature, adding a required field to a `pub
struct`, changing a `pub` trait's required methods, or adding a
variant to an existing `pub enum` (none of `ttui`'s current public
enums are `#[non_exhaustive]`, so a new variant breaks an exhaustive
`match`). Adding a wholly new `pub` item, or a new optional
builder-pattern method on an existing type, is additive (minor).

This policy takes effect immediately for sizing findings —
`Cargo.toml`'s `version` field itself is not bumped until the actual
v1.0.0 tag cuts.

`CHANGELOG.md` (repo root) tracks changes in [Keep a
Changelog](https://keepachangelog.com) format, with entries accruing
under `## [Unreleased]` from now on.

## Labels

Four labels, deliberately minimal — answer "what's the semver impact
and release-queue status," not "how bad is it" (that's still the
existing Critical/Important/Minor review vocabulary):

| Label | Color | Description |
|---|---|---|
| `semver:patch` | `#cccccc` | Fix/change with no public API impact |
| `semver:minor` | `#0e8a16` | Adds public API surface, backward compatible |
| `semver:major` | `#d93f0b` | Breaking change to `ttui`'s public API surface |
| `v1-blocking` | `#fbca04` | Must land before the v1.0.0 tag |

## Filing work

Every finding (from a sweep audit or discovered organically) gets
filed as a GitHub issue first, even one about to be fixed immediately
— always a record.

**Triage:**
1. Touches `ttui`'s public API surface? Label `semver:minor` or
   `semver:major` **and** `v1-blocking` — free to fix now, costly
   after 1.0.
2. Otherwise, label `semver:patch`. No `v1-blocking` — post-v1 by
   default, unless trivial enough to fix along the way (see below).

**Routing the fix:**
- **Trivial, mechanical, well-understood** (stale doc comment,
  superseded helper, a one-line ordering bug with a clear root cause,
  a rename) — fixed directly, no subagent dispatch, referencing the
  issue in the commit and closing it.
- **Needs real design judgment** (new abstraction, behavior change
  with tradeoffs, anything warranting clarifying questions) — goes
  through the full `superpowers:brainstorming` → spec → plan →
  `subagent-driven-development` cycle.

## Agent authorship

Commits carry the `Co-Authored-By`/`Claude-Session` trailers already
established by the harness (see `development-conventions.md`'s commit
conventions). No additional PR-level label — the trailers are the
record.

## Model-tiered dispatch

Governed by `subagent-driven-development`'s "Model Selection" section
— cheap model for mechanical/transcription work, standard model for
integration/judgment tasks, most capable model for architecture and
final review. No separate, TTUI-specific scheme.

## Agent-authored work and merge gates

Resolved by `git-github-standards.md`'s Direct/Gated/Human autonomy
tiers: `admin`/`git-adjacent`-tagged work with zero `src/`/`examples/`
behavior change may commit straight to `main` (Direct);
`coding`/`research`-tagged work requires a PR with all four checks
green (Gated); branch-protection/CI-config changes, force-pushes, and
anything explicitly flagged as sensitive require human sign-off
(Human). See that file for the full tier definitions.
