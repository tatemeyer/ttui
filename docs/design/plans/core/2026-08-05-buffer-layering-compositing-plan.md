# Buffer Layering / Compositing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let `Paint` write into an ordered stack of same-dimension `Buffer`s instead of one, and flatten that stack to a single `Buffer` immediately before `Diff` runs, via a new `LayerStack` type — with zero behavior change for apps that never use more than one layer.

**Architecture:** `LayerStack` (new, in `src/buffer.rs`) wraps `Vec<Buffer>` and `Deref`/`DerefMut`s to the base (index-0) layer, so existing single-buffer code keeps working unchanged through deref coercion. `App::view` takes `&mut LayerStack` instead of `&mut Buffer`; `app.rs::run()` calls `stack.composite()` right before `diff()`, which is otherwise untouched. Composite uses a discrete "last non-default cell in stack order wins" rule, with a fast path that skips scanning entirely when the stack has exactly one layer.

**Tech Stack:** Rust, `crossterm` (already the sole dependency; unchanged by this work).

## Global Constraints

- TDD is mandatory for every task below (all are `coding`-tagged; none of the four documented exceptions apply) — write the failing test before the implementation, per `.claude/rules/development-conventions.md`.
- Inline `#[cfg(test)] mod tests` is the test structure convention — no new `tests/` directory.
- `cargo fmt` / `cargo clippy` must stay clean throughout.
- No new dependencies — this is achievable with `std::ops::{Deref, DerefMut}` and existing `Buffer`/`Cell`.
- `Diff` (`src/buffer.rs::diff`) and `Terminal::draw_diff` (`src/terminal.rs`) are explicitly out of scope — do not modify them.

---

### Task 1: `LayerStack` construction — `new`, `push_layer`, `layer_mut`

**Files:**
- Modify: `src/buffer.rs` (add `LayerStack` struct + impl, after the `diff()` function and before the existing `#[cfg(test)] mod tests` block)
- Test: `src/buffer.rs` (inline, same `mod tests` block)

**Interfaces:**
- Consumes: `Buffer::new(width, height) -> Buffer` (existing), `Buffer::get`/`Buffer::set` (existing).
- Produces: `LayerStack::new(width: u16, height: u16) -> LayerStack`, `LayerStack::push_layer(&mut self) -> &mut Buffer`, `LayerStack::layer_mut(&mut self, index: usize) -> &mut Buffer`. Later tasks (2, 3, 4) build directly on these three methods.

- [ ] **Step 1: Write the failing tests**

Add to the existing `mod tests` block in `src/buffer.rs` (below `diff_of_identical_buffers_is_empty`):

```rust
    #[test]
    fn new_layer_stack_has_one_default_filled_base_layer() {
        let mut stack = LayerStack::new(3, 2);
        assert_eq!(*stack.layer_mut(0).get(0, 0), Cell::default());
        assert_eq!(*stack.layer_mut(0).get(2, 1), Cell::default());
    }

    #[test]
    fn push_layer_appends_a_same_dimension_default_filled_layer() {
        let mut stack = LayerStack::new(3, 2);
        let cell = Cell {
            symbol: 'x',
            fg: Color::Red,
            bg: Color::Reset,
        };
        stack.push_layer().set(1, 1, cell.clone());

        assert_eq!(*stack.layer_mut(1).get(1, 1), cell);
        assert_eq!(*stack.layer_mut(0).get(1, 1), Cell::default());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib buffer::tests`
Expected: FAIL to compile — `LayerStack` is not defined.

- [ ] **Step 3: Write minimal implementation**

Add to `src/buffer.rs`, after the `diff()` function and before `#[cfg(test)]`:

```rust
pub struct LayerStack {
    width: u16,
    height: u16,
    layers: Vec<Buffer>,
}

impl LayerStack {
    pub fn new(width: u16, height: u16) -> Self {
        LayerStack {
            width,
            height,
            layers: vec![Buffer::new(width, height)],
        }
    }

    pub fn push_layer(&mut self) -> &mut Buffer {
        self.layers.push(Buffer::new(self.width, self.height));
        self.layers.last_mut().unwrap()
    }

    pub fn layer_mut(&mut self, index: usize) -> &mut Buffer {
        &mut self.layers[index]
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib buffer::tests`
Expected: PASS (all buffer tests, including the two new ones).

- [ ] **Step 5: Commit**

```bash
git add src/buffer.rs
git commit -m "feat(buffer): add LayerStack with push_layer/layer_mut"
```

---

### Task 2: `Deref`/`DerefMut` for `LayerStack`

**Files:**
- Modify: `src/buffer.rs` (add trait impls, directly below the `LayerStack` impl block from Task 1)
- Test: `src/buffer.rs` (inline, same `mod tests` block)

**Interfaces:**
- Consumes: `LayerStack` (Task 1), `Buffer::get`/`Buffer::set` (existing).
- Produces: `LayerStack: Deref<Target = Buffer>` and `DerefMut`. This is what lets Task 4's `App::view(&mut LayerStack)` accept bodies written against `&mut Buffer` unchanged.

- [ ] **Step 1: Write the failing test**

Add to `mod tests` in `src/buffer.rs`:

```rust
    #[test]
    fn layer_stack_derefs_to_the_base_layer() {
        let mut stack = LayerStack::new(3, 2);
        let cell = Cell {
            symbol: 'y',
            fg: Color::Reset,
            bg: Color::Red,
        };
        stack.set(0, 1, cell.clone()); // DerefMut -> base layer, no layer_mut(0) needed

        assert_eq!(*stack.get(0, 1), cell); // Deref -> base layer
        assert_eq!(*stack.layer_mut(0).get(0, 1), cell); // same cell via explicit index
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib buffer::tests::layer_stack_derefs_to_the_base_layer`
Expected: FAIL to compile — no method named `set`/`get` found for `LayerStack`.

- [ ] **Step 3: Write minimal implementation**

Add to `src/buffer.rs`, directly below the `LayerStack` impl block:

```rust
impl std::ops::Deref for LayerStack {
    type Target = Buffer;
    fn deref(&self) -> &Buffer {
        &self.layers[0]
    }
}

impl std::ops::DerefMut for LayerStack {
    fn deref_mut(&mut self) -> &mut Buffer {
        &mut self.layers[0]
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib buffer::tests`
Expected: PASS (all buffer tests).

- [ ] **Step 5: Commit**

```bash
git add src/buffer.rs
git commit -m "feat(buffer): LayerStack derefs to its base layer"
```

---

### Task 3: `composite()` — depth-1 fast path and multi-layer scan

**Files:**
- Modify: `src/buffer.rs` (add `composite` method to the `LayerStack` impl block)
- Test: `src/buffer.rs` (inline, same `mod tests` block)

**Interfaces:**
- Consumes: `LayerStack` (Task 1), `Cell::default()`/`PartialEq` (existing), `Buffer::get`/`set` (existing).
- Produces: `LayerStack::composite(&self) -> Buffer`. Task 4's `app.rs::run()` calls this directly before `diff()`.

- [ ] **Step 1: Write the failing tests**

Add to `mod tests` in `src/buffer.rs`:

```rust
    #[test]
    fn composite_of_a_single_layer_stack_matches_that_layer() {
        let mut stack = LayerStack::new(2, 2);
        let cell = Cell {
            symbol: 'z',
            fg: Color::Green,
            bg: Color::Reset,
        };
        stack.set(1, 0, cell.clone());

        let out = stack.composite();

        assert_eq!(*out.get(1, 0), cell);
        assert_eq!(*out.get(0, 0), Cell::default());
        assert_eq!(*out.get(0, 1), Cell::default());
        assert_eq!(*out.get(1, 1), Cell::default());
    }

    #[test]
    fn composite_of_a_three_layer_stack_lets_topmost_non_default_cell_win() {
        let mut stack = LayerStack::new(3, 1);
        let a = Cell {
            symbol: 'a',
            fg: Color::Reset,
            bg: Color::Reset,
        };
        let b = Cell {
            symbol: 'b',
            fg: Color::Reset,
            bg: Color::Reset,
        };
        let c = Cell {
            symbol: 'c',
            fg: Color::Reset,
            bg: Color::Reset,
        };

        stack.set(0, 0, a.clone()); // base layer: 'a' at x=0
        stack.push_layer().set(1, 0, b.clone()); // layer 1: 'b' at x=1
        stack.push_layer().set(0, 0, c.clone()); // layer 2 (top): 'c' at x=0

        let out = stack.composite();

        assert_eq!(*out.get(0, 0), c); // layer 2's 'c' overwrites layer 0's 'a'
        assert_eq!(*out.get(1, 0), b); // layer 1's 'b' survives (layer 2 is default here)
        assert_eq!(*out.get(2, 0), Cell::default()); // every layer default here
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib buffer::tests`
Expected: FAIL to compile — no method named `composite` found for `LayerStack`.

- [ ] **Step 3: Write minimal implementation**

Add to the `LayerStack` impl block in `src/buffer.rs` (after `layer_mut`):

```rust
    pub fn composite(&self) -> Buffer {
        if self.layers.len() == 1 {
            return self.layers[0].clone();
        }
        let mut out = Buffer::new(self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                let mut cell = Cell::default();
                for layer in &self.layers {
                    let c = layer.get(x, y);
                    if *c != Cell::default() {
                        cell = c.clone();
                    }
                }
                out.set(x, y, cell);
            }
        }
        out
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib buffer::tests`
Expected: PASS (all buffer tests, including the four new `LayerStack` tests).

- [ ] **Step 5: Commit**

```bash
git add src/buffer.rs
git commit -m "feat(buffer): add LayerStack::composite with depth-1 fast path"
```

---

### Task 4: Wire `LayerStack` into `App` and `app.rs::run()`

**Files:**
- Modify: `src/app.rs:1` (import), `src/app.rs:9` (`App::view` signature), `src/app.rs:19-37` and `src/app.rs:71-88` (both `run()` call sites), `src/app.rs:99-107` (the `Dummy` test impl's `view` signature)

**Interfaces:**
- Consumes: `LayerStack::new`, `LayerStack::composite` (Task 1, 3).
- Produces: `App::view(&self, area: Rect, buf: &mut LayerStack)` — the signature `examples/demo.rs` and `examples/omnitrix.rs` (Task 5) must match.

- [ ] **Step 1: Write the failing test**

`src/app.rs` already has `tick_rate_defaults_to_none` and `on_tick_default_is_a_no_op`, both exercised through the `Dummy` test `App`. No new test is needed here — instead, this task's test-first step is to update `Dummy`'s `view` signature to the new type as part of Step 3 below, and rely on the existing test suite (this file's two tests, plus every widget/buffer test) to prove nothing broke. Confirm the current (pre-change) baseline is green:

Run: `cargo test --lib app::tests`
Expected: PASS (baseline, before this task's changes).

- [ ] **Step 2: Make the change fail to compile first**

Change only the trait signature (not yet the call sites or `Dummy`) to confirm the compiler forces the rest of the changes:

In `src/app.rs`, change:

```rust
    fn view(&self, area: Rect, buf: &mut Buffer);
```

to:

```rust
    fn view(&self, area: Rect, buf: &mut LayerStack);
```

Run: `cargo build --lib`
Expected: FAIL — `Dummy`'s `view` impl and both `run()` call sites no longer match the trait.

- [ ] **Step 3: Write the minimal implementation to make it compile again**

In `src/app.rs`, change the import on line 1 from:

```rust
use crate::buffer::{diff, Buffer};
```

to:

```rust
use crate::buffer::{diff, Buffer, LayerStack};
```

Change the first `run()` call site (originally lines 25-36) from:

```rust
    let mut prev = Buffer::new(w, h);
    let mut next = Buffer::new(w, h);
    app.view(
        Rect {
            x: 0,
            y: 0,
            width: w,
            height: h,
        },
        &mut next,
    );
    term.draw_diff(&diff(&prev, &next))?;
    prev = next;
```

to:

```rust
    let mut prev = Buffer::new(w, h);
    let mut stack = LayerStack::new(w, h);
    app.view(
        Rect {
            x: 0,
            y: 0,
            width: w,
            height: h,
        },
        &mut stack,
    );
    let next = stack.composite();
    term.draw_diff(&diff(&prev, &next))?;
    prev = next;
```

Change the second `run()` call site (inside the loop, originally lines 76-87) from:

```rust
            let mut next = Buffer::new(w, h);
            app.view(
                Rect {
                    x: 0,
                    y: 0,
                    width: w,
                    height: h,
                },
                &mut next,
            );
            term.draw_diff(&diff(&prev, &next))?;
            prev = next;
```

to:

```rust
            let mut stack = LayerStack::new(w, h);
            app.view(
                Rect {
                    x: 0,
                    y: 0,
                    width: w,
                    height: h,
                },
                &mut stack,
            );
            let next = stack.composite();
            term.draw_diff(&diff(&prev, &next))?;
            prev = next;
```

Change the `Dummy` test impl's `view` signature from:

```rust
        fn view(&self, _area: Rect, _buf: &mut Buffer) {}
```

to:

```rust
        fn view(&self, _area: Rect, _buf: &mut LayerStack) {}
```

- [ ] **Step 4: Run tests to verify everything passes**

Run: `cargo test --lib`
Expected: PASS — full library test suite (buffer, app, layout, widgets) green. This is the check that proves the migration is behavior-preserving for single-layer apps.

- [ ] **Step 5: Commit**

```bash
git add src/app.rs
git commit -m "feat(app): Paint writes into a LayerStack, composited before Diff"
```

---

### Task 5: Migrate `examples/demo.rs` and `examples/omnitrix.rs`

**Files:**
- Modify: `examples/demo.rs:4` (import), `examples/demo.rs:71` (`view` signature)
- Modify: `examples/omnitrix.rs:8` (import), `examples/omnitrix.rs:73` (`view` signature)

**Interfaces:**
- Consumes: `App::view(&self, area: Rect, buf: &mut LayerStack)` (Task 4), `ttui::buffer::LayerStack` (Task 1).
- Produces: nothing consumed by a later task — this is the last task in the plan.

- [ ] **Step 1: Confirm the pre-change state fails to build**

After Task 4 lands, the workspace as a whole (library + examples) no longer compiles, because both examples still implement `view` against `&mut Buffer`. This is the "test" for this task: a full workspace build.

Run: `cargo build --examples`
Expected: FAIL — `examples/demo.rs` and `examples/omnitrix.rs` report `view` not matching the `App` trait (expected `&mut LayerStack`, found `&mut Buffer`).

- [ ] **Step 2: Update `examples/demo.rs`**

Change line 4 from:

```rust
use ttui::buffer::Buffer;
```

to:

```rust
use ttui::buffer::LayerStack;
```

Change line 71 from:

```rust
    fn view(&self, area: Rect, buf: &mut Buffer) {
```

to:

```rust
    fn view(&self, area: Rect, buf: &mut LayerStack) {
```

Leave the rest of the function body untouched — deref coercion means every existing `buf.set(...)` call and every `widget.render(area, buf)` call keeps compiling as-is.

- [ ] **Step 3: Update `examples/omnitrix.rs`**

Change line 8 from:

```rust
use ttui::buffer::Buffer;
```

to:

```rust
use ttui::buffer::LayerStack;
```

Change line 73 from:

```rust
    fn view(&self, area: Rect, buf: &mut Buffer) {
```

to:

```rust
    fn view(&self, area: Rect, buf: &mut LayerStack) {
```

Leave the rest of the function body untouched, for the same reason.

- [ ] **Step 4: Run the full build and test suite**

Run: `cargo build --examples && cargo test`
Expected: PASS — both examples build, and the full test suite (library + any example-adjacent tests) stays green.

- [ ] **Step 5: Manual smoke check**

Run: `cargo run --example demo`
Expected: renders identically to before this plan (single-layer app, so `composite()`'s depth-1 fast path makes this a straight passthrough). Press `q` (or the demo's documented quit key) to exit.

Run: `cargo run --example omnitrix`
Expected: renders identically to before this plan, including the tick-driven animation from #25. Exit via the example's documented quit key.

- [ ] **Step 6: Commit**

```bash
git add examples/demo.rs examples/omnitrix.rs
git commit -m "chore(examples): migrate to LayerStack-based App::view"
```

---

## Self-Review Notes

- **Spec coverage:** `LayerStack` construction (Task 1), Deref/DerefMut back-compat (Task 2), `composite()` incl. depth-1 fast path (Task 3), `App`/`run()` wiring (Task 4), and example migration (Task 5) cover every section of the design spec (`docs/design/specs/2026-08-05-buffer-layering-compositing-design.md`). The spec's "App-space boundary" section requires no task — it's a boundary statement (naming/decay stay out of core), not a core-framework deliverable.
- **Placeholder scan:** no TBDs; every step has literal code and literal commands.
- **Type consistency:** `LayerStack::new(width: u16, height: u16)`, `push_layer(&mut self) -> &mut Buffer`, `layer_mut(&mut self, index: usize) -> &mut Buffer`, and `composite(&self) -> Buffer` are introduced once in Task 1/3 and referenced identically in every later task and in `app.rs`/the examples.
