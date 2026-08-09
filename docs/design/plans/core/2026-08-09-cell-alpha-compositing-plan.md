# Cell Alpha Compositing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give `Cell` a persistent `alpha: f32` field and rewrite `LayerStack::composite()` to do real top-to-bottom Porter-Duff "over" compositing instead of a hard cutout — designed so every existing app (all cells at `alpha: 1.0` post-migration) renders byte-identically to today.

**Architecture:** The algorithm change is entirely in `src/buffer.rs`. Everything else in this plan is migration: every `Cell { ... }` construction site in the workspace needs an explicit `alpha: 1.0` (never inherited from `..Default::default()`, since `Cell::default().alpha` must stay `0.0` to preserve `Buffer::new()`'s "fresh buffer is fully transparent" invariant — see the design spec's Context section for why this direction is non-negotiable). `diff()`/`CellDiff`/`render_diff`/`src/terminal.rs` are untouched — alpha is a compositing-time-only concept.

**Tech Stack:** Rust, existing `ttui` core (`buffer`, `easing`).

## Global Constraints

- **Tag: `coding`. Full TDD applies to every task — no exceptions.**
- `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` are hard gates on every task.
- **The one fact every task in this plan depends on:** `Cell::default().alpha == 0.0`. Any `Cell { ..., ..Default::default() }` (or any partial spread) construction site that is supposed to represent a *real, visible, painted* cell must have `alpha: 1.0` written explicitly in that literal — never left to the spread. Getting this wrong doesn't produce a compile error; it produces an invisible cell. This is why Tasks 2 and 3 exist as separate, carefully-scoped tasks rather than "run cargo build and fix errors" alone.
- One worktree for this whole Arc, created via `superpowers:using-git-worktrees` before Task 1, per `.claude/rules/git-github-standards.md`.
- `coding`-tagged → **Gated** autonomy tier: ships as a PR to `main` with all four required checks green, squash-merged at the end.
- Spec being implemented: `docs/design/specs/core/2026-08-09-cell-alpha-compositing-design.md`.

---

### Task 1: `Cell.alpha` + real `composite()` — the algorithm, in `src/buffer.rs` only

**Files:**
- Modify: `src/buffer.rs`

**Interfaces:**
- Consumes: `crate::easing::lerp_color` (existing, unchanged signature: `fn lerp_color(from: Color, to: Color, t: f32) -> Color`).
- Produces: `Cell.alpha: f32` (default `0.0`), `LayerStack::composite()`'s new blended behavior — every other task in this plan depends on this field existing with this exact default.

This task alone will leave the rest of the workspace failing to compile (every `Cell { ... }` literal outside this file is now missing a field) — that's expected. Do not touch any file outside `src/buffer.rs` in this task.

- [ ] **Step 1: Write the failing tests**

Add to `src/buffer.rs`'s `#[cfg(test)] mod tests`, after the existing `bold_dim_and_normal_are_pairwise_distinct` test:

```rust
    #[test]
    fn cell_default_alpha_is_zero() {
        assert_eq!(Cell::default().alpha, 0.0);
    }

    #[test]
    fn composite_blends_partial_alpha_between_two_layers() {
        let mut stack = LayerStack::new(1, 1);
        let base = Cell {
            symbol: 'a',
            fg: Color::Rgb { r: 0, g: 0, b: 0 },
            bg: Color::Reset,
            alpha: 1.0,
            ..Default::default()
        };
        stack.set(0, 0, base);
        let top = Cell {
            symbol: 'b',
            fg: Color::Rgb {
                r: 200,
                g: 100,
                b: 50,
            },
            bg: Color::Reset,
            alpha: 0.5,
            ..Default::default()
        };
        stack.push_layer().set(0, 0, top);

        let out = stack.composite();

        // top's contribution = 0.5 * remaining(1.0) = 0.5; base's is the
        // other 0.5 (remaining after top). Exact midpoint.
        assert_eq!(
            out.get(0, 0).fg,
            Color::Rgb {
                r: 100,
                g: 50,
                b: 25
            }
        );
        assert_eq!(out.get(0, 0).symbol, 'b'); // top's contribution (0.5) meets the >= 0.5 threshold
        assert_eq!(out.get(0, 0).alpha, 1.0);
    }

    #[test]
    fn composite_accumulates_correctly_across_three_partially_transparent_layers() {
        let mut stack = LayerStack::new(1, 1);
        let bottom_fg = Color::Rgb { r: 0, g: 0, b: 0 };
        let mid_fg = Color::Rgb {
            r: 100,
            g: 100,
            b: 100,
        };
        let top_fg = Color::Rgb {
            r: 200,
            g: 200,
            b: 200,
        };
        stack.set(
            0,
            0,
            Cell {
                symbol: 'a',
                fg: bottom_fg,
                bg: Color::Reset,
                alpha: 1.0,
                ..Default::default()
            },
        );
        stack.push_layer().set(
            0,
            0,
            Cell {
                symbol: 'b',
                fg: mid_fg,
                bg: Color::Reset,
                alpha: 0.5,
                ..Default::default()
            },
        );
        stack.push_layer().set(
            0,
            0,
            Cell {
                symbol: 'c',
                fg: top_fg,
                bg: Color::Reset,
                alpha: 0.5,
                ..Default::default()
            },
        );

        let out = stack.composite();

        // Hand-verified accumulation, top to bottom: top ('c') contributes
        // 0.5*1.0=0.5 (remaining -> 0.5); mid ('b') then contributes
        // 0.5*0.5=0.25 (remaining -> 0.25); bottom ('a', fully opaque)
        // claims the last 0.25. Expected fg is computed via the exact same
        // incremental pairwise-lerp steps the implementation performs (not
        // a closed-form average — each step truncates to u8 independently,
        // same as the real algorithm), so this is the algorithm's own
        // formula used as its test oracle, not an independently-derived
        // magic number.
        let expected_fg = {
            let after_mid = lerp_color(top_fg, mid_fg, 0.25 / 0.75); // mid's contribution / total-so-far
            lerp_color(after_mid, bottom_fg, 0.25 / 1.0) // bottom's contribution / total-so-far
        };
        assert_eq!(out.get(0, 0).fg, expected_fg);
        assert_eq!(out.get(0, 0).symbol, 'c'); // topmost to cross the 0.5 threshold
    }

    #[test]
    fn a_fully_opaque_layer_occludes_everything_beneath_it() {
        let mut stack = LayerStack::new(1, 1);
        // Bottom layer's color would show up in the result if (incorrectly) blended in.
        stack.set(
            0,
            0,
            Cell {
                symbol: 'z',
                fg: Color::Rgb { r: 255, g: 0, b: 0 },
                bg: Color::Reset,
                alpha: 1.0,
                ..Default::default()
            },
        );
        stack.push_layer().set(
            0,
            0,
            Cell {
                symbol: 'y',
                fg: Color::Rgb { r: 0, g: 255, b: 0 },
                bg: Color::Reset,
                alpha: 1.0,
                ..Default::default()
            },
        );

        let out = stack.composite();

        assert_eq!(out.get(0, 0).symbol, 'y');
        assert_eq!(out.get(0, 0).fg, Color::Rgb { r: 0, g: 255, b: 0 });
    }

    #[test]
    fn non_rgb_colors_fall_back_to_the_lerp_color_target_not_a_true_blend() {
        let mut stack = LayerStack::new(1, 1);
        stack.set(
            0,
            0,
            Cell {
                symbol: 'a',
                fg: Color::Green,
                bg: Color::Reset,
                alpha: 1.0,
                ..Default::default()
            },
        );
        stack.push_layer().set(
            0,
            0,
            Cell {
                symbol: 'b',
                fg: Color::Yellow,
                bg: Color::Reset,
                alpha: 0.5,
                ..Default::default()
            },
        );

        let out = stack.composite();

        // Neither Green nor Yellow is Color::Rgb. The accumulator seeds to
        // Yellow (top layer), then blends against Green (bottom) via
        // lerp_color, which falls back to returning its `to` argument
        // outright for non-Rgb pairs (per easing.rs) — so the result is
        // Green exactly, not a true yellow/green mix. This is a known,
        // pre-existing lerp_color limitation this spec does not attempt to
        // fix (see the design doc's Non-goals) — this test documents it,
        // not hides it.
        assert_eq!(out.get(0, 0).fg, Color::Green);
    }

    #[test]
    fn glyph_selection_uses_the_first_layer_to_reach_half_contribution() {
        let mut stack = LayerStack::new(1, 1);
        stack.set(
            0,
            0,
            Cell {
                symbol: 'a',
                fg: Color::Reset,
                bg: Color::Reset,
                alpha: 1.0,
                ..Default::default()
            },
        );
        stack.push_layer().set(
            0,
            0,
            Cell {
                symbol: 'b',
                fg: Color::Reset,
                bg: Color::Reset,
                alpha: 0.5,
                ..Default::default()
            },
        );

        let out = stack.composite();
        // top layer's contribution is exactly 0.5 * 1.0 = 0.5, which meets
        // (not just exceeds) the >= 0.5 threshold.
        assert_eq!(out.get(0, 0).symbol, 'b');
    }

    #[test]
    fn glyph_selection_falls_back_to_the_topmost_contributor_when_none_reach_half() {
        let mut stack = LayerStack::new(1, 1);
        stack.set(
            0,
            0,
            Cell {
                symbol: 'a',
                fg: Color::Reset,
                bg: Color::Reset,
                alpha: 0.3,
                ..Default::default()
            },
        );
        stack.push_layer().set(
            0,
            0,
            Cell {
                symbol: 'b',
                fg: Color::Reset,
                bg: Color::Reset,
                alpha: 0.3,
                ..Default::default()
            },
        );

        let out = stack.composite();
        // top ('b') contributes 0.3*1.0=0.3; bottom ('a') then contributes
        // 0.3*0.7=0.21 — neither reaches the 0.5 threshold individually, so
        // the rule falls back to "topmost non-transparent contributor", 'b'.
        assert_eq!(out.get(0, 0).symbol, 'b');
    }
```

Also update every existing `Cell { ... }` literal already in this file's test module (lines ~272-448 as of this writing — search for `..Default::default()` inside `mod tests` to find them all) to add `alpha: 1.0,` before their `..Default::default()` line. There are 11 such sites in the existing test module. This is required for those tests to keep passing once `alpha` exists — do this as part of Step 1, since the RED phase in Step 2 needs the whole file to at least parse/typecheck against the *new* tests, and these existing sites will otherwise also break once Step 3 adds the field.

- [ ] **Step 2: Run tests to verify the new ones fail**

Run: `cargo test --lib buffer::tests`
Expected: FAIL to compile — `Cell` has no `alpha` field yet.

- [ ] **Step 3: Add `Cell.alpha` and rewrite `Default`**

Change:

```rust
/// One terminal character cell: glyph, foreground/background color,
/// and style.
#[derive(Clone, PartialEq, Debug)]
pub struct Cell {
    /// The glyph to render.
    pub symbol: char,
    /// Foreground (text) color.
    pub fg: Color,
    /// Background color.
    pub bg: Color,
    /// Bold/etc. styling.
    pub style: CellStyle,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            symbol: ' ',
            fg: Color::Reset,
            bg: Color::Reset,
            style: CellStyle::default(),
        }
    }
}
```

to:

```rust
/// One terminal character cell: glyph, foreground/background color,
/// style, and coverage.
#[derive(Clone, PartialEq, Debug)]
pub struct Cell {
    /// The glyph to render.
    pub symbol: char,
    /// Foreground (text) color.
    pub fg: Color,
    /// Background color.
    pub bg: Color,
    /// Bold/etc. styling.
    pub style: CellStyle,
    /// How much this cell covers whatever is beneath it during
    /// `LayerStack::composite()` — `0.0` fully transparent, `1.0`
    /// fully opaque. Meaningless once a `Buffer` has been composited
    /// and is headed for `diff`/the terminal; every cell leaving
    /// `composite()` is either untouched (`0.0`, stays default) or
    /// real content (`1.0`).
    pub alpha: f32,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            symbol: ' ',
            fg: Color::Reset,
            bg: Color::Reset,
            style: CellStyle::default(),
            alpha: 0.0,
        }
    }
}
```

- [ ] **Step 4: Rewrite `composite()`**

Change:

```rust
    pub fn composite(&self) -> Buffer {
        if self.layers.len() == 1 {
            return self.layers[0].clone();
        }
        let width = self.layers[0].width;
        let height = self.layers[0].height;
        let mut out = Buffer::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let mut cell = Cell::default();
                for layer in self.layers.iter().rev() {
                    let c = layer.get(x, y);
                    if *c != Cell::default() {
                        cell = c.clone();
                        break;
                    }
                }
                out.set(x, y, cell);
            }
        }
        out
    }
```

to:

```rust
    pub fn composite(&self) -> Buffer {
        if self.layers.len() == 1 {
            return self.layers[0].clone();
        }
        let width = self.layers[0].width;
        let height = self.layers[0].height;
        let mut out = Buffer::new(width, height);
        for y in 0..height {
            for x in 0..width {
                out.set(x, y, composite_cell(&self.layers, x, y));
            }
        }
        out
    }
```

Add this free function after the `impl LayerStack` block (before `impl std::ops::Deref for LayerStack`):

```rust
// Top-to-bottom Porter-Duff "over" accumulation. `remaining` tracks how
// much of this pixel is still undecided; each layer claims
// `alpha * remaining` of it, and `remaining` shrinks by `(1 - alpha)`.
// When every cell involved has alpha 1.0 (true for every existing app
// post-migration), the first non-transparent layer claims 100% on
// contact and the loop breaks immediately — byte-identical to the old
// "topmost non-default cell wins" scan, for the same reason (early exit
// on full coverage).
fn composite_cell(layers: &[Buffer], x: u16, y: u16) -> Cell {
    let mut remaining = 1.0_f32;
    let mut acc_weight = 0.0_f32;
    let mut acc_fg = Color::Reset;
    let mut acc_bg = Color::Reset;
    let mut winner: Option<(char, CellStyle)> = None;
    let mut first: Option<(char, CellStyle)> = None;

    for layer in layers.iter().rev() {
        if remaining <= 0.0 {
            break;
        }
        let cell = layer.get(x, y);
        if cell.alpha <= 0.0 {
            continue;
        }
        let contribution = cell.alpha * remaining;

        if first.is_none() {
            first = Some((cell.symbol, cell.style));
        }
        if winner.is_none() && contribution >= 0.5 {
            winner = Some((cell.symbol, cell.style));
        }

        acc_fg = if acc_weight <= 0.0 {
            cell.fg
        } else {
            crate::easing::lerp_color(acc_fg, cell.fg, contribution / (acc_weight + contribution))
        };
        acc_bg = if acc_weight <= 0.0 {
            cell.bg
        } else {
            crate::easing::lerp_color(acc_bg, cell.bg, contribution / (acc_weight + contribution))
        };
        acc_weight += contribution;

        remaining *= 1.0 - cell.alpha;
    }

    match winner.or(first) {
        None => Cell::default(),
        Some((symbol, style)) => Cell {
            symbol,
            fg: acc_fg,
            bg: acc_bg,
            style,
            alpha: 1.0,
        },
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --lib buffer::tests`
Expected: PASS — all new tests from Step 1, plus every pre-existing `LayerStack`/`composite` test in this file (now migrated to `alpha: 1.0`) unchanged in their assertions. This is the concrete proof the new algorithm is byte-identical to the old one whenever every cell is opaque.

- [ ] **Step 6: Run clippy and fmt**

Run: `cargo clippy --lib -- -D warnings` and `cargo fmt --check -- src/buffer.rs`
Expected: both clean. (The rest of the workspace will still fail to build — expected, per this task's scope note above.)

- [ ] **Step 7: Commit**

```bash
git add src/buffer.rs
git commit -m "feat(core): give Cell a persistent alpha field, real composite() blending

LayerStack::composite() was a hard cutout with no color math. Replaces
it with top-to-bottom Porter-Duff 'over' accumulation, designed so
alpha:1.0 (every existing cell, post-migration) degenerates exactly to
the old 'topmost wins, early exit' behavior — proven by every
pre-existing composite test passing unchanged. Closes the one lever
the rendering-fidelity spike explicitly deferred."
```

---

### Task 2: Fix every spread-based `Cell` construction site (the dangerous ones)

**Files:** — the exact list below, 73 sites across 25 files.

**Interfaces:**
- Consumes: `Cell.alpha` (Task 1).
- Produces: nothing new — this task's only job is closing the silent-invisible-cell risk the design spec flags at length.

**Why this task exists separately from Task 3:** every site below uses `..Default::default()` (or an equivalent partial spread) to fill in `Cell`'s unlisted fields. Rust's compiler does **not** flag these as errors once `alpha` is added — they compile cleanly and silently produce `alpha: 0.0` (invisible) cells unless fixed. This is the one class of mistake in this whole plan that doesn't announce itself; Task 3 (the exhaustive-literal sites) gets caught by the compiler for free, this doesn't.

**The fix is identical at every location:** insert a new line `alpha: 1.0,` immediately before the `..Default::default()` line. Do this at every location below, in every listed file. No other changes.

- [ ] **Step 1: Apply the fix at every listed location**

`src/camera.rs`: lines 182-183, 204-205 (2 sites)

`src/canvas.rs`: lines 248-249, 265-266, 283-284, 301-302, 314-315, 353-354, 387-388 (7 sites)

`src/effects.rs`: lines 33-34, 49-50, 66-67, 83-84 (4 sites)

`src/glitch.rs`: lines 63-64 (1 site)

`src/particles.rs`: lines 79-80, 152-153, 184-185, 273-274 (4 sites)

`src/widgets/damage_meter.rs`: lines 47-48 (1 site)

`src/widgets/dna_console.rs`: lines 46-47, 59-60 (2 sites)

`src/widgets/energy_core.rs`: lines 53-54 (1 site)

`src/widgets/roundel.rs`: lines 48-49 (1 site)

`src/widgets/smash_border.rs`: lines 45-46, 55-56, 67-68, 77-78, 88-89, 98-99, 108-109, 118-119 (8 sites)

`src/widgets/text.rs`: lines 36-37 (1 site)

`examples/omnitrix/boot.rs`: lines 13-14, 32-33 (2 sites)

`examples/omnitrix/fasttrack.rs`: lines 41-42 (1 site)

`examples/omnitrix/omnitrix.rs`: lines 184-185, 194-195, 208-209, 218-219, 236-237, 257-258 (6 sites)

`examples/omnitrix/upgrade.rs`: lines 26-27, 38-39 (2 sites)

`examples/smash_crabs/boot.rs`: lines 29-30, 67-68, 93-94, 141-142 (4 sites)

`examples/smash_crabs/smash_crabs.rs`: lines 264-265, 320-321, 364-365, 393-394 (4 sites)

`examples/smash_crabs/stage_hazards.rs`: lines 76-77 (1 site)

`examples/smash_crabs/target_smash.rs`: lines 72-73 (1 site)

`examples/smash_crabs/versus.rs`: lines 34-35 (1 site)

`examples/tardis/boot.rs`: lines 31-32, 49-50 (2 sites)

`examples/tardis/psychic_paper.rs`: lines 13-14 (1 site)

`examples/tardis/star_charts.rs`: lines 15-16, 58-59, 75-76 (3 sites)

`examples/tardis/tardis.rs`: lines 333-334, 381-382 (2 sites)

For every one of these, the pattern being edited looks like this concrete example (from `src/widgets/text.rs:33-37`):

```rust
                Cell {
                    symbol: ch,
                    fg: self.fg,
                    bg: self.bg,
                    ..Default::default()
                },
```

becomes:

```rust
                Cell {
                    symbol: ch,
                    fg: self.fg,
                    bg: self.bg,
                    alpha: 1.0,
                    ..Default::default()
                },
```

Every other site follows the identical shape — same insertion, one line, immediately before `..Default::default()`, regardless of what the surrounding `symbol`/`fg`/`bg` values happen to be at that location.

- [ ] **Step 2: Self-verify no site was missed**

Run this exact search yourself and confirm it returns **zero matches** — every `bg:`-then-spread pattern from the list above must now have `alpha: 1.0,` between them:

```bash
grep -rzoP 'bg: [^,\n]+,\s*\n\s*\.\.Default::default\(\)' --include="*.rs" .
```

If your shell's `grep` doesn't support `-P` (Perl regex), search manually file-by-file from the list above instead — the point is confirming zero remaining `bg: ...,` immediately followed by `..Default::default()` with nothing in between, anywhere in the workspace.

Also grep for any `Cell { ... ..Default::default() }` pattern this plan's list might have missed (e.g. a site where `style:` rather than `bg:` is the field immediately preceding the spread) — search for `..Default::default()` in every file NOT already covered above and manually confirm each remaining hit is spreading into a `CellStyle`/`Theme`/other struct, not directly into a `Cell`. Report anything ambiguous rather than guessing.

- [ ] **Step 3: Build (expect it to still fail — that's Task 3's job)**

Run: `cargo build --all-targets`
Expected: still fails, but only with "missing field `alpha`" errors on *exhaustive* `Cell { ... }` literals (no `..` spread) — every error remaining at this point is the safe, compiler-caught kind Task 3 fixes next. If you see any other kind of error (e.g. a type mismatch), stop and report it rather than guessing at a fix.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "fix(core): add explicit alpha: 1.0 to every spread-based Cell literal

Cell::default().alpha is 0.0 (required so Buffer::new()'s fresh cells
stay transparent) — any Cell{..., ..Default::default()} construction
site that's actually painting real content needs alpha set explicitly,
or it silently becomes invisible. This is every such site in the
workspace as of this plan; Task 3 handles the exhaustive-literal sites
the compiler already catches on its own."
```

---

### Task 3: Fix every remaining (compiler-caught) `Cell` construction site

**Files:** whatever `cargo build --all-targets` reports after Task 2 — every exhaustive `Cell { symbol, fg, bg, style }` literal with no `..` spread, which the compiler already flags as "missing field `alpha`" with an exact file:line.

**Interfaces:**
- Consumes: `Cell.alpha` (Task 1).
- Produces: a fully-compiling workspace.

This task is mechanical and safe by construction — the compiler enumerates every remaining site for you, one error at a time, and nothing here can silently compile wrong (an exhaustive literal missing a field is always a hard error until fixed).

- [ ] **Step 1: Iteratively fix every compiler error**

Run: `cargo build --all-targets 2>&1 | head -50`

For each "missing field `alpha` in initializer of `ttui::buffer::Cell`" error, open the reported file:line and add `alpha: 1.0,` to that literal (as a new field in the exhaustive list — these literals name every field explicitly, so add it alongside `symbol`/`fg`/`bg`/`style` in whatever position reads cleanly, typically last). Repeat — build, fix the next batch of errors, build again — until `cargo build --all-targets` reports zero errors related to `Cell`.

**Every site fixed in this task must use `alpha: 1.0` specifically** — this task exists purely to make painted, visible cells (which is what every exhaustive literal in this codebase represents) fully opaque, matching pre-migration behavior exactly. There is no case in this task where a different value is correct; if you find a literal where `1.0` seems wrong, stop and report it rather than guessing — that would mean this plan's premise (every existing cell is opaque) missed a real case.

- [ ] **Step 2: Confirm the whole workspace compiles**

Run: `cargo build --all-targets`
Expected: succeeds — this is the first point since Task 1 that the whole workspace compiles, and (combined with Task 2's explicit spread handling) the first point where it's both compiling *and* correct.

- [ ] **Step 3: Run the full test suite**

Run: `cargo test`
Expected: full suite green, including every pre-existing test across every widget and example that constructs a `Cell` and checks its fields.

- [ ] **Step 4: Run clippy and fmt**

Run: `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
Expected: both clean.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "fix(core): add alpha: 1.0 to every remaining (exhaustive) Cell literal

Compiler-guided — every site here was a hard 'missing field' error
until fixed, so there's no silent-failure risk in this batch, unlike
Task 2's spread-based sites."
```

---

### Task 4: Final workspace verification

**Files:** none (verification only).

- [ ] **Step 1: Full test suite**

Run: `cargo test`
Expected: full suite green, including every existing and new test across Tasks 1-3.

- [ ] **Step 2: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
Expected: both clean.

- [ ] **Step 3: Build every target**

Run: `cargo build --all-targets`
Expected: succeeds.

- [ ] **Step 4: Manual visual regression check**

Run `cargo run --example omnitrix` / `tardis` / `smash_crabs` / `launcher` if you have a way to do so in this environment; if not (no interactive PTY), confirm each launches into its raw-mode event loop with no startup panic, and note that the actual visual-regression guarantee for this task rests on Task 1's byte-identical-composite proof (every pre-existing `composite()` test passing unchanged) plus Tasks 2-3's alpha:1.0 migration — not on a live look, which this plan cannot perform in this environment. Say so explicitly in your report rather than implying a live check happened.

- [ ] **Step 5: Commit (if Step 4 required any fix) or proceed**

If Step 4 surfaces no issues, there is nothing to commit for this task.

---

## Final verification (whole plan)

- [ ] `cargo test` — full suite green.
- [ ] `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` — clean.
- [ ] `cargo build --all-targets` — library, examples, benches all compile.
- [ ] Every pre-existing `LayerStack`/`composite` test in `src/buffer.rs` passes with unchanged assertions (the concrete "byte-identical to before" proof).
- [ ] Per `.claude/rules/git-github-standards.md`: open a PR from this Arc's worktree branch to `main`, wait for all four required checks green, squash-merge, then remove the worktree via `ExitWorktree`.
