use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::style::Color;
use std::time::Duration;
use ttui::app::App;
use ttui::buffer::{Cell, LayerStack};
use ttui::glitch::GlitchBuffer;
use ttui::layout::{Constraint, Direction, Layout, Rect};
use ttui::particles::{Particle, ParticleSystem};
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

pub(crate) struct Falcon {
    theme: Theme,
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
        Falcon {
            theme: falcon_theme(),
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
        let base_w = slot.width.saturating_sub(2).max(8);
        let base_h = slot.height.saturating_sub(4).clamp(4, 10);
        let focus_w = (base_w + 4).min(slot.width.saturating_sub(1));
        let focus_h = (base_h + 2).min(slot.height.saturating_sub(1));
        let box_w = if focused { focus_w } else { base_w };
        let box_h = if focused { focus_h } else { base_h };
        Rect {
            x: slot.x + slot.width.saturating_sub(box_w) / 2,
            y: slot.y + slot.height.saturating_sub(box_h) / 2,
            width: box_w,
            height: box_h,
        }
    }

    fn render_dashboard(&self, area: Rect, buf: &mut LayerStack) {
        let bg = Cell {
            symbol: ' ',
            fg: self.theme.primary,
            bg: self.theme.background,
            alpha: 1.0,
            ..Default::default()
        };
        for y in 0..area.height {
            for x in 0..area.width {
                buf.set(area.x + x, area.y + y, bg.clone());
            }
        }

        let slots = Self::panel_slots(area);
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

        buf.push_layer();
        for (i, gb) in self.glitches.iter().enumerate() {
            if gb.is_active() {
                gb.render(panel_inners[i], self.theme.tertiary, self.tick_count, buf);
            }
        }
        self.particles.render(buf);
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
            KeyCode::Char(' ') => {
                if self.glitches[self.focused].is_active() {
                    self.glitches[self.focused].clear();
                    let slots = Self::panel_slots(self.last_area.get());
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
        for (i, gb) in self.glitches.iter_mut().enumerate() {
            gb.tick(elapsed);
            if !gb.is_active() && self.tick_count % IDLE_FLICKER_PERIOD_TICKS == i as u64 * 30 {
                gb.trigger(Duration::from_millis(IDLE_FLICKER_DURATION_MS));
            }
        }
        self.particles.update(elapsed);
    }
}
