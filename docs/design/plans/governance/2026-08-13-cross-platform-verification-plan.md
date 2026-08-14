# Cross-Platform (Linux) Verification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Verify TTUI behaves correctly on Linux for the first time
via real, interactive testing (not just CI's headless `ubuntu-latest`
runner) — WSL Ubuntu setup, a real local gate suite, `portable-pty`'s
actual Unix PTY backend exercised for the first time this project, and
a human eyes-on check of raw-mode/mouse behavior in a real terminal.

**Architecture:** Four sequential stages, each a separate task. Stage
1 sets up the environment; Stages 2-3 run entirely inside WSL via
`wsl.exe -d Ubuntu -- bash -lc "<command>"` from this session; Stage 4
is a checklist for the user to execute themselves, since this
session's only path into WSL is non-interactive and provides no more
of a real TTY than a Windows shell does. Any real finding from any
stage is filed as a GitHub issue and triaged via
`.claude/rules/code-forge.md`'s process — this plan itself makes no
code changes.

**Tech Stack:** `rustup`, `gh` CLI, WSL2/Ubuntu, `tools/visual-snapshot`.

## Global Constraints

- **`admin`/`research`-tagged, no TDD** — this plan produces no
  library code; it verifies and files findings.
- **Linux (WSL Ubuntu) only** — macOS is explicitly out of scope for
  v1.0.0.
- **This worktree's WSL-mounted path** is
  `/mnt/d/Dev/Projects/TTUI/.claude/worktrees/cross-platform-verification`
  — Windows paths are directly visible from WSL, no separate clone
  needed.
- **Nothing in this plan touches `.github/workflows/`.**
- **Any real finding** (something that works on Windows but breaks on
  Linux) gets filed as a GitHub issue and triaged via
  `.claude/rules/code-forge.md`'s rule (touches `ttui`'s public API →
  `semver:minor`/`major` + `v1-blocking`; otherwise → `semver:patch`)
  — fixing it is separate, later work, not part of this plan.

---

### Task 1: WSL setup

**Files:** none — this task installs software into the WSL Ubuntu
environment, not repo files.

**Interfaces:** none.

- [ ] **Step 1: Install `rustup`**

```bash
wsl.exe -d Ubuntu -- bash -lc "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"
```

- [ ] **Step 2: Verify the Rust toolchain**

```bash
wsl.exe -d Ubuntu -- bash -lc "source \$HOME/.cargo/env && cargo --version && rustc --version"
```

Expected: both print a real version string (not "command not found").
If `source $HOME/.cargo/env` is needed here, note that every
subsequent WSL command in this plan must also source it (rustup's
installer updates `~/.bashrc` for future interactive shells, but a
non-interactive `bash -lc` invocation may or may not pick that up
depending on the WSL Ubuntu's shell config — verify directly rather
than assuming).

- [ ] **Step 3: Install `gh` CLI**

```bash
wsl.exe -d Ubuntu -- bash -lc "sudo apt-get update && sudo apt-get install -y gh"
```

If this fails because Ubuntu's default repos don't carry `gh`, fall
back to GitHub's official apt-repo setup:

```bash
wsl.exe -d Ubuntu -- bash -lc "(type -p wget >/dev/null || sudo apt-get install wget -y) && sudo mkdir -p -m 755 /etc/apt/keyrings && wget -qO- https://cli.github.com/packages/githubcli-archive-keyring.gpg | sudo tee /etc/apt/keyrings/githubcli-archive-keyring.gpg > /dev/null && sudo chmod go+r /etc/apt/keyrings/githubcli-archive-keyring.gpg && echo \"deb [arch=\$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main\" | sudo tee /etc/apt/sources.list.d/github-cli.list > /dev/null && sudo apt-get update && sudo apt-get install -y gh"
```

- [ ] **Step 4: Verify `gh`**

```bash
wsl.exe -d Ubuntu -- bash -lc "gh --version"
```

Expected: a real version string. `gh` doesn't need to be authenticated
for this plan — no `gh` commands in Tasks 2-3 require it; it's
installed for completeness/parity with the Windows dev environment and
for filing any findings from within WSL if that turns out to be more
convenient than filing from the Windows session.

- [ ] **Step 5: Record the result**

No commit (nothing in the repo changed) — note in this task's
completion record that `rustup`/`gh` are both installed and
version-confirmed, quoting the actual version strings observed.

---

### Task 2: Automated gate suite

**Files:** none — read-only verification against the current worktree.

**Interfaces:**
- Consumes: the working Rust toolchain from Task 1.

- [ ] **Step 1: Build**

```bash
wsl.exe -d Ubuntu -- bash -lc "source \$HOME/.cargo/env && cd /mnt/d/Dev/Projects/TTUI/.claude/worktrees/cross-platform-verification && cargo build --all-targets"
```

Expected: succeeds. If it fails on a missing system library (e.g.
ALSA headers for `rodio`, which CI's `test`/`clippy` jobs already
install via `sudo apt-get install -y libasound2-dev` — see
`.github/workflows/ci.yml`), install the same package:

```bash
wsl.exe -d Ubuntu -- bash -lc "sudo apt-get install -y libasound2-dev"
```

then retry the build.

- [ ] **Step 2: Test**

```bash
wsl.exe -d Ubuntu -- bash -lc "source \$HOME/.cargo/env && cd /mnt/d/Dev/Projects/TTUI/.claude/worktrees/cross-platform-verification && cargo test --workspace"
```

Expected: matches the Windows-observed count as of this branch's base
(`345 passed, 0 failed, 2 ignored` — the 2 ignored are the permanently-
manual real-TTY tests, which stay `#[ignore]`d and manual regardless
of platform). A different count is itself a finding — investigate
before treating it as neutral.

- [ ] **Step 3: Clippy**

```bash
wsl.exe -d Ubuntu -- bash -lc "source \$HOME/.cargo/env && cd /mnt/d/Dev/Projects/TTUI/.claude/worktrees/cross-platform-verification && cargo clippy --all-targets -- -D warnings"
```

Expected: clean.

- [ ] **Step 4: Format check**

```bash
wsl.exe -d Ubuntu -- bash -lc "source \$HOME/.cargo/env && cd /mnt/d/Dev/Projects/TTUI/.claude/worktrees/cross-platform-verification && cargo fmt --check"
```

Expected: clean.

- [ ] **Step 5: Record the result**

No commit. Note the actual test count and pass/fail status for all 4
commands in this task's completion record. If anything differs from
the Windows-observed baseline, that's a finding — file it per this
plan's Global Constraints before proceeding to Task 3, since Task 3's
captures would be unreliable evidence if the build itself is broken.

---

### Task 3: Real Unix PTY captures

**Files:** none — read-only captures via `tools/visual-snapshot`.

**Interfaces:**
- Consumes: a working, tested build from Task 2.

- [ ] **Step 1: `demo` — baseline single-frame capture**

```bash
wsl.exe -d Ubuntu -- bash -lc "source \$HOME/.cargo/env && cd /mnt/d/Dev/Projects/TTUI/.claude/worktrees/cross-platform-verification && echo '[]' > /tmp/empty.json && cargo run -p visual-snapshot -- --example demo --size 80x24 --script /tmp/empty.json --out /tmp/demo-linux.png"
```

Copy the result out for reading (WSL's filesystem is reachable from
Windows via `\\wsl$\Ubuntu\tmp\demo-linux.png`, or `cp` it into the
`/mnt/d/...` path so it's directly visible as a Windows file):

```bash
wsl.exe -d Ubuntu -- bash -lc "cp /tmp/demo-linux.png /mnt/d/Dev/Projects/TTUI/.claude/worktrees/cross-platform-verification/demo-linux.png"
```

`Read` `demo-linux.png` (from the Windows-visible path). Expected:
renders the same nested-panes layout every prior capture of `demo`
this project has ever produced.

- [ ] **Step 2: `control_panel` — mouse click interaction**

Reuses the exact script this project's Control Panel Arc already
proved out on Windows:

```bash
wsl.exe -d Ubuntu -- bash -lc "cat > /tmp/cp-script.json << 'EOF'
[{\"x\": 50, \"y\": 6}, {\"wait_ms\": 200}, {\"x\": 16, \"y\": 16}, {\"wait_ms\": 100}, {\"x\": 50, \"y\": 25}, {\"wait_ms\": 100}]
EOF
source \$HOME/.cargo/env && cd /mnt/d/Dev/Projects/TTUI/.claude/worktrees/cross-platform-verification && cargo run -p visual-snapshot -- --example control_panel --size 100x30 --script /tmp/cp-script.json --out /tmp/control-panel-linux.gif && cp /tmp/control-panel-linux.gif /mnt/d/Dev/Projects/TTUI/.claude/worktrees/cross-platform-verification/control-panel-linux.gif"
```

`Read` `control-panel-linux.gif`. Expected: the LAUNCH button's
particle burst, the toggle flip, and the dial advance all render
identically to the captures from Control Panel's own final
verification (PR #105) — this is the first time this exact click-
scripting path has ever run through `portable-pty`'s real Unix PTY
backend instead of Windows ConPTY.

- [ ] **Step 3: `falcon` — focus switching and the glitch trigger**

```bash
wsl.exe -d Ubuntu -- bash -lc "cat > /tmp/falcon-script.json << 'EOF'
[{\"key\": \"Tab\"}, {\"wait_ms\": 300}, {\"key\": \"Up\"}, {\"key\": \"Up\"}, {\"key\": \"Down\"}, {\"key\": \"Down\"}, {\"wait_ms\": 200}]
EOF
source \$HOME/.cargo/env && cd /mnt/d/Dev/Projects/TTUI/.claude/worktrees/cross-platform-verification && cargo run -p visual-snapshot -- --example falcon --size 100x30 --script /tmp/falcon-script.json --out /tmp/falcon-linux.gif && cp /tmp/falcon-linux.gif /mnt/d/Dev/Projects/TTUI/.claude/worktrees/cross-platform-verification/falcon-linux.gif"
```

(`Tab` = `FalconAction::FocusNext`, cycling the HUD's focused panel;
`Up,Up,Down,Down` = the `FullPower` chord, triggering all 3
`GlitchBuffer`s at once — both bindings confirmed in
`examples/falcon/input.rs`.) `Read` `falcon-linux.gif`. Expected: HUD
focus visibly changes after the `Tab` frame, and glitch noise overlays
appear after the chord completes, matching this project's prior
Windows captures of the same interactions.

- [ ] **Step 4: `mission_control` — tick-driven data-viz animation**

```bash
wsl.exe -d Ubuntu -- bash -lc "cat > /tmp/mc-script.json << 'EOF'
[{\"wait_ms\": 300}, {\"wait_ms\": 300}]
EOF
source \$HOME/.cargo/env && cd /mnt/d/Dev/Projects/TTUI/.claude/worktrees/cross-platform-verification && cargo run -p visual-snapshot -- --example mission_control --size 100x30 --script /tmp/mc-script.json --out /tmp/mission-control-linux.gif && cp /tmp/mission-control-linux.gif /mnt/d/Dev/Projects/TTUI/.claude/worktrees/cross-platform-verification/mission-control-linux.gif"
```

`Read` `mission-control-linux.gif`. Expected: the sparklines and bar
chart show visibly different values across the 3 frames (proving the
tick-driven random walk actually advances under WSL's timing, not just
that a static frame renders).

- [ ] **Step 5: `tardis` — buffer layering / boot transition**

```bash
wsl.exe -d Ubuntu -- bash -lc "cat > /tmp/tardis-script.json << 'EOF'
[{\"wait_ms\": 300}, {\"wait_ms\": 300}, {\"wait_ms\": 300}]
EOF
source \$HOME/.cargo/env && cd /mnt/d/Dev/Projects/TTUI/.claude/worktrees/cross-platform-verification && cargo run -p visual-snapshot -- --example tardis --size 100x30 --script /tmp/tardis-script.json --out /tmp/tardis-linux.gif && cp /tmp/tardis-linux.gif /mnt/d/Dev/Projects/TTUI/.claude/worktrees/cross-platform-verification/tardis-linux.gif"
```

`Read` `tardis-linux.gif`. Expected: the boot/materialization sequence
progresses visibly across frames, matching prior Windows captures.

- [ ] **Step 6: `launcher` — cross-app composition, idle nexus view**

```bash
wsl.exe -d Ubuntu -- bash -lc "echo '[]' > /tmp/empty2.json && source \$HOME/.cargo/env && cd /mnt/d/Dev/Projects/TTUI/.claude/worktrees/cross-platform-verification && cargo run -p visual-snapshot -- --example launcher --size 100x30 --script /tmp/empty2.json --out /tmp/launcher-linux.png && cp /tmp/launcher-linux.png /mnt/d/Dev/Projects/TTUI/.claude/worktrees/cross-platform-verification/launcher-linux.png"
```

`Read` `launcher-linux.png`. Expected: the portal-nexus view renders
with all 3 app portals visible, matching prior Windows captures.

- [ ] **Step 7: Handle the known glyph-coverage limitation, if hit**

Per `.claude/rules/development-conventions.md`'s "Visual review"
section: if any capture in Steps 1-6 hard-errors naming an unmapped
glyph, that is a **known, already-tracked limitation** (font8x8's
coverage gaps), not itself a new cross-platform finding — note which
capture hit it and move on to the remaining captures, per the existing
convention (this is not a reason to block the rest of this task).

- [ ] **Step 8: Record the result**

No commit — the 6 PNG/GIF files land in this worktree's root
(gitignored by default patterns for build artifacts; if `git status`
shows them as untracked and not ignored, do not commit them — they're
scratch verification output, not part of the repo). Note in this
task's completion record: which of the 6 captures rendered correctly,
which (if any) hit the known glyph-coverage limitation, and which (if
any) showed a genuine, new discrepancy from the Windows-observed
behavior. Any genuine discrepancy is a finding — file it per this
plan's Global Constraints.

---

### Task 4: Human-only checklist

**Files:** none.

**Interfaces:** none — this task is a message to the user, not
something executed by whoever is running this plan.

- [ ] **Step 1: Present the checklist to the user**

Deliver this exact checklist (the numbered steps below are the literal
content to hand the user — do not paraphrase or shorten them):

1. Open a real WSL Ubuntu terminal window — Windows Terminal's WSL
   profile, or run `wsl` directly from PowerShell/cmd/Windows Terminal.
   Not through Claude Code — this needs a real interactive terminal.
2. `cd /mnt/d/Dev/Projects/TTUI/.claude/worktrees/cross-platform-verification`
3. `cargo run --example control_panel`. Click the LAUNCH button and
   confirm the particle burst renders. Click a toggle switch and
   confirm it flips. Click the mode dial and confirm it advances.
   Press `q` to quit. Confirm the terminal returns to a normal shell
   prompt — cursor visible, no leftover alternate-screen artifacts.
   Type any command (e.g. `ls`) afterward and confirm it echoes and
   behaves normally (proves raw mode was actually restored, not just
   that the screen looks normal).
4. `cargo run --example demo`. Press `Tab` a few times, then `Up`/
   `Down` a few times. Confirm focus switching and list navigation
   feel instant, with no visible lag. Press `q` to quit and confirm
   the terminal restores cleanly again, same check as step 3.
5. Report back here: did everything behave identically to the Windows
   experience? If anything looked wrong, stuck, garbled, or the
   terminal didn't restore cleanly, describe exactly what happened and
   which step it happened on.

- [ ] **Step 2: Wait for the user's report**

Do not mark this task complete until the user has actually responded
with their observations — this step cannot be completed by whoever is
executing this plan alone.

- [ ] **Step 3: Record the result**

No commit. Note the user's reported outcome verbatim in this task's
completion record. If they report any problem, that's a finding — file
it per this plan's Global Constraints.

## Final verification (whole plan)

- [ ] Task 1: `rustup`/`gh` installed and version-confirmed inside WSL.
- [ ] Task 2: all 4 gate-suite commands run for real inside WSL, results
      recorded (matching or explaining any difference from the Windows
      baseline).
- [ ] Task 3: all 6 captures attempted, results recorded (rendered
      correctly / hit the known glyph limitation / genuine
      discrepancy).
- [ ] Task 4: the user's checklist report was actually received and
      recorded, not skipped.
- [ ] Every genuine finding from any task is filed as a GitHub issue
      with the correct `semver:*`/`v1-blocking` labels per
      `code-forge.md`'s rule.
- [ ] If zero findings resulted, that's also recorded plainly — "cross-
      platform verification passed, no gaps found" is a valid and
      useful outcome, not a reason to skip a final summary.
