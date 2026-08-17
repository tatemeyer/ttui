# Slice 1 Task Brief — Diagnose why frame 0 rasterizes black (#139)

You are gathering **evidence**, not reaching a verdict. Report raw data.
The coordinator interprets it. A confident wrong conclusion already
happened once on this issue and is the specific failure mode to avoid.

## Background you need

`tools/visual-snapshot` spawns a TUI app under a PTY, feeds its output
into a `vt100` parser, waits for the draw to settle, then rasterizes the
parser's screen to an image via `render::render_screen`.

The first frame of every capture comes out **100% black**. Measured on
`main`: capturing `omnitrix` at 120x40 with a zero-step script produces
1228800/1228800 pure `(0,0,0)` pixels, and the run resolves in roughly
1.1s against a 2000ms deadline.

That ~1.1s matters: `Session::capture_frame` calls
`wait_for_first_output(deadline)` with `MAX_SETTLE_WAIT = 2000ms`, and
that function breaks either (a) early, once the screen changed at least
once and then held still for one `POLL_INTERVAL` (20ms), or (b) at the
deadline. Finishing well under 2000ms means it took path (a) — the child
genuinely drew something, and the tool saw it.

So the tool waited, observed a real draw, and still rasterized nothing.

## The question

`examples/omnitrix/boot.rs`'s `render_boot` opens by filling the whole
area with `Cell { symbol: ' ', bg: Color::Black }`. That alone rasterizes
to pure black. But at the instant quiescence resolved, boot `progress`
should still be under `0.4`, which means `render_boot` should *also* have
blitted a 5x5 hourglass through `camera::dim(&scratch, factor)` at
near-full brightness — visibly green. It is not in the image.

**Which of these is true?**

1. **Torn frame (capture-layer).** The capture landed between the black
   area fill and the hourglass blit. The parser's screen genuinely had no
   hourglass cells yet.
2. **Lost glyph (render-layer).** The hourglass cells *were* present in
   the parser's screen, but with a colour that rasterizes to black (or
   were dropped by `dim`/`blit`). This would be a `ttui` `src/` bug and
   would mean `tools/visual-snapshot` is innocent. `camera::dim` calls
   `scale_color`, whose non-`Rgb` fallback is a known parked problem
   (#122).

These are distinguishable by dumping the parser's cell grid at the exact
moment quiescence resolves.

## What to do

Work in `D:\Dev\Projects\TTUI\.claude\worktrees\capture-quiescence-fidelity`
on branch `worktree-capture-quiescence-fidelity`.

1. **Instrument** `tools/visual-snapshot/src/pty.rs`, gated behind an env
   var `VS_DEBUG_QUIESCENCE=1` so normal runs are completely unaffected.
   At the point `wait_for_first_output` breaks, emit to stderr:
   - elapsed ms since `Session::spawn`
   - which break path was taken (`changed_at_least_once` vs deadline)
   - the number of polls performed
   - `Screen::contents()` verbatim
   - **the cell grid**: for every cell where the symbol is not a space
     *or* the bg is not the default, print `(row, col, ch, fg, bg)`.
     This is the load-bearing part — it is what separates case 1 from
     case 2. Cap the output at the first ~200 such cells so a full screen
     of bg-filled cells cannot flood the log.

2. **Verify the instrumentation is inert** when the env var is unset:
   `cargo test --workspace` must be unchanged and green, and
   `cargo clippy --all-targets -- -D warnings` clean.

3. **Run it** and capture the stderr output verbatim:
   - `omnitrix` at 120x40 with a zero-step script (`[]`, output `.png`)
   - `tardis` at 120x40, same (its own `BOOT_MS` is 3000)
   Use a temp dir for outputs; do not add capture artifacts to the repo.

4. **Commit** the instrumentation on the branch. Conventional Commits,
   `test(visual-snapshot):` or `chore(visual-snapshot):` scope, body
   explaining it is a temporary `research`-tagged spike per
   `docs/design/plans/core/2026-08-16-capture-quiescence-fidelity-plan.md`
   Slice 1. Include the trailers:
   ```
   Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
   Claude-Session: https://claude.ai/code/session_01RJs5Myj27GQYMS6DpUEA4b
   ```

## What to report back

- The **verbatim stderr dump** for both apps. Do not summarize it away —
  the raw grid is the evidence.
- Whether hourglass-like glyph cells (the 5x5 box-drawing/slash pattern
  from `HOURGLASS` in `examples/omnitrix/omnitrix.rs`) were present in
  omnitrix's grid at that moment: **present**, **absent**, or **unclear**.
- Their `fg`/`bg` values if present.
- Elapsed ms and break path for each run.
- Anything that surprised you or that you could not determine.

**Do not** state which of case 1 or case 2 is true. Report what you
observed. If the data is ambiguous, say so plainly — "unclear" is a
useful and acceptable answer here.

**Do not** attempt any fix. This slice is diagnosis only.

## Constraints

- TDD does not apply — this is a `research`-tagged throwaway spike.
- Do not change capture behaviour, frame counts, or the output contract.
- Do not touch the Parallax repo.
- If something is unclear or you get stuck, ask rather than guess.
