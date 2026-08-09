// examples/render_spike.rs
//
// SPIKE PROTOTYPE for the rendering-fidelity spike
// (docs/design/specs/core/2026-08-08-rendering-fidelity-spike-design.md).
// Not a themed vision-doc app — a bare showcase proving out six
// rendering-fidelity levers together. This file grows across that
// spec's implementation plan; expect prototype-quality code throughout.

use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::style::Color;
use std::time::Duration;
use ttui::app::{run, App};
use ttui::buffer::{Cell, LayerStack};
use ttui::layout::Rect;

struct RenderSpike {
    hue_shift: f32,
    quit: bool,
}

impl RenderSpike {
    fn new() -> Self {
        RenderSpike {
            hue_shift: 0.0,
            quit: false,
        }
    }
}

/// Cheap HSV(hue, 1.0, 1.0)->RGB — used only to paint smooth test
/// gradients in this spike, not a general color-space utility.
fn hue_to_rgb(hue: f32) -> Color {
    let h = hue.rem_euclid(360.0) / 60.0;
    let x = 1.0 - (h.rem_euclid(2.0) - 1.0).abs();
    let (r, g, b) = match h as u32 {
        0 => (1.0, x, 0.0),
        1 => (x, 1.0, 0.0),
        2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0),
        4 => (x, 0.0, 1.0),
        _ => (1.0, 0.0, x),
    };
    Color::Rgb {
        r: (r * 255.0) as u8,
        g: (g * 255.0) as u8,
        b: (b * 255.0) as u8,
    }
}

impl App for RenderSpike {
    fn update(&mut self, event: &Event) {
        let Event::Key(k) = event else { return };
        if k.kind != KeyEventKind::Press {
            return;
        }
        if k.code == KeyCode::Char('q') {
            self.quit = true;
        }
    }

    fn view(&self, area: Rect, buf: &mut LayerStack) {
        // Lever 1: color-depth audit. A smooth 360-degree hue sweep
        // across the full width, animated by hue_shift. If this bands
        // into discrete steps instead of a smooth ramp, truecolor
        // isn't actually reaching the terminal — record that in the
        // spec's recommendations section (Task 9).
        for x in 0..area.width {
            let hue = (x as f32 / area.width.max(1) as f32) * 360.0 + self.hue_shift;
            let color = hue_to_rgb(hue);
            for y in 0..area.height {
                buf.set(
                    x,
                    y,
                    Cell {
                        symbol: '█',
                        fg: color,
                        bg: color,
                        ..Default::default()
                    },
                );
            }
        }
    }

    fn should_quit(&self) -> bool {
        self.quit
    }

    fn tick_rate(&self) -> Option<Duration> {
        Some(Duration::from_millis(50))
    }

    fn on_tick(&mut self, _elapsed: Duration) {
        self.hue_shift = (self.hue_shift + 2.0) % 360.0;
    }
}

fn main() -> std::io::Result<()> {
    let mut app = RenderSpike::new();
    run(&mut app)
}
