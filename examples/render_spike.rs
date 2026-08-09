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
use ttui::canvas::{Canvas, CanvasMode};
use ttui::easing::lerp_color;
use ttui::layout::{Constraint, Direction, Layout, Rect};

struct RenderSpike {
    hue_shift: f32,
    gauge_phase: f32,
    plot_phase: f32,
    quit: bool,
}

impl RenderSpike {
    fn new() -> Self {
        RenderSpike {
            hue_shift: 0.0,
            gauge_phase: 0.0,
            plot_phase: 0.0,
            quit: false,
        }
    }

    fn render_gauge(&self, area: Rect, buf: &mut LayerStack) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let mut canvas = Canvas::new(area.width, area.height, CanvasMode::HalfBlock);
        let subpixel_height = area.height * 2;
        let fill = self.gauge_phase.sin() * 0.5 + 0.5; // 0..1
        let filled = (subpixel_height as f32 * fill).round() as u16;
        for row in 0..filled {
            let t = row as f32 / subpixel_height.max(1) as f32;
            let color = lerp_color(
                Color::Rgb { r: 220, g: 40, b: 40 },
                Color::Rgb { r: 40, g: 220, b: 90 },
                t,
            );
            for col in 0..area.width {
                canvas.set_pixel(col, subpixel_height - 1 - row, color);
            }
        }
        canvas.blit(buf, area.x, area.y);
    }

    fn render_plot(&self, area: Rect, buf: &mut LayerStack) {
        if area.width < 2 || area.height == 0 {
            return;
        }
        let mut canvas = Canvas::new(area.width, area.height, CanvasMode::Braille);
        let grid_w = area.width * 2;
        let grid_h = area.height * 4;
        let sample = |gx: u16| -> f32 { (gx as f32 * 0.25 + self.plot_phase).sin() };
        for gx in 0..grid_w.saturating_sub(1) {
            let y0 = grid_h - 1 - ((sample(gx) * 0.5 + 0.5) * (grid_h - 1) as f32).round() as u16;
            let y1 =
                grid_h - 1 - ((sample(gx + 1) * 0.5 + 0.5) * (grid_h - 1) as f32).round() as u16;
            canvas.line(gx, y0, gx + 1, y1, Color::Rgb { r: 90, g: 180, b: 255 });
        }
        canvas.blit(buf, area.x, area.y);
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
        self.render_gauge(
            Rect { x: 2, y: 2, width: 10, height: 8 },
            buf,
        );
        self.render_plot(
            Rect { x: 14, y: 2, width: 30, height: 8 },
            buf,
        );
    }

    fn should_quit(&self) -> bool {
        self.quit
    }

    fn tick_rate(&self) -> Option<Duration> {
        Some(Duration::from_millis(50))
    }

    fn on_tick(&mut self, elapsed: Duration) {
        self.hue_shift = (self.hue_shift + 2.0) % 360.0;
        self.gauge_phase += elapsed.as_secs_f32() * 1.5;
        self.plot_phase += elapsed.as_secs_f32() * 4.0;
    }
}

fn main() -> std::io::Result<()> {
    let mut app = RenderSpike::new();
    run(&mut app)
}
