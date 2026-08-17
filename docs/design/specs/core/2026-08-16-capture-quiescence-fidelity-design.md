# Capture Quiescence Fidelity — Design

**Status:** draft, pending review before we move to planning.
**Date:** 2026-08-16
**Relationship to prior work:** extends
`docs/design/specs/core/2026-08-09-visual-snapshot-tooling-design.md`.
It changes one mechanism inside `tools/visual-snapshot` — how the tool
decides a draw is finished — and nothing about the tool's CLI, output
contract, or the scenarios that consume it. First Arc opened after the
v1.0.0 tag.

## Problem

`tools/visual-snapshot` decides when a child app has finished drawing by
comparing `vt100::Screen::contents()` between polls. That method returns
**plain text only** — no color, no background, no attributes.

What the tool then rasterizes is the **full** cell state: `render_screen`
reads color and attributes and turns them into pixels.

So the signal the tool waits on and the artifact the tool produces do not
measure the same thing. A screen can be textually identical and visually
completely different, and a screen can change textually while remaining
visually blank. Every symptom below is that one mismatch.

The blind spot is already documented — `wait_for_further_output`'s own
doc comment says *"a redraw that changes only color/attributes … won't
register as 'changed'"* — but it was recorded as a caveat rather than
treated as a defect, and the four issues it causes were filed
independently without being connected.

### The four symptoms

| Issue | Symptom | How the mismatch causes it |
|---|---|---|
| **#139** | Frame 0 of every multi-frame capture is 100% black | A screen filled with `symbol: ' '`, `bg: Black` is a genuine *text* change that rasterizes to nothing. Quiescence resolves satisfied; the image is empty. |
| **#131** | Bg-fill animations are invisible to quiescence | Sprites drawn as `Cell { symbol: ' ', bg: <color> }` (`GripperMascot`, Assembly Line's crate) never change text at all, so the wait always burns the full `MAX_SETTLE_WAIT` and captures only the settled end state. |
| **#127** | Quiescence wait per silent key can exceed a short `InputBinder` chord timeout | Downstream: a signal that cannot see the change spends its entire 2000ms budget, and the app's own chord window expires inside that budget. |
| **#138** | Post-key settle wait makes chorded input flaky to script | Same mechanism as #127, observed from the scenario-authoring side. `falcon-glitch-burst` is ~1-in-3 flaky because of it. |

#131 identified the root cause correctly when it was filed, and its first
suggested direction — *"extend quiescence comparison to include per-cell
color/bg state"* — is the direction this design adopts. It also flagged
#127 as belonging to the same umbrella. This spec is that umbrella made
explicit.

### Why this matters now

Parallax's first audited Plumb run (`4e46639`) returned **NO-GO** on
TTUI's `omnitrix-dial-rotate` scenario. The blocker its blinded critics
found, independently and without seeing any source, was *"frame 1 renders
as an empty black panel while frames 2-6 show the full interface"* —
#139.

TTUI is Plumb consumer #1. Until this is fixed, the one scenario this
project has actually audited end-to-end fails on an artifact of the
capture tool rather than on anything wrong with TTUI's rendering. That
makes this the highest-value work available: it is what stands between
the perceptual-verification pipeline and a verdict anyone can trust.

## Scope

**Tag: `research` (Slice 1) then `coding` (Slices 2-4).** Slice 1 is a
throwaway diagnostic spike and is TDD-exempt under
`development-conventions.md`'s `research` exception. Slices 2-4 are
`coding` and follow TDD test-first with no exceptions.

**Autonomy tier: Gated** — `coding`-tagged, so a PR with all four
required checks green (`build`/`test`/`clippy`/`fmt`).

**SemVer: `semver:patch`.** `tools/visual-snapshot` is internal dev
tooling and sits outside the SemVer policy per `code-forge.md`. No `ttui`
public API surface is touched. Not `v1-blocking` — v1.0.0 has shipped.

### In scope

- The quiescence signal itself: what `wait_for_first_output` and
  `wait_for_further_output` compare between polls.
- The timing constants that signal drives (`POLL_INTERVAL`,
  `MAX_SETTLE_WAIT`) where the new signal changes what a correct value is.
- A definitive root cause for #139, established by instrumentation rather
  than inference.

### Out of scope

- **The CLI, the output contract, and frame counts.** A script with zero
  steps still yields one frame and requires `.png`; a script with 1+ steps
  still yields `steps + 1` frames and requires `.gif`. Plumb's manifests,
  contact-sheet geometry, and `report::geometry`'s frame rectangles all
  depend on that arithmetic, and this Arc must not perturb it. In
  particular, **deleting the initial frame is explicitly rejected** as a
  fix for #139 — see "Rejected alternatives".
- **Plumb.** The contact-sheet handoff already works
  (Parallax `8bf6a5d`); nothing in this Arc touches the Parallax repo.
- **Rasterizer glyph coverage.** The known unmapped-glyph limitation is a
  separate, tracked gap.
- **Fixing whatever Slice 1 finds if it turns out to live in `src/`.**
  See "The Slice 1 fork" — that gets filed and triaged, not fixed here.

## The Slice 1 fork

There is one thing this design deliberately does **not** assume: that
#139 is a quiescence bug at all.

The evidence is genuinely ambiguous. Measured on `main`, capturing
`omnitrix` at 120x40 with a zero-step script:

- The capture resolves in **~1.1s against a 2000ms deadline**, so
  `wait_for_first_output` broke early via its `changed_at_least_once`
  path. The child really did draw something.
- The resulting frame is **1228800/1228800 pure `(0,0,0)`** — one color,
  no others.

But at the instant quiescence resolved, `render_boot`'s `progress` should
be well under `0.4`, which means it should also have blitted the
hourglass via `camera::dim(&scratch, factor)` at near-full brightness.
That should be visibly green. It is not there.

Two candidate explanations, and they live in different layers:

1. **Capture-layer.** The capture lands between the full-area black fill
   and the hourglass blit — a torn mid-flush frame. Quiescence's fault,
   and this Arc's to fix.
2. **Render-layer.** The `dim`/`blit` path loses the glyph. `camera::dim`
   calls `scale_color`, and the non-`Rgb` color fallback is a known parked
   problem (#122). That would be a `ttui` `src/` bug, and
   `tools/visual-snapshot` would be innocent — faithfully capturing a
   screen that really was blank.

**Slice 1 exists to settle this before any fix is designed**, because the
answer decides which layer gets changed. If it lands on (2), the finding
is reported on #139 and filed as a new `src/` issue triaged per
`code-forge.md`, and **this Arc continues regardless** — Slices 2-4 stand
on their own for #131/#127/#138, whose root cause is not in doubt.

This is the part of the design most likely to be wrong, and it is
deliberately structured so that being wrong costs one spike rather than a
whole implementation.

## Approach

### The signal

Replace the plain-text comparison with a comparison over the **cell state
that actually reaches the rasterizer**: symbol, foreground, background,
and attributes, for every cell.

The rule that makes this correct and easy to state:

> **Quiescence must compare exactly what `render_screen` reads.**
> Anything `render_screen` turns into pixels must be able to make the
> screen count as "still changing"; anything it ignores must not.

Keeping those two in lockstep is what prevents this class of bug from
recurring. Today they are separately maintained and have silently
diverged. The design intent is that they share one definition of "the
observable screen state," so a future change to the rasterizer cannot
reintroduce a blind spot without also changing quiescence.

### Why not just compare the rendered image

The obvious alternative — rasterize on every poll and compare pixels — is
rejected on cost. `render_screen` produces a 1920x640 RGBA image for a
120x40 terminal; doing that every 20ms for up to 2000ms is up to 100 full
rasterizations per capture, most of them thrown away. Comparing the cell
grid gets the same fidelity from data already in the parser, at a
fraction of the work. Pixels are the *definition* of correct here, but the
cell grid is a faithful and much cheaper proxy for it.

### What this does to timing

Two effects, in opposite directions, and both need checking rather than
assuming:

- **Captures that used to burn the full 2000ms should get fast.** #131's
  bg-fill animations become visible to the signal, so they can settle
  early instead of timing out. This is the effect that plausibly resolves
  #127/#138 on its own — a wait that ends when the app is actually done
  stops colliding with the app's chord window.
- **Captures that used to resolve early may get slower**, because
  color-only churn (a pulsing border, a cursor blink, a color transition)
  now counts as "still changing." An app that never stops animating color
  would never quiesce and would ride the deadline every time.

That second effect is the real risk in this design, and TTUI is full of
exactly that kind of app — `omnitrix`'s border breathes on every one of
its 33ms ticks. A naive "any cell differs" rule could make *every* capture
of a themed app hit `MAX_SETTLE_WAIT`, which would be a worse tool than
the one we have.

Resolving that tension is the substance of Slice 2 and is deliberately
left open here rather than guessed at. The likely shape is a stability
criterion that tolerates continuous low-amplitude change without treating
it as an unfinished draw — but which specific criterion, and how it is
tuned, must be decided against real measurements from Slice 1's
instrumentation and TTUI's actual examples, not chosen up front. **A
capture that always rides the deadline is a failed Slice 2**, and that is
the acceptance bar.

## Verification

Every slice's changes are verified by the four required checks plus:

- **Slice 1** produces a written root cause on #139 with the instrumented
  evidence attached — not a conclusion from code reading. The spike code
  is deleted or rewritten before Slice 2 ships, per the `research` tag's
  contract.
- **Slices 2-3** are TDD test-first. The regression tests must include a
  fixture whose redraw is **color-only** — the exact case the old signal
  could not see — since a test suite that passes under the plain-text
  comparison proves nothing about this change.
- **Slice 4** re-runs the Plumb scenarios against the rebuilt tool and
  records the run directory in the PR's Verification section, per
  `development-conventions.md`. The concrete success criterion is that
  `omnitrix-dial-rotate`'s NO-GO clears **on the merits** — frame 1 shows
  real interface content — and not because the frame was removed or the
  finding suppressed with a ruling.
- **Timing is measured, not assumed.** Slice 2 records before/after wall
  time for a capture of each of the five existing scenarios. A scenario
  that regresses into riding `MAX_SETTLE_WAIT` is a blocker, not a
  tradeoff to note.

Visual review applies throughout — this Arc changes what every captured
image contains, which is as rendering-affecting as work gets, even though
it touches no rendering code.

## Rejected alternatives

- **Drop the initial frame entirely** (making frame count equal step
  count). It would "fix" #139 by deleting the evidence, does nothing for
  #131/#127/#138, and breaks the output contract that Plumb's manifests,
  contact-sheet geometry, and `report::geometry`'s frame rectangles all
  depend on. Rejected.
- **A fixed initial settle delay / a new `--initial-wait-ms` knob.** This
  is what `SETTLE_DELAY` already was before `MAX_SETTLE_WAIT` replaced it,
  and it was removed for good reason: real startup-to-first-draw latency
  was measured varying up to ~1.9s. Reintroducing a fixed delay
  reintroduces that flakiness, and pushes the problem onto whoever authors
  each scenario. Rejected.
- **A capture-at-fixed-offset script step** (#131's second suggested
  direction). A reasonable future convenience for scenarios that
  *deliberately* want a mid-animation frame, but it is an escape hatch
  rather than a fix — it leaves the signal wrong and asks every scenario
  author to work around it. Out of scope; worth revisiting after the
  signal is honest.
- **Rasterize-and-compare-pixels every poll.** Correct but wasteful; see
  "Why not just compare the rendered image".

## Open questions for planning

1. **The stability criterion** (from "What this does to timing") — the
   central open question, to be answered against Slice 1's measurements.
2. **Whether `wait_for_first_output` and `wait_for_further_output` stay
   two functions.** They differ in patience, not in signal; the new
   comparison may make one of them redundant.
3. **Whether `MAX_SETTLE_WAIT` and `POLL_INTERVAL` still hold.** Both were
   calibrated against the old signal. `POLL_INTERVAL` (20ms) being shorter
   than a typical `tick_rate()` (33ms for `omnitrix`) is what makes
   "changed once, then quiet for one poll" so easy to satisfy, and that
   relationship deserves a deliberate decision rather than an inherited
   one.
