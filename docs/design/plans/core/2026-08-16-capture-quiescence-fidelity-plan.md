# Capture Quiescence Fidelity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `tools/visual-snapshot` wait on the same screen state it
rasterizes, closing #139, #131, #127 and #138 — and clear the NO-GO that
Plumb's blinded critics returned on `omnitrix-dial-rotate`.

**Architecture:** Four sequential slices. Slice 1 is a throwaway
diagnostic spike that decides whether #139 belongs to this Arc at all.
Slice 2 replaces the quiescence signal and is the substance of the work.
Slice 3 checks whether #127/#138 survive the new signal. Slice 4 proves
it end-to-end through Plumb and closes out. **Slices 2-4 do not depend on
Slice 1's verdict** — if Slice 1 finds a `src/` rendering bug, that is
filed and triaged separately and this plan continues.

**Tech Stack:** Rust, `vt100`, `portable-pty`, `image`,
`tools/visual-snapshot`, Plumb (`plumb.exe`, read-only consumer).

**Design:** `docs/design/specs/core/2026-08-16-capture-quiescence-fidelity-design.md`

## Global Constraints

- **Slice 1 is `research`-tagged** — TDD-exempt under
  `development-conventions.md`'s throwaway-spike exception. Its code is
  deleted or rewritten before Slice 2 ships.
- **Slices 2-4 are `coding`-tagged** — TDD test-first, no exceptions.
  Autonomy tier **Gated**: PR with `build`/`test`/`clippy`/`fmt` green.
- **The output contract is frozen.** A zero-step script yields 1 frame and
  requires `.png`; an N-step script yields N+1 frames and requires `.gif`.
  Plumb's manifests, contact-sheet tiling, and `report::geometry` all
  depend on this arithmetic. No task may change it.
- **No Parallax changes.** Plumb is consumed, never modified.
- **`semver:patch`, not `v1-blocking`** — `tools/visual-snapshot` is
  outside the SemVer policy per `code-forge.md`; no `ttui` public API is
  touched. No `CHANGELOG.md` entry (the changelog tracks the `ttui` crate).
- **A capture that always rides `MAX_SETTLE_WAIT` is a failure**, not an
  acceptable tradeoff. This is the acceptance bar for Slice 2.

---

## Slice 1 — Diagnosis (`research`) — ✅ COMPLETE

Settled the fork in the design's "The Slice 1 fork": is #139 a torn
mid-flush capture (capture-layer, ours) or a lost glyph in
`dim`/`blit` (render-layer, a `src/` bug)?

**Verdict: neither — capture-layer, via colour-blindness.** The hourglass
glyphs are present at rows 17-21, cols 57-61 with `fg = Rgb(0, 0, 0)` on
a black fill. `render_boot` fades the hourglass in from exactly black
(`scale_color` multiplies by `1.0 - factor`; `factor` starts at 1.0), so
the opening of boot is a **colour-only animation** that
`Screen::contents()` cannot see. Quiescence resolved after 3 polls at
`progress == 0` — the blackest instant. `tardis`'s frame 0 is *not* black
(0.189% non-black, cyan POLICE BOX, 77 polls), confirming the defect
appears exactly where an app opens on a colour-only transition.

**#139 is #131 on a different app**, and Slice 2 should close both. No
`src/` bug found; #122 not implicated. Full detail in the design's
"Verdict" section.

### Task 1: Instrument the moment quiescence resolves

**Files:** `tools/visual-snapshot/src/pty.rs` (temporary spike code)

**Interfaces:** none — debug output only, behind an env var
(`VS_DEBUG_QUIESCENCE=1`) so it cannot affect normal runs.

- [x] **Step 1:** At the point `wait_for_first_output` breaks, dump: elapsed time since spawn, which break path was taken (`changed_at_least_once` vs deadline), the poll count, and `Screen::contents()`.
- [x] **Step 2:** Also dump the pre-rasterization cell grid — for every non-empty cell, `(row, col, ch, fg, bg)` — so a screen that is "all spaces on black" is distinguishable from "glyphs present but colored black."
- [x] **Step 3:** Verify the instrumentation is inert when the env var is unset (`cargo test --workspace` unchanged).

### Task 2: Capture the evidence

**Files:** none — this task runs the tool and records findings.

- [x] **Step 1:** Run the zero-step `omnitrix` capture under `VS_DEBUG_QUIESCENCE=1` at 120x40. Record elapsed time, break path, and the cell dump.
- [x] **Step 2:** Determine which case holds. **Torn frame:** the dump shows only the black fill, no hourglass cells — the capture landed mid-flush. **Lost glyph:** the dump shows hourglass cells present with a black or `Reset` fg — the cells arrived but rasterize invisibly.
- [x] **Step 3:** Repeat against `tardis` (its own `BOOT_MS = 3000`) to establish whether the behavior is omnitrix-specific or general. A general result is much stronger evidence for the capture-layer explanation.
- [x] **Step 4:** Post the instrumented evidence as a comment on #139, superseding the corrected-but-still-open root cause. State the verdict plainly, including "inconclusive" if that is the honest answer.

### Task 3: Route the verdict

**Files:** `docs/design/specs/core/2026-08-16-capture-quiescence-fidelity-design.md`

- [x] **Step 1:** Record the verdict in the design's "The Slice 1 fork" section, replacing the two candidate explanations with what was actually found.
- [x] **Step 2:** ~~If **render-layer**: file a new `src/` issue…~~ **N/A** — not render-layer. `dim`, `scale_color` and `render_boot` are mutually consistent and behaving as written; no `src/` issue filed, #122 not implicated.
- [x] **Step 3:** If **capture-layer**: note on #139 that it is subsumed by Slice 2 and will close with it. **Done** — #139 is the same defect as #131.
- [x] **Step 4:** Spike instrumentation **kept**, not removed — the plan permits reducing it to what Slice 2 needs, and `VS_DEBUG_QUIESCENCE` is exactly the measurement harness Task 7's timing work requires. Its cell filter was narrowed to glyph cells (`ch != ' '`), since filtering on background drowned the 200-cell cap in a full-screen fill before reaching the glyphs. Gates confirmed green.

---

## Slice 2 — A color-aware quiescence signal (`coding`)

The substance of the Arc. Closes #131, and #139 if Slice 1 pointed here.

**Status: Tasks 4-6 and 8 Step 1 ✅ landed. Task 7 ❌ failed its
acceptance bar — the design premise was wrong, and the Arc is paused for
a decision. Task 8 Steps 3-5 are blocked behind it.**

The color-aware signal works and costs essentially nothing (re-measured
baselines are within noise of pre-Arc). What failed is the *stability
criterion*: a hold-still window cannot work for apps that never hold
still, and TTUI's examples mostly never do — `MAX_SETTLE_WAIT` became the
primary exit for 4 of 5 scenarios at every candidate value. See the
design's "Measured: the stability-criterion premise is wrong" and
"Revised direction: settle after the observed change" for the evidence
and the proposed replacement.

Rejected work preserved on `spike/stable-window-sweep` (`c32dc4d`), not
merged into this Arc.

**#139 and #131 remain open.** #131 is likely closable by Tasks 4-6
alone; #139 is not, and provably cannot be closed by any hold-still
criterion.

### Task 4: Give quiescence and the rasterizer one definition of observable state

**Files:** `tools/visual-snapshot/src/render.rs`, `tools/visual-snapshot/src/pty.rs`

**Interfaces:**
```rust
/// The per-cell state that actually reaches the rasterizer. Quiescence
/// compares this and `render_screen` renders it, so the two cannot
/// silently diverge.
pub(crate) struct ObservableCell { ch: char, fg: Color, bg: Color, bold: bool, underline: bool, inverse: bool }

pub(crate) fn observable_screen(screen: &vt100::Screen) -> Vec<ObservableCell>;
```

- [x] **Step 1:** Write a failing test asserting `observable_screen` distinguishes two screens with identical text but different `bg`.
- [x] **Step 2:** Extract the `(ch, fg, bg, bold, underline, inverse)` tuple `render_screen` already builds at `render.rs:43` into `ObservableCell`, and have `render_screen` consume it — so there is one extraction path, not two.
- [x] **Step 3:** Add a doc comment on both stating the invariant from the design: *quiescence must compare exactly what `render_screen` reads.*

### Task 5: A fixture whose redraw is color-only

**Files:** `tools/visual-snapshot/examples/color_only_redraw.rs` (new)

**Interfaces:** none — a test fixture binary, exempt from TDD as an example.

- [x] **Step 1:** Write a fixture that draws a fixed text layout, then after a known delay changes **only** `bg` colors — no text change anywhere. This is the exact case the old signal could not see.
- [x] **Step 2:** Confirm by hand that the old plain-text comparison does not detect its redraw (run it before Task 6 lands), so the fixture is proven to discriminate rather than assumed to.

### Task 6: Switch quiescence to the new signal

**Files:** `tools/visual-snapshot/src/pty.rs`, `tools/visual-snapshot/tests/pty_roundtrip.rs`

- [x] **Step 1:** Write a failing integration test driving `color_only_redraw`: the capture must observe the color-only change rather than timing out at `MAX_SETTLE_WAIT`.
- [x] **Step 2:** Replace `Screen::contents()` with `observable_screen` in `wait_for_first_output` and `wait_for_further_output`. Make the test pass.
- [x] **Step 3:** Delete the now-obsolete "Caveat" paragraph in `wait_for_further_output`'s doc comment — the blind spot it documents no longer exists. Replace it with the new invariant.
- [x] **Step 4:** Confirm the existing `pty_roundtrip` and `raw_mode_roundtrip` suites still pass unchanged.

### Task 7: Stop continuously-animating apps riding the deadline

**Files:** `tools/visual-snapshot/src/pty.rs`

**Interfaces:** the stability criterion — deliberately not specified in
the design, to be chosen against real measurements.

- [ ] **Step 1:** Measure first. With Task 6 in place, capture all five scenarios and record wall time per capture. `omnitrix` breathes its border every 33ms tick, so it is the expected worst case.
- [ ] **Step 2:** If any capture now rides `MAX_SETTLE_WAIT`, design a stability criterion that tolerates continuous low-amplitude change without treating it as an unfinished draw. Candidates to evaluate against the data, not to pick blind: requiring N consecutive stable polls; ignoring changes below a per-cell count threshold; treating "changing at a steady rate" as settled. Document why the chosen one won.
- [ ] **Step 3:** Write tests covering both directions — a genuinely unfinished draw must still be detected as changing, and a steadily-animating app must still settle.
- [ ] **Step 4:** Re-measure. Acceptance: no scenario rides the deadline, and none regresses meaningfully against the Step 1 baseline.

### Task 8: Revisit the inherited constants

**Files:** `tools/visual-snapshot/src/pty.rs`

- [ ] **Step 1:** Re-examine `POLL_INTERVAL` (20ms) against typical `tick_rate()` (33ms for `omnitrix`). The design flags that "changed once, then quiet for one poll" is trivially satisfied when the poll is shorter than a tick; decide deliberately whether that relationship should hold.
- [ ] **Step 2:** Re-examine `MAX_SETTLE_WAIT` (2000ms), calibrated against the old signal and a measured ~1.9s worst-case first draw. Keep or retune with the Task 7 measurements as justification, and record the reasoning in the constant's doc comment either way.
- [ ] **Step 3:** Decide whether `wait_for_first_output` and `wait_for_further_output` remain two functions — they differ in patience, not in signal, and the new comparison may collapse them (design open question 2).

---

## Slice 3 — The chord-timeout interaction (`coding`)

#127 and #138 may already be resolved by Slice 2: a wait that ends when
the app is actually done stops colliding with the app's chord window.
This slice checks rather than assumes.

### Task 9: Re-measure #127 and #138 against the new signal

**Files:** none initially — measurement first.

- [ ] **Step 1:** Reproduce #138 by running `falcon-glitch-burst` **at least 10 times** on the new signal, recording how many produce the intended three-panel burst versus ambient single-panel flicker. The documented pre-existing rate is roughly 1-in-3 failures.
- [ ] **Step 2:** If the flakiness is gone, close #127 and #138 with the measured evidence and skip Step 3.
- [ ] **Step 3:** If it persists, fix the remaining interaction — the settle wait must not outlast the app's own `InputBinder` chord timeout — TDD test-first, and re-run the 10-run measurement to confirm.

### Task 10: Correct the documented limitations

**Files:** `.claude/rules/development-conventions.md`

- [ ] **Step 1:** Update the "Two known limitations" paragraph, which currently tells readers `falcon-glitch-burst` is ~1-in-3 flaky because of ttui#138 and that a single-panel run is a capture miss rather than a regression. If Slice 3 resolved it, that guidance is now actively misleading and must go.
- [ ] **Step 2:** Re-check the rest of the Visual review section for statements the new signal invalidates.

---

## Slice 4 — Prove it through Plumb and close out

### Task 11: Re-run the audited scenario

**Files:** none — verification.

- [ ] **Step 1:** Rebuild `tools/visual-snapshot` and re-run `omnitrix-dial-rotate` through Plumb's `command` adapter, all four lenses.
- [ ] **Step 2:** Confirm the NO-GO clears **on the merits** — frame 1 shows real interface content. A verdict cleared by removing the frame, or suppressed with a ruling, does not count and means the Arc is not done.
- [ ] **Step 3:** Run the remaining four scenarios and read every contact sheet, describing what is actually on each — per `.plumb/SCENARIOS.md`, exit code 0 is not evidence of success.
- [ ] **Step 4:** Record the run directory for the PR's Verification section.

### Task 12: Ship

**Files:** PR against `main`.

- [ ] **Step 1:** Open the PR using `.claude/templates/github/PULL_REQUEST_TEMPLATE.md`. Verification section must carry: the four gates, the Slice 2 before/after timing table, the Slice 3 10-run flakiness measurement, and the Slice 4 Plumb run directory.
- [ ] **Step 2:** Close #139, #131 and (if Slice 3 resolved them) #127/#138 — **one `Closes #N` keyword per issue**, since a comma-separated list only closes the first (this bit PR #141).
- [ ] **Step 3:** Squash-merge once all four checks are green, then remove the worktree via `ExitWorktree`.
