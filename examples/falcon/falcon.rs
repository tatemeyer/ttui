use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::style::Color;
use std::time::Duration;
use ttui::app::App;
use ttui::buffer::{Cell, LayerStack};
use ttui::canvas::{Canvas, CanvasMode};
use ttui::glitch::GlitchBuffer;
use ttui::layout::{Constraint, Direction, Layout, Rect};
use ttui::particles::{Particle, ParticleSystem};
use ttui::perspective::{Camera, Line3, Point3};
use ttui::theme::{BorderSet, Theme};
use ttui::transition::Transition;
use ttui::widgets::{cockpit_panel::CockpitPanel, text::Text};

#[path = "boot.rs"]
mod boot;

const TICK_INTERVAL: Duration = Duration::from_millis(33); // ~30 FPS, matches every other app
const BOOT_TOTAL_MS: u64 = 1400;
const IDLE_FLICKER_PERIOD_TICKS: u64 = 90; // ~3s at 33ms/tick, per panel
const IDLE_FLICKER_DURATION_MS: u64 = 600;
const WHACK_SPARK_COUNT: usize = 6;
const WHACK_SPARK_LIFETIME_MS: u64 = 300;
const STAR_COUNT: usize = 60;
const STAR_SPEED: f32 = 3.0; // z-units/second
const STAR_RESPAWN_Z: f32 = 20.0;
const CANOPY_NEAR_Z: f32 = 2.0;
const CANOPY_FAR_Z: f32 = 10.0;
const CANOPY_HALF_W: f32 = 5.0;
const CANOPY_HALF_H: f32 = 3.0;
const HYPERDRIVE_PHASE_SPEED: f32 = 1.5; // radians/sec
const HYPERDRIVE_DASH_COUNT: usize = 8;
const HYPERDRIVE_START: Point3 = Point3 {
    x: 0.0,
    y: 0.0,
    z: 2.5,
};
const HYPERDRIVE_END: Point3 = Point3 {
    x: 6.0,
    y: 2.0,
    z: 22.0,
};
const SENSOR_SWEEP_SPEED: f32 = std::f32::consts::TAU / 4.0; // one revolution per ~4s
const SENSOR_PLANE_Z: f32 = 6.0;
const SENSOR_RADIUS: f32 = 3.0;
const SENSOR_TRAIL_COUNT: usize = 4;
const SENSOR_TRAIL_STEP: f32 = 0.25; // radians between trailing lines

/// The canopy's 8 corners: two parallel rectangles (near/far) of the
/// same world-space size, connected by 4 verticals — the perspective
/// convergence comes entirely from the projection, not from shrinking
/// the far rectangle's world-space size. Index order:
/// `i = (dx_idx*2 + dy_idx)*2 + z_idx` for `dx_idx, dy_idx, z_idx`
/// each in `{0 (near/-), 1 (far/+)}`.
fn canopy_vertices() -> [Point3; 8] {
    let mut v = [Point3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    }; 8];
    let mut i = 0;
    for dx in [-CANOPY_HALF_W, CANOPY_HALF_W] {
        for dy in [-CANOPY_HALF_H, CANOPY_HALF_H] {
            for z in [CANOPY_NEAR_Z, CANOPY_FAR_Z] {
                v[i] = Point3 { x: dx, y: dy, z };
                i += 1;
            }
        }
    }
    v
}

/// 4 dx-parallel edges (2 near + 2 far), 4 dy-parallel edges (2 near +
/// 2 far), then 4 near-to-far connectors — same topology as a cube's
/// 12 edges.
#[rustfmt::skip]
const CANOPY_EDGES: [(usize, usize); 12] = [
    (0, 4), (1, 5), (2, 6), (3, 7), // edges along dx
    (0, 2), (1, 3), (4, 6), (5, 7), // edges along dy
    (0, 1), (2, 3), (4, 5), (6, 7), // near-to-far connectors
];

#[derive(Clone, Copy, PartialEq)]
enum PanelKind {
    Hyperdrive,
    Sensors,
    Weapons,
}

const PANELS: [PanelKind; 3] = [
    PanelKind::Hyperdrive,
    PanelKind::Sensors,
    PanelKind::Weapons,
];

impl PanelKind {
    fn name(&self) -> &'static str {
        match self {
            PanelKind::Hyperdrive => "Hyperdrive",
            PanelKind::Sensors => "Sensors",
            PanelKind::Weapons => "Weapons",
        }
    }
}

fn falcon_theme() -> Theme {
    Theme {
        background: Color::Rgb { r: 10, g: 10, b: 8 },
        primary: Color::Rgb {
            r: 255,
            g: 176,
            b: 0,
        },
        secondary: Color::Rgb {
            r: 76,
            g: 187,
            b: 23,
        },
        tertiary: Color::Rgb {
            r: 255,
            g: 49,
            b: 49,
        },
        accent: Color::Rgb {
            r: 255,
            g: 215,
            b: 0,
        },
        primary_end: None,
        border: BorderSet::default(),
        border_bold: false,
        border_thick: false,
    }
}

fn falcon_camera() -> Camera {
    Camera {
        near: 0.5,
        focal_length: 8.0,
    }
}

struct Star {
    x: f32,
    y: f32,
    z: f32,
}

/// Deterministic pseudo-random scatter for star placement — no RNG
/// dependency, matching every prior Arc's posture.
fn scatter(seed: u32, spread: f32) -> f32 {
    let h = (seed.wrapping_mul(2_654_435_761)) ^ (seed.wrapping_mul(40_503).rotate_left(13));
    ((h % 10_000) as f32 / 10_000.0 - 0.5) * spread
}

pub(crate) struct Falcon {
    theme: Theme,
    camera: Camera,
    hyperdrive_phase: f32,
    sensor_sweep_angle: f32,
    stars: Vec<Star>,
    focused: usize,
    // `App::view` takes `&self`, so this records the last-seen
    // terminal area through a `Cell` (interior mutability) rather
    // than a plain field, so `update`'s WHACK handler below can read
    // the focused panel's current on-screen position. Referenced by
    // full path (`std::cell::Cell`) rather than a `use` import,
    // since `ttui::buffer::Cell` is already imported under the plain
    // name `Cell` and the two would collide.
    last_area: std::cell::Cell<Rect>,
    glitches: [GlitchBuffer; 3],
    particles: ParticleSystem,
    tick_count: u64,
    booting: Option<Transition>,
    quit: bool,
}

impl Falcon {
    pub(crate) fn new() -> Self {
        let stars = (0..STAR_COUNT)
            .map(|i| {
                let seed = i as u32;
                Star {
                    x: scatter(seed, 16.0),
                    y: scatter(seed.wrapping_add(1_000), 10.0),
                    z: 2.0 + (seed as f32 % 20.0),
                }
            })
            .collect();
        Falcon {
            theme: falcon_theme(),
            camera: falcon_camera(),
            hyperdrive_phase: 0.0,
            sensor_sweep_angle: 0.0,
            stars,
            focused: 0,
            last_area: std::cell::Cell::new(Rect {
                x: 0,
                y: 0,
                width: 80,
                height: 24,
            }),
            glitches: [
                GlitchBuffer::new(),
                GlitchBuffer::new(),
                GlitchBuffer::new(),
            ],
            particles: ParticleSystem::new(),
            tick_count: 0,
            booting: Some(Transition::start(Duration::from_millis(BOOT_TOTAL_MS))),
            quit: false,
        }
    }

    fn panel_slots(area: Rect) -> [Rect; 3] {
        let slots = Layout::new(Direction::Horizontal, vec![Constraint::Fill(1); 3]).split(area);
        [slots[0], slots[1], slots[2]]
    }

    fn panel_box(slot: Rect, focused: bool) -> Rect {
        let base_w = slot.width.saturating_sub(6).max(8).min(slot.width);
        let base_h = slot.height.saturating_sub(4).clamp(4, 10).min(slot.height);
        let focus_w = (base_w + 4).min(slot.width);
        let focus_h = (base_h + 2).min(slot.height);
        let box_w = if focused { focus_w } else { base_w };
        let box_h = if focused { focus_h } else { base_h };
        Rect {
            x: slot.x + slot.width.saturating_sub(box_w) / 2,
            y: slot.y + slot.height.saturating_sub(box_h) / 2,
            width: box_w,
            height: box_h,
        }
    }

    /// Splits `area` into the windshield (top ~78%) and console (bottom
    /// strip) regions — factored out so `render_dashboard` and the WHACK
    /// handler in `update()` can never disagree on where the split falls.
    fn windshield_console_split(area: Rect) -> (Rect, Rect) {
        let regions = Layout::new(
            Direction::Vertical,
            vec![Constraint::Percentage(78), Constraint::Fill(1)],
        )
        .split(area);
        (regions[0], regions[1])
    }

    fn render_dashboard(&self, area: Rect, buf: &mut LayerStack) {
        let (windshield, console) = Self::windshield_console_split(area);

        self.render_windshield(windshield, buf, 12);

        let bg = Cell {
            symbol: ' ',
            fg: self.theme.primary,
            bg: self.theme.background,
            alpha: 1.0,
            ..Default::default()
        };
        for y in 0..console.height {
            for x in 0..console.width {
                buf.set(console.x + x, console.y + y, bg.clone());
            }
        }

        let slots = Self::panel_slots(console);
        let mut panel_inners = [Rect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }; 3];
        for (i, kind) in PANELS.iter().enumerate() {
            let focused = i == self.focused;
            let panel_box = Self::panel_box(slots[i], focused);
            let inner = CockpitPanel::new(focused).render(panel_box, &self.theme, buf);
            panel_inners[i] = inner;
            Text::new(kind.name()).render(inner, buf);
            if inner.height > 1 {
                let hint = Rect {
                    x: inner.x,
                    y: inner.y + 1,
                    width: inner.width,
                    height: 1,
                };
                Text::new("(not yet built)").render(hint, buf);
            }
        }

        let overlay = buf.push_layer();
        for (i, gb) in self.glitches.iter().enumerate() {
            if gb.is_active() {
                gb.render(
                    panel_inners[i],
                    self.theme.tertiary,
                    self.tick_count,
                    overlay,
                );
            }
        }
        self.particles.render(overlay);
    }

    fn render_starfield(&self, area: Rect, buf: &mut LayerStack) {
        let center_x = area.x as f32 + area.width as f32 / 2.0;
        let center_y = area.y as f32 + area.height as f32 / 2.0;
        for star in &self.stars {
            let p = Point3 {
                x: star.x,
                y: star.y,
                z: star.z,
            };
            let Some((sx, sy, scale)) = self.camera.project(p, center_x, center_y) else {
                continue;
            };
            let x = sx.round();
            let y = sy.round();
            if x < area.x as f32
                || y < area.y as f32
                || x >= (area.x + area.width) as f32
                || y >= (area.y + area.height) as f32
            {
                continue;
            }
            let symbol = if scale > 3.0 {
                '@'
            } else if scale > 1.5 {
                '*'
            } else {
                '.'
            };
            let brightness = (scale * 50.0).clamp(25.0, 255.0) as u8;
            buf.set(
                x as u16,
                y as u16,
                Cell {
                    symbol,
                    fg: Color::Rgb {
                        r: brightness,
                        g: brightness,
                        b: brightness,
                    },
                    bg: Color::Reset,
                    alpha: 1.0,
                    ..Default::default()
                },
            );
        }
    }

    fn render_canopy(&self, area: Rect, buf: &mut LayerStack, edges_shown: usize) {
        let center_x = area.width as f32 / 2.0;
        let center_y = area.height as f32 / 2.0;
        let verts = canopy_vertices();
        let mut canvas = Canvas::new(area.width, area.height, CanvasMode::Braille);
        for &(a, b) in CANOPY_EDGES.iter().take(edges_shown) {
            let line = Line3 {
                start: verts[a],
                end: verts[b],
            };
            // Subtract one subpixel's worth from each clip bound (in cell-space
            // units, so 1/2 for the 2-subpixel-wide column and 1/4 for the
            // 4-subpixel-tall row) so the closed-interval clip boundary
            // (`project_line` clips against `[0, screen_w]`) maps to the last
            // valid subpixel column/row instead of one past it — see the
            // "cosmetic quirk" doc comment on `Camera::project_line` in
            // src/perspective.rs. Without this, a clipped endpoint landing exactly
            // on the boundary silently drops (Canvas::set_pixel's bounds check),
            // which at 80 columns culls one of the canopy's near-rectangle pillars.
            if let Some((x0, y0, x1, y1)) = self.camera.project_line(
                line,
                center_x,
                center_y,
                area.width as f32 - 1.0 / 2.0,
                area.height as f32 - 1.0 / 4.0,
                2.0,
                4.0,
                0.0,
            ) {
                canvas.line(x0, y0, x1, y1, self.theme.secondary);
            }
        }
        canvas.blit(buf, area.x, area.y);
    }

    fn render_windshield(&self, area: Rect, buf: &mut LayerStack, canopy_edges_shown: usize) {
        let bg = Cell {
            symbol: ' ',
            fg: Color::Reset,
            bg: self.theme.background,
            alpha: 1.0,
            ..Default::default()
        };
        for y in 0..area.height {
            for x in 0..area.width {
                buf.set(area.x + x, area.y + y, bg.clone());
            }
        }
        self.render_starfield(area, buf);
        self.render_canopy(area, buf, canopy_edges_shown);
    }

    fn render_hud_hyperdrive(&self, area: Rect, buf: &mut LayerStack) {
        let center_x = area.width as f32 / 2.0;
        let center_y = area.height as f32 / 2.0;
        let mut canvas = Canvas::new(area.width, area.height, CanvasMode::Braille);
        let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
        for i in 0..HYPERDRIVE_DASH_COUNT {
            let t0 = i as f32 / HYPERDRIVE_DASH_COUNT as f32;
            let t1 = (i + 1) as f32 / HYPERDRIVE_DASH_COUNT as f32;
            let seg = Line3 {
                start: Point3 {
                    x: lerp(HYPERDRIVE_START.x, HYPERDRIVE_END.x, t0),
                    y: lerp(HYPERDRIVE_START.y, HYPERDRIVE_END.y, t0),
                    z: lerp(HYPERDRIVE_START.z, HYPERDRIVE_END.z, t0),
                },
                end: Point3 {
                    x: lerp(HYPERDRIVE_START.x, HYPERDRIVE_END.x, t1),
                    y: lerp(HYPERDRIVE_START.y, HYPERDRIVE_END.y, t1),
                    z: lerp(HYPERDRIVE_START.z, HYPERDRIVE_END.z, t1),
                },
            };
            let phase_offset = i as f32 * (std::f32::consts::TAU / HYPERDRIVE_DASH_COUNT as f32);
            let brightness = 0.3 + 0.7 * (0.5 + 0.5 * (self.hyperdrive_phase - phase_offset).sin());
            let color =
                ttui::easing::lerp_color(self.theme.background, self.theme.accent, brightness);
            if let Some((x0, y0, x1, y1)) = self.camera.project_line(
                seg,
                center_x,
                center_y,
                area.width as f32 - 1.0 / 2.0,
                area.height as f32 - 1.0 / 4.0,
                2.0,
                4.0,
                0.0,
            ) {
                canvas.line(x0, y0, x1, y1, color);
            }
        }
        canvas.blit(buf, area.x, area.y);
    }

    fn render_hud_sensors(&self, area: Rect, buf: &mut LayerStack) {
        let center_x = area.width as f32 / 2.0;
        let center_y = area.height as f32 / 2.0;
        let mut canvas = Canvas::new(area.width, area.height, CanvasMode::Braille);
        let center = Point3 {
            x: 0.0,
            y: 0.0,
            z: SENSOR_PLANE_Z,
        };
        for k in 0..=SENSOR_TRAIL_COUNT {
            let angle = self.sensor_sweep_angle - k as f32 * SENSOR_TRAIL_STEP;
            let tip = Point3 {
                x: SENSOR_RADIUS * angle.cos(),
                y: SENSOR_RADIUS * angle.sin(),
                z: SENSOR_PLANE_Z,
            };
            let brightness = 1.0 - (k as f32 / (SENSOR_TRAIL_COUNT + 1) as f32);
            let color =
                ttui::easing::lerp_color(self.theme.background, self.theme.secondary, brightness);
            let line = Line3 {
                start: center,
                end: tip,
            };
            if let Some((x0, y0, x1, y1)) = self.camera.project_line(
                line,
                center_x,
                center_y,
                area.width as f32 - 1.0 / 2.0,
                area.height as f32 - 1.0 / 4.0,
                2.0,
                4.0,
                0.0,
            ) {
                canvas.line(x0, y0, x1, y1, color);
            }
        }
        canvas.blit(buf, area.x, area.y);
    }
}

impl App for Falcon {
    fn update(&mut self, event: &Event) {
        let Event::Key(k) = event else { return };
        if k.kind != KeyEventKind::Press {
            return;
        }
        if k.code == KeyCode::Char('q') {
            self.quit = true;
            return;
        }
        if self.booting.is_some() {
            return;
        }
        match k.code {
            KeyCode::Tab => self.focused = (self.focused + 1) % PANELS.len(),
            KeyCode::BackTab => self.focused = (self.focused + PANELS.len() - 1) % PANELS.len(),
            KeyCode::Char(' ') if self.glitches[self.focused].is_active() => {
                self.glitches[self.focused].clear();
                let (_, console) = Self::windshield_console_split(self.last_area.get());
                let slots = Self::panel_slots(console);
                let panel_box = Self::panel_box(slots[self.focused], true);
                let cx = panel_box.x as f32 + panel_box.width as f32 / 2.0;
                let cy = panel_box.y as f32 + panel_box.height as f32 / 2.0;
                for i in 0..WHACK_SPARK_COUNT {
                    let angle = i as f32 * std::f32::consts::TAU / WHACK_SPARK_COUNT as f32;
                    self.particles.spawn(Particle {
                        x: cx,
                        y: cy,
                        vx: angle.cos() * 6.0,
                        vy: angle.sin() * 3.0,
                        symbol: '*',
                        color: self.theme.accent,
                        lifetime: Duration::from_millis(WHACK_SPARK_LIFETIME_MS),
                        age: Duration::ZERO,
                    });
                }
            }
            _ => {}
        }
    }

    fn view(&self, area: Rect, buf: &mut LayerStack) {
        self.last_area.set(area);
        if let Some(t) = &self.booting {
            self.render_boot(area, t.progress(), buf);
            return;
        }
        self.render_dashboard(area, buf);
        let (windshield, _) = Self::windshield_console_split(area);
        self.render_hud_hyperdrive(windshield, buf); // TEMPORARY — Task 4 replaces this with real focus-based dispatch
        self.render_hud_sensors(windshield, buf); // TEMPORARY — Task 4 replaces this with real focus-based dispatch
    }

    fn should_quit(&self) -> bool {
        self.quit
    }

    fn tick_rate(&self) -> Option<Duration> {
        Some(TICK_INTERVAL)
    }

    fn on_tick(&mut self, elapsed: Duration) {
        if let Some(t) = &mut self.booting {
            t.tick(elapsed);
            if t.is_complete() {
                self.booting = None;
            }
        }
        self.tick_count += 1;
        self.hyperdrive_phase = (self.hyperdrive_phase
            + HYPERDRIVE_PHASE_SPEED * elapsed.as_secs_f32())
            % std::f32::consts::TAU;
        self.sensor_sweep_angle = (self.sensor_sweep_angle
            + SENSOR_SWEEP_SPEED * elapsed.as_secs_f32())
            % std::f32::consts::TAU;
        for (i, gb) in self.glitches.iter_mut().enumerate() {
            gb.tick(elapsed);
            if !gb.is_active() && self.tick_count % IDLE_FLICKER_PERIOD_TICKS == i as u64 * 30 {
                gb.trigger(Duration::from_millis(IDLE_FLICKER_DURATION_MS));
            }
        }
        self.particles.update(elapsed);
        let dz = STAR_SPEED * elapsed.as_secs_f32();
        for (i, star) in self.stars.iter_mut().enumerate() {
            star.z -= dz;
            if star.z <= self.camera.near {
                let seed = i as u32;
                star.z = STAR_RESPAWN_Z;
                star.x = scatter(seed.wrapping_add(self.tick_count as u32), 16.0);
                star.y = scatter(
                    seed.wrapping_add(self.tick_count as u32)
                        .wrapping_add(1_000),
                    10.0,
                );
            }
        }
    }
}
