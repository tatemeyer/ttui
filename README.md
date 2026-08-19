# TTUI — Tate's Terminal User Interface

[![crates.io](https://img.shields.io/crates/v/ttui.svg)](https://crates.io/crates/ttui)
[![docs.rs](https://img.shields.io/docsrs/ttui)](https://docs.rs/ttui)
[![CI](https://github.com/tatemeyer/ttui/actions/workflows/ci.yml/badge.svg)](https://github.com/tatemeyer/ttui/actions/workflows/ci.yml)
[![MSRV](https://img.shields.io/badge/MSRV-1.87-blue)](#minimum-supported-rust-version)
[![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

A terminal UI framework built from first principles: direct control over
text rendering, color, pane layout, and multiplexing.

You own your state. TTUI owns the loop, the compositing, and the
terminal — and only redraws the cells that actually changed.

## Install

```sh
cargo add ttui
```

Or in `Cargo.toml`:

```toml
[dependencies]
ttui = "1.1"
```

## Quick start

An app is one type implementing `App`:

```rust
use crossterm::event::{Event, KeyCode};
use ttui::app::{run, App};
use ttui::buffer::LayerStack;
use ttui::layout::Rect;
use ttui::widgets::text::Text;

struct Hello {
    quit: bool,
}

impl App for Hello {
    fn update(&mut self, event: &Event) {
        if let Event::Key(key) = event {
            if key.code == KeyCode::Char('q') {
                self.quit = true;
            }
        }
    }

    fn view(&self, area: Rect, buf: &mut LayerStack) {
        Text::new("hello, terminal — press q to quit").render(area, buf);
    }

    fn should_quit(&self) -> bool {
        self.quit
    }
}

fn main() -> std::io::Result<()> {
    run(&mut Hello { quit: false })
}
```

`run` enables raw mode and installs a panic hook that restores the
terminal, so a panic doesn't leave the user's shell wrecked.

## How it fits together

```text
App state ──view()──▶ LayerStack ──composite()──▶ Buffer ──diff()──▶ terminal
    ▲                                                                    │
    └──────────────── update(event) / on_tick(elapsed) ◀─────────────────┘
```

- **Layered buffers** — layers composite top-to-bottom by alpha, so an
  effects layer can sit over a UI layer without either knowing about the
  other.
- **Stateless widgets** — a widget is handed an area and a buffer and
  draws. It never holds selection or scroll state, so your app stays the
  single source of truth.
- **Constraint layout** — `Rect`s are split by `Fixed`/`Percentage`/`Fill`
  constraints. No widget computes its own position.
- **Real-time animation** — `Transition` tracks actual elapsed time as
  `0.0..=1.0` progress, and `Phases` subdivides it into named stages
  without writing every boundary twice.

Beyond the core: sub-cell drawing (half-block and braille), fixed-forward
3D projection, particle systems, glitch and screen-shake effects, a
key/chord input binder, and data-viz widgets.

## Status

**v1.1.0.** The core render pipeline, constraint layout, alpha-compositing
buffer layering, and a full widget set — `Text`/`List`/`Table`/`Block`
plus glitch effects, particles, a perspective camera, a chord input
binder, and data-viz widgets — are implemented and exercised by ten
example apps and the `showcase` demo reel.

v1.1 is additive: `transition::Phases<N>` for phase arithmetic, plus
three shared primitives — `easing::scale_color`, `noise::scatter` and
`Buffer::blit` — replacing ten duplicate definitions the example apps and
the library itself had each grown by hand.

See [`CHANGELOG.md`](CHANGELOG.md) for the full release history.

## Try the examples

From a checkout:

```sh
cargo run --bin showcase             # flagship demo reel: five vignettes
cargo run --example demo             # nested panes, Tab focus, list + table
cargo run --example launcher         # cross-app portal nexus
cargo run --example falcon           # cockpit: starfield, HUD, glitch burst
cargo run --example mission_control  # animated telemetry dashboard
cargo run --example control_panel    # click-driven console: toggles, dial
```

See [`examples/README.md`](examples/README.md) for the full list and what
each one demonstrates.

## Minimum supported Rust version

**1.87.0.** Verified rather than assumed: 1.87 is the first release with
`unsigned_is_multiple_of` stable, and 1.86 fails to compile the library.

An MSRV bump is treated as a minor version bump, not a patch.

## Documentation

- [API docs on docs.rs](https://docs.rs/ttui)
- [`examples/README.md`](examples/README.md) — what each example app demonstrates
- [Design docs](https://github.com/tatemeyer/ttui/blob/main/docs/design/README.md)
  — the living index of design Arcs, and how specs, plans and tasks relate.
  The design tree lives in the repository and is not shipped in the
  published crate.

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for how to build, test, and what
the four required checks are. Bug reports and feature requests go through
the [issue templates](https://github.com/tatemeyer/ttui/issues/new/choose).

This project is developed using [superpowers](https://github.com/obra/superpowers)
— every feature goes through brainstorm → design doc → plan →
subagent-driven implementation. Project-specific conventions layered on
top of that live in `.claude/rules/`.

## Security

See [`SECURITY.md`](SECURITY.md) for how to report a vulnerability.

## License

MIT — see [`LICENSE`](LICENSE).
