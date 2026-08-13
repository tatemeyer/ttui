# Sweep Audit — Design

**Status:** draft, pending review before we move to planning.
**Date:** 2026-08-13
**Relationship to prior work:** sub-project #2 of the TTUI v1.0.0
initiative, depending on sub-project #1 (Release Governance, merged
`50375df`) for the triage process this audit routes its findings
through. `.claude/rules/code-forge.md`'s SemVer policy and four
labels (`semver:patch`/`minor`/`major`, `v1-blocking`) already exist.

## Problem

This project's SDD workflow deletes each Arc's `progress.md` ledger
(which recorded `Minor (deferred): ...` notes) once a plan's final
review passes — deliberate, since "the git history is the record
now." That's true, but the record isn't centralized: it's scattered
across 50 merged PRs' bodies (which this session's Arcs consistently
wrote detailed Verification/Strengths/Issues sections into) and a
smaller number of in-code comments that reference specific past
findings by name (e.g. `tools/visual-snapshot/src/pty.rs:342`: "See
the final-review fix report's finding #4"). Nothing has ever walked
this scattered record end to end and turned it into an actionable,
triaged list.

## Scope

**Tag: `admin`/`git-adjacent`** — this is research and issue-filing
work, not code changes. No TDD (matches existing tag conventions).

**Backward-looking only.** Mines what past reviews already found and
deferred; does not perform a fresh review of current code for new,
never-flagged gaps. All 50 merged PRs (`#1`–`#107`, sparse — see exact
list in Critical files) are in scope, not a bounded subset.

Three stages, in dependency order:

1. **Mine PR bodies** (parallel) — extract deferred/parked findings
   from all 50 PRs' descriptions.
2. **Mine in-code comments** (single pass) — grep the current
   codebase for comments referencing past review findings by name.
3. **Filter + file** (serial) — check each surviving candidate against
   current code (discard anything already fixed incidentally), dedupe,
   triage via `code-forge.md`'s rule, file as GitHub issues.

## Design

### Stage 1: Mine PR bodies (parallel)

5 subagents, 10 PRs each, dispatched via the `Agent` tool in parallel
(not the `Workflow` tool — no multi-agent-orchestration opt-in was
given for this sub-project). Each subagent runs `gh pr view <N> --json
title,body,url` for its 10 PR numbers and reads each body's Verification/
Strengths/Issues sections (this session's Arc PRs consistently
structured their bodies this way), extracting anything language-
flagged as a deferred, parked, follow-up, known-limitation, or
"Minor (non-blocking)" finding. Trigger phrases to look for (a
starting list, not exhaustive — use judgment for equivalent phrasing):
"deferred", "parked", "follow-up"/"follow up", "known limitation",
"Minor (non-blocking)", "not yet fixed", "left as", "out of scope for
this", "post-v1", "future Arc".

Each subagent returns a structured list:
```
{pr_number: int, pr_title: str, finding_text: str, file_hint: str | null}
```
`file_hint` is the file/function the finding names, if the PR body
says so explicitly (e.g. "Minor (deferred): implementer moved
`crossterm` from `[dev-dependencies]` to `[dependencies]`" → file_hint
`tools/visual-snapshot/Cargo.toml`); `null` if the body doesn't name
one.

**Batches** (all 50 merged PR numbers, sparse — gaps are real, not
typos: PRs #27–31 and #34–83 don't exist as merged PRs in this repo's
history):
- Batch A: `1,2,3,4,5,6,7,8,9,10`
- Batch B: `11,12,13,14,15,16,17,18,19,20`
- Batch C: `21,22,23,24,25,26,32,33,84,85`
- Batch D: `86,89,90,91,92,93,94,95,96,97`
- Batch E: `98,99,100,101,102,103,104,105,106,107`

### Stage 2: Mine in-code comments (single pass)

One pass over the current codebase (not parallelized — it's one repo,
not 50 PRs), grepping for the pattern already established in the code
(`grep -rn "final.review\|final-branch review\|Guards finding\|
deferred" --include="*.rs" src/ tools/ examples/`, confirmed to match
real instances, e.g. `tools/visual-snapshot/src/main.rs:168`,
`tools/visual-snapshot/src/pty.rs:342,533,583`,
`tools/visual-snapshot/tests/pty_roundtrip.rs:181`). Each match is
read in context to determine whether it documents a **closed** finding
(a guard/fix that's already landed — not a new candidate) or an
**open** one (a comment noting something still deferred) — most
matches found so far are the former (test comments explaining what
regression they guard against), so this stage is expected to
contribute few or zero new candidates on top of Stage 1, but must
still run to confirm that rather than assume it.

### Stage 3: Filter + file (serial, by the controller)

For each candidate from Stages 1–2:

1. **Staleness check:** read the actual current state of the named
   file/area (or, if no `file_hint`, the area the finding_text
   describes). Does the described problem still exist? If it was
   already fixed by later work (incidentally or otherwise), discard —
   no issue filed.
2. **Dedupe:** the same finding sometimes appears in multiple PR
   bodies (e.g. a Minor finding parked in a task review, then
   mentioned again in that Arc's final-review summary). Collapse
   duplicates into one candidate, keeping the most detailed
   description and all source PR references.
3. **Triage** via `.claude/rules/code-forge.md`'s rule: does fixing
   this touch `ttui`'s public API surface? If yes → label with
   `semver:minor` or `semver:major` (whichever applies) **and**
   `v1-blocking`. If no → label `semver:patch` only (post-v1 by
   default).
4. **File** via `gh issue create`, with:
   - Title: a concise imperative summary of the finding.
   - Body: the finding's original text, which PR(s) it came from
     (linked), the affected file/area, and the triage labels applied.
   - Labels: as determined in step 3.

## Non-goals

- **A fresh top-to-bottom review of current code.** Backward-looking
  only — mining what's already been found, not hunting for new gaps.
- **Using the `Workflow` tool.** No multi-agent-orchestration opt-in
  was given; Stage 1's parallelism uses the `Agent` tool directly.
- **Fixing anything found.** This sub-project files and triages;
  actual fixes happen in sub-project #3 (the pre-v1 fix wave), sized
  only once this audit completes.
- **Auditing PRs outside this repo's merged-PR history** (e.g. issues,
  unmerged/closed PRs, or work from before PR-based workflow started —
  there is none; PR #1 is this project's first merged PR).

## Testing

`admin`/`git-adjacent`-tagged, no TDD — this is research and issue-
filing, not application code. Verification is: every one of the 50 PR
numbers was actually queried (no silent skips), the in-code grep
actually ran and its matches were triaged (closed vs. open), and every
surviving candidate has a corresponding filed GitHub issue with
correct labels.

## Critical files

- No source files created or modified — this sub-project's output is
  GitHub issues, not repo files.
- Reads: all 50 merged PR bodies (`#1,2,3,4,5,6,7,8,9,10,11,12,13,14,
  15,16,17,18,19,20,21,22,23,24,25,26,32,33,84,85,86,89,90,91,92,93,
  94,95,96,97,98,99,100,101,102,103,104,105,106,107`).
- Reads: `src/`, `tools/`, `examples/` (grep for in-code review-finding
  comments).
- Consumes: `.claude/rules/code-forge.md`'s triage rule and label set
  (already merged).

## Verification

- All 5 Stage 1 batches completed and returned results (no batch
  silently dropped).
- Stage 2's grep ran against the current tree; every match was
  classified closed/open.
- Every surviving (non-stale, deduped) candidate has a filed GitHub
  issue with the correct `semver:*` and (if applicable) `v1-blocking`
  labels, per `code-forge.md`'s rule.
- A final count is reported: candidates found → discarded as stale →
  deduped → filed, so the audit's actual yield is visible rather than
  just "done."
