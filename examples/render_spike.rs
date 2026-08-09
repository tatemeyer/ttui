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
use ttui::blend::{blend_over, fade_toward};
use ttui::buffer::{Buffer, Cell, LayerStack};
use ttui::canvas::{Canvas, CanvasMode};
use ttui::easing::lerp_color;
use ttui::layout::{Constraint, Direction, Layout, Rect};
use ttui::particles::{Particle, ParticleSystem};

struct RenderSpike {
    hue_shift: f32,
    gauge_phase: f32,
    plot_phase: f32,
    particles: ParticleSystem,
    trail: Buffer,
    quit: bool,
}

impl RenderSpike {
    fn new() -> Self {
        RenderSpike {
            hue_shift: 0.0,
            gauge_phase: 0.0,
            plot_phase: 0.0,
            particles: ParticleSystem::new(),
            trail: Buffer::new(160, 50),
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
                Color::Rgb {
                    r: 220,
                    g: 40,
                    b: 40,
                },
                Color::Rgb {
                    r: 40,
                    g: 220,
                    b: 90,
                },
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
            canvas.line(
                gx,
                y0,
                gx + 1,
                y1,
                Color::Rgb {
                    r: 90,
                    g: 180,
                    b: 255,
                },
            );
        }
        canvas.blit(buf, area.x, area.y);
    }

    fn render_attribute_showcase(&self, area: Rect, buf: &mut LayerStack) {
        use ttui::buffer::CellStyle;
        let words: [(&str, CellStyle); 4] = [
            (
                "UNDERLINE",
                CellStyle {
                    underline: true,
                    ..Default::default()
                },
            ),
            (
                "ITALIC",
                CellStyle {
                    italic: true,
                    ..Default::default()
                },
            ),
            (
                "REVERSE",
                CellStyle {
                    reverse: true,
                    ..Default::default()
                },
            ),
            (
                "STRIKETHROUGH",
                CellStyle {
                    strikethrough: true,
                    ..Default::default()
                },
            ),
        ];
        let mut x = area.x;
        for (word, style) in words {
            for ch in word.chars() {
                if x >= area.x + area.width {
                    break;
                }
                buf.set(
                    x,
                    area.y,
                    Cell {
                        symbol: ch,
                        fg: Color::Reset,
                        bg: Color::Reset,
                        style,
                    },
                );
                x += 1;
            }
            x += 2; // gap between words
        }
    }

    fn blend_trail(&self, buf: &mut LayerStack) {
        let scene = buf.composite();
        let scene = blend_over(&scene, &self.trail, 1.0);
        // Assumes `buf` is always depth 1 (no `push_layer()` calls) —
        // if a future edit adds an upper layer, it would get
        // composited over this blended result and silently undo the
        // trail blend.
        *buf.layer_mut(0) = scene;
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

fn draw_gradient_ring(area: Rect, buf: &mut LayerStack, hue_shift: f32) {
    use ttui::buffer::CellStyle;
    if area.width < 2 || area.height < 2 {
        return;
    }
    let ring_cell = |x: u16, y: u16, symbol: char| -> Cell {
        let t = (x as f32 - area.x as f32) / area.width.max(1) as f32
            + (y as f32 - area.y as f32) / area.height.max(1) as f32;
        Cell {
            symbol,
            fg: hue_to_rgb(t * 180.0 + hue_shift),
            bg: Color::Reset,
            style: CellStyle {
                intensity: ttui::buffer::Intensity::Bold,
                ..Default::default()
            },
        }
    };
    for x in area.x..area.x + area.width {
        buf.set(x, area.y, ring_cell(x, area.y, '▀'));
        buf.set(
            x,
            area.y + area.height - 1,
            ring_cell(x, area.y + area.height - 1, '▄'),
        );
    }
    for y in area.y..area.y + area.height {
        buf.set(area.x, y, ring_cell(area.x, y, '█'));
        buf.set(
            area.x + area.width - 1,
            y,
            ring_cell(area.x + area.width - 1, y, '█'),
        );
    }
}

impl App for RenderSpike {
    fn update(&mut self, event: &Event) {
        let Event::Key(k) = event else { return };
        if k.kind != KeyEventKind::Press {
            return;
        }
        match k.code {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Char(' ') => {
                let center = (40.0, 15.0);
                for i in 0..16 {
                    let angle = i as f32 * (std::f32::consts::TAU / 16.0);
                    self.particles.spawn(Particle {
                        x: center.0,
                        y: center.1,
                        vx: angle.cos() * 20.0,
                        vy: angle.sin() * 10.0,
                        symbol: '*',
                        color: Color::Rgb {
                            r: 255,
                            g: 180,
                            b: 40,
                        },
                        lifetime: Duration::from_millis(700),
                        age: Duration::ZERO,
                    });
                }
            }
            _ => {}
        }
    }

    fn view(&self, area: Rect, buf: &mut LayerStack) {
        draw_gradient_ring(area, buf, self.hue_shift);
        let inner = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };

        let rows = Layout::new(
            Direction::Vertical,
            vec![Constraint::Fixed(1), Constraint::Fill(1)],
        )
        .split(inner);

        self.render_attribute_showcase(rows[0], buf);

        let cols = Layout::new(
            Direction::Horizontal,
            vec![Constraint::Percentage(50), Constraint::Fill(1)],
        )
        .split(rows[1]);
        self.render_gauge(cols[0], buf);
        self.render_plot(cols[1], buf);

        self.blend_trail(buf);
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
        self.particles.update(elapsed);
        self.trail = fade_toward(&self.trail, Color::Rgb { r: 0, g: 0, b: 0 }, 0.2);
        self.particles.render(&mut self.trail);
    }
}

fn main() -> std::io::Result<()> {
    if std::env::args().any(|a| a == "--bench") {
        return bench_frame_cost();
    }
    let mut app = RenderSpike::new();
    run(&mut app)
}

/// Ad hoc timing harness for this spike's recommendations write-up —
/// not a criterion benchmark, not kept as a permanent measurement
/// tool. Builds the densest frame this scene produces (mid-burst, all
/// six levers active) and times `view` + `composite` + `render_diff`
/// directly, bypassing the terminal.
fn bench_frame_cost() -> std::io::Result<()> {
    use ttui::buffer::{diff, LayerStack};
    use ttui::terminal::render_diff;

    let mut app = RenderSpike::new();
    for i in 0..16 {
        let angle = i as f32 * (std::f32::consts::TAU / 16.0);
        app.particles.spawn(Particle {
            x: 40.0,
            y: 15.0,
            vx: angle.cos() * 20.0,
            vy: angle.sin() * 10.0,
            symbol: '*',
            color: Color::Rgb {
                r: 255,
                g: 180,
                b: 40,
            },
            lifetime: Duration::from_millis(700),
            age: Duration::ZERO,
        });
    }
    app.on_tick(Duration::from_millis(16));

    let area = Rect {
        x: 0,
        y: 0,
        width: 120,
        height: 40,
    };
    let mut prev = ttui::buffer::Buffer::new(area.width, area.height);
    let start = std::time::Instant::now();
    const FRAMES: u32 = 200;
    let mut total_diffs = 0usize;
    for _ in 0..FRAMES {
        let mut stack = LayerStack::new(area.width, area.height);
        app.view(area, &mut stack);
        let next = stack.composite();
        let diffs = diff(&prev, &next);
        total_diffs += diffs.len();
        let mut sink = Vec::new();
        render_diff(&mut sink, &diffs)?;
        prev = next;
        app.on_tick(Duration::from_millis(16));
    }
    let elapsed = start.elapsed();
    println!(
        "{FRAMES} frames in {:?} ({:?}/frame avg), avg {} diffed cells/frame",
        elapsed,
        elapsed / FRAMES,
        total_diffs / FRAMES as usize
    );
    Ok(())
}
