# Cross-Platform (Linux) Verification — Design

**Status:** draft, pending review before we move to planning.
**Date:** 2026-08-13
**Relationship to prior work:** sub-project #4 of the TTUI v1.0.0
initiative. Depends on sub-project #1 (Release Governance, merged) for
the triage process any real finding routes through. Independent of
#2/#3 — this verifies the current state of `main`, not anything
specific to the pre-v1 fix wave.

## Problem

TTUI's CI already runs `build`/`test`/`clippy`/`fmt` on `ubuntu-latest`
for every PR, so the automated suite has been continuously green on
Linux this whole project. What's never happened: a real interactive
session — a human actually typing into a TUI app, a real Unix PTY
actually exercising `portable-pty`'s non-Windows code path — since
every prior Arc's manual/visual verification ran on this project's
Windows dev machine. Rev A originally scoped TTUI as Windows-first,
deferring Linux/macOS; this sub-project is where that deferral gets
tested for real, on the one platform actually available to verify
(WSL Ubuntu, confirmed reachable from this session via `wsl.exe`).

**Fresh-environment finding:** the WSL Ubuntu install has `git` but no
Rust toolchain and no `gh` CLI — real setup is needed before anything
else can run.

## Scope

**Tag: `admin`/`research`** — environment setup and verification, not
new library code. No TDD in the plan's own tasks; any *fix* a finding
produces is separately `coding`-tagged and goes through the normal
process, not built here.

Four stages, in dependency order:

1. **Setup** — `rustup` + `gh` CLI in WSL Ubuntu.
2. **Automated gate suite** — `cargo build/test/clippy/fmt`, for real,
   on Linux.
3. **Real Unix PTY captures** — `tools/visual-snapshot` against a
   representative example subset, run from within WSL.
4. **Human-only checklist** — a short, exact set of steps for the user
   to run themselves in a real WSL terminal window, closing the one
   gap nothing automated can close: eyes-on confirmation of raw-mode
   enter/exit and mouse click behavior on a real Linux terminal.

Any real finding from any stage: filed as a GitHub issue and triaged
via `.claude/rules/code-forge.md`'s process (same as the sweep audit)
— sized by public-API impact, routed to `v1-blocking` or deferred.

## Design

### Stage 1: Setup

Install via `wsl.exe -d Ubuntu -- bash -lc "<command>"`:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
```
(non-interactive install; adds `~/.cargo/bin` to the default shell's
`PATH` via rustup's own profile script).

```bash
sudo apt-get update && sudo apt-get install -y gh
```
(`gh` is available directly via Ubuntu's package manager on modern
Ubuntu releases — no separate repo setup needed; confirm this holds
for whatever Ubuntu version the WSL install actually reports before
falling back to GitHub's official apt-repo instructions if it doesn't.)

Verify both: `cargo --version`, `rustc --version`, `gh --version`, all
run inside WSL.

### Stage 2: Automated gate suite

From within WSL, in the repo's `/mnt/d/...` path (the same worktree
this Arc uses — Windows paths are directly mounted and visible to
WSL, no separate clone needed):

```bash
cargo build --all-targets
cargo test --workspace
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

Expected: all four green, matching what CI's `ubuntu-latest` runner
already proves — this stage's value is confirming the *local* WSL
environment (not just GitHub's hosted runner) produces the same
result, and catching anything a hosted-runner-specific quirk might
mask.

### Stage 3: Real Unix PTY captures

`tools/visual-snapshot` is built on `portable-pty`, which has separate
Windows (ConPTY) and Unix (real PTY via `openpty`) backends — every
capture this entire project has ever produced ran the Windows backend.
Running the same tool from within WSL exercises the Unix backend for
the first time.

Representative subset (6 of 10 examples, chosen to cover every
distinct subsystem rather than repeat similar apps):

| Example | Subsystem covered |
|---|---|
| `demo` | Core pipeline (layout/widgets/buffer/diff) — the baseline |
| `control_panel` | Mouse capture — genuinely unverified on a real Unix PTY |
| `falcon` | Glitch effects, camera/perspective HUD, particles — most subsystem-dense |
| `mission_control` | Data-viz (`BarChart`/`Sparkline`), tick-driven animation |
| `tardis` | Buffer layering/compositing, camera-flight transitions |
| `launcher` | Cross-app composition |

For each: run the same style of capture command used throughout this
project (`cargo run -p visual-snapshot -- --example <name> --size
<cols>x<rows> --script <path.json> --out <path>`), from within WSL,
and `Read` the resulting PNG/GIF — same mandatory-visual-review
standard as every other Arc, just executed on a different PTY backend.
A script exercising each example's most subsystem-relevant interaction
(e.g. `control_panel`'s click-scripting for mouse, `falcon`'s glitch
trigger) rather than a bare idle-state capture.

### Stage 4: Human-only checklist

Written as an exact, numbered list for the user — not something I can
execute myself, since `wsl.exe -d Ubuntu -- bash -lc "..."` (this
session's only path into WSL) is non-interactive and provides no more
of a real TTY than this Windows session's own shell does.

1. Open a real WSL Ubuntu terminal window (Windows Terminal's WSL
   profile, or `wsl.exe` launched directly from a Windows terminal —
   not through Claude Code).
2. `cd` to this Arc's worktree path (given as a Linux path,
   `/mnt/d/...`, in the final instructions).
3. `cargo run --example control_panel`. Click the LAUNCH button
   (confirm the particle burst renders), click a toggle (confirm it
   flips), click the dial (confirm it advances). Press `q` to quit.
   Confirm the terminal returns to a normal prompt — no stuck raw
   mode, cursor visible, no leftover alternate-screen artifacts (type
   an arbitrary command afterward and confirm it echoes normally).
4. `cargo run --example demo`. Press `Tab` a few times, then Up/Down.
   Confirm focus switching and navigation feel instant (no visible
   lag) — Rev A's tactile-responsiveness commitment, now checked on a
   real Linux terminal for the first time. Quit and confirm clean
   terminal restoration again.
5. Report back: did everything behave identically to the Windows
   experience, or did anything look wrong, stuck, or garbled?

## Non-goals

- **macOS verification.** Explicitly out of scope for v1.0.0 per the
  earlier decision — Linux only, since it's the only platform actually
  available to test on this machine.
- **Fixing anything found.** This sub-project verifies and files;
  fixes are separately `coding`-tagged work through the normal
  process, sized by the same triage rule as the sweep audit.
- **All 10 examples in Stage 3.** The 6-example representative subset
  is deliberate — covers every distinct subsystem without redundant
  captures of similarly-structured apps (e.g. `omnitrix`/`smash_crabs`
  are structurally similar to `tardis`, already covering the
  dial-hub/layering pattern).
- **CI changes.** Nothing here touches `.github/workflows/` — this is
  a one-time local verification pass, not a new automated check.

## Testing

`admin`/`research`-tagged, no TDD — this produces no library code.
"Testing" here means the verification stages themselves succeeding or
surfacing findings, not unit tests.

## Critical files

- No source files created or modified by this sub-project itself.
- Reads: all 6 named examples, `tools/visual-snapshot`.
- Any real finding: filed as a new GitHub issue (not a file in this
  repo) per `code-forge.md`'s process.

## Verification

- `rustup`/`gh` both installed and version-confirmed inside WSL
  Ubuntu.
- Stage 2's four gate-suite commands all reported green, run for real
  inside WSL (not assumed from CI).
- Stage 3's 6 captures all produced and `Read`, confirming each
  renders correctly (or naming exactly what doesn't, per the mandatory
  visual-review convention's existing "note the specific error" rule
  for known limitations like unmapped glyphs).
- Stage 4's checklist was actually run by the user (not skipped), with
  their reported result recorded.
- Any real finding from any stage is filed as a GitHub issue with the
  correct `semver:*`/`v1-blocking` labels per `code-forge.md`'s rule,
  same as every sweep-audit finding.
