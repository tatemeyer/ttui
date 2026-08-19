# TTUI Core Framework — Design (Rev A)

**Status:** Rev A (draft, pending your review before we move to planning).
**Date:** 2026-08-04

## Context / Motivation

TTUI is a terminal UI framework built from first principles rather than on
top of an existing one (ratatui, Bubble Tea, notcurses, textual, ...). The
driving motivation is control/ownership: a framework fully owned and
understood end to end, shaped around future projects rather than someone
else's roadmap.

First real use case: CLI apps and dashboards — panes of content, layout,
basic widgets, keyboard navigation. Not a terminal multiplexer.

Core UX philosophy — **tactile responsiveness**: every keystroke should
instantly and visibly shape the screen, so the text feels like physical
clay under the user's fingers. This principle isn't just a vibe; it drives
concrete architecture decisions below (input-driven redraw, immediate
flush, and the choice of rendering model).

## Scope (v1)

**In scope:** rendering engine, layout system, a core widget set (`Text`,
`List`, `Table`, `Block`), keyboard input handling, focus management via
app state.

**Explicitly out of scope, not designed around:**
- Multiplexing / running and supervising child processes in panes
  (tmux/screen-like functionality). A separate future project that would
  *consume* TTUI, not a layer of it.
- Cross-language FFI/IPC exposure of TTUI's core (Go/Rust FFI, protobuf/
  JSON IPC). A separate mixed-language exploration tracked independently.
- Cross-platform (Linux/macOS) support as a v1 requirement — Windows-first
  (Windows Terminal/ConPTY, mintty). Building on `crossterm` keeps the
  door open for later, but nothing here is designed or tested against
  Linux/macOS yet.
- A visual-effects layer (cursor trails, transient highlights, etc.) —
  intentionally not designed here; see "Explicitly deferred" below for
  the seam reserved for it.

**Language:** Rust.

## Success criteria (v1 "done")

A working demo dashboard/CLI app built with TTUI: multiple panes via
nested layout splits, at least one `Block`-bordered pane, `Text`/`List`/
`Table` widgets, `Tab`-based focus switching, and Up/Down navigation
within the focused widget. This proves the framework end to end on a real
(if small) application, not just isolated primitives passing unit tests.

## Architecture

A five-stage pipeline, driven directly by input events — never a polling
tick:

```
App state -> View builder -> Layout -> Paint -> Diff -> Terminal writer
```

1. **View builder** — a pure function of app state that produces a tree
   describing panes/layout/widgets. Rebuilt fresh every frame; no widget
   objects persist between frames (immediate-mode declarative, the
   pattern ratatui — Rust's most established TUI framework — already
   proves out).
2. **Layout** — resolves the UI tree plus the current terminal size into
   concrete rectangles via nested constraint-based splits.
3. **Paint** — walks the laid-out tree and writes styled cells into a
   `Buffer` (a 2D grid of char + fg/bg color + style).
4. **Diff** — compares this frame's `Buffer` against the previous frame's,
   producing the minimal set of changed cells.
5. **Terminal writer** — writes only the changed cells and **flushes
   immediately** — no buffering delay between "cell changed" and "pixel
   visible."

**Event loop:** block on the next input event (key or resize) -> run the
app's `update()` -> synchronously run the pipeline above -> loop. Redraw
is a direct consequence of the event, never something a tick-based loop
happens to pick up.

**Terminal I/O is built on the `crossterm` crate** (raw mode, event
reading, ANSI, cross-platform terminal quirks) rather than hand-rolled
terminal-protocol handling. Ownership/control is exercised at the
framework level — buffer, diff, layout, and widgets are all ours — while
genuinely fiddly, cross-platform terminal plumbing is outsourced to a
mature crate instead of reinvented.

The five stages are kept as distinct internal boundaries specifically so
a future effects layer (cursor trails, etc.) could be inserted between
Paint and Diff later without restructuring the pipeline — not built now,
just kept as a clean seam.

### Tactile responsiveness — concrete commitments

- **Input-driven redraw, not tick-based.** Render happens synchronously
  as a direct result of each input event.
- **Immediate, unbuffered flush** after every frame's diff-write — no
  OS-level or library-level write buffering sitting between a cell
  changing and it being visible.

## Components

A single crate for v1 (not a workspace) with clear internal module
boundaries. Split into separate crates later only if a module actually
outgrows this shape — not preemptively.

- **`terminal`** — thin `crossterm` wrapper: raw mode / alternate screen
  enter-exit, reading key/resize events, low-level cell writes.
- **`buffer`** — the `Buffer` cell-grid type and the diffing algorithm
  between two buffers.
- **`layout`** — the constraint-based rect-splitting engine. A `Layout`
  is a direction (`Horizontal`/`Vertical`) plus a list of `Constraint`s
  (`Fixed(u16)`, `Percentage(u16)`, `Min(u16)`, `Fill(u16 weight)`),
  plus `margin`/`spacing`. Layouts nest arbitrarily — arbitrary pane
  grids come from composing splits, not one monolithic grid config.
- **`widgets`** — `Text`, `List`, `Table`, and `Block` (an opt-in
  border/title wrapper around any other widget — panes have no chrome
  unless explicitly wrapped). Each widget is a stateless
  `(data, area) -> paint calls into the Buffer` function; no widget owns
  state between frames.

  **Correction (2026-08-19, #113):** "wrapper around any other widget"
  describes `Block`'s *role* — chrome around someone else's content —
  not its type. It is not a combinator that takes a child. What shipped,
  and what v1.0.0 froze, is:

  ```rust
  pub fn render(&self, area: Rect, buf: &mut Buffer) -> Rect
  ```

  It draws the border and title into `area` and returns the inner
  rectangle, which the caller renders into:

  ```rust
  let inner = Block::new().title("Items").render(cols[0], buf);
  List::new(&items).render(inner, buf);
  ```

  This is the form that satisfies the rule stated two lines above: every
  widget is a stateless `(data, area) -> paint calls` function. A literal
  wrapper would have to be generic over its child and would break that
  uniformity for exactly one widget, to no benefit — the caller already
  knows what it wants inside the border. The bullet is left as written
  rather than rewritten, since it is the original approved text; this
  note is the correction.
- **`app`** — the event loop; an `App` trait (or equivalent) exposing
  `update(state, event) -> state` and `view(state) -> UI tree`; owns
  terminal setup/teardown and panic safety (see Error handling).

## Data flow (concrete example)

1. User presses Down.
2. Event loop reads the key event.
3. `update()` checks `state.focus` to determine which widget's selection
   moves (e.g. `Focus::List` -> increment `state.list_selected`, clamped
   to the item count).
4. `view(state)` rebuilds the UI tree from the new state.
5. Layout resolves rects; Paint writes the new frame into `Buffer` — the
   `List` widget paints the new selected row highlighted.
6. Diff against the previous `Buffer` finds the changed cells (old and
   new highlighted rows).
7. Terminal writer writes just those cells and flushes.

## Focus & interaction model

- Focus is a field in app state (e.g. a `Focus` enum naming the
  focusable panes/widgets).
- `Tab` cycles focus in an app-defined order; the app's `update()` routes
  Up/Down/other navigation keys to whichever widget is currently
  focused.
- No framework-side focus manager. This is ordinary app state and app
  logic, consistent with the stateless-widget model — no retained-mode
  machinery creeping back in.

## Error handling

Terminal safety is the critical concern: a TUI that crashes and leaves
the user's terminal stuck in raw mode / the alternate screen is a real
and bad failure mode.

- Raw mode / alternate screen entry is paired with a guard (a Rust
  `Drop` impl) that restores normal terminal state on every exit path,
  including panics — via a panic hook that runs cleanup before the
  default panic output.
- I/O errors during writes propagate as `Result`, not panics; the event
  loop attempts clean terminal restoration before surfacing the error.

## Testing strategy

- **`buffer` diffing** — pure unit tests: given two buffers, assert the
  diff produces the expected minimal cell set.
- **`layout`** — pure unit tests: given an area and constraints, assert
  the resulting rectangles. Fully deterministic, no terminal needed.
- **`widgets`** — snapshot-style tests: render a widget with known data
  into a `Buffer`, assert the resulting cell grid matches expectation.
- **Full event loop / real terminal I/O** — not practically
  unit-testable (needs a real terminal). Covered by the v1 success
  criterion instead (building the actual demo dashboard app) — an
  accepted gap for v1, not an oversight.
- Testing, verification, and error handling are a stated ongoing
  emphasis for this project beyond v1. A related but separate research
  thread — using a local vision model as a low-severity verification
  tier — is being explored in Model-Experiments
  ([tatemeyer/Model-Experiments#89](https://github.com/tatemeyer/Model-Experiments/issues/89)),
  deliberately kept out of TTUI's own design.

## Explicitly deferred / open questions for future revisions

- Multiplexing / child-process supervision in panes.
- Cross-platform (Linux/macOS) terminal support.
- Cross-language FFI/IPC exposure of TTUI's core.
- A visual-effects layer (cursor trails, etc.) — the pipeline
  intentionally reserves a seam between Paint and Diff for this; not
  designed further here.
