# Sweep Audit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Mine all 50 merged PRs' bodies plus in-code comments
referencing past review findings, filter out anything already stale,
and file/triage every real survivor as a GitHub issue via
`.claude/rules/code-forge.md`'s process.

**Architecture:** Three sequential stages. Stage 1 fans out across 5
parallel `Agent`-tool dispatches (10 PRs each) to mine PR bodies; Stage
2 is one direct grep pass over the current codebase; Stage 3 is
serial, controller-executed filtering/dedup/triage/filing. Unlike this
project's usual plans, no task here produces a file change — the
deliverable is filed GitHub issues, and the "implementer" for each
task is the session executing this plan directly (or, for Stage 1
specifically, that session's own parallel sub-dispatches) rather than
a single fresh subagent making a code change. See the Execution
Handoff note at the end.

**Tech Stack:** `gh` CLI (`pr view`, `issue create`), `grep`.

## Global Constraints

- **`admin`/`git-adjacent`-tagged, no TDD** — research and issue-filing
  work, not application code.
- **Backward-looking only.** Mining what past reviews already found
  and deferred — no fresh review of current code for new gaps.
- **All 50 merged PRs are in scope**, not a bounded subset:
  `1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,
  26,32,33,84,85,86,89,90,91,92,93,94,95,96,97,98,99,100,101,102,103,
  104,105,106,107` (gaps are real — PRs #27–31 and #34–83 don't exist
  as merged PRs in this repo).
- **No `Workflow` tool.** Stage 1's parallelism uses direct `Agent`
  tool dispatches — no multi-agent-orchestration opt-in was given for
  this sub-project.
- **Nothing gets fixed in this plan.** Filing and triaging only —
  actual fixes are a separate, later sub-project (#3), sized once this
  audit's results are known.
- **Every candidate finding that survives filtering gets triaged**
  via `.claude/rules/code-forge.md`'s rule: touches `ttui`'s public API
  surface → `semver:minor` or `semver:major` **and** `v1-blocking`;
  otherwise → `semver:patch` only.

---

### Task 1: Mine PR bodies (parallel)

**Files:** none.

**Interfaces:**
- Produces: a consolidated list of raw candidate findings (format
  below), consumed by Task 3.

- [ ] **Step 1: Dispatch 5 parallel `Agent` tool calls**

Dispatch all 5 in the same turn (parallel, not sequential — this is
independent research work, exactly the case for concurrent dispatch).
Use this exact prompt template for each, substituting only the PR
number list:

```
You are researching the git/GitHub history of the TTUI Rust terminal-UI
framework project (repo: tatemeyer/ttui) to find previously-deferred
code-review findings that were never actually fixed.

For each of these merged PR numbers: <BATCH_LIST>

Run `gh pr view <N> --json title,body,url` for each number. Read each
PR body's Verification/Strengths/Issues/Recommendations sections
(structure varies PR to PR — read what's actually there). Extract any
finding described as deferred, parked, a follow-up, a known
limitation, "Minor (non-blocking)", "not yet fixed", "left as", "out
of scope for this", "post-v1", or clearly equivalent phrasing — a
real, specific, named gap someone chose not to fix immediately. Do NOT
extract findings that the same PR body says were fixed within that PR
— only ones explicitly left open.

If a PR mentions no such findings, that's a normal, expected result —
skip it, don't invent one.

Return your results as a plain list, one finding per line, in exactly
this format:
PR #<N> (<title>): <finding_text> [file: <file_hint or "none named">]

Where finding_text is the actual deferred finding, quoted or closely
paraphrased from the PR body (not summarized so tightly that the
specifics are lost), and file_hint is the specific file/function/area
the PR body names for this finding, if it names one explicitly — "none
named" otherwise. Return nothing else — no preamble, no summary, just
the list (or an explicit "no findings in this batch" if truly none).
```

The 5 batches (`<BATCH_LIST>` for each dispatch):
- **Batch A:** `1,2,3,4,5,6,7,8,9,10`
- **Batch B:** `11,12,13,14,15,16,17,18,19,20`
- **Batch C:** `21,22,23,24,25,26,32,33,84,85`
- **Batch D:** `86,89,90,91,92,93,94,95,96,97`
- **Batch E:** `98,99,100,101,102,103,104,105,106,107`

- [ ] **Step 2: Consolidate the 5 results**

Merge all 5 dispatches' returned lists into one combined list. Do not
deduplicate yet (Task 3 handles that, after Stage 2 adds its own
candidates to the same pool) — just concatenate.

- [ ] **Step 3: Record the raw count**

Note how many total raw candidates Step 2 produced (e.g. "23 raw
candidates across 5 batches") — this number is part of the final
report in Task 3's Step 5.

---

### Task 2: Mine in-code comments (single pass)

**Files:** none (read-only).

**Interfaces:**
- Consumes: nothing from Task 1.
- Produces: zero or more additional candidates, in the same format as
  Task 1's output, appended to the consolidated list before Task 3.

- [ ] **Step 1: Run the grep**

```bash
grep -rn "final.review\|final-branch review\|Guards finding\|deferred" --include="*.rs" src/ tools/ examples/
```

- [ ] **Step 2: Classify every match**

For each matched line, read it in context (the surrounding function/
comment block). Classify as:
- **Closed** — the comment documents a fix or test that already guards
  against a past finding (the common case — e.g. `tools/visual-
  snapshot/tests/pty_roundtrip.rs:181`'s "Guards finding #4..." comment
  describes a test that exists and passes). Not a candidate.
- **Open** — the comment itself says something is still deferred or
  not yet addressed. Add it to the consolidated list in the same
  format Task 1 uses: `<file>:<line>: <finding_text> [file: <file>]`.

- [ ] **Step 3: Record the count**

Note how many matches were found, how many were closed vs. open (e.g.
"5 matches, 5 closed, 0 open") — part of Task 3's final report.

---

### Task 3: Filter, dedupe, triage, and file (serial)

**Files:** none — output is GitHub issues, not repo files.

**Interfaces:**
- Consumes: Task 1's consolidated list + Task 2's open candidates
  (combined into one pool at the start of this task).

- [ ] **Step 1: Staleness check, one candidate at a time**

For each candidate in the pool: read the actual current state of the
named file/area (from `file_hint`, or — if "none named" — infer the
likely area from `finding_text` and check there). Does the described
problem still exist in the code as it stands today?
- **Still present** → keep, proceed to Step 2.
- **Already fixed** (by this finding's own later commits, a
  subsequent Arc, or otherwise no longer true) → discard. Note it in
  the running tally as "discarded: stale" with a one-line reason (what
  changed that fixed it).

- [ ] **Step 2: Dedupe survivors**

The same finding sometimes appears in multiple PR bodies (a Minor note
parked in a task review, then repeated in that Arc's final-review
summary — or now also possibly in an in-code comment from Task 2).
Collapse exact or near-exact duplicates into one candidate, keeping
the most detailed `finding_text` and listing every source PR/location
it came from.

- [ ] **Step 3: Triage each surviving, deduped candidate**

Per `.claude/rules/code-forge.md`'s rule: does fixing this candidate
require changing `ttui`'s public API surface (any `pub` item under
`src/`, per that file's definition of "breaking")?
- **Yes** → labels: `semver:minor` or `semver:major` (major if it
  would remove/rename/change-signature of an existing `pub` item or
  add a variant to an existing `pub enum`; minor if it's purely
  additive) **and** `v1-blocking`.
- **No** → label: `semver:patch` only.

- [ ] **Step 4: File each as a GitHub issue**

```bash
gh issue create --title "<concise imperative summary>" --body "<finding_text>

Source: <PR link(s) or file:line reference(s)>
Affected area: <file/function named, or best-guess area>
" --label "<label(s) from Step 3, comma-separated>"
```

- [ ] **Step 5: Write the final report**

Report, in this session's own output (no file needed — this is the
plan's actual deliverable, stated plainly):
- Raw candidates found: Task 1's count + Task 2's open count.
- Discarded as stale: count, from Step 1.
- Deduped away: count (raw survivors minus deduped survivors), from
  Step 2.
- Filed: final count, with the issue numbers and their labels.

## Final verification (whole plan)

- [ ] All 5 Task 1 batches were dispatched and returned results (or an
      explicit "no findings" for any that had none) — no batch
      silently skipped.
- [ ] Task 2's grep ran against the current tree and every match was
      classified closed or open.
- [ ] Every surviving, deduped candidate from Task 3 has a
      corresponding filed GitHub issue with the correct `semver:*`
      and (if applicable) `v1-blocking` label.
- [ ] The final report (Task 3 Step 5) states the full funnel: raw
      candidates → discarded → deduped → filed — not just "done."

---

## Execution note

This plan's tasks don't fit the usual "one implementer subagent makes
a file change, one reviewer checks it" SDD shape — there are no files
to review a diff of. Task 1 is itself a parallel-dispatch step; Tasks
2–3 are direct research/judgment work with no code artifact.
**Inline Execution is the natural fit** here — the session executing
this plan does the work directly (including Task 1's own internal
`Agent` dispatches), rather than the plan's tasks each becoming a
separate SDD implementer dispatch. Subagent-Driven remains available
if preferred, but expect it to add a layer of indirection (an SDD
"implementer" for Task 1 would itself need to dispatch Task 1's 5
sub-agents) without a corresponding benefit, since there's no code
diff for a task reviewer to check.
