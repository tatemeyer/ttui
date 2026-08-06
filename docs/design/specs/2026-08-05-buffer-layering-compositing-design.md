# Buffer Layering / Compositing — Design

**Status:** draft, pending your review before we move to planning.
**Date:** 2026-08-05
**Relationship to Rev A / Rev B:** this is an addendum to Rev A
(`2026-08-04-ttui-core-framework-design.md`), designing a section Rev B
(`2026-08-05-ttui-rev-b-vision-alignment-design.md`, "Deferred
(documented, not designed): buffer layering") recorded but explicitly
left undesigned. Both prior specs are unchanged and fully implemented;
this spec adds new, opt-in capability on top of them. Written in
response to Issue #27.

## Context / Motivation

Rev A's render pipeline is `App state -> View builder -> Layout ->
Paint -> Diff -> Terminal writer`, with Paint writing directly into a
single `Buffer` (`src/app.rs`) that Diff (`src/buffer.rs::diff`)
compares against the previous frame's `Buffer`. Two vision documents
recorded outside this repo (`TTUI-Ideas/vision/UI`) — Super Smash Crabs
and TARDIS — both need Paint to write into more than one buffer before
Diff runs: Smash Crabs wants three explicit, z-ordered buffers
(background/UI/effects) composited before diffing; TARDIS wants a
single decaying "Glitch Buffer" overlaid on its primary buffer.

Rev B recorded the target shape but deliberately left it undesigned,
gated on the Omnitrix tick-subscription prototype validating that the
render path holds up under continuous animated load. That validation
shipped in #25 (`examples/omnitrix.rs`; re-measured at n=161 ticks,
avg=47.40ms against a nominal 33ms budget). Issue #27 asks for the
actual design now that gate has cleared.

Rev B's recorded direction, taken as fixed here: Paint writes into an
ordered stack of same-dimension `Buffer`s instead of one; a
**Composite** step is inserted at the seam Rev A already reserved
between Paint and Diff, flattening the stack into a single `Buffer`
immediately before Diff runs; Diff and the terminal writer are
untouched, since they only ever see the final flattened buffer.
Compositing uses discrete rules ("last non-default cell in stack order
wins"), not true alpha blending — `Cell` has no alpha channel, and
adding one is a larger change than layering itself. What Rev B left
open, and what this spec resolves, is the concrete API.

## Scope of this spec

**Committed and designed here:** a `LayerStack` type wrapping an
ordered stack of `Buffer`s, a `composite()` operation flattening that
stack per Rev B's discrete rule, and the `App` trait / render-loop
changes needed to wire Paint through it.

**Explicitly not designed here:** the camera/viewport abstraction (also
deferred by Rev B, needed only by TARDIS) — unrelated to layering and
out of scope for this spec. Also out of scope: any per-app layer
semantics (what "background" or "glitch buffer" means), and any
decay/aging behavior for a TARDIS-style overlay — both are app-space
concerns, addressed below under "App-space boundary."

## Design

### `LayerStack`

A new type in `src/buffer.rs`, alongside `Buffer`/`Cell`/`diff()` (this
file already holds the other half of the same pipeline seam; at ~121
lines today there's no size pressure to split it out):

```rust
pub struct LayerStack {
    width: u16,
    height: u16,
    layers: Vec<Buffer>, // always len >= 1; layers[0] is the base layer
}

impl LayerStack {
    pub fn new(width: u16, height: u16) -> Self {
        LayerStack { width, height, layers: vec![Buffer::new(width, height)] }
    }

    pub fn push_layer(&mut self) -> &mut Buffer {
        self.layers.push(Buffer::new(self.width, self.height));
        self.layers.last_mut().unwrap()
    }

    pub fn layer_mut(&mut self, index: usize) -> &mut Buffer {
        &mut self.layers[index] // index must already exist via push_layer(); no auto-grow
    }

    pub fn composite(&self) -> Buffer {
        if self.layers.len() == 1 {
            return self.layers[0].clone(); // fast path: no scan needed
        }
        let mut out = Buffer::new(self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                let mut cell = Cell::default();
                for layer in &self.layers { // bottom-to-top; last non-default wins
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
}

impl std::ops::Deref for LayerStack {
    type Target = Buffer;
    fn deref(&self) -> &Buffer { &self.layers[0] }
}
impl std::ops::DerefMut for LayerStack {
    fn deref_mut(&mut self) -> &mut Buffer { &mut self.layers[0] }
}
```

Because `push_layer()` always constructs `Buffer::new(self.width,
self.height)` internally, mismatched-dimension layers are structurally
unreachable through the public API — no runtime validation is needed.

`layer_mut(index)` does not auto-grow the stack: the index must already
exist via a prior `push_layer()` call. This keeps the mental model a
plain ordered stack (push to add, index to address what's already
there) rather than introducing sparse/auto-vivifying layers.

**Depth-1 fast path:** `composite()` special-cases a stack of exactly
one layer and returns a clone of it directly, skipping the per-cell
scan. This preserves the zero-overhead-when-unused property the
project already established for `tick_rate`/`on_tick` in Rev B — apps
that never call `push_layer()` pay no compositing cost beyond a clone.

### `App` trait and `app.rs::run()`

`App::view`'s signature changes:

```rust
// was: fn view(&self, area: Rect, buf: &mut Buffer);
fn view(&self, area: Rect, buf: &mut LayerStack);
```

Both call sites in `run()` (the initial draw before the loop, and the
redraw inside it) change from:

```rust
let mut next = Buffer::new(w, h);
app.view(area, &mut next);
```

to:

```rust
let mut stack = LayerStack::new(w, h);
app.view(area, &mut stack);
let next = stack.composite();
```

`prev` stays type `Buffer`; `diff(&prev, &next)` and
`Terminal::draw_diff` are untouched, matching Rev B's stated boundary
that Diff and the terminal writer only ever see the final flattened
buffer.

### Migration of existing `App` impls

`examples/demo.rs` and `examples/omnitrix.rs` are the only two `App`
implementors in the repo today. Each needs exactly a one-line signature
edit on `view` (`&mut Buffer` -> `&mut LayerStack`); **bodies are
unchanged**. `LayerStack` derefs to `Buffer`, and Rust applies deref
coercion at both method-call and plain function-call argument
positions — this covers direct `buf.set(...)` calls as well as
`widget.render(area, buf)` calls into `Text`/`Block`/`List`/`Table`,
none of which need to change, since their `render` signatures keep
taking `&mut Buffer` directly.

### App-space boundary (naming layers, decay)

Core only exposes an ordered, indexed stack — it has no concept of
"background/UI/effects" or "glitch buffer," and models no decay/aging
behavior. Smash Crabs and TARDIS each assign meaning to indices
themselves (e.g. app-local `const BACKGROUND: usize = 0;` constants),
and a TARDIS-style overlay's decay (aging/repainting its layer over
time) is implemented in app code via the already-existing `on_tick`
hook. This mirrors the boundary Rev B already drew for the deferred
camera/viewport item: "this lives at app-space, not as core framework
machinery."

## Testing

Per `.claude/rules/development-conventions.md`, this is `coding`-tagged
work with none of the four TDD exceptions applying (it's not
config/git-adjacent, not an example/demo, not real-TTY-dependent, and
not a throwaway research spike) — so it follows TDD, test-first, inline
in `src/buffer.rs`'s existing `#[cfg(test)] mod tests`:

- `LayerStack::new` produces a stack with exactly one default-filled
  base layer of the given dimensions.
- `push_layer()` appends a same-dimension, default-filled layer; writes
  to it via `layer_mut` persist independently of the base layer.
- Deref/DerefMut: `stack.set(...)`/`stack.get(...)` read/write the base
  layer exactly as a bare `Buffer` would — this is what backs the
  zero-body-change migration claim above.
- `composite()` on a 1-layer stack returns a value equal to that layer
  (the fast path).
- `composite()` on a 3-layer stack (mirroring Smash Crabs' background/
  UI/effects): a topmost non-default cell wins; a topmost-default cell
  falls through to reveal the layer below it; a cell default in every
  layer composites to `Cell::default()`.
- Existing `src/app.rs` tests (the `Dummy` App) continue to pass with
  only the `view` signature updated — proving no behavioral change for
  apps that never call `push_layer()`.

## Critical files

- `src/buffer.rs` — add `LayerStack`, `composite()`, and their tests.
- `src/app.rs` — `App::view` signature, both `run()` call sites, the
  `Dummy` test impl's signature.
- `examples/demo.rs`, `examples/omnitrix.rs` — one-line `view`
  signature update each.

## Verification

- `cargo test` — new `LayerStack`/`composite()` unit tests plus the
  full existing suite (buffer, app, widgets) green.
- `cargo run --example demo` and `cargo run --example omnitrix` —
  manual smoke check that single-layer rendering is visually unchanged
  (the depth-1 fast path means these should be pixel-identical to
  before).
- `cargo fmt` / `cargo clippy` clean.
