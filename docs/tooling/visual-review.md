# Visual review of TTUI apps for an agent (no eyes-on-terminal problem)

**Status:** research/knowledge doc, not a design spec or implementation
plan — nothing here is approved for implementation. If any option below
gets built, it needs its own pass through `superpowers:brainstorming` /
`superpowers:writing-plans` like any other coding work per
`.claude/rules/development-conventions.md`.

**Date:** 2026-08-06

## Problem statement

TTUI apps (`examples/omnitrix.rs`, `examples/smash_crabs.rs`) are
deliberately flashy: glow borders, screen-shake, particle bursts, dial
navigation, corruption/transition effects (see `src/effects.rs`,
`src/particles.rs`, `src/transition.rs`, and the Arc 0 design spec).
"Correct" for this kind of UI means *looks right* — color, glyph choice,
animation timing, layout — which is exactly the category of bug that
`cargo test` and a non-interactive smoke run cannot catch. A screen-shake
that shakes the wrong axis, a glow border rendered in the wrong color, a
dial that misaligns by one cell — all pass every automated check and are
only visible by looking at the thing.

For web UI work, the agent has `claude-in-chrome`: it can open a page,
screenshot it, and — because Claude is multimodal — actually *see* the
result, plus click/type to drive it interactively. TTUI has no equivalent
today. In this dev environment (Windows 11, agent frequently running
headlessly in the background) the agent can:

- build the code and run `cargo test` (including the real-TTY tests, but
  only when a human runs `cargo test -- --ignored` locally per
  `development-conventions.md` — the agent itself has no TTY in a
  headless run),
- launch `cargo run --example X` as a non-interactive smoke test (does it
  panic on startup),

but it cannot see color/glyphs/animation, and it cannot drive keyboard
interaction into a raw-mode session, because `crossterm` reads the
console device directly — piping stdin does not reach it.

This doc surveys options to close that gap, with a recommendation and a
concrete first step.

## What the current rendering model gives us

Confirmed by reading the source directly (not assumed):

- `src/buffer.rs`: `Cell { symbol: char, fg: Color, bg: Color, style:
  CellStyle { bold: bool } }` (`Color` is `crossterm::style::Color`).
  `Buffer` is a flat `Vec<Cell>` grid with `width`/`height`. `LayerStack`
  holds one or more `Buffer`s and exposes `composite() -> Buffer`
  (topmost non-default cell per position wins).
- `src/app.rs`: `App::view(&self, area: Rect, buf: &mut LayerStack)` is a
  pure function of app state; `app::run()` drives the loop, composites
  the `LayerStack` to a `Buffer` every frame, diffs against the previous
  frame, and hands the diff to `Terminal::draw_diff`.
- `src/terminal.rs`: `Terminal::draw_diff` is the only place that talks
  to a real terminal — it walks a `Vec<CellDiff>` and issues
  `crossterm` `execute!` calls (`MoveTo`, `SetAttribute`,
  `SetForegroundColor`/`SetBackgroundColor`, `Print`). Everything
  upstream of that (`view`, `composite`, `diff`) is pure data —
  no I/O, no terminal required.

That last point matters: **a composited `Buffer` is fully-structured
"what should be on screen" data — char + fg + bg + bold, already
resolved** — before it ever touches a terminal. Nothing about producing
one requires a real TTY. That opens up options a framework without this
separation wouldn't have.

Also confirmed: `Cargo.toml` for the `ttui` crate lists exactly one
dependency, `crossterm = "0.27"`. The core framework design spec
(`docs/design/specs/core/2026-08-04-ttui-core-framework-design.md`) frames
that as a deliberate ownership/control stance for the **library**, not a
blanket ban on tooling elsewhere in the repo — nothing in the spec or
`Cargo.toml` addresses dev-only tooling, examples, or a separate crate.
Any option that adds dependencies should keep them **out of the `ttui`
library's own `Cargo.toml`** (e.g. a sibling crate or an example gated
so it doesn't become a default build dependency) rather than assume the
constraint doesn't apply at all.

## Options considered

| Option | Gives the agent | Setup complexity | Windows | Human install needed first? | Fits existing dev loop? |
|---|---|---|---|---|---|
| **A. Buffer→PNG rasterizer** (custom, in-repo) | Static image(s) of *exact* `Buffer` content — true colors/glyphs/bold, deterministic | Small (one new crate + a bitmap-font dep) | Yes — pure Rust, no PTY/console involved at all | No — agent can build it entirely itself | Yes — `cargo run -p ...`, could become a test helper or example |
| **B. VHS** (`charmbracelet/vhs`) | Real terminal-rendered GIF/PNG/MP4 from a scripted `.tape` (types keys, waits, screenshots) | Medium — single binary, but pulls in `ttyd` + `ffmpeg` under the hood | Winget/scoop package exists; last releases active as of mid-2026 | **Yes** — one-time install | Yes — a `.tape` file per example, run in CI or by the agent, output PNGs read directly |
| **C. ttyd + claude-in-chrome (live)** | Real, *interactive*, live browser session — closest analogue to the "Claude in Chrome" loop | Medium-high — needs a running local server, port, and keystroke delivery has to actually reach `crossterm`'s raw-mode reader through ConPTY | Native Windows support added in ttyd 1.7.0 (ConPTY-based), but there's at least one open "ttyd crashes on Windows" issue — **unverified/flaky** | **Yes** — install + run a persistent local process | Possible but heavier; best for exploratory review, not routine verification |
| **D. wetty** (Node/xterm.js/node-pty) | Same shape as C | Higher — needs Node.js + npm install on top of everything C needs | node-pty supports Windows via ConPTY (1809+) in principle | **Yes** | Same as C, more moving parts for no clear benefit over ttyd |
| **E. gotty** (`sorenisanerd/gotty` fork) | Same shape as C | Similar to ttyd | Claimed cross-platform but far less scrutinized for Windows/ConPTY specifically than ttyd | **Yes** | Not preferred — ttyd is the more actively-verified Windows option in this family |
| **F. asciinema + agg** | GIF from a recorded session | `agg` itself is a Rust crate, builds fine on Windows via `cargo install` | **Recording** (`asciinema rec`) is the blocker — asciinema's recorder has historically been Linux/macOS-only; no first-class Windows recorder | N/A if recording step doesn't work | Not recommended here — the encode half works, the capture half doesn't cleanly on Windows |
| **G. termtosvg** | SVG animation | N/A | Explicitly Linux/macOS/BSD only; upstream unmaintained since 2020 | N/A | Rejected |
| **H. terminalizer** | GIF | Unclear/stale | No confirmed current Windows story found | N/A | Rejected (insufficient evidence it's still viable) |
| **I. Headless terminal-emulation libs** (e.g. Rust `vt100` crate) | Parsed terminal *text* state (grid of chars + SGR attributes) from raw ANSI bytes | Small crate, pure Rust | Yes | No | Redundant for TTUI specifically — `vt100` exists to recover structured state *from ANSI output*, but TTUI already has that structured state pre-ANSI as a `Buffer`. Useful pattern if TTUI ever needs to test against `crossterm`'s actual byte stream, not useful for visual review. |

### Why F, G, H are effectively out

Asciinema's Windows recording story, termtosvg's explicit non-support,
and terminalizer's stale/unclear status all fail the "does this actually
work in this dev environment" bar before comparison on features even
matters. Not included further below.

## Recommendation

**Start with Option A (Buffer→PNG rasterizer), and treat Option B (VHS)
as the next tier up once/if A proves insufficient.** Reasons:

- **A needs no human install step at all.** The agent can write, build,
  and run it entirely within the sandbox, headlessly, on the first try —
  no persistent server, no port, no second binary ecosystem. Given the
  agent runs headlessly "a lot of the time" per the prompt, this is the
  only option in the table with zero external dependency on a human
  doing something first.
- **A is the most accurate rendering of "what TTUI actually computed."**
  It rasterizes the real `Buffer` — the same struct that
  `Terminal::draw_diff` would have painted — rather than going through a
  second terminal emulator's interpretation of ANSI bytes. There's no
  translation layer to introduce its own bugs or become a second thing
  to debug when a screenshot looks wrong.
- **A slots into the existing test/example structure** described in
  `development-conventions.md` almost for free — closer to "add an
  example" than "stand up new infrastructure."
- Its real limitation is that it's **not the real terminal renderer**:
  it reimplements glyph rasterization, so a bug in the *rasterizer*
  could mask or fabricate a rendering difference that a real terminal
  wouldn't show (e.g. font metrics, double-width glyphs, terminal-
  specific color handling). That's exactly what makes VHS (Option B)
  worth keeping in reserve — VHS drives the actual compiled binary
  through `ttyd` and a real (headless) terminal stack, so it's a fidelity
  upgrade for anything A's approximation leaves in doubt, at the cost of
  a one-time human install.
- **Option C (ttyd + claude-in-chrome) is the closest match to the
  "Claude in Chrome" experience the prompt is chasing** — genuinely live,
  interactive, screenshot-and-click — but two things are unverified and
  should not be assumed to work: (1) ttyd's native Windows/ConPTY support
  landed relatively recently (1.7.0) and has at least one open crash
  report on Windows specifically, and (2) whether keystrokes sent by
  `claude-in-chrome`'s `computer` tool through xterm.js → ttyd → ConPTY
  actually reach `crossterm`'s raw-mode event reader the way a real
  console would has not been tested against this codebase. If A and B
  both turn out insufficient for some interactive-navigation bug that
  can only be seen live, C is the next thing to try — but budget time
  for it not working cleanly on the first attempt, and it needs a human
  to install ttyd and be comfortable with a locally-running server
  first.

## Proof-of-concept plan: Option A (Buffer→PNG rasterizer)

This is a plan, not a completed implementation — no code has been
written or repo files changed besides this doc.

### What it needs

- A small embedded **bitmap font** so text rendering needs no system
  font dependency or license concerns — the Rust `font8x8` crate (MIT,
  pure Rust, no build dependencies) is a well-known fit for exactly this
  "render terminal-style glyphs to pixels" use case (it's what the
  `tui-big-text` ratatui widget uses internally). 8x8 glyphs are coarse
  but perfectly adequate for verifying color/layout/animation — this
  isn't trying to be a pixel-perfect font renderer.
- The **`image` crate** (or the lower-level `png` crate) to build an
  `RgbImage`/`ImageBuffer` and encode it to a `.png` file.
- A small **`crossterm::style::Color` → RGB** mapping for the 16
  ANSI colors plus passthrough for `Color::Rgb`/`Color::AnsiValue` (a
  fixed lookup table, no crate needed).
- `CellStyle.bold` can be approximated by brightening the mapped color
  (ANSI "bold as bright" convention) — good enough for a first pass;
  doesn't need real font-weight rendering.

### Where it lives (keeping the `ttui` lib single-dependency)

Add a **separate crate**, e.g. `tools/visual-snapshot/`, with its own
`Cargo.toml` depending on `ttui` via a path dependency (`ttui = { path
= "../.." }`) plus `image` and `font8x8`. This keeps every new
dependency out of the `ttui` library's own `Cargo.toml` — the posture
`development-conventions.md`/the core framework spec establish governs
the library crate, and this sidesteps the question entirely rather than
relying on an interpretation of it. Concretely this likely means adding
a `[workspace]` table to the root `Cargo.toml` with `members = [".",
"tools/visual-snapshot"]` so Cargo resolves the path dependency — that's
a real change to an existing file and should go through the normal
brainstorming/planning process before it happens, not be done as a side
effect of writing this doc.

### Rough shape of the tool

1. Construct (or reuse) an `App` and a fixed `Rect`/terminal size.
2. Call `app.view(area, &mut layer_stack)` — same call `app::run()`
   makes — then `layer_stack.composite()` to get a `Buffer`. This can be
   done for a single frame, or repeatedly after calling `app.on_tick()`
   / synthetic `Event`s to capture a sequence (e.g. dial rotating through
   several ticks, or a keypress-driven mode transition), giving a set of
   frames instead of one.
3. For each `Cell` in the `Buffer`, blit an 8x8 (or NxN scaled) glyph
   from `font8x8` into an `ImageBuffer` at the cell's pixel position,
   using the fg color for the glyph and the bg color for the cell's
   background rectangle.
4. Encode to PNG via `image::save_buffer` or similar; write to a path
   the agent can then open with the **Read** tool directly (Read
   supports images) — no `claude-in-chrome` needed for this path at all.
5. Optional follow-up once single-frame snapshots are proven: encode a
   sequence of frames as an animated GIF via the `image` crate's GIF
   encoder (still pure Rust, no external binary) to check animation
   timing/motion, not just a single static layout.

### How it'd wire into the dev loop

- As a `cargo run -p visual-snapshot --example <name> -- --out
  frame.png` command the agent runs after any change to
  `src/effects.rs`, `src/particles.rs`, `src/transition.rs`, or an
  example's `view()`, then `Read`s the PNG to check the result — this
  is the direct analogue of the `claude-in-chrome` screenshot-and-look
  loop, just for a terminal buffer instead of a web page.
  `.claude/skills/run/` claims (per its listing) to already know how to
  "launch and drive this project's app to see a change working" — worth
  checking whether that skill should grow an "and take a snapshot"
  branch once this tool exists, rather than duplicating that logic.
- Could later become a `#[test]`-adjacent snapshot check (compare
  against a checked-in reference PNG, flag on diff) if that's ever
  wanted — but that's a testing-strategy decision for
  `development-conventions.md`'s still-open coding-style questions, not
  assumed here.

### What still needs human/agent verification before trusting this

- Whether `font8x8`'s glyph coverage is sufficient for whatever
  Unicode/box-drawing characters TTUI's `BorderSet`/`Dial`/`Block`
  widgets actually emit (untested — worth grepping `src/theme.rs` and
  the widgets for the actual glyph set used before committing to this
  font).
- The ANSI-16-color → RGB table is a real judgment call (terminal color
  palettes vary); it only needs to be "close enough to tell colors
  apart," not match any specific terminal's exact palette, but that's
  worth stating explicitly rather than implying pixel-perfect fidelity.

## Deferred / not recommended right now

- **VHS (Option B)** — good second step if Option A's approximation
  ever becomes the limiting factor (e.g. suspected font/glyph-rendering
  bug that only shows in a real terminal), or if scripted *keyboard-
  driven* sequences (not just synthetic `Event`s fed to `update()`) are
  needed for confidence. Needs a human to `winget install` or
  `scoop install` it first — not attempted here.
- **ttyd + claude-in-chrome live session (Option C)** — the most
  "Claude in Chrome"-like option and worth revisiting if a bug is
  specifically about live interactive feel (input latency, key-repeat
  behavior) that a scripted tool can't exercise. Explicitly flagged
  above as having two unverified assumptions (Windows ConPTY stability
  in ttyd, and whether input actually round-trips to `crossterm`'s
  reader) — do not assume this works without a human first installing
  ttyd and an agent or human actually trying it end-to-end against one
  of the examples.
- **wetty / gotty** — no evidence either beats ttyd on Windows
  specifically; not worth the extra Node.js dependency (wetty) or the
  less-scrutinized Windows story (gotty) unless ttyd itself turns out to
  be the blocker.

## Sources consulted

- `src/buffer.rs`, `src/app.rs`, `src/terminal.rs`, `Cargo.toml`,
  `docs/design/specs/core/2026-08-04-ttui-core-framework-design.md`,
  `docs/design/README.md` (read directly in this repo).
- Web research (August 2026) on: ttyd (native Windows/ConPTY support
  since 1.7.0, open Windows crash issue #1292), wetty/node-pty (Windows
  ConPTY support since Windows 10 1809+), `sorenisanerd/gotty` (active
  fork, cross-platform claim less scrutinized for Windows), VHS
  (`charmbracelet/vhs`, winget/scoop packages, uses ttyd + ffmpeg
  internally), asciinema/`agg` (agg itself is a portable Rust crate;
  asciinema's own recorder has no first-class Windows story),
  termtosvg (explicitly Linux/macOS/BSD only, unmaintained since 2020),
  terminalizer (no confirmed current Windows status found), the `vt100`
  Rust crate (headless ANSI-to-screen-state parser), and `font8x8` /
  `tui-big-text` (bitmap-font-to-pixels precedent in the Rust TUI
  ecosystem).
