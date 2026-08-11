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

Written after implementing `examples/depth_spike.rs` across Tasks 1-5.
**Environment caveat, upfront and honest:** this sandbox has no
interactive PTY/TTY at all — not "limited," genuinely none. No task in
this plan, including this one, was able to run
`cargo run --example depth_spike` interactively and look at it. What
every task *could* do, and did: build the code cleanly, hand-trace the
projection/clipping/fill/topology math against representative inputs,
and (Tasks 3-5) capture raw ANSI output from short `timeout`-bounded
runs and read the escape-sequence stream for evidence of correct
structure (contiguous fill spans, moving star glyphs, a fill-then-
wireframe cube silhouette). That is real evidence of code-level
correctness, not a substitute for a human (or a real rendered capture)
judging whether the result *looks* convincing. Two of the five
questions were left open at that point — the final whole-branch review
closed both by actually running `tools/visual-snapshot` against
`depth_spike` and reading the captured frames; those two entries below
are updated accordingly, with the other three left as originally
written.

- **Does the projection read as convincing depth?** **Confirmed.** A
  `tools/visual-snapshot` capture at 100×35, ~10 real seconds (13
  frames, two full cube-drift cycles), settles this: bright outer stars
  (`@`/`*`) traverse roughly 10 cells between frames while the dim
  central `.` cloud moves only 1-3 cells — a 3-10x apparent-speed
  differential between near and far stars, with brightness and glyph
  density both tracking depth radially (bright near the edges, dimming
  toward the vanishing point at center). None of this is separately
  tuned; it falls directly out of the shared `1/z` projection math with
  zero per-star speed adjustment, exactly the spike's central
  hypothesis. Task 1's hand-trace (nearer test points land farther from
  center-screen and brighter, in the correct order) and Task 4's
  captured frame deltas (screen-position/brightness genuinely changing
  tick-over-tick) already pointed this way; the visual capture is the
  confirming evidence.
- **Does near-plane clipping behave sanely?** **Confirmed.** Task 1
  hand-traced the exact boundary case (`z=0.2 <= NEAR_PLANE=0.5` is
  excluded, `z=2.0` and above render). Task 4 walked `on_tick`'s
  respawn logic and confirmed a star can overshoot the near plane by at
  most one tick's `dz` (~0.13 units) before being caught and respawned
  at `STAR_RESPAWN_Z`, with `project()`'s own `None`-on-clip as a
  second line of defense even if a stray value slipped through in the
  same frame — no negative-`z` or divide-blowup path exists in the
  code as written.
- **Does the scanline fill work cleanly?** **Confirmed.** Task 3 hand-
  traced two full scanline rows of the test quad and got exactly two
  crossings per row (even parity, as expected for a convex polygon),
  with span width shrinking correctly toward the vertex tip, and
  independently confirmed the near/far edge heights (6.667 vs 4.0)
  match the expected perspective taper. The captured partial terminal
  output for that same run showed contiguous solid Braille spans in the
  fill color, not scattered pixels or gaps — direct evidence against an
  off-by-one in the crossing test or span-pairing logic.
- **Does parallax genuinely fall out of the math for free?** **Confirmed
  at the code level.** `render_starfield` derives glyph and brightness
  purely from the projection-derived `scale = focal_length / z` value —
  there is no second, hand-tuned depth-to-visual curve anywhere in the
  starfield code (Task 4). Speed is likewise uniform per-star
  (`STAR_SPEED` is a single constant applied to every star's `z`); the
  *apparent* screen-space speed difference between near and far stars
  is a pure side effect of the same `z`-dependent projection, not a
  separately coded speed curve. This is exactly the "no second heuristic
  layered on top" property the spike set out to test, and it holds by
  inspection of the code — the "does it *look* like convincing
  parallax in motion" question is no longer open either; see the
  confirmed answer above.
- **Does a projected shape stay legible across its depth range?**
  **Qualified — yes near, no far.** The same `tools/visual-snapshot`
  capture answers this directly. Near the camera (`cube_z` roughly
  4-8, spanning ~32-20 cells wide at 100×35) the cube reads as an
  unambiguous cube: back face, all four converging depth edges, and a
  correct perspective taper are all visible. Past roughly
  `cube_z ≈ 10` (down to ~6 cells wide and smaller) it degenerates into
  a featureless filled blob with no visible internal structure — front
  and back faces sit within about one cell of each other at that
  distance — exactly the "indistinct dot far away" failure mode the
  plan's own Task 5 named as a risk to watch for. Task 5's 12-edge cube
  topology hand-trace (every edge pair differs in exactly one axis,
  every vertex has degree 3, no duplicate or missing edges, a valid
  non-self-crossing front-face perimeter) and its captured partial runs
  (fill-then-wireframe layering, a growing screen-column footprint as
  the cube nears) already pointed at correct geometry; the visual
  capture confirms it reads correctly near the camera and identifies
  where legibility breaks down. **Graduation condition:** a real module
  needs a minimum-projected-size threshold or an explicit draw-distance/
  LOD cutoff, rather than rendering arbitrarily-small projected
  geometry all the way out.

- **New finding: `Canvas`'s Braille mode cannot render a filled shape
  with a differently-colored outline.** The same visual capture showed
  3 of the cube's 12 wireframe edges (front-top, front-left,
  back-bottom) invisible in the rendered output — swallowed by the
  front-face fill color. Root cause, found by reading `src/canvas.rs`'s
  `blit_braille` (not modified here — this plan's Global Constraints
  forbid any `src/` change this Arc): it picks each cell's color from
  the *last subpixel in scan order* (bottom-right-most set dot), not
  the *last-written* color, despite that function's
  `// last-write-wins per cell` comment (`src/canvas.rs:150`) claiming
  otherwise — so a wireframe edge lying inside a filled region loses
  its color to the fill unless it happens to land on that specific
  subpixel. **Graduation condition:** a real module needs either
  separate canvases/layers per color, a per-cell color-priority rule,
  or an explicit constraint that outline and fill must share one color.
  The misleading comment at `src/canvas.rs:150` is also worth fixing,
  as a separate future follow-up — not this branch.

**Path forward on the two open questions — resolved.** This project's
`visual-snapshot` tooling (`tools/visual-snapshot/`, merged separately
per `docs/design/specs/core/2026-08-09-visual-snapshot-tooling-design.md`)
was built to solve precisely this "no eyes-on-terminal" gap
(`docs/tooling/visual-review.md`). The final whole-branch review used it
to capture and read real rendered frames from `depth_spike` — both open
questions above are now answered from that evidence, not merely
"possible to answer this way." The graduation recommendation below no
longer rests entirely on hand-traced math.

**Graduation recommendation.** Graduate this into a real `src/`
module — the three confirmed findings (clipping is sound, the scanline
fill is correct, parallax genuinely falls out of the projection math
with no second heuristic) are exactly the structural risks this spike
existed to de-risk, and all three came back clean. Recommended name:
`src/perspective.rs`, per the working name already used in this spec's
Scope/Non-goals sections — nothing surfaced during implementation that
argues for a different name or location. Concretely, before it's
committed with full TDD:

- **Write tests fresh against the public API**, not by promoting the
  spike's example code as-is — per this project's `research`-tag TDD
  exemption and the spec's own Testing section. The projection formula,
  the near-plane clipping boundary (`z <= NEAR_PLANE`), and the
  scanline fill's even-odd crossing logic are the three things with
  hand-traced math already in hand from Tasks 1/3 to turn directly into
  unit-test cases (e.g. the exact `z=0.2` clip boundary and the two
  scanline rows Task 3 traced).
- **Resolve the visual-quality open questions first** (see "Path
  forward" above) — don't graduate a `NEAR_PLANE`/`FOCAL_LENGTH`/
  `ASPECT_COMPENSATION` constant set into a committed module before
  confirming at least once that the projection they produce actually
  looks right, since those constants are exactly the kind of thing a
  real visual pass might want to tune.
- **Keep the simplified clipping rule** ("all vertices in front of the
  camera, or skip the whole shape") rather than building true near-plane
  segment/polygon clipping — the spec's Non-goals already deferred this
  unless the showcase scene proved it was needed, and nothing in Tasks
  1-5 found a case (test lines, test polygon, cube edges, starfield)
  where a shape needed to be partially visible across the near plane
  rather than fully clipped. This should be re-confirmed, not just
  assumed, once the "shape stays legible" visual question is answered
  with a real run — if the graduated cube (or a future Falcon canopy
  frame) is ever seen clipping through the near plane during normal
  use, that's the signal to revisit this.
- **Keep the fixed-forward-only camera** — confirmed by design and
  untouched by any of the five implementation tasks; no finding argues
  for pulling general camera movement/rotation into the graduation
  scope.
- **Decide `fill_polygon`'s home separately from the projection math.**
  It's `Canvas`-adjacent (scanline-fills an arbitrary polygon into a
  `Canvas`, no `Point3`/camera dependency in its own signature beyond
  taking already-projected 2D points) rather than inherently 3D — worth
  deciding during the graduation design doc whether it lands in
  `src/canvas.rs` alongside `fill_rect`, or stays in `src/perspective.rs`
  next to the `Polygon3`/`project_line` code that produces its inputs.
- **Clamp `fill_polygon`'s scanline loop to the canvas's actual
  height.** `for y in min_y.max(0)..=max_y` has no upper bound clamp —
  `set_pixel` bounds-checks individual writes but not the iteration
  count itself, so a polygon vertex very close to the near plane could
  produce an enormous `max_y` and a per-frame stall. Doesn't trigger in
  this spike's own scene (the cube's front face never gets nearer than
  z=2.0), but is a landmine for graduation; recommend clamping the loop
  bounds to `0..canvas.grid_height()`.
- **Clip, don't saturate, `project_line`'s off-screen coordinates.**
  Its subpixel conversion saturates off-screen coordinates to 0 (via
  `.max(0.0) as u16`) rather than actually clipping the line at the
  screen edge, which could draw a spurious edge-hugging segment for a
  line that goes off-screen. Doesn't trigger in this spike's scene;
  worth deciding alongside the near-plane-clipping approach when this
  graduates.
