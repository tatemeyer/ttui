# Rendering Fidelity Spike — Design

**Status:** draft, pending your review before we move to planning.
**Date:** 2026-08-08
**Relationship to prior specs:** builds on the unchanged Rev A
(`2026-08-04-ttui-core-framework-design.md`) pipeline and Rev B
(`2026-08-05-ttui-rev-b-vision-alignment-design.md`) tick/Theme
mechanism, the shipped `LayerStack`/`composite()` design
(`2026-08-05-buffer-layering-compositing-design.md`), Arc 0's
`CellStyle`/`effects`/`easing`/`particles` primitives
(`2026-08-06-core-capabilities-design.md`), and the coalesced
`render_diff` encoder (`2026-08-08-render-diff-performance-design.md`).
None of those specs' guarantees change here. This spec does not
supersede any of them — it is a `research`-tagged exploration that
produces a recommendation for which future Arc(s) get scoped next.

## Context / Motivation

Prior core work has landed rendering *infrastructure* (buffer layering,
tick-driven animation, bold text, screen-shake, particles, a coalesced
diff encoder) but each capability was designed and shipped in
isolation, validated against one narrow consumer at a time. No spec has
yet asked "what does maximum achievable visual fidelity in this
terminal stack actually look like, and do these capabilities compose
well together, or do they fight each other (diff efficiency, color
bleed, frame cost) once combined in one dense scene?"

This spec exists to answer that question before committing to more
core API surface. Regardless of which specific widgets or example apps
get built next, TTUI should be *capable* of rendering beautifully — this
spike is where that capability gets proven or falsified with a real,
working, maximally-dense scene, not assumed.

## Scope

**Tag: `research`.** Per `.claude/rules/development-conventions.md`,
this arc is explicitly exempt from TDD — its code is prototype-quality
by design. The deliverable is **answers and a recommendation**, not a
stable public API. Code produced here is expected to be partly thrown
away or substantially rewritten once real, committed follow-up Arcs are
scoped from its findings.

Six rendering levers, prototyped together in one showcase scene rather
than in isolation, specifically to surface interaction effects a
single-technique prototype would miss:

1. **Color depth audit.** Confirm `crossterm::style::Color::Rgb`
   (24-bit truecolor) is what actually reaches the terminal in this
   project's target environment (Windows Terminal/ConPTY, per Rev A's
   Windows-first scope), not a silent downsample to ANSI 256/16.
   Gradients (lever 4) are worthless if this is wrong, so this is
   checked first, before building on top of it.
2. **Sub-cell rendering.** A `Canvas` primitive (prototype-quality, not
   the committed API a future spec would design) with two modes:
   - **Half-block** — 1×2 subpixels per cell (2x vertical resolution),
     full 2-color fidelity per cell (`▀` with distinct fg/bg, or `█`
     when both subpixels match).
   - **Braille** — 2×4 subpixels per cell (4x resolution), one fg color
     per cell (last pixel written to a cell wins — the same constraint
     every terminal braille-canvas implementation accepts; dots aren't
     individually colorable).
   Basic shape helpers (`set_pixel`, `line`, `rect`, `fill_rect`) plus
   `blit(buf, x, y)` into a `Buffer`; unset subpixels stay
   `Cell::default()` (transparent), consistent with `LayerStack`'s
   existing transparency rule.
3. **Full `CellStyle` attribute set.** `CellStyle` is bold-only today;
   the spike prototypes underline, italic, dim, reverse, and
   strikethrough, reusing `render_diff`'s existing SGR-coalescing
   machinery rather than reverting to per-cell full resets.
4. **Gradient/color-ramp helpers.** Linear interpolation across a
   region's cells (a glowing border ramping between two hex colors, a
   meter fill ramping color by fill percentage) — gated on lever 1
   confirming truecolor is real.
5. **Real alpha/blend compositing.** `LayerStack::composite()` today is
   a hard cutout ("topmost non-default cell wins," no color math). The
   spike prototypes an actual blend (e.g. a per-cell alpha or blend-mode
   field) so glows, fades, and transitions look smooth instead of
   chunky. This is the one lever most likely to require a shape change
   to `Cell` if it graduates — the spike is explicitly where that risk
   gets surfaced, not hidden.
6. **Particle/trail polish.** The existing `ParticleSystem` (Arc 0) plus
   `easing` plus lever 5's blend model, used to render fading color
   trails behind moving particles — a stress case that exercises levers
   2, 4, and 5 simultaneously.

**Showcase vehicle:** a new example, `examples/render_spike.rs`. This
also fills the bare, non-themed smoke-test niche that `examples/demo.rs`
(retirement tracked in issue #83) currently occupies alone — the spike
does not depend on or modify `demo.rs`. Tick-driven (reusing
`tick_rate`/`on_tick`, unchanged since Rev B), rendering one continuous
scene combining all six levers at once: a gradient-bordered glowing
panel (4, 3), a half-block-filled energy gauge (2), a braille line plot
(2), and a particle burst with blended fading trails (5, 6, 3).

**Explicitly not delivered here:** stable public APIs for any of the
six levers, test coverage beyond what's needed to sanity-check the
prototype runs, and integration into the existing themed example apps
(Omnitrix/TARDIS/Smash Crabs). Those are precisely what the
recommendation section below hands off to future, separately-brainstormed
Arcs.

## Success criteria

- The showcase example runs and is visibly, dramatically better than
  anything currently on `main` — the concrete bar for "beautiful,"
  judged by you running it, not by a metric.
- Input stays tactile-responsive (Rev A's stated commitment) under the
  densest frame this scene produces — measured, not assumed. Reuse the
  `render_diff` `criterion` benchmark harness
  (`2026-08-08-render-diff-performance-design.md`) if the scene's diff
  profile is a natural fit for it; otherwise a simple frame-time
  instrumentation print is sufficient for a spike.
- A **recommendations write-up**, appended to this spec once the spike
  is run, ranking which of the six levers graduate into real,
  TDD-covered, committed core Arcs — and a suggested order, given
  lever 5's higher structural risk to `Cell`'s shape relative to the
  other five.

## Testing

Per `.claude/rules/development-conventions.md`'s `research`-tagged
exception: no TDD requirement. `cargo build --examples` must still
succeed (the example has to compile and run), but no unit tests are
required for the prototype code in this arc. Any lever's logic that is
later promoted to a committed Arc gets full TDD coverage at that point,
written fresh against whatever API that follow-up spec actually
commits to — not by promoting spike code as-is.

## Critical files

- `examples/render_spike.rs` — new, the showcase scene.
- Prototype-quality additions as needed to support it (a scratch
  `Canvas` type, gradient helpers, attribute wiring in
  `src/terminal.rs`, a blend-mode experiment on `LayerStack`) — exact
  file list is an implementation-plan concern, not fixed here, since
  spike code is expected to be reshaped as findings emerge.

## Verification

- `cargo build --examples` succeeds.
- `cargo run --example render_spike` — manual visual check (real-TTY
  exception applies to the terminal-facing half of this, same as every
  other example) confirming the scene renders and animates smoothly.
- Frame-time/diff-size measurement recorded in the recommendations
  write-up, showing the scene doesn't visibly degrade input
  responsiveness.
- `cargo fmt` / `cargo clippy --all-targets` clean is **not** a hard
  gate for spike-only prototype files, consistent with the `research`
  tag — but should still be run and any trivial warnings fixed if
  they're free, since a messy spike is harder to read when writing the
  recommendations.

## Explicitly deferred / open questions for future revisions

- Committed public APIs for any of the six levers — deferred to
  whichever follow-up Arc(s) the recommendations write-up identifies.
- `Cell`'s shape change for real alpha/blending (lever 5) — flagged as
  the highest-risk lever; the spike surfaces whether it's worth that
  cost, doesn't commit to it.
- Integration into Omnitrix/TARDIS/Smash Crabs — each would be its own
  future brainstorm, gated on this spike's findings.
- The recommendations write-up itself — not yet written; appended after
  the spike runs, before this spec is considered closed.

## Recommendations (post-spike)

Written after running `examples/render_spike.rs` and its `--bench`
timing harness.

- **Color depth (lever 1):** Smooth — Task 1's visual check showed a
  smooth, continuous color gradient with no visible banding; the 24-bit
  RGB escape sequences observed in the output confirm what `ttui`
  *emits* is truecolor, but the actual evidence that the *terminal*
  rendered it at full fidelity (rather than silently downsampling to
  ANSI 256/16) is that reported absence of banding when the example was
  run, not the escape sequences themselves.
- **Frame cost:** `--bench` (debug build, 120x40 area, 200-frame
  average, all six levers wired into the scene): `200 frames in
  1.4719147s (7.359573ms/frame avg), avg 622 diffed cells/frame`. This
  is an average over the harness's full 200-frame run, not a single
  densest-frame measurement — one particle burst (700ms lifetime) is
  spawned up front and measured at a 16ms tick, so only the first
  ~40-60 frames carry active burst content; the remaining ~140-160 are
  the static ring+gauge+plot scene with no burst. The 622-cells/frame
  average is therefore dominated by that static scene, and the true
  peak per-frame cost (all six levers simultaneously dense) is likely
  modestly higher than this average suggests. Even so, the recorded
  number is fast in absolute terms for an unoptimized debug build — Rev
  A's tactile-responsiveness commitment is qualitative (input-driven
  redraw, immediate unbuffered flush), not a numeric frame-budget
  threshold, but a ~7.4ms average leaves comfortable headroom under any
  reasonable interactive-redraw expectation, and a release build would
  only widen that margin further.
- **Graduation ranking**, highest-confidence first:
  1. Full `CellStyle` attributes (lever 3, minus `dim`) — cheapest,
     already SGR-coalesced, no structural risk. Recommend committing
     as-is via a real brainstorm, including the `Intensity` enum
     refactor flagged in Task 5 (folding `bold` and a proper `dim`
     into one tri-state field instead of independent bools).
  2. Sub-cell `Canvas` (lever 2) — both modes worked; recommend a real
     spec deciding whether `HalfBlock`/`Braille` stay one type with a
     mode enum (as prototyped) or split into two types.
  3. Gradient color ramps (lever 4) — `easing::lerp_color` already
     covers this; mainly needs a real widget-level home (e.g. a
     gradient option on `Block`/`Theme`), not new core math.
  4. Alpha blending (lever 5) — works for opaque-to-Rgb-target fades
     (as used for the particle trail) but **cannot gradually fade
     toward true transparency** (`Color::Reset`) — `lerp_color`'s
     non-Rgb fallback makes any real "fade to transparent" require an
     actual alpha channel on `Cell`, confirming this lever's
     flagged structural risk. Recommend a dedicated spec if pursued,
     given the `Cell`-shape cost. Two further findings from close
     review of the assembled scene reinforce this same conclusion:
     first, `blend_trail`'s call to `blend_over(&scene, &self.trail,
     1.0)` runs at `alpha = 1.0`, at which `blend_over`'s interpolation
     degenerates to a hard stamp (`lerp_color(_, to, 1.0) == to`, glyph
     and style taken outright from the overlay) — so the assembled
     showcase never actually exercises real alpha interpolation
     end-to-end; the visible smooth fade comes entirely from
     `fade_toward` decaying the trail buffer's own stored colors over
     time, not from `blend_over`'s blending math (which was validated
     directly against its own inputs, not through the assembled scene).
     Second, a sharper instance of the `Color::Reset` limitation above:
     `ParticleSystem::render` writes particle cells with `bg:
     Color::Reset`, so on the very first `fade_toward` tick applied to
     that cell, `lerp_color(Color::Reset, <Rgb target>, factor)` hits
     the non-Rgb fallback and returns the target color outright — the
     background channel jumps from `Reset` to fully-opaque-target-color
     in one tick instead of fading gradually, even though the
     foreground channel fades smoothly. Wherever a particle trail
     passes over the colored scene (the gradient ring, the half-block
     gauge, the braille plot) this punches an instantly-opaque patch
     through whatever was underneath — invisible against a plain black
     background, but visible against the scene's actual painted colors,
     which is exactly where the trail travels. Both findings are
     additional supporting evidence for the same recommendation above:
     a real alpha channel on `Cell` is needed for this lever to fully
     deliver on smooth, gradual blending.
  5. Particle trails (lever 6) — validated as an application of levers
     3/5/existing `ParticleSystem`, not a new primitive of its own;
     no separate Arc needed, it falls out of whichever of 1/4 lands.
