# Slice 2 Brief (part 2) — Tasks 7 & 8: the stability criterion

Tasks 4-6 are done and committed. The colour-aware signal works. Two
measured facts from that work drive everything here:

1. **omnitrix does not ride the deadline — by accident, not by merit.**
   `POLL_INTERVAL` is 20ms; omnitrix's tick is 33ms. A consecutive poll
   pair frequently lands entirely inside one tick gap, so the screen reads
   as "unchanged for one poll" no matter how continuously it animates.
   The criterion is aliasing against the frame rate, not detecting
   stability.
2. **#139 is still open.** omnitrix frame 0 is 0.000% non-black across
   5/5 runs. The child's first paint lands at `progress ≈ 0` (where `dim`
   scales the hourglass to exactly `Rgb(0,0,0)`), the next poll falls
   inside the same 33ms tick gap and matches it, and the wait breaks
   before a single further frame of the fade is observed.

## The decision (made by the coordinator — implement it, don't re-litigate)

**Replace "unchanged for one poll" with "unchanged for a time-based
window."**

Polls are not a stable unit of measurement here, for two independent
reasons the measurements already proved: the effective poll cadence
drifted from ~21.5ms to ~27.5ms once Task 4 added per-poll allocation, and
a poll count aliases against whatever tick rate the app happens to use. A
duration is invariant to both.

```rust
/// How long the observable screen must hold completely still before a
/// draw is treated as finished. Must exceed a typical app frame period —
/// `POLL_INTERVAL` alone is shorter than most `tick_rate()`s, so a single
/// stable poll pair can land inside one frame gap and read as stable
/// while the app is still animating.
pub const STABLE_WINDOW: Duration = Duration::from_millis(??);
```

`STABLE_WINDOW` replaces the "one stable poll" rule in **both** waits.
Keep `MAX_SETTLE_WAIT` as the outer bound.

### Do not treat "no deadline rides" as success on its own

Under the old criterion nothing rode the deadline either, and #139 still
reproduced. The criterion has to make frame 0 *meaningful*, not merely
make the wait terminate.

## Task 7 — choose `STABLE_WINDOW` empirically

Do not guess the value. Measure candidates and let the data choose.

- [ ] **Step 1:** Measure **60ms, 100ms, 150ms, 250ms** across all five
      `.plumb` scenarios. For each: per-capture wait, break path, poll
      count, total per scenario.
- [ ] **Step 2:** For each candidate, measure omnitrix **frame 0** and
      tardis **frame 0**: non-black percentage *and* the number of
      distinct colours present.
- [ ] **Step 3:** Pick the value meeting the acceptance bar below. Record
      the full comparison table and the reasoning in the constant's doc
      comment.
- [ ] **Step 4:** Write tests covering both directions — a genuinely
      unfinished draw must still register as changing, and a steadily
      animating app must still settle.

### Acceptance bar — all four must hold

1. **No capture rides `MAX_SETTLE_WAIT`** in any scenario.
2. **omnitrix frame 0 shows recognizable content.** Not pure black — and
   equally **not a uniform solid fill**. A probe at 3 stable polls
   produced 100.000% non-black by landing in boot's full-screen flash
   phase; a solid green panel is exactly as uninformative to a Plumb
   critic as a solid black one, and would not clear the NO-GO. The target
   is the **hourglass legible against its background**, which lives around
   `0.1 < progress < 0.4` — roughly 250-1000ms into a 2500ms boot. Use the
   distinct-colour count from Step 2 to tell "real content" from "solid
   fill": a solid fill has ~1-2 distinct colours, real content has more.
3. **tardis frame 0 stays correct** (it is already good — 0.189%
   non-black, cyan POLICE BOX — and must not regress into a solid fill).
4. **Cost stays proportionate:** no scenario's total quiescence time
   exceeds roughly **2x** its pre-Arc baseline. Baselines, in ms:
   omnitrix 411, tardis 1935, falcon 586, mission-control 385,
   control-panel 264.

If no candidate satisfies all four, **stop and report** with the table
rather than relaxing a bar. That is a real finding, not a failure —
it would mean frame 0's usefulness is limited by the app's own opening
animation rather than by the tool, which is a legitimate outcome worth
knowing. Say so plainly instead of tuning until something passes.

## Task 8 — the inherited constants and the allocation

- [ ] **Step 1:** Eliminate the per-poll allocation. `observable_screen`
      builds ~4800 `String`s per poll at 120x40 because
      `vt100::Cell::contents()` returns a `String`. That cost is real: it
      stretched the effective poll cadence by ~37% and thereby shifted the
      aliasing in fact 1 above. Compare the observable state **without
      allocating per cell** — a rolling hash, a reused scratch buffer, or
      comparing in place are all acceptable; pick one and say why. The
      Task 4 test asserting colour-only changes are still distinguished
      must keep passing unchanged.
- [ ] **Step 2:** Re-measure poll cadence (elapsed ÷ polls) after Step 1
      and confirm it is close to the nominal `POLL_INTERVAL` again.
- [ ] **Step 3:** Re-examine `POLL_INTERVAL` (20ms) now that
      `STABLE_WINDOW` carries the stability decision. State in its doc
      comment what relationship it must hold to `STABLE_WINDOW`.
- [ ] **Step 4:** Re-examine `MAX_SETTLE_WAIT` (2000ms) — it was
      calibrated against the old signal and a measured ~1.9s worst-case
      first draw. Keep or retune, recording the reasoning either way.
- [ ] **Step 5:** Decide whether `wait_for_first_output` and
      `wait_for_further_output` remain two functions. They differ in
      patience, not in signal, and `STABLE_WINDOW` may collapse them. If
      they merge, `capture_frame_after_key`'s "must observe the reaction"
      contract must survive the merge — re-read its doc comment first.

**Ordering note:** do Task 8 Step 1 (the allocation) **before** Task 7's
measurements if you can — it changes poll cadence, and measuring
`STABLE_WINDOW` candidates against a distorted cadence would mean
re-measuring afterwards. Say what order you actually used.

## Constraints

- Work in `D:\Dev\Projects\TTUI\.claude\worktrees\capture-quiescence-fidelity`
  on branch `worktree-capture-quiescence-fidelity`. Never touch the main
  TTUI checkout or `D:\Dev\Projects\Parallax`.
- **TDD mandatory** — `coding`-tagged, no exceptions.
- **Output contract frozen**: zero-step → 1 frame → `.png`; N-step → N+1
  frames → `.gif`. Do not change frame counts.
- Keep the `VS_DEBUG_QUIESCENCE` instrumentation and the
  `VS_QUIESCENCE_TIMING` line — they are the measurement harness.
- Any throwaway probe must be fully reverted before you finish, and you
  must verify the revert.
- All four gates green before each commit. Conventional Commits, trailers:
  ```
  Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01RJs5Myj27GQYMS6DpUEA4b
  ```
- The `code-review-graph` post-commit hook prints a Python
  `UnicodeEncodeError` traceback. Pre-existing, harmless, commit lands.
  Ignore it.
- Report honestly. Slow or disappointing numbers are the input to a
  decision, not a failure to conceal.
