# Contributing to TTUI

Thanks for your interest. This document covers how to build, test, and
get a change merged.

## Prerequisites

- **Rust 1.87.0 or newer** — see the MSRV section in the
  [README](README.md#minimum-supported-rust-version).
- **On Linux:** ALSA headers, for the audio used by some examples.
  ```sh
  sudo apt-get install -y libasound2-dev
  ```
  Not needed on macOS or Windows.

## Build and test

```sh
cargo build --workspace
cargo test --workspace
```

Some tests exercise real-terminal behaviour and are `#[ignore]`d by
default, because they need a TTY the test harness does not have. Run them
locally before merging anything that touches `src/terminal.rs`:

```sh
cargo test -- --ignored
```

One caveat worth knowing: a `--test`-filtered invocation does **not**
rebuild `examples/*` first, so it can silently test a stale fixture binary
and report a false green. Run the full `cargo test` (or
`cargo build -p visual-snapshot --examples` first) when the change affects
an example.

## The four required checks

Every PR must have all four green before merge. Run them locally first —
they are exactly what CI runs:

```sh
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

`clippy` is `-D warnings`, so a warning is a failure. `#![warn(missing_docs)]`
is on for the library, which means every `pub` item needs a doc comment
and the clippy gate enforces it.

## Conventions

### Commits

[Conventional Commits](https://www.conventionalcommits.org/): `type(scope): description`.

- **Type** — `feat`, `fix`, `docs`, `chore`, `ci`, `test` (lowercase).
- **Scope** — the crate area or app touched: `core`, `widgets`, `omnitrix`,
  `tardis`, or a module name.
- **Subject** — imperative mood: "add X", not "added X".
- **Body** — required on any `feat`/`fix` whose change isn't self-evident
  from the subject. State *why*, not what.
- **Issue reference** — a trailing `(#N)` when the commit closes a tracked
  issue.

### Tests

Test-first is the default for code changes, with four standing
exceptions: pure config work, examples and demos, real-TTY behaviour, and
throwaway research spikes.

Inline `#[cfg(test)] mod tests` per module is the norm. `tests/` is for
exercising the crate as an external consumer would, through the public
`ttui::` API.

### Documentation

Doc comments are agent-first, not exhaustive rustdoc:

- Every `src/` module gets a `//!` header of 1–3 sentences: what it is,
  and what it deliberately isn't.
- Every `pub` item gets a single-line `///` summary — purpose and usage,
  not a restatement of the name.
- Inline comments inside function bodies stay sparse: only for a genuinely
  non-obvious invariant, workaround, or subtlety.

### Rendering changes

Anything that affects rendering — `src/effects.rs`, `src/particles.rs`,
`src/transition.rs`, `src/widgets/`, `src/canvas.rs`, `src/glitch.rs`, or
an example's `view()`/`on_tick()` — must be captured and reviewed visually
before merge, not reasoned about from the diff:

```sh
cargo run -p visual-snapshot -- --example <name> --size 120x40 \
  --script <path.json> --out <path.gif>
```

Record which captures you reviewed in the PR template's Verification
section.

One hard-won caveat: **these captures are not deterministic.** Apps
animate continuously, so two runs of the *identical* binary produce
different frames. Always compare against a same-code control run before
concluding a change altered output.

### Versioning

[SemVer](https://semver.org/), applied to the `ttui` library crate. Its
public API surface is every `pub` item under `src/`.

- **Breaking** — removing or renaming a `pub` item, changing a `pub fn`
  signature, adding a required field to a `pub struct`, changing a trait's
  required methods, or adding a variant to an existing `pub enum` (none are
  `#[non_exhaustive]`, so a new variant breaks an exhaustive `match`).
- **Minor** — a wholly new `pub` item, or a new optional builder method.
  An MSRV bump is also a minor bump.
- **Patch** — everything else.

`tools/visual-snapshot` is internal dev tooling with no external
consumers and sits outside this policy.

Add an entry to `CHANGELOG.md` under `## [Unreleased]`, in
[Keep a Changelog](https://keepachangelog.com) format.

## Pull requests

1. Branch from `main`.
2. Make the change, with tests.
3. Run the four checks locally.
4. Open a PR using the template, filling in the Verification section with
   what you actually ran — including any visual captures or `--ignored`
   tests.
5. All four checks must be green before merge.

Larger changes go through a design doc and implementation plan first
(`docs/design/specs/` and `docs/design/plans/`) — see
[the design docs index](https://github.com/tatemeyer/ttui/blob/main/docs/design/README.md)
for how Arcs, Slices and Tasks are structured. If you're planning
something substantial, open an issue to discuss it before writing code.

## Code of conduct

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md).
