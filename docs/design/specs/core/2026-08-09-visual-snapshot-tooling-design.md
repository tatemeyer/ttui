# Visual Snapshot Tooling — Design

**Status:** draft, pending your review before we move to planning.
**Date:** 2026-08-09
**Relationship to prior work:** builds on the unchanged `Buffer`/
`LayerStack`/`Terminal::draw_diff` separation confirmed in
`docs/tooling/visual-review.md` (2026-08-06, research doc, superseded
by this spec as the actual implementation plan) and the `CellStyle`
shape landed since that doc was written (`Intensity` enum plus
underline/italic/reverse/strikethrough, not just `bold: bool` — see
`src/buffer.rs`). Does not modify `src/`'s public API or the `ttui`
library's own `Cargo.toml`.

## Context / Motivation

TTUI apps (`examples/launcher/`, `examples/omnitrix/`,
`examples/tardis/`, `examples/smash_crabs/`) are deliberately flashy —
glow borders, screen-shake, particle bursts, gradient borders,
transitions. "Correct" for this kind of UI means *looks right*, which
`cargo test` and a non-interactive smoke run cannot catch. This dev
environment has no interactive TTY for an agent to drive/screenshot the
way `claude-in-chrome` does for web work.

`docs/tooling/visual-review.md` surveyed nine options in depth and
recommended starting with a pure-Rust `Buffer`→PNG rasterizer
(no subprocess, no human install). During brainstorming for this spec,
that recommendation was revisited: the synthetic approach (calling
`App::view()`/`on_tick()` directly, never touching `app::run()`'s real
event loop) would not have caught the launcher Arc's event-loop
starvation bug (freezing on a held key) — only the codebase's real
polling/tick logic exercises that failure mode, and a synthetic harness
bypasses `app::run()` entirely by construction.

This spec instead centers on **`portable-pty`** (the Windows-ConPTY
crate WezTerm ships in production — mature, pure Rust, no external
binary, no human install, no browser) to run the *actual compiled
example binary* under a pseudo-console this tool creates and controls
end-to-end, combined with **`vt100`** to parse the resulting terminal
byte stream into structured cell state, then the same bitmap-font
rasterization approach the original doc proposed to turn that into a
PNG/GIF an agent can `Read` directly.

This supersedes `docs/tooling/visual-review.md`'s Option A as the
primary approach — that doc is left as-is for historical record, not
rewritten, and referenced here as prior art.

## Why not ttyd (Option C from the prior doc)

`docs/tooling/visual-review.md` flagged ttyd + `claude-in-chrome` as
the most "Claude in Chrome"-like option but demoted it because of two
unverified, Windows-specific risks: ttyd's native Windows/ConPTY support
is relatively recent (1.7.0) with an open crash report on Windows, and
whether keystrokes actually round-trip xterm.js → ttyd → ConPTY → the
child's raw-mode reader was never tested against this codebase.

`portable-pty` avoids both: it's the same ConPTY mechanism, but
in-process, with no third-party server binary and no browser layer — we
write input bytes directly into the pseudo-console's input handle
ourselves, so there's no multi-hop round-trip to leave unverified. It
also needs no human install step (a Cargo dependency, not a system
package), so it keeps the "the agent can build and run this itself,
headlessly" property that ruled out ttyd/wetty/gotty in the first place.

## Scope

**Tag: `coding`.** TDD applies to all library code in the new crate
except where explicitly exempted below.

In scope:
- A new crate, `tools/visual-snapshot/`, that spawns a compiled
  example binary under a `portable-pty` pseudo-console, drives it with
  scripted key input and real-time waits, captures its output via
  `vt100`, and rasterizes the resulting screen state to PNG (single
  frame) or GIF (multiple frames).
- A CLI: `cargo run -p visual-snapshot -- --example <name> --size
  <colsxrows> --script <path.json> --out <path>`.
- Updating `.claude/rules/development-conventions.md` so task/final
  code reviews touching rendering-affecting code are required to use
  this tool, not left to invent an ad hoc method each time.
- A root `Cargo.toml` `[workspace]` table adding `tools/visual-snapshot`
  as a member (no change to the `ttui` library's own `[dependencies]`
  or `[dev-dependencies]`).

Out of scope (see Deferred):
- Checked-in reference-image diffing (phase 2).
- Canned per-example scenario-script fixtures (follow naturally once
  phase 2 gives them a reason to exist as durable regression checks).
- Any change to example app code — this tool talks to compiled example
  binaries purely as subprocesses over PTY bytes, so no example's
  `main.rs`/`App` struct changes.

## Architecture

- **`tools/visual-snapshot/`** — new workspace member. Depends on
  `portable-pty`, `vt100`, `image`, `font8x8`. No dependency on `ttui`
  itself — it never touches `Buffer`/`LayerStack`/`App` at the Rust API
  level, only the ANSI byte stream a real terminal would see.
- **Root `Cargo.toml`** gains a `[workspace]` table: `members = [".",
  "tools/visual-snapshot"]`. No new dependency of any kind is added to
  the `ttui` library's own `[dependencies]`/`[dev-dependencies]` — this
  tool is fully external to it.
- **Pipeline:**
  1. Run `cargo build --example <name>` as a plain (non-PTY) subprocess
     so build output never interleaves with the app's TUI output.
  2. Spawn the resulting binary under a `portable-pty` pseudo-console
     sized to `--size`.
  3. For each script step:
     - `{"wait_ms": N}` → sleep `N` ms of real wall-clock time (this is
       what actually exercises an app's real `tick_rate()`-driven
       animation, since we never call `on_tick()` ourselves).
     - `{"key": "Right"}` → write the byte sequence for that named key
       into the pseudo-console's input handle (fixed lookup table:
       arrows, `Enter`, `Esc`, `Tab`, literal chars, common `Ctrl+`
       combos — covers the `KeyCode` variants TTUI apps actually read).
     - After each step, wait a fixed 100ms settle delay, then pump
       available output bytes into a `vt100::Parser` and snapshot its
       current `Screen` as one frame. 100ms is well above a single
       `draw_diff` write's cost and gives the child process's own
       `crossterm` polling loop time to wake and redraw; not derived
       from a measurement, so the implementation plan should treat it
       as a starting point to tune if it proves flaky.
  4. After the last step, kill the child process (no graceful
     in-app quit needed — killing a process attached to a
     pseudo-console we allocated doesn't affect the host's own
     terminal state).
  5. 1 frame → PNG; 2+ frames → animated GIF, each frame held for its
     step's real elapsed duration (a `wait_ms` step's own duration; a
     `key` step's frame gets a fixed 150ms display duration — input is
     instantaneous, but a GIF frame needs a nonzero hold time to be
     visible at all when viewed).
- **Rendering:** each `vt100` `Cell` blits an 8x8 `font8x8` glyph
  (2x nearest-neighbor upscaled to 16x16 per cell — legible detail
  without a real font-rendering dependency, matching the blocky
  aesthetic of the block-element glyphs (`░▒▓█▀▄▌`) TTUI widgets
  already emit) using the cell's fg color over its bg color rectangle.
  Style axes: bold/dim brighten/darken the fg color, `reverse` swaps
  fg/bg, underline/strikethrough draw a 1px line overlay, italic is
  tracked but not visually rendered (no slanted variant in a fixed
  bitmap font). A glyph outside `font8x8`'s coverage (verify against
  `vt100`'s cell contents, not assumed — the dingbat star `✦` used by
  `EnergyCore`'s charged state is a known likely gap) is a **hard
  error** naming the missing codepoint; no image is written. This keeps
  a snapshot from silently mispresenting a cell's true appearance.

## Testing strategy

- **Key encoder** (name → byte sequence): pure unit tests, no
  subprocess required.
- **`vt100::Screen` → pixel rasterizer** (color table, style
  approximation, glyph blit, hard-error-on-unmapped-glyph): unit tests
  feed `vt100::Parser` synthetic ANSI byte sequences directly (no PTY,
  no subprocess) and assert the rasterized pixel output, mirroring the
  style-axis test pattern already in `src/terminal.rs`
  (`all_five_style_axes_combine_independently_in_one_cell`).
- **PTY orchestration** (spawn, size, byte pump, settle-delay, kill):
  a `portable-pty` pseudo-console is created *by* this tool's own
  process — it does not require the host process to already have a
  real TTY, unlike `Terminal::new()` in `src/terminal.rs`. This code
  does **not** qualify for `development-conventions.md`'s existing
  real-TTY `#[ignore]` exception (that exception is for code "only
  verifiable against a real terminal"; this isn't). It gets real,
  deterministic integration tests against a minimal fixture binary
  under `tools/visual-snapshot/tests/fixtures/` (e.g. one that echoes
  a received key back as visible text) run in normal `cargo test` —
  slower than unit tests due to subprocess spawn overhead, not
  `--ignored`.
- **PNG/GIF file encoding**: round-trip tests through a temp file,
  re-decoding to confirm dimensions/frame count. Normal I/O test, not a
  TDD exception.
- **End-to-end against a real example** (`launcher`, `omnitrix`, etc.):
  verified manually as part of task completion — actually run the tool,
  actually `Read` the resulting PNG/GIF. Automating pixel-exact
  assertions against a real flashy app's output is brittle and isn't
  the goal; this mirrors the existing examples/demos TDD exception's
  "correctness checked by running it" bar, applied to this tool's own
  final verification step rather than to example code.

## Wiring into the review workflow

`run`, `subagent-driven-development`, and `requesting-code-review` are
global superpowers/built-in skills, not files in this repo — this spec
does not modify them. Wiring in means project-level guidance any
dispatch prompt written for this repo must follow:

- `.claude/rules/development-conventions.md` gets a new subsection
  (near the existing Real-TTY tests section) stating: any task or final
  review touching rendering-affecting code (`src/effects.rs`,
  `src/particles.rs`, `src/transition.rs`, `src/widgets/`,
  `src/canvas.rs`, `src/glitch.rs`, or an example's `view()`/
  `on_tick()`) must run `visual-snapshot` against the affected
  example(s) and `Read` the resulting PNG/GIF before approving — not
  optional, not "reasoned through it, no PTY available."
- The same section notes that visual-affecting PRs should record which
  snapshots were reviewed in the PR template's existing freeform
  Verification section (`.claude/templates/github/PULL_REQUEST_TEMPLATE.md`),
  the same pattern already used for real-TTY test results — no change
  to the template itself, which stays freeform by design.

## Deferred / not built now

- **Checked-in reference-image diffing.** Compare a fresh render
  against a committed golden PNG/GIF per scenario, flag pixel drift
  automatically. Natural phase 2 once canned scenario scripts (below)
  give it something to diff against. Not designed further here.
- **Canned per-example scenario-script fixtures** (e.g.
  `launcher/focus-portal.json`). v1 ships only ad hoc JSON scripts a
  reviewer writes per review. Durable named fixtures are a fast-follow
  once phase 2 (above) gives them a reason to exist as regression
  checks, not before.
- **Synthetic `Buffer`-construction fallback** (the original
  `docs/tooling/visual-review.md` Option A: call `App::view()`/
  `on_tick()` directly, no subprocess/PTY at all). Noted as a
  lower-fidelity but zero-subprocess-risk fallback if `portable-pty`/
  ConPTY proves unreliable in practice on this Windows environment —
  not built now.
- **ttyd + `claude-in-chrome` live session.** Still the closest match
  to a genuinely live, click-and-type interactive review experience if
  a bug specifically needs that (vs. scripted key sequences). Revisit
  only if `portable-pty`'s scripted approach turns out insufficient for
  some interaction-latency-specific bug.

## Resolved during planning (docs.rs verification)

Both open questions above were checked against the real crate docs
before writing the implementation plan:

- **`vt100::Cell` exposes `bold()`, `italic()`, `underline()`, and
  `inverse()` — there is no `dim()` or `strikethrough()` accessor.**
  This is a real fidelity gap, not an oversight: `CellStyle::intensity`'s
  `Dim` variant and `CellStyle::strikethrough` are not visually
  verifiable through this tool, because `vt100` itself doesn't expose
  them from the parsed ANSI stream. The "Style axes" paragraph above is
  narrowed accordingly — the rasterizer approximates bold (brighten),
  inverse (swap), and underline (line overlay); italic is tracked
  conceptually but never rendered (no slanted bitmap variant); dim and
  strikethrough are not rendered at all, by API constraint rather than
  choice. If a future bug specifically hinges on dim or strikethrough
  rendering, that's a reason to revisit the deferred synthetic
  `Buffer`-construction fallback (which has direct access to the full
  `CellStyle`), not to route around `vt100`.
- **`font8x8`'s `MISC_FONTS` table covers only ~10 currency/fraction
  glyphs — it does not reach the Dingbats block (U+2700–U+27BF), the
  Geometric Shapes block (U+25A0–U+25FF), the Arrows block
  (U+2190–U+21FF), or any Emoji/Supplementary-Plane codepoint.**
  Confirmed by direct testing against `glyph::glyph_for`, not just
  spec-time reasoning (see the final-branch review fix that expanded
  this section — `.superpowers/sdd/2026-08-09-visual-snapshot-tooling-
  plan/final-review-fix-report.md`). The full set of TTUI's actual
  non-ASCII glyph usage that `font8x8` does not cover, as of this
  writing:
  - **`✦` U+2726** (dingbat star) — `EnergyCore`'s charged state
    (`src/widgets/energy_core.rs`).
  - **`►` U+25BA, `◄` U+25C4, `◆` U+25C6, `◈` U+25C8, `◉` U+25C9,
    `○` U+25CB, `●` U+25CF** (geometric shapes) — used across
    `examples/launcher/portal.rs`, `examples/launcher/nexus.rs`,
    `examples/omnitrix/fasttrack.rs`, `examples/omnitrix/upgrade.rs`,
    and `examples/tardis/star_charts.rs`.
  - **`←` U+2190, `→` U+2192** (arrows) — used by
    `examples/launcher/nexus.rs` and, notably, by a `src/widgets/`
    file: `src/widgets/damage_meter.rs`'s hit-direction indicator.
    `DamageMeter` is therefore also affected by this gap, not only
    `EnergyCore` as originally scoped.
  - **`💥` U+1F4A5** (emoji, outside the Basic Multilingual Plane) —
    `examples/smash_crabs/smash_crabs.rs`'s explosion effect.

  Running this tool against any of the widgets/examples above is
  expected to hit the hard-error path. That's treated as correct
  behavior (the error names the real gap), not a bug to route around —
  see `.claude/rules/development-conventions.md`'s "Visual review"
  section for the escape hatch this implies for a mandated review that
  hits one of these. A future Arc could add small supplemental
  hand-drawn 8x8 bitmaps for these specific glyphs (following the
  precedent set by Braille below), but that's new scope, not part of
  this plan.
- **Braille Patterns (U+2800–U+28FF), used by `TimeRotor`
  (`src/widgets/time_rotor.rs`, via `Canvas`'s `Braille` mode) and by
  `examples/omnitrix/omnitrix.rs`'s ambient noise effect, is also not
  covered by any `font8x8` table — but unlike the gaps above, this one
  was closed during the final-branch review fix, not deferred.** The
  block's encoding makes it renderable algorithmically rather than
  needing a lookup table: each codepoint's low 8 bits directly name
  one of 8 dots in a fixed 2-column x 4-row grid, the same bit layout
  `src/canvas.rs`'s `blit_braille` already uses to *emit* these
  glyphs. `glyph::braille_glyph_for` (`tools/visual-snapshot/src/
  glyph.rs`) decodes that same bit layout into an 8x8 pixel bitmap
  (each dot scaled to a 4x2-pixel block), checked ahead of the
  `font8x8` table lookups in `glyph_for`. `TimeRotor` is fully
  renderable by this tool as of this fix; `EnergyCore` and
  `DamageMeter` are not (dingbat star and arrows respectively, per the
  gaps listed above).

## Sources consulted

- `docs/tooling/visual-review.md` (2026-08-06 research doc — read in
  full, all Options table entries).
- `src/buffer.rs`, `src/app.rs`, `src/terminal.rs`, `Cargo.toml` (read
  directly in this repo to confirm the prior doc's claims still hold
  and to catalog the actual glyph/style surface: `Intensity` enum,
  underline/italic/reverse/strikethrough beyond the prior doc's
  bold-only description).
- `src/glitch.rs`, `src/canvas.rs`, `src/widgets/energy_core.rs`,
  `src/widgets/dna_console.rs`, `src/theme.rs` (grepped for non-ASCII
  glyphs to catalog the real font-coverage requirement: ASCII borders
  `-|+`, Unicode block elements `░▒▓█▀▄▌`, and `✦`).
- `examples/launcher/main.rs` (confirmed `App` structs are private to
  their own example binary, motivating the subprocess/PTY access
  pattern over a shared-library access pattern).
- `docs/design/README.md` (Arc bucketing convention, `core/`'s scope).
- `.claude/rules/development-conventions.md` (TDD exceptions, Real-TTY
  test convention, PR template Verification section).
