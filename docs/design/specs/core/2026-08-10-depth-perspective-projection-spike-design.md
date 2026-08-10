# Depth & Perspective Projection Spike — Design

**Status:** draft, pending your review before we move to planning.
**Date:** 2026-08-10
**Relationship to prior specs:** the first of four Arcs from a
"next tier of TTUI capability" brainstorm (rendering depth/perspective,
audio, advanced input, data-viz — this one first, since it's the
concrete forcing function: it directly unblocks the paused Falcon
cockpit-view redesign, which wants a windshield with parallax starfield
and a perspective-framed canopy). Builds on `Canvas`
(`2026-08-08-rendering-fidelity-spike-design.md`, graduated in Arc A) for
sub-cell line/pixel drawing, `Transition`/`easing` for tick-driven
animation, and Arc C's alpha compositing for layering a projected scene
over other content. Does not touch `LayerStack`/`Cell`'s shape — nothing
here needs a new `Cell` field, only new math producing 2D coordinates
that existing primitives already know how to render. `src/camera.rs`
already exists (2D buffer dimming, e.g. `camera::dim`) and is unrelated
to and unchanged by this spec — naming here is chosen specifically to
avoid colliding with it.

## Context / Motivation

Every visual effect TTUI has shipped so far is fundamentally 2D: cells
placed at fixed positions, colors blended, borders drawn around
rectangles. Nothing in the framework currently produces the illusion of
things being nearer or farther away — a starfield can drift, but every
star drifts at the same speed and brightness regardless of "distance";
a border can be angled, but nothing computes what an angled surface
would actually look like if it receded toward a vanishing point.

The Falcon cockpit-view redesign wants exactly this: a windshield
showing space with real parallax (near stars drift faster and brighter
than far ones) and a canopy frame that reads as converging into the
distance rather than as flat diagonal brackets. Rather than hand-tuning
those two effects separately with unrelated heuristics, this spike asks
whether one real projection system — a fixed camera, a formula mapping
3D points to 2D screen position plus a depth-derived scale — can drive
both, and generalize to future scenes beyond Falcon (any "flying
through/toward something" visual).

## Scope

**Tag: `research`.** Per `.claude/rules/development-conventions.md`,
this arc is exempt from TDD — its code is prototype-quality by design.
The deliverable is **answers and a recommendation**, not a stable public
API. Code produced here is expected to be partly thrown away or
substantially rewritten once a real, committed graduation Arc is scoped
from its findings.

Five things prototyped together in one showcase scene:

1. **Fixed-forward pinhole projection core.** A camera fixed at the
   origin `(0, 0, 0)`, looking down `+Z` — no position/orientation
   changes (confirmed: fixed-forward only, not a general positionable
   camera, for this spike and its likely graduation). A `Point3 { x:
   f32, y: f32, z: f32 }` in camera-relative space projects to 2D screen
   position plus a depth-derived scale:

   ```
   ndc_x = point.x / point.z
   ndc_y = point.y / point.z
   scale = focal_length / point.z          // larger = nearer/bigger/brighter

   screen_x = center_x + ndc_x * focal_length * ASPECT_COMPENSATION
   screen_y = center_y - ndc_y * focal_length
   ```

   `ASPECT_COMPENSATION` (~2.0) corrects for terminal cells being
   roughly twice as tall as wide — the same compensation the `Dial`
   widget already applies to its ring radius. The `-` on `screen_y`
   maps 3D "up" (`+y`) to a decreasing screen row. Points with `point.z
   <= NEAR_PLANE` (a small positive epsilon) are clipped — projection is
   undefined at `z = 0` and inverted for `z < 0` (behind the camera).

2. **Line projection.** `Line3 { start: Point3, end: Point3 }` projects
   to a 2D segment `Canvas::line` can already draw. Simplified clipping
   for this spike: both endpoints must satisfy `z > NEAR_PLANE`, or the
   whole line is skipped — true near-plane segment clipping (computing
   the intersection point when one endpoint is behind the camera and
   one is in front) is deferred as a documented limitation unless the
   spike's showcase scene finds it's actually needed for a
   fixed-camera, static-geometry use case.

3. **Polygon projection + fill.** `Polygon3` (3+ `Point3` vertices, same
   simplified all-vertices-in-front-of-camera clipping rule as lines)
   projects to a 2D polygon. `Canvas` currently only fills axis-aligned
   rectangles (`fill_rect`) — this spike prototypes a scanline fill
   (for each row the polygon spans, find the intersection x-range(s)
   with the polygon's edges and fill between them) as new,
   prototype-quality `Canvas`-adjacent code, since nothing in the
   framework fills an arbitrary-shaped region today.

4. **Depth-driven visual falloff, reused across all three.** The same
   `scale` value from the projection formula drives point size/
   brightness (nearer stars render bigger and with a brighter color,
   via `easing::lerp_color` toward the theme's brightest tone as `scale`
   grows) — the goal is confirming parallax speed/brightness/size *fall
   out of the real projection math automatically* as `z` changes over
   time, rather than needing a second, hand-tuned depth-to-visual curve
   layered on top.

5. **Showcase scene.** A new example, `examples/depth_spike.rs` (matches
   `render_spike.rs`'s precedent as a bare, non-themed proving ground).
   Tick-driven (`tick_rate`/`on_tick`): a starfield of `Point3`s spawn
   far away (`z` large), drift toward the camera (`z` decreasing per
   tick via the existing `Transition`/manual-`z`-decrement pattern
   already used for other tick-driven animation in this codebase),
   growing and brightening as they approach, and despawn once
   `z <= NEAR_PLANE`. Alongside it, one simple projected shape (a
   wireframe cube or a basic canopy-frame-like form) stress-tests line
   and polygon projection together, confirming the result reads as
   legibly "3D-ish" in glyphs rather than as noise.

**Explicitly not delivered here:** a stable public API for the
projection math, a committed module name or location (`src/
perspective.rs` is a working name, not a commitment — see Non-goals),
integration into Falcon or any other example, general camera movement/
rotation, and true near-plane segment/polygon clipping.

## Success criteria

- The showcase example runs and visibly demonstrates real parallax
  (near vs far objects moving at different apparent speeds) and a
  legible sense of depth on the projected shape — judged by you running
  it, not by a metric.
- A **recommendations write-up**, appended to this spec once the spike
  is run, answering the five questions from the brainstorm (does the
  projection read as convincing depth; does near-plane clipping behave
  sanely; does the scanline fill work cleanly; does parallax genuinely
  fall out of the math for free; does a projected shape stay legible)
  and recommending whether/how this graduates into a real `src/` module.

## Testing

Per `.claude/rules/development-conventions.md`'s `research`-tagged
exception: no TDD requirement. `cargo build --examples` must still
succeed, but no unit tests are required for the prototype code in this
arc. Any part of this that's later promoted to a committed Arc gets full
TDD coverage at that point, written fresh against whatever API that
follow-up spec actually commits to — not by promoting spike code as-is.

## Critical files

- `examples/depth_spike.rs` — new, the showcase scene.
- Prototype-quality projection math as needed to support it (`Point3`/
  `Line3`/`Polygon3` and the projection formula above, a scanline fill
  helper) — exact file list is an implementation-plan concern, not
  fixed here, since spike code is expected to be reshaped as findings
  emerge.

## Verification

- `cargo build --examples` succeeds.
- `cargo run --example depth_spike` — manual visual check (real-TTY
  exception applies, same as every other example) confirming the
  starfield shows real parallax and the projected shape reads as 3D-ish.
- `cargo fmt` / `cargo clippy --all-targets` clean is **not** a hard
  gate for spike-only prototype files, consistent with the `research`
  tag — but should still be run and any trivial warnings fixed if
  they're free.

## Non-goals

- **A committed module name/location.** `src/perspective.rs` is used
  above as a working name for where this would eventually live — not a
  commitment. The graduation Arc (if this spike recommends one) picks
  the real name and location.
- **General camera movement/rotation.** Confirmed fixed-forward only —
  a full positionable/orientable camera (view-matrix math, arbitrary-
  angle near-plane clipping) is explicitly out of scope, for this spike
  and its likely graduation.
- **True near-plane segment/polygon clipping.** The simplified
  "all vertices must be in front of the camera or skip the whole shape"
  rule is a documented limitation, not solved here, unless the spike's
  own showcase scene finds it's actually needed.
- **Falcon integration.** Building the actual cockpit-view redesign
  with this math is deferred until this spike and a graduation Arc both
  land — this spec's findings feed directly into that follow-up, but
  don't build it.
- **The other three brainstormed directions** (audio, advanced input,
  data-viz widgets) — each is a separate, not-yet-brainstormed future
  Arc, unrelated to this spec.

## Recommendations (post-spike)

Written after running `examples/depth_spike.rs`. Not yet available —
this section is appended once the spike is implemented and run, before
this spec is considered closed, matching the same deferred-until-run
convention as `2026-08-08-rendering-fidelity-spike-design.md`.
