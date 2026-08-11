# Example apps

Each app is a full vertical-slice demo of the `ttui` framework, built
against a specific vision doc (`TTUI-Ideas/vision/UI/`). Run with
`cargo run --example <name>`.

- **`omnitrix`** — a dial-navigated gadget hub with three sub-apps
  (Brainstorm, Fasttrack, Upgrade) and a materialization boot sequence.
  Built from `TTUI-Ideas/vision/UI/idea-1-Omnitrix.md`.
- **`tardis`** — a hexagonal console hub with four sub-apps (Artron
  Energy, Psychic Paper, Star Charts, plus the Hub itself) and a
  camera-flight transition system. Built from
  `TTUI-Ideas/vision/UI/idea-3-TardisTUI.md`.
- **`smash_crabs`** — a character-select hub with three fighters
  (Versus Mode, Target Smash, Stage Hazards) and a Smash-Bros-style
  intro splash. Built from
  `TTUI-Ideas/vision/UI/idea-2-SuperSmashCrabs.md`.
- **`launcher`** — a cross-app "portal nexus" that presents the three
  apps above as bootable portals and switches between whole apps: each
  launches through its own boot + signature transition, `F12` (or an
  app's own `q`) returns to the nexus, and the nexus's `q` quits. Built
  from `docs/design/specs/launcher/2026-08-08-cross-app-launcher-design.md`.
- **`demo`** — the original core-framework smoke-test example, predates
  the vision-doc apps above. Retirement tracked in issue #83.
- **`render_spike`** — a bare showcase proving out six
  rendering-fidelity levers together; a research-spike prototype, not a
  themed vision-doc app. Built from
  `docs/design/specs/core/2026-08-08-rendering-fidelity-spike-design.md`
  rather than a vision doc.
- **`depth_spike`** — a bare showcase proving out a fixed-forward
  pinhole-camera projection system (points, lines, filled polygons); a
  research-spike prototype, not a themed vision-doc app. Built from
  `docs/design/specs/core/2026-08-10-depth-perspective-projection-spike-design.md`.

Each themed app's `App` type lives in `examples/<app>/<app>.rs` and is
reused by `launcher` via `#[path]`; `examples/<app>/main.rs` is a thin
standalone entry, so `cargo run --example <app>` still runs each app on
its own.
