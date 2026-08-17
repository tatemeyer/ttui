# Future of TTUI — Session Briefing

**Purpose:** context for a separate, high-level brainstorming session
about where TTUI goes after v1.0.0.

**This document deliberately reaches no conclusions.** It records what is
true as of 2026-08-16 and what is genuinely open. It does not recommend a
direction, rank the questions, or pre-answer anything — the point is for
that session's brainstorm to be real rather than a rubber-stamp of one
already taken. Where a tension is noted, it is stated as an observation
with the evidence attached, not as a problem with an implied fix.

**Read first:** `CLAUDE.md`, `docs/design/README.md`, and the four files
in `.claude/rules/`. This briefing assumes but does not restate them.

---

## 1. Where v1.0.0 landed

TTUI shipped. Not "is nearly ready" — shipped, to a public registry.

- Tagged `v1.0.0` on 2026-08-14; GitHub release published.
- **Published to crates.io** as `ttui` 1.0.0, MIT licensed.
- `CHANGELOG.md`'s `[Unreleased]` is empty. Nothing user-facing has landed
  since the tag.

That last point matters more than it looks. Every decision from here is
made under SemVer obligations to real downstream consumers, however few.
`.claude/rules/code-forge.md` defines exactly what counts as breaking; the
pre-v1 freedom to reshape the public API on discovery is gone.

### How it got there

The v1.0.0 initiative ran as six numbered sub-projects: release governance
(#1), a sweep audit (#2), a pre-v1 fix wave (#3), cross-platform Linux
verification (#4), the showcase demo reel (#5), and showcase polish
(#5.1). **Sub-project #4 never ran.** Its spec and a 27-task plan sit on
`main` with a worktree created and zero commits against it. The tag cut
without it, so v1.0.0 is on crates.io having never been interactively
exercised on Linux — CI's headless `ubuntu-latest` has been green
throughout, but `portable-pty`'s Unix backend has never met a real PTY.

**As of 2026-08-16 the user has explicitly deprioritized Linux** — "no
longer a major priority, we don't need to consider it until I reopen it."
It is recorded here as a known, deliberate gap, not as pending work.

---

## 2. What exists

**Core library** — `src/`, ~8,800 lines across 17 modules plus
`src/widgets/`. Rendering is buffer-based with a `LayerStack` doing
Porter-Duff "over" alpha compositing. Beyond the basics there is a
noticeably ambitious set of subsystems for a TUI library: a 3D perspective
camera (`perspective.rs`, `camera.rs`), a particle system, a canvas with
subpixel Braille rendering, glitch effects, easing/transitions, and audio.

**Widgets** — 16, plus `mod.rs`. Seven are general-purpose (`block`,
`list`, `table`, `text`, `bar_chart`, `sparkline`, `dial`). The other nine
are props built for one themed app each — `dna_console`, `roundel`,
`time_rotor`, `scuttle_cursor`, `smash_border`, `damage_meter`,
`energy_core`, `cockpit_panel`, `analog_toggle`. All of them are `pub` and
therefore inside the SemVer commitment.

**Examples** — ~5,900 lines. Seven themed apps (`omnitrix`, `tardis`,
`smash_crabs`, `launcher`, `falcon`, `mission_control`, `control_panel`),
two spikes (`depth_spike`, `render_spike`), and `demo.rs` (whose retirement
is tracked as #83). Plus `showcase`, a flagship demo reel run via its own
`[[bin]]`.

**Tooling** — `tools/visual-snapshot`, a PTY-driving screenshot/GIF
capture tool with a real-TTY test harness and an optional local
vision-model judge. It is internal dev tooling, explicitly outside the
SemVer policy.

**Verification** — TTUI is consumer #1 of **Plumb**, the perceptual-review
tool living in the Parallax repo (`D:/Dev/Projects/Parallax`). Five capture
scenarios exist. Plumb runs `tools/visual-snapshot` verbatim through a
`command` adapter and layers blinded, adversarial multi-lens review on
top, rendering GO / NO-GO / HOLD.

---

## 3. Where the work stands right now

**Open issues: 15.** All `semver:patch`; none `v1-blocking` (moot post-tag).
The mechanical backlog was cleared in PR #141; what remains needs judgment.

**The active Arc** is *Capture Quiescence Fidelity*
(`docs/design/specs/core/2026-08-16-capture-quiescence-fidelity-design.md`),
covering #139, #131, #127 and #138. Root cause: `tools/visual-snapshot`
decides a draw is finished by comparing `vt100::Screen::contents()` —
plain text only — while rasterizing full color state. The signal and the
artifact do not measure the same thing.

**Its stakes:** Parallax's first audited Plumb run returned **NO-GO** on
TTUI's `omnitrix-dial-rotate` scenario. The blocker its blinded critics
found — "frame 1 renders as an empty black panel" — is #139. Until it
clears, the one scenario audited end-to-end fails on a capture-tool
artifact rather than on TTUI's rendering.

**Also queued:** #30 (`on_tick` cannot trigger app exit — a stated one-line
remedy needing a test-first pass) and giving the Parallax repo a git remote
(it has none, so cross-repo findings currently land in TTUI's tracker by
default — that is why closed issue #140 lived here).

---

## 4. Open questions

Genuinely unresolved. Listed in no particular order, with evidence — not
ranked, and not paired with suggested answers.

### 4.1 What is TTUI actually for?

The README calls it "a terminal UI framework built from first
principles." The repository is also, measurably, a collection of themed
demo apps: ~5,900 lines of examples against ~8,800 of library, and nine of
sixteen widgets are props for exactly one app.

Both readings are supported by the evidence. The question is which one the
project intends, because they imply different answers to almost everything
below. Note that this is now a **published** API question, not just an
organizational one — every one of those app-specific widgets is `pub` and
under SemVer.

### 4.2 Who is the audience?

There is no stated target user. Solo creative projects? A ratatui
alternative? A vehicle for the author's own apps? Downloads sit in the low
double digits, so there is no usage signal to read this off — it is a
decision, not an observation. The project has never positioned itself
against `ratatui`, the dominant Rust TUI library, in any document in the
tree.

### 4.3 What does v1.1 or v2 contain?

`[Unreleased]` is empty. No roadmap exists past v1.0.0 — the six
sub-projects were scoped to reach the tag and stopped there. There is no
document describing what comes next at the feature level.

### 4.4 How much more framework, versus more apps?

The last several Arcs (showcase, showcase-polish, control-panel) were
example apps that incidentally proved out library features. Whether that
is the intended engine of development, or a pattern that has simply
persisted, has not been examined.

### 4.5 What is the relationship to Parallax?

Parallax is a five-sub-project platform binding TTUI, Model-Experiments,
and Plumb. TTUI is described there as a consumer and "the cockpit's first
genuine external consumer." Parallax currently has **no git remote** and
substantial in-flight work (a 120-task baseline plan, a 110-task Plumb
plan, a 42-task evidence-and-report plan, all with unchecked boxes). How
much of TTUI's future is subordinate to that platform's roadmap — versus
independent of it — is undefined.

### 4.6 Is the methodology still proportionate?

Every change goes through brainstorm → spec → plan → subagent execution →
PR → four green checks. It has produced an unusually well-documented
codebase and a real audit trail. It also produced four separate governance
specs before the first v1.0.0 fix landed. Whether that ratio should hold
post-v1, tighten, or relax is a live question the process itself has never
been turned on.

### 4.7 What happens to the deferred Linux work?

Deprioritized by explicit decision, with an approved 27-task plan and a
created worktree sitting idle. Whether that plan stays valid for whenever
it reopens, or should be closed and re-derived, is undecided.

---

## 5. Constraints that session should not relitigate

Decided, with rationale recorded in the tree. Treat as fixed unless the
brainstorm's whole purpose is to reopen one — in which case, read the
rationale first.

- **Rust**, per the core framework design (Rev A).
- **Windows-first.** Linux deferred by explicit decision; macOS out of
  scope.
- **Spec-and-plan-before-code**, per `CLAUDE.md`.
- **The Direct / Gated / Human autonomy tiers**, per
  `git-github-standards.md`.
- **`tools/visual-snapshot` is not replaced by Plumb.** A stated non-goal
  of the Plumb spec; TTUI adopts it via the `command` adapter and keeps
  the tool verbatim.
- **SemVer applies to the root `ttui` crate only**, per `code-forge.md`.

---

## 6. Traps

Things this repository has already paid for. Each cost real rework.

- **Exit code 0 is not evidence of success.** `.plumb/SCENARIOS.md` opens
  with this. Two of the first three "successful" captures exited clean
  while showing the wrong screen entirely. The only real check is reading
  the image and describing what is on it.
- **Documents can be confidently wrong.** Within this session: PR #136's
  description asserted a Plumb blocker that had already been fixed
  upstream, and an issue was filed from it (#140) without checking the
  current state. Separately, #139 was filed with a root cause that reading
  the code disproved. Verify against the code and a measurement, not
  against prose — including prose in this briefing.
- **Issues can already be fixed.** #137 and #140 were both closed as
  already-resolved during the same session that queued them. Check current
  state before planning work against an issue.
- **`Closes #1, #2, #3` only closes #1.** GitHub binds the keyword to the
  first number only. This bit PR #141; six issues had to be closed by hand.
- **A merged worktree still shows as unmerged.** The repo squash-merges,
  so every branch looks unmerged by commit count. Diff the content, not
  the commit graph, before concluding a branch has unshipped work.
