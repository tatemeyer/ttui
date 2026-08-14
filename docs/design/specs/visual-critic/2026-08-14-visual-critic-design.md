# visual-critic — Design

**Status:** draft, pending your review before we move to planning.
**Date:** 2026-08-14
**Relationship to prior work:** generalizes `tools/visual-snapshot`
(`docs/design/specs/core/2026-08-09-visual-snapshot-tooling-design.md`)
out of this repo into a cross-project Claude Code plugin, and
supersedes the unmerged `worktree-local-vision-judge` branch's approach
to judgment (see "Relationship to the local vision judge" below). The
research doc on `docs/rescue-visual-review`
(`docs/tooling/visual-review.md`, 2026-08-06) remains the survey of
capture options that led here; nothing in it is contradicted.

**Home:** this tool lives in its own repository — it is cross-project by
definition. This spec is written into TTUI because TTUI is consumer #1
and this is where it was designed; it moves with the code once that
repo exists.

**Place in the roadmap:** sub-project #1 of the Assay platform
(`docs/design/specs/assay/2026-08-14-assay-platform-design.md`), where
it serves as the **perceptual verification provider** — tier 3 of that
document's verification ladder, the first rung above what CI can reach.
It has no dependency on the platform and ships standalone; the platform
consumes it. Model-Experiments' `mx-viz` output is consumer #2, with no
modification to either side.

## Context / Motivation

`.claude/rules/development-conventions.md` mandates visual review
before merging any rendering-affecting change: run
`tools/visual-snapshot`, `Read` the resulting PNG/GIF, judge whether it
looks right. That process works, and the capture half of it is solved.

The judgment half is not. Three problems compound:

1. **It doesn't travel.** `visual-snapshot` is welded to this repo — it
   takes `--example <name>` and builds via `cargo build --example`. Any
   other project starts from zero.
2. **The reviewer is a soft grader.** Claude is already multimodal;
   `Read` on a PNG gives it eyes. The scarce thing was never sight. An
   agent looking at output it just produced says "looks good" because
   it wants to be done — the same instinct that makes self-review weak
   for code makes it weaker for pixels, where there is no compiler to
   disagree.
3. **Critique with no declared target is generic.** Absent a stated
   aesthetic, a design opinion regresses to stock advice — more
   whitespace, less density, calmer color — which for TTUI is actively
   wrong. TTUI's examples are deliberately flashy; generic UI heuristics
   fight the entire point of the project.

`visual-critic` addresses all three: a portable capture contract, an
adversarial multi-lens reviewer that never sees the code it is judging,
and a per-project taste profile the critique is measured against.

### Relationship to the local vision judge

The unmerged `worktree-local-vision-judge` branch adds a `judge`
subcommand to `visual-snapshot` that POSTs a frame to a local Ollama
instance (default model `moondream`, ~1.8B) and prints a free-text
"LOOKS OK / POSSIBLE ISSUE" verdict, explicitly advisory and never a
gate.

That design is sound for what it targets — cheap, offline, fast
detection of gross corruption during iteration. It does not reach the
scope here. A 1.8B vision model can plausibly flag a garbled glyph; it
cannot hold a defensible opinion on visual hierarchy, spec conformance,
or animation pacing. Those need frontier vision, which is what the
adversarial-subagent judge below provides.

The two are not in conflict and the branch is not invalidated: if
merged, `--review` remains a useful inner-loop sanity check, and
`visual-critic` is the outer-loop reviewer that carries authority. This
spec does not depend on that branch and does not require it to land.

## Design

### Overview

A single git-installable Claude Code plugin. Capture is a *contract*
(an adapter is anything that writes images to a path), judgment is a
fan-out of narrow, blinded subagents, and per-project state lives in a
`.visual-critic/` directory the plugin scaffolds on first use.

```
visual-critic/
  .claude-plugin/plugin.json
  commands/review.md              → /visual-critic:review
  skills/visual-review/SKILL.md   → the orchestrator
  agents/
    critic-breakage.md            → blocker-capable
    critic-intent.md              → blocker-capable
    critic-design.md              → advisory
    critic-motion.md              → advisory
  capture/                        → Rust crate; built on first use, cached
  templates/
    taste.md
    config.example.yaml
```

### Capture adapters

Three adapters, one contract: **given args, write one or more images to
a declared path, or fail with a typed error.** Nothing downstream knows
or cares which adapter produced a frame.

- **`pty`** — spawns an arbitrary command under a pseudo-console,
  drives it with a scripted key/wait sequence, parses the resulting
  byte stream, and rasterizes to PNG (one frame) or GIF (many). This is
  `tools/visual-snapshot` generalized: the same `portable-pty` +
  `vt100` + `font8x8` + `image` stack, with `--example <name>` replaced
  by an arbitrary `command`. Cross-platform, no external binary, no
  human install. Extraction, not invention — the hard parts (ConPTY
  behavior, script semantics, frame-count-to-extension validation) are
  already proven in this repo.
- **`window`** — captures a native OS window by title. New code.
  **Windows-only in v1**, with the adapter boundary drawn so macOS and
  Linux slot in behind the same contract later.
- **`command`** — runs any shell command that writes images to a
  declared path. The escape hatch that makes adoption free: TTUI adopts
  `visual-critic` by declaring `cargo run -p visual-snapshot -- ...`
  and keeps its existing tool verbatim, unmodified.

Adding a surface later means one new adapter behind the same contract
and no change anywhere else in the system.

### Per-project state

```
.visual-critic/
  config.yaml      scenarios: how to capture, and what each is for
  taste.md         the design language the design lens judges against
  rulings.jsonl    findings you overruled, and your reasoning
  runs/            captured images and verdicts, timestamped
```

A **scenario** is the unit of review:

```yaml
scenarios:
  - name: omnitrix-dial-rotate
    adapter: command
    args: >
      cargo run -p visual-snapshot -- --example omnitrix
      --size 120x40 --script scripts/dial-rotate.json --out {out}.gif
    intent: >
      The dial rotates through four alien modes; the selected mode's
      label sits centred beneath the dial and its glow border matches
      the mode colour.
    touches:
      - src/widgets/dial.rs
      - src/effects.rs
      - examples/omnitrix/**
```

`{out}` is substituted by the orchestrator with the run's output path.
`intent` is what the intent lens checks against. `touches` is what lets
a diff select relevant scenarios instead of capturing everything.

### Flow

1. **Trigger** — `/visual-critic:review` by hand; the skill invoked by a
   project convention at task-completion or pre-PR; or
   `--scenario <name>` for a single targeted look while iterating.
2. **Select** — diff the branch, match changed paths against each
   scenario's `touches` globs. No matches and no explicit scenario
   named → say so and stop. Never silently review everything, never
   silently review nothing.
3. **Capture** — run each selected scenario's adapter, writing to
   `.visual-critic/runs/<timestamp>/<scenario>.{png,gif}` plus a run
   manifest recording size, frame count, and any disclosed caveats.
4. **Fan out** — one subagent per applicable lens per scenario, in
   parallel.
5. **Merge** — dedupe across lenses, suppress prior rulings, sort by
   severity, write `verdict.md` beside the images.
6. **Disposition** — every finding is **fixed**, **overruled** (writes a
   ruling), or **deferred with a note**.

### Lenses

| Lens | Applies when | Max severity |
|---|---|---|
| `breakage` | always | **blocker** |
| `intent` | scenario declares `intent` | **blocker** |
| `design` | `taste.md` exists | major |
| `motion` | capture is multi-frame | major |

Applicability is checked, never assumed. A design lens with no taste
profile is **skipped with a notice**, not run generically — a generic
aesthetic opinion is worse than none, because it costs the same and
must then be argued down.

Concurrency is capped (default 8). If selection would exceed the cap,
the orchestrator batches and **reports what it deferred**. A review that
quietly covered half its scenarios reads as a pass it did not earn.

### Finding contract

Every lens agent returns a list against a fixed schema, or an explicit
empty list:

```json
{
  "lens": "design",
  "scenario": "omnitrix-dial-rotate",
  "severity": "blocker|major|minor|nit",
  "region": "mode-label row, upper-right quadrant",
  "claim": "one sentence: what is wrong",
  "evidence": "what in the image supports this",
  "confidence": "high|medium|low"
}
```

`region` is mandatory and load-bearing. A finding that cannot name
where on screen it lives is **dropped by the orchestrator**. That single
requirement eliminates most vague critique, because "the layout feels
unbalanced" cannot survive being forced to point at something.

### Gate semantics

An unresolved `blocker` means the agent may not claim the task complete
or open the PR. The mechanism is convention-enforced inside the harness
— the skill instructs it, and `verdict.md` is a durable artifact a
pre-PR check can read — not something the kernel prevents. Advisory
findings are always reported and never block.

This maps onto `.claude/rules/git-github-standards.md`'s autonomy
tiers without inventing a fourth: a clean or advisory-only verdict
leaves **Gated** work gated on its usual four checks; an unresolved
blocker holds the work until it is fixed or explicitly overruled by
you, which is a human decision in the same sense the **Human** tier
already means.

### Making the critique trustworthy

This is the part that decides whether the tool is worth running.

**The reviewer sees pixels, not source.** Lens agents receive the
image, the run manifest, the taste profile, and — for the intent lens
only — the scenario's `intent` string. They do **not** receive the
diff, the source, or the fact that anything changed. This is the
highest-leverage rule in the design: an agent that can read the code
reasons *"the code draws three panes, so there are three panes"*
instead of looking. Blinded, it has nothing to do but see.

**Third-party framing.** Each agent is told it is reviewing someone
else's work, submitted for critique — never "verify my change" or
"confirm this looks right." The reflex that produces "looks good!" is
an artifact of authorship; removing the authorship removes it.

**No quota.** An empty findings list is a legitimate, expected outcome,
stated explicitly in every agent prompt. Adversarial means *look hard
and do not flatter*; it does not mean *find something*. A tool that
manufactures a finding per run trains you to skim it within a week —
a slower path to the same soft-grader failure.

**Confidence governs voice.** High-confidence findings assert.
Low-confidence design findings must be phrased as questions — *"is the
mode label meant to overlap the frame corner?"* — because that is what
a low-confidence taste observation actually is. Prompt-enforced only,
but it keeps the advisory tier readable rather than hectoring.

**Taste profile.** `taste.md` (about a page) carries three things:
aesthetic intent, non-negotiables, and — most importantly —
**deliberate violations of generic UI norms**. TTUI's would state that
density is intentional, that glow and chromatic bleed are the point,
and that this is not a productivity dashboard and must not be optimized
for calm. Without that section the design lens relitigates the entire
aesthetic every run.

### Rulings, and the calcification guard

**Rulings are applied as a post-hoc suppression filter and are never
fed to the agents.** The reviewer's eyes stay permanently
uncontaminated; only the *report* learns to stop repeating itself.
Feeding "the user likes X" into the prompt would bias the whole review
and quietly blind it to real regressions in that region. This way the
eyes never learn to stop seeing.

A ruling records a finding fingerprint (lens + scenario + region +
normalized claim), your reasoning, the date, and a **content hash of
`taste.md` at ruling time**. Three consequences:

- Suppression is **scoped to the scenario by default**
  (`scope: project-wide` is opt-in), so overruling one screen's density
  does not mute density everywhere.
- When `taste.md` changes, rulings made under the old hash are marked
  **stale and surfaced for re-validation** rather than silently applied
  forever. Your aesthetic moving is precisely when old rejections stop
  being valid.
- Suppressed findings still appear in the verdict as a collapsed
  `previously overruled (N)` line. A finding four independent runs keep
  raising is a signal the ruling may have been wrong — visible instead
  of buried.

### Failure handling

**Capture failure is never a pass.** A scenario that fails to capture is
reported as `CAPTURE FAILED` with the adapter's error; other scenarios
proceed; the verdict is not clean.

The known realistic failure is already documented in this repo: the
rasterizer hard-errors on unmapped glyphs (`✦`, `💥`, em dash — see
`.claude/rules/development-conventions.md`'s "Known glyph-coverage
limitation"), which turns an entire scenario into a non-result. The
`pty` adapter therefore gains one behavior `visual-snapshot` lacks:

- `--on-unmapped-glyph {error,substitute}`. In `substitute` mode it
  renders a visible placeholder box, records every substitution in the
  run manifest, and the lens agents receive that manifest as a
  **disclosed caveat**: these cells are placeholders, do not judge
  them. A hard stop becomes a reviewable frame with a stated blind
  spot.

`error` remains the default, preserving `visual-snapshot`'s existing
behavior for anyone who wants it.

Other modes:

- Subagent returns malformed or unparseable output → one retry, then
  that lens is `INCONCLUSIVE`. Inconclusive is not a pass; if a
  blocker-capable lens is inconclusive, the verdict says so explicitly.
- No `.visual-critic/` directory → the skill offers to scaffold it from
  `templates/` rather than erroring.
- Capture binary not built → build and cache it. A missing Rust
  toolchain is a clear, actionable message, not a stack trace.
- Harness-level failure never degrades to silent success. Everything
  that could not be checked is named in the verdict.

## Non-goals

- **Golden-image diffing / visual regression against baselines.**
  Judgment is per-run against stated intent and taste, with no stored
  reference images and no baseline-maintenance burden. Additive later.
- **A web/browser adapter.** `claude-in-chrome` already covers that
  surface; deliberately out of scope.
- **macOS/Linux window capture.** v1 ships Windows-only `window`,
  behind an adapter boundary that admits the others later.
- **CI integration.** The gate is harness-level and human-overridable,
  not a required status check.
- **Prebuilt release binaries.** v1 builds the capture crate from
  bundled source on first use and caches it.
- **Replacing `tools/visual-snapshot`.** TTUI keeps it and adopts
  `visual-critic` via the `command` adapter.

## Testing

Three tiers, consistent with `.claude/rules/development-conventions.md`:

- **Capture crate** — real Rust, TDD mandatory. `visual-snapshot`'s
  existing `pty_roundtrip` / `raw_mode_roundtrip` tests are the model
  to follow.
- **Orchestration logic** — scenario selection from globs, finding
  merge and dedupe, ruling suppression, staleness detection, severity
  tiering, `{out}` substitution. All pure functions, all unit-tested.
- **Reviewer regression corpus** — a fixture set of images with known
  ground truth: garbled glyphs, overlapping panels, clipped content, a
  black frame, a one-cell misalignment, plus clean controls. Assert the
  breakage lens catches the bad and passes the clean. This is what
  allows tuning agent prompts against evidence rather than vibes, and
  it catches prompt regressions when a lens definition is edited.

  Model output is non-deterministic, so this is a **threshold suite**
  (N-of-M must pass), run on demand rather than as a hard gate — the
  same posture this repo already takes toward real-TTY tests and the
  real-external-service exemption.

## Critical files

New repository. First-cut inventory:

- `capture/src/pty.rs`, `capture/src/encode.rs`, `capture/src/script.rs`
  — generalized from `tools/visual-snapshot`.
- `capture/src/window.rs` — new; Windows window capture.
- `capture/src/glyphs.rs` — the `--on-unmapped-glyph` substitute path.
- `skills/visual-review/SKILL.md` — orchestration: select, capture, fan
  out, merge, verdict.
- `agents/critic-{breakage,intent,design,motion}.md` — the four lenses.
- `templates/taste.md`, `templates/config.example.yaml` — scaffolding.
- In TTUI, on adoption: `.visual-critic/config.yaml`,
  `.visual-critic/taste.md`, and an additive note in
  `.claude/rules/development-conventions.md`'s "Visual review" section.

## Verification

- Capture crate: `cargo test`, `cargo clippy --all-targets -- -D
  warnings`, `cargo fmt --check` clean.
- End-to-end: `/visual-critic:review` against TTUI's `omnitrix` example
  via the `command` adapter produces captured frames, four lens
  verdicts, and a merged `verdict.md`.
- Blinding verified by construction: assert the dispatched agent
  prompts contain no diff, no source, and no authorship framing.
- Regression corpus meets its threshold on a full run.
- Ruling round-trip: overrule a finding, re-run, confirm it is
  suppressed and appears in the `previously overruled` line; then edit
  `taste.md` and confirm the ruling is surfaced as stale.
