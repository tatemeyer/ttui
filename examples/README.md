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
- **`demo`** — the original core-framework smoke-test example, predates
  the vision-doc apps above. Retirement tracked in issue #83.
