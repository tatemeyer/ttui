# Showcase Polish Design

**Status:** approved (brainstorming complete 2026-08-14)

Sub-project #5.1 of the TTUI v1.0.0 initiative — a follow-up Arc to the
Flagship Showcase (`showcase/`, PR #129, unmerged at the time this Arc
started). Covers three items deliberately deferred out of PR #129's
final review: mascot idle animation, a more noticeable eye/blink, and
a real visual/functional rework of the Assembly Line vignette
(resolving GitHub issue #128 as part of the redesign, not separately).

A fourth item raised during PR #129's review — solid box-drawing pane
borders — turned out to require a breaking change to `ttui::theme::
BorderSet`'s public API (proper distinct corner glyphs need 4 corner
fields; `BorderSet` currently has one `corner: char` reused at every
position) with project-wide blast radius (every app using a
`BorderSet` literal, not just `showcase/`). Split out during this
Arc's own brainstorming into its own sub-project, tracked as GitHub
issue #130 (`semver:major`, `v1-blocking` per `code-forge.md`) — not
part of this design.

This worktree (`worktree-showcase-polish`) branched off `origin/main`
and then merged in `origin/worktree-flagship-launcher` directly (a
clean fast-forward), since PR #129 hasn't landed on `main` yet and
this Arc's code changes all build on top of `showcase/`'s existing
files.

## Mascot idle animation

`showcase/mascot.rs`'s `GripperMascot` currently has zero motion while
idling — it only changes pose on discrete triggers (`Reacting` on menu
highlight change, `Grabbing` on a catch/unlock). Two independent,
internally-driven timers are added, both consulted only while
`pose == MascotPose::Idle` (they keep accumulating time in the
background regardless of pose, so returning to `Idle` doesn't cause a
stutter, but only affect what's rendered during `Idle`):

- **Breathing** — toggles between the existing `IDLE` grid and a new
  `IDLE_B` grid every `BREATHE_INTERVAL` (2000ms). `IDLE_B` shifts the
  head/antenna assembly down by one pixel row (a small "settle") and
  adds a two-dot antenna-vent hint, compensated by dropping one body
  row so the claw/feet position (rows 8-11) stays fixed — the mascot's
  footprint doesn't shift, only its upper body.
- **Blinking** — every `BLINK_INTERVAL` (3500ms), holds a new `BLINK`
  grid (visor row fully dark) for `BLINK_DURATION` (150ms), overriding
  whichever breathing frame is currently showing. `BLINK` is always
  rendered from the antenna-up base regardless of the current breathe
  phase — a deliberate simplification, since a 150ms blink is too
  brief for the antenna-position mismatch to be perceptible, and it
  avoids needing a second blink variant per breathe phase.

Neither timer is exposed to callers — `showcase.rs` is unaffected,
still just calling `set_pose`/`tick` as it already does.

## Eye redesign

New palette entry, code `9` (`#ffffff`, bright white) — a pupil core
applied to the middle two cells of every open-eye visor band. Every
grid gets updated (exact data below); this fully replaces the current
single-tone cyan band everywhere it appears, not just in new frames.

```
IDLE (was: cols 3-8 solid cyan; now: pupil core at cols 4-5):
[0,0,0,0,0,1,0,0,0,0,0,0]
[0,0,0,0,0,1,0,0,0,0,0,0]
[0,6,2,2,2,2,2,2,2,6,0,0]
[0,2,2,4,9,9,4,4,4,2,2,0]
[0,2,2,2,2,2,2,2,2,2,2,0]
[0,0,2,2,2,2,2,2,2,2,0,0]
[6,2,2,2,2,2,2,2,2,2,2,6]
[0,0,2,2,2,2,2,2,2,2,0,0]
[0,0,0,1,2,2,2,2,1,0,0,0]
[0,0,0,0,1,2,2,1,0,0,0,0]
[0,0,0,0,0,3,3,0,0,0,0,0]
[0,0,0,3,3,0,3,3,0,0,0,0]

IDLE_B (breathing variant — head/antenna shifted down 1 row, one body row dropped):
[0,0,0,0,0,0,0,0,0,0,0,0]
[0,0,0,0,0,1,0,0,0,0,0,0]
[0,0,0,1,0,1,0,1,0,0,0,0]
[0,6,2,2,2,2,2,2,2,6,0,0]
[0,2,2,4,9,9,4,4,4,2,2,0]
[0,2,2,2,2,2,2,2,2,2,2,0]
[6,2,2,2,2,2,2,2,2,2,2,6]
[0,0,2,2,2,2,2,2,2,2,0,0]
[0,0,0,1,2,2,2,2,1,0,0,0]
[0,0,0,0,1,2,2,1,0,0,0,0]
[0,0,0,0,0,3,3,0,0,0,0,0]
[0,0,0,3,3,0,3,3,0,0,0,0]

BLINK (visor fully dark, antenna-up base):
[0,0,0,0,0,1,0,0,0,0,0,0]
[0,0,0,0,0,1,0,0,0,0,0,0]
[0,6,2,2,2,2,2,2,2,6,0,0]
[0,2,2,1,1,1,1,1,1,2,2,0]
[0,2,2,2,2,2,2,2,2,2,2,0]
[0,0,2,2,2,2,2,2,2,2,0,0]
[6,2,2,2,2,2,2,2,2,2,2,6]
[0,0,2,2,2,2,2,2,2,2,0,0]
[0,0,0,1,2,2,2,2,1,0,0,0]
[0,0,0,0,1,2,2,1,0,0,0,0]
[0,0,0,0,0,3,3,0,0,0,0,0]
[0,0,0,3,3,0,3,3,0,0,0,0]

REACTING (narrower band, cols 4-7; pupil core at cols 5-6):
[0,0,0,0,0,1,0,0,0,0,0,0]
[0,0,0,0,0,1,0,0,0,0,0,0]
[0,6,2,2,2,2,2,2,2,6,0,0]
[0,2,2,2,4,9,9,4,2,2,2,0]
[0,2,2,2,2,2,2,2,2,2,2,0]
[0,0,2,2,2,2,2,2,2,2,0,0]
[6,2,2,2,2,2,2,2,2,2,2,6]
[0,0,2,2,2,2,2,2,2,2,0,0]
[0,0,0,1,2,2,2,2,1,0,0,0]
[0,0,0,0,1,2,2,1,0,0,0,0]
[0,0,0,0,0,3,3,0,0,0,0,0]
[0,0,0,3,3,0,3,3,0,0,0,0]

GRABBING (same visor treatment as IDLE, claw closed):
[0,0,0,0,0,1,0,0,0,0,0,0]
[0,0,0,0,0,1,0,0,0,0,0,0]
[0,6,2,2,2,2,2,2,2,6,0,0]
[0,2,2,4,9,9,4,4,4,2,2,0]
[0,2,2,2,2,2,2,2,2,2,2,0]
[0,0,2,2,2,2,2,2,2,2,0,0]
[6,2,2,2,2,2,2,2,2,2,2,6]
[0,0,2,2,2,2,2,2,2,2,0,0]
[0,0,0,1,2,2,2,2,1,0,0,0]
[0,0,0,0,1,2,2,1,0,0,0,0]
[0,0,0,0,0,3,3,0,0,0,0,0]
[0,0,0,0,3,3,3,3,0,0,0,0]
```

## Assembly Line rework

Replaces the plain `#`-row crate and the stationary top-right mascot
with a real "the mascot walks over and picks it up" interaction,
resolving #128's chain-overlap and ~26s-runtime findings as part of
the redesign rather than a narrower retune.

**Crate visual** — a 6×3 pixel-tile sprite (solid-color `Cell`s, same
bg-fill technique as the mascot — not a glyph), warm-wood palette:

```
[10,10,10,10,10,10]   10 = #4a2f1a (dark trim)
[10,11,12,12,11,10]   11 = #c7a06a (wood body)
[10,10,10,10,10,10]   12 = #6b7278 (strap band)
```

**Mascot enters the lane** — for this vignette only, the mascot is
positioned within the crate lane (not its usual top-right spot used
everywhere else). It starts at the left edge of the 50-cell travel
span described below (the same x crates spawn at), and, on a
successful catch, slides horizontally to that crate's x-position over
`MASCOT_SLIDE_DURATION` (300ms) before `Grabbing` plays and the crate
is removed — sequencing matters here: clicking a crate marks it
*targeted* (frozen in place, stops scrolling) rather than immediately
caught; the catch (puff, `Grabbing`, removal) happens only once the
mascot's slide animation actually arrives at the target x-position.
Freezing a targeted crate avoids needing to extrapolate a moving
target's position during the slide. A new click while a previous
slide is still in progress simply redirects the slide from the
mascot's current (possibly mid-slide) position toward the new target —
no special-casing, same `MASCOT_SLIDE_DURATION` either way. The mascot
does not auto-return to a resting position after a catch; it just
stays at the last catch's x-position until the next one (or the
vignette ends).

The crate lane sits vertically aligned with the mascot's claw (its
bottom pixel rows, 8-11), not an arbitrary mid-screen row.

**Hit-testing becomes a real 2D bounding-box check** — `handle_click`
now tests the click against the crate's actual on-screen `Rect`
(reusing `ttui::layout::Rect::contains`, matching `control_panel`'s
existing click-hit-testing pattern) instead of the old single cached
`row_y` plus a hand-rolled row-tolerance check. This is a
simplification enabled by the crate now genuinely being a small 2D
box rather than a 1-row band.

**Retuned constants** (resolving #128 — spacing now exceeds width, and
total duration comes down from ~26s toward roughly double the passive
vignettes' scale, not 5-6x):

| Constant | Old | New |
|---|---|---|
| `CRATE_COUNT` | 6 | 4 |
| `CRATE_WIDTH` | 8 | 6 (matches the sprite) |
| `CRATE_HEIGHT` | — | 3 (new — matches the sprite) |
| `CRATE_SPEED` | 4.5 cells/sec | 6.0 cells/sec |
| `SPAWN_INTERVAL` | 700ms | 1100ms (spacing 6.6 cells > 6-wide crate — no chain-overlap) |
| Lane travel | full `area.width` (~100 cells, ~22s traversal) | a fixed 50-cell span centered in the vignette area (~8.3s traversal) |
| Total vignette duration | ~26s | ~12s (spawn window ~3.3s + traversal ~8.3s) |

Confining the lane to a fixed 50-cell span (rather than however wide
the terminal happens to be) is the main lever bringing duration down —
tuning speed alone can't fix both catchability and total runtime at
once, since a faster crate is harder to click. `ROW_TOLERANCE`
disappears entirely (replaced by the 2D `Rect::contains` check, which
is exact against the crate's real bounds — no separate tolerance
constant needed since the crate's own dimensions already define a
generous click target).

## Out of scope

- Border-glyph changes (`BorderSet` distinct corners) — split into
  issue #130, its own sub-project.
- Any change to Overload Vent, Diagnostic Scan, Override Sequence, or
  Telemetry — this Arc only touches the mascot and Assembly Line.
- Issue #121 (Diagnostic Scan's small wireframe) and #127
  (`tools/visual-snapshot`'s chord-timeout/quiescence-wait interaction)
  — both already tracked separately, neither raised as in-scope during
  this Arc's brainstorming.

## Testing

Same discipline as the original Flagship Showcase Arc: TDD exemption
applies to `showcase/*.rs` (demo code, verified by running +
`tools/visual-snapshot` review, not by assertion) — `GripperMascot`'s
and `AssemblyLineState`'s existing unit-test modules get updated/
extended for the new timers and hit-testing logic, matching the
precedent already set (both files already carry real unit tests
despite the exemption, since their state-machine logic is worth
asserting on directly). Mandatory `tools/visual-snapshot` capture +
review for the mascot's new breathing/blink frames and the full
reworked Assembly Line vignette before this Arc's PR merges. The
human-only real-TTY checklist (per `development-conventions.md`)
covers Assembly Line specifically again, since it's the vignette that
already needed two live-testing-driven fixes in PR #129 — automated
review alone wasn't sufficient for it once already.
