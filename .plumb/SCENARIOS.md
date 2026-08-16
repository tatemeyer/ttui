# Plumb capture scenarios: what each one actually cost

Recorded for Slice 6.2 calibration checkpoint (Task 23), which turns
these three measurements into a rate for the rest of Arc 6. Honest
numbers, including the failures.

Each scenario intent in .plumb/config.yaml was corrected to describe
what its capture actually shows, not what a first-draft guess assumed
it would show. Both corrections below came from actually reading the
contact sheet, not from reasoning about the code alone.

## omnitrix-dial-rotate (Task 14, prior art - re-measured here)

Do not treat this as validated. Re-running it with the current
plumb.exe (which now produces a proper tiled contact sheet, fixing the
blank/broken-GIF-decode failure Task 14 originally hit) surfaced a
second, independent problem: examples/omnitrix/omnitrix.rs sets
BOOT_MS = 2500, but omnitrix-dial-rotate.json's first step is only
wait_ms: 400. The scripts entire 1300ms runtime (400+250+250+400)
happens during boot - the Right/Enter keys are sent into the boot
transition and have no effect, and every captured frame is boot
animation, not the dial-rotate UI the intent describes. Filed as
ttui issue #137 (https://github.com/tatemeyer/ttui/issues/137);
not fixed here - out of Task 22 scope (two new scenarios only).

- Attempts to get timing right (original, Task 14): 1 (per Task 14
  report) - but "right" was never actually checked against a contact
  sheet at the time, because the contact sheet did not exist yet. The
  apparent success was a false negative.
- Glyph rasterizer blocked it: no.
- Cost this task actually spent on it: one re-capture run (about 72s
  wall clock, dominated by cargo run -p visual-snapshot overhead) plus
  reading the resulting contact sheet - done opportunistically while
  authoring the two new scenarios below, not as separately budgeted
  work.
- Takeaway for the rate: the real cost of a scenario is not
  "captures without a nonzero exit code" - it is "captures and the
  contact sheet was actually read against the intent." This one did
  not get that check until now.

## tardis-console-idle

Chosen because: the brief designated it the floor of the cost range - a
pure-animation, no-input capture, the simplest shape a scenario can
take.

Script: .plumb/scripts/tardis-console-idle.json - five wait_ms steps
(3200, 500, 500, 500, 500), six frames.

What made it hard: examples/tardis/tardis.rs sets BOOT_MS = 3000. The
brief illustrative script (four 500ms waits, 2000ms total) would have
made the exact same mistake omnitrix-dial-rotate made - every frame
still inside boot. Caught by reading tardis.rs before running anything
(BOOT_MS, Transition::start), not by a failed capture: the first step
was raised to wait_ms: 3200 (roughly a 200ms margin over BOOT_MS)
before the first attempt. The brief sample intent text was also wrong
for what actually renders: it claimed the artron energy gauge holds a
steady reading and the surrounding instrument panels stay legible -
but Tardis::new() starts with selected_face 0 (the Psychic Paper face)
and no keys are sent, so the Artron Energy gauge (a different face,
reached only via navigation) is never on screen. The Hub screen camera
viewport also crops to exactly one face - no neighbouring panels are
ever visible, dimmed or otherwise. Corrected to describe what is
actually there: the selected face name, the rotor, and the hint line.

- Capture attempts: 1 (succeeded on the first real run, after the
  pre-run timing correction above).
- Glyph rasterizer blocked it: no.
- Intent-writing: required one correction after reading the contact
  sheet (see above) - not written to a standard a blinded reviewer
  could check on the first pass.
- Wall clock: about 53-58s per capture run end-to-end (plumb.exe
  capture invocation to manifest written), effectively unchanged
  between a cold and a warm run - dominated by cargo run -p
  visual-snapshot startup/check overhead, not by the roughly 5.2s of
  scripted animation time.
- Artifacts (from .plumb/runs/tardis-attempt1/, not committed -
  .plumb/runs/ is gitignored):
  - tardis-console-idle.png (contact sheet): 5792x1304, 3x2 tile grid,
    all 6 tiles filled.
  - tardis-console-idle.gif (per-frame 1920x640, 6 frames).
  - Manifest: frame_count 6, expects [], caveats [].
- What the contact sheet actually shows: frame 1 (t=0, captured before
  the first wait elapses) is mid-boot - the closed police box
  silhouette on black. Frames 2-6 (after the 3200ms wait and each
  500ms wait after it) show the idle Hub: "Psychic Paper" named at the
  top, a single diagonal line (the time rotor Braille render at this
  zoom level) rotating through a different angle each frame, and the
  "Left/Right rotate, Enter select, q quit" hint legible at the bottom
  of every idle frame. Real UI, not black frames - confirmed by
  reading the PNG directly.

## falcon-glitch-burst

Chosen because: the brief designated it the exercise of the
expects: [visual-corruption] exemption against a real image - the
first scenario in this repo to declare it, giving Task 2/8 unit tests
a real-image counterpart.

Script: .plumb/scripts/falcon-glitch-burst.json - wait_ms: 1600
(clears examples/falcon/falcon.rs BOOT_TOTAL_MS = 1400), then
Up, Up, Down, Down (the FullPower chord bound in
examples/falcon/input.rs, CHORD_TIMEOUT = 1500ms), then wait_ms: 200
and wait_ms: 600. Eight frames.

What made it hard - the real finding: three capture attempts, only ONE
actually showed the intended three-panel FullPower corruption burst.

- Attempt 1: no burst. Only Falcon unrelated ambient single-panel
  idle-flicker (IDLE_FLICKER_PERIOD_TICKS, a background worn-machine
  effect, unrelated to the chord) was visible, on the Sensors panel.
- Attempt 2 (the run whose artifacts are referenced in the intent and
  reviewed below): succeeded - frame 6 (captured right after the
  fourth key) shows all three panels (Hyperdrive, Sensors, Weapons)
  fully corrupted at once, matching FalconAction::FullPower handler
  exactly (it triggers glitches[i] for all three panels
  simultaneously). Frames 7-8 show the display fully clear again.
- Attempt 3 (a warm re-run, done for timing data, not credited as the
  verification run): no burst again - only single-panel ambient
  flicker, this time on Hyperdrive.

Root cause, traced through the code, not guessed:
tools/visual-snapshot/src/pty.rs capture_frame_after_key uses a
patient quiescence strategy (wait_for_first_output) that requires
seeing the screen change and then holding steady for a full poll
before returning - and Falcon screen never truly holds steady
(starfield, particle system, and phase-driven canopy render every
33ms tick). In practice this makes the wait between two consecutive
Key steps land anywhere from milliseconds to the full MAX_SETTLE_WAIT
(2000ms), unpredictably. Falcon chord requires each keypress within
1500ms of the last (InputBinder::expire). When the real gap between
two Key steps happens to exceed that, the chord silently resets and
FullPower never fires - with no error, no exit-code signal, nothing
distinguishing a working run from a failed one except reading the
contact sheet. Filed as ttui issue #138
(https://github.com/tatemeyer/ttui/issues/138) - a real tooling gap,
not something fixable inside this task scope (needs a design decision
about Step::Key quiescence contract or the script schema).

Intent correction: the brief sample intent claimed the instrument
panel readings remain legible through it. Reading attempt 2 frame 6 at
full resolution shows the opposite at the burst peak - all three
panels are almost entirely replaced by red corruption glyphs, no
readable text survives that single frame. This is not a taste-profile
violation (taste.md non-negotiable 1 only bars a glitch that
permanently destroys a reading; this one clears by the very next
frame), but the scenario own intent sentence was still an overclaim a
blinded intent lens would have had grounds to flag. Corrected to:
garbled at the peak, then clears and settles - no claim of legibility
during the burst itself.

- Capture attempts: 3 (1 success, 2 showed the wrong/unrelated
  corruption source instead of failing outright - the harder failure
  mode to catch, since the capture succeeds and something red is on
  screen either way).
- Glyph rasterizer blocked it: no (substitute mode was not needed;
  nothing in Falcon dashboard, boot, or glitch overlay hit an unmapped
  codepoint).
- Intent-writing: required one correction after reading a
  full-resolution crop of the peak-corruption frame (see above).
- Wall clock: about 72-90s per capture attempt end-to-end, same
  cargo run overhead as tardis-console-idle; the four Key steps
  variable real-time gaps (the flakiness own mechanism) added visible
  but not dominant variance across the three attempts.
- Artifacts (from .plumb/runs/falcon-attempt2/, the successful run;
  not committed):
  - falcon-glitch-burst.png (contact sheet): 5792x1952, 3x3 tile grid,
    8 of 9 tiles filled.
  - falcon-glitch-burst.gif (per-frame 1920x640, 8 frames).
  - Manifest: frame_count 8, expects [visual-corruption], caveats [].
- What the contact sheet actually shows: frame 1 (t=0) is early boot
  (a single lit point on black). Frames 2-5 show the dashboard
  powering up with the starfield, wireframe canopy, and the three
  bottom panels, with the Sensors panel ambient idle-flicker visible
  in frames 2-3. Frame 6 is the FullPower burst at its peak - all
  three panels text replaced by dense red corruption blocks, the
  canopy/starfield above unaffected. Frames 7-8 are fully clear again,
  all three panel labels legible. Real UI throughout, not black
  frames.

Step 3 (declared-exemption dual-direction check) was not run. The
brief calls for /plumb:review --scenario falcon-glitch-burst to
confirm the breakage lens honours expects: [visual-corruption] and
flags the same image when the declaration is removed. /plumb:review is
a slash-command skill this session does not have available to invoke
(matches Task 14 report noting the same absence). What was confirmed
instead, by reading the source: the Expectation parser in config.rs
correctly rejects a typo variant visual-corrupton rather than silently
treating it as empty (see config.rs declared_visual_corruption_parses
test and its sibling reject test), the manifest correctly carries
"expects": ["visual-corruption"] through to what a lens agent reads
(confirmed in falcon-glitch-burst.manifest.json above), and taste.md
own worked example already documents this exact scenario name and
declaration. The actual lens dispatch in both directions is unverified
and should be someone first action once /plumb:review (or the
equivalent plan/merge commands with real lens subagents) is available
in-session.

## Summary for the rate

| Scenario | Capture attempts | Timing corrections | Intent corrections | Wall clock (per attempt) |
|---|---|---|---|---|
| omnitrix-dial-rotate (re-measured) | 1 (2500ms boot-timing bug undiscovered until now) | 1 needed, not made (out of scope) | not assessed this pass | about 72s |
| tardis-console-idle | 1 | 1, made before running (code read caught the brief script being under BOOT_MS) | 1 | about 53-58s |
| falcon-glitch-burst | 3 (1 success) | 1, made before running (boot); a second, structural timing issue (chord vs settle-wait) was not fixable by script changes | 1 | about 72-90s |

Two things the rate needs to account for, not just the successes:

1. A capture that returns exit 0 is not evidence the scenario worked.
   Two of three omnitrix/falcon measurements here "succeeded" while
   showing the wrong thing. The only real check is reading the contact
   sheet against the stated intent.
2. Chord-based key scripts against continuously-animating apps are a
   materially more expensive class of scenario than wait-only ones -
   not because they are harder to write, but because their success is
   probabilistic given the current tool, at roughly 1-in-3 in this
   sample. Any future scenario relying on a multi-key chord should
   budget for this, or wait on ttui#138.
