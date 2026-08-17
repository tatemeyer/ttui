# Slice 2 Brief — Tasks 4-6: a colour-aware quiescence signal

Implement Tasks 4, 5 and 6 of
`docs/design/plans/core/2026-08-16-capture-quiescence-fidelity-plan.md`,
then **measure and report**. Do **not** attempt Task 7 — see "The wall
you will hit".

## The problem, in one paragraph

`tools/visual-snapshot` decides a draw is finished by comparing
`vt100::Screen::contents()` between polls. That is **plain text only**.
What it then rasterizes (`render::render_screen`) is the **full** cell
state including colour. So the signal and the artifact measure different
things, and any redraw that changes only colour is invisible to
quiescence.

Slice 1 proved this against a real app. `omnitrix`'s boot fades an
hourglass in from `Rgb(0,0,0)`; the glyphs never move, only their
foreground colour changes. Quiescence saw "text appeared, then text
unchanged" and declared the draw complete after 3 polls — at
`progress == 0`, the blackest instant — producing a 100%-black frame
(#139). #131 is the same defect on background-fill sprites.

## Task 4 — one definition of observable state

**Files:** `tools/visual-snapshot/src/render.rs`, `tools/visual-snapshot/src/pty.rs`

`render_screen` already builds exactly the tuple quiescence needs, at
roughly `render.rs:43`:

```rust
let (ch, fg, bg, bold, underline, inverse) = match cell { ... };
```

Extract that into a shared type so there is **one** extraction path, not
two that can silently diverge:

```rust
/// The per-cell state that actually reaches the rasterizer. Quiescence
/// compares this and `render_screen` renders it, so the two cannot
/// silently diverge.
pub(crate) struct ObservableCell { ch: char, fg: Color, bg: Color, bold: bool, underline: bool, inverse: bool }

pub(crate) fn observable_screen(screen: &vt100::Screen) -> Vec<ObservableCell>;
```

- Write a **failing test first**: `observable_screen` must distinguish two
  screens with identical text but different `bg`.
- Have `render_screen` consume `ObservableCell` rather than duplicating
  the match.
- Document the invariant on both: *quiescence must compare exactly what
  `render_screen` reads.*

## Task 5 — a fixture whose redraw is colour-only

**File:** `tools/visual-snapshot/examples/color_only_redraw.rs` (new)

A fixture that draws a fixed text layout, then after a known delay
changes **only** colours — no text change anywhere. Model it on the
existing fixtures in `tools/visual-snapshot/examples/` (e.g.
`delayed_draw.rs`) for structure and raw-mode handling.

**Prove the fixture discriminates**: before Task 6 lands, confirm by hand
that the *current* plain-text comparison does **not** detect its redraw
(the capture should ride `MAX_SETTLE_WAIT`). A fixture that passes under
the old signal proves nothing. Report what you observed.

Fixtures are examples and exempt from TDD.

## Task 6 — switch the signal

**Files:** `tools/visual-snapshot/src/pty.rs`, `tools/visual-snapshot/tests/pty_roundtrip.rs`

- **Failing integration test first**, driving `color_only_redraw`: the
  capture must observe the colour-only change rather than timing out.
- Replace `Screen::contents()` with `observable_screen` in **both**
  `wait_for_first_output` and `wait_for_further_output`.
- Delete the now-obsolete "Caveat" paragraph in `wait_for_further_output`'s
  doc comment (it documents the blind spot you just removed) and replace
  it with the invariant.
- The existing `pty_roundtrip` and `raw_mode_roundtrip` suites must still
  pass.

## The wall you will hit — expected, do not fix it

`omnitrix`'s theme pulses its border colour on **every 33ms tick,
indefinitely** (`theme()` derives `primary` from a sine wave). Under a
colour-aware signal that screen is *never* stable, so quiescence will
never settle early and **every omnitrix capture will ride the full
2000ms `MAX_SETTLE_WAIT`**. Other themed examples likely behave the same.

This is known and expected. It is Task 7's job — a "stability criterion"
that tolerates continuous low-amplitude change without treating it as an
unfinished draw — and **Task 7 is deliberately not yours**. The criterion
is a real design decision that must be made against measurements, and the
coordinator is making it.

**Do not invent one. Do not tune constants to hide the effect. Do not
revert to text comparison to make timings look good.** Report the slow
timings honestly — they are the input to the decision.

## What to measure and report

After Task 6 is committed, capture each of the five scenarios in
`.plumb/config.yaml` (`omnitrix-dial-rotate`, `tardis-console-idle`,
`falcon-glitch-burst`, `mission-control-telemetry`,
`control-panel-launch-click`) using its script in `.plumb/scripts/`, and
record **wall-clock time per capture**, before and after your change.
Get the "before" numbers by measuring at the commit prior to Task 6.

`VS_DEBUG_QUIESCENCE=1` is already implemented on this branch and prints
elapsed ms, break path (`changed_at_least_once` vs `deadline`) and poll
count — use it. The **break path per scenario is the key datum**: it says
whether that capture settled honestly or rode the deadline.

Report a table: scenario, before ms, after ms, after break-path, polls.

Also report, for each of the five, whether the captured image still looks
correct (frame count as expected, content present) — and for
`omnitrix-dial-rotate` specifically, whether frame 0 is still black. Use
Python + PIL to measure non-black pixel percentage; PIL is installed.

## Constraints

- Work in `D:\Dev\Projects\TTUI\.claude\worktrees\capture-quiescence-fidelity`
  on branch `worktree-capture-quiescence-fidelity`. Never touch the main
  TTUI checkout or `D:\Dev\Projects\Parallax`.
- **TDD is mandatory** for Tasks 4 and 6 (`coding`-tagged, no exceptions).
- **The output contract is frozen**: zero-step script → 1 frame → `.png`;
  N-step → N+1 frames → `.gif`. Plumb's manifests and contact-sheet
  geometry depend on this arithmetic. Do not change frame counts.
- Do not remove or disable the `VS_DEBUG_QUIESCENCE` instrumentation.
- All four gates green before each commit: `cargo build`,
  `cargo test --workspace`, `cargo clippy --all-targets -- -D warnings`,
  `cargo fmt --all -- --check`.
- Conventional Commits, one commit per task, with trailers:
  ```
  Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01RJs5Myj27GQYMS6DpUEA4b
  ```
- A `code-review-graph` post-commit hook prints a Python
  `UnicodeEncodeError` traceback after every commit. Pre-existing and
  harmless — the commit lands. Ignore it.
- If something is genuinely unclear, ask rather than guess.
