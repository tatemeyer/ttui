// examples/tardis.rs
use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::style::Color;
use std::time::Duration;
use ttui::app::{run, App};
use ttui::buffer::{Buffer, Cell, LayerStack};
use ttui::camera::{self, Camera};
use ttui::easing;
use ttui::effects;
use ttui::glitch::GlitchBuffer;
use ttui::layout::Rect;
use ttui::particles::{Particle, ParticleSystem};
use ttui::theme::{BorderSet, Theme};
use ttui::transition::Transition;
use ttui::widgets::{
    analog_toggle::AnalogToggle, roundel::Roundel, text::Text, time_rotor::TimeRotor,
};

const TICK_INTERVAL: Duration = Duration::from_millis(33);
const FACE_COUNT: usize = 6;
const FACE_NAMES: [&str; 6] = [
    "Psychic Paper",
    "Auxiliary Roundel Bay",
    "Star Charts",
    "Auxiliary Roundel Bay",
    "Artron Energy",
    "Auxiliary Roundel Bay",
];
const ROTATE_TWEEN_MS: u64 = 200;
const DIM_FACTORS: [f32; 4] = [0.0, 0.35, 0.65, 0.85];
const ENERGY_GAIN_PER_HIT: f32 = 12.0;
const ENERGY_VENT_AMOUNT: f32 = 35.0;
const ENERGY_DECAY_PER_SEC: f32 = 4.0;
const VENT_FLASH_MS: u64 = 300;
const VENTING_THRESHOLD: f32 = 80.0;
const LAG_THRESHOLD: f32 = 90.0;
const GLITCH_DURATION_MS: u64 = 500;
const LAGGING_TICK_INTERVAL: Duration = Duration::from_millis(66);
const FLIGHT_TRANSITION_MS: u64 = 900;

fn tardis_theme() -> Theme {
    Theme {
        background: Color::Rgb { r: 0, g: 0, b: 0 },
        primary: Color::Rgb {
            r: 0,
            g: 255,
            b: 20,
        },
        secondary: Color::Rgb {
            r: 184,
            g: 115,
            b: 51,
        },
        tertiary: Color::Rgb {
            r: 0,
            g: 255,
            b: 255,
        },
        accent: Color::Rgb {
            r: 255,
            g: 191,
            b: 0,
        },
        border: BorderSet {
            horizontal: '=',
            vertical: '#',
            corner: '+',
        },
        border_bold: false,
        border_thick: false,
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Hub,
    PsychicPaper,
    StarCharts,
    ArtronEnergy,
}

fn screen_for_face(face: usize) -> Option<Screen> {
    match face {
        0 => Some(Screen::PsychicPaper),
        2 => Some(Screen::StarCharts),
        4 => Some(Screen::ArtronEnergy),
        _ => None,
    }
}

fn hex_distance(a: usize, b: usize) -> usize {
    let diff = a.abs_diff(b);
    diff.min(FACE_COUNT - diff)
}

struct Tardis {
    theme: Theme,
    screen: Screen,
    selected_face: usize,
    face_tween: Option<(f32, Transition)>,
    energy: f32,
    vent_flash: Option<Transition>,
    glitch: GlitchBuffer,
    particles: ParticleSystem,
    transitioning_to: Option<(Screen, Transition)>,
    tick_count: u64,
    quit: bool,
}

impl Tardis {
    fn new() -> Self {
        Tardis {
            theme: tardis_theme(),
            screen: Screen::Hub,
            selected_face: 0,
            face_tween: None,
            energy: 0.0,
            vent_flash: None,
            glitch: GlitchBuffer::new(),
            particles: ParticleSystem::new(),
            transitioning_to: None,
            tick_count: 0,
            quit: false,
        }
    }

    fn displayed_face_index(&self) -> f32 {
        match &self.face_tween {
            Some((from, t)) => easing::ease_out(*from, self.selected_face as f32, t.progress()),
            None => self.selected_face as f32,
        }
    }

    fn time_rotor_speed(&self) -> f32 {
        1.0 + self.energy / 50.0
    }

    fn is_lagging(&self) -> bool {
        self.energy >= LAG_THRESHOLD
    }

    fn render_face_content(&self, face: usize, area: Rect, buf: &mut Buffer) {
        let name_row = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height.min(1),
        };
        Text::new(FACE_NAMES[face]).render(name_row, buf);
        if screen_for_face(face).is_none() {
            for i in 0..3u16 {
                let rx = area.x + (area.width / 4) * (i + 1);
                let ry = area.y + area.height / 2;
                let pulse = ((self.tick_count as f32 * 0.05 + i as f32).sin() + 1.0) / 2.0;
                Roundel::new(pulse, self.theme.tertiary).render(
                    Rect {
                        x: rx,
                        y: ry,
                        width: 1,
                        height: 1,
                    },
                    buf,
                );
            }
        }
    }

    fn render_hub(&self, area: Rect, buf: &mut LayerStack) {
        let vw = area.width;
        let vh = area.height;
        let mut virtual_buf = Buffer::new(vw * FACE_COUNT as u16, vh);
        for face in 0..FACE_COUNT {
            let face_area = Rect {
                x: face as u16 * vw,
                y: 0,
                width: vw,
                height: vh,
            };
            self.render_face_content(face, face_area, &mut virtual_buf);
            let factor = DIM_FACTORS[hex_distance(face, self.selected_face)];
            if factor > 0.0 {
                let face_camera = Camera::new(face_area.x as f32, face_area.y as f32, 1.0);
                let cropped = camera::viewport(&virtual_buf, &face_camera, vw, vh);
                let dimmed = camera::dim(&cropped, factor);
                blit(&dimmed, face_area, &mut virtual_buf);
            }
        }
        let cam = Camera::new(self.displayed_face_index() * vw as f32, 0.0, 1.0);
        let view = camera::viewport(&virtual_buf, &cam, vw, vh);
        blit(&view, area, buf);

        let rotor_area = Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width,
            height: area.height.saturating_sub(2),
        };
        TimeRotor::new(self.time_rotor_speed()).render(rotor_area, self.tick_count, buf);

        let hint_row = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(1),
            width: area.width,
            height: area.height.saturating_sub(1).min(1),
        };
        Text::new("Left/Right rotate * Enter select * q quit").render(hint_row, buf);
    }

    fn render_placeholder(&self, screen: Screen, area: Rect, buf: &mut LayerStack) {
        let name = match screen {
            Screen::PsychicPaper => "Psychic Paper",
            Screen::StarCharts => "Star Charts",
            Screen::ArtronEnergy => "Artron Energy",
            Screen::Hub => "",
        };
        let name_row = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height.min(1),
        };
        let placeholder_row = Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width,
            height: area.height.saturating_sub(2),
        };
        let hint_row = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(1),
            width: area.width,
            height: area.height.saturating_sub(1).min(1),
        };
        Text::new(name).render(name_row, buf);
        Text::new("(not yet built)").render(placeholder_row, buf);
        Text::new("Esc back * q quit").render(hint_row, buf);
    }

    fn render_artron_energy(&self, area: Rect, buf: &mut LayerStack) {
        let name_row = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height.min(1),
        };
        Text::new("Artron Energy").render(name_row, buf);

        for i in 0..3u16 {
            let seg_intensity = ((self.energy - i as f32 * 33.0) / 33.0).clamp(0.0, 1.0);
            let rx = area.x + 4 + i * 4;
            let ry = area.y + 2;
            Roundel::new(seg_intensity, self.theme.tertiary).render(
                Rect {
                    x: rx,
                    y: ry,
                    width: 1,
                    height: 1,
                },
                buf,
            );
        }

        let toggle_row = Rect {
            x: area.x,
            y: area.y + 4,
            width: area.width.min(10),
            height: 1,
        };
        AnalogToggle::new(self.vent_flash.is_some()).render(toggle_row, buf);

        let rotor_area = Rect {
            x: area.x,
            y: area.y + 6,
            width: area.width,
            height: area.height.saturating_sub(8),
        };
        TimeRotor::new(self.time_rotor_speed()).render(rotor_area, self.tick_count, buf);

        if self.glitch.is_active() {
            self.glitch.render(area, Color::Red, self.tick_count, buf);
        }

        self.particles.render(buf);

        let hint_row = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(1),
            width: area.width,
            height: area.height.saturating_sub(1).min(1),
        };
        Text::new("Space channel * v vent * Esc back * q quit").render(hint_row, buf);
    }

    fn render_destination_preview(&self, screen: Screen, area: Rect) -> Buffer {
        let local = Rect {
            x: 0,
            y: 0,
            width: area.width,
            height: area.height,
        };
        let mut stack = LayerStack::new(area.width, area.height);
        match screen {
            Screen::ArtronEnergy => self.render_artron_energy(local, &mut stack),
            Screen::PsychicPaper | Screen::StarCharts => {
                self.render_placeholder(screen, local, &mut stack)
            }
            Screen::Hub => self.render_hub(local, &mut stack),
        }
        let mut out = Buffer::new(area.width, area.height);
        blit(&stack, local, &mut out);
        out
    }

    fn render_transition(&self, destination: Screen, area: Rect, progress: f32, buf: &mut Buffer) {
        if progress < 0.3 {
            let local = Rect {
                x: 0,
                y: 0,
                width: area.width,
                height: area.height,
            };
            let mut stack = LayerStack::new(area.width, area.height);
            self.render_hub(local, &mut stack);
            let magnitude: i16 = 1 + (progress / 0.3 * 2.0) as i16;
            let dx = if self.tick_count.is_multiple_of(2) {
                magnitude
            } else {
                -magnitude
            };
            let dy = if (self.tick_count / 2).is_multiple_of(2) {
                magnitude
            } else {
                -magnitude
            };
            let shaken = effects::shake(&stack, dx, dy);
            blit(&shaken, area, buf);
            return;
        }

        if progress < 0.85 {
            for y in 0..area.height {
                for x in 0..area.width {
                    buf.set(
                        area.x + x,
                        area.y + y,
                        Cell {
                            symbol: ' ',
                            fg: Color::Reset,
                            bg: Color::Rgb { r: 5, g: 0, b: 15 },
                            ..Default::default()
                        },
                    );
                }
            }
            let void_progress = ((progress - 0.3) / 0.4).clamp(0.0, 1.0);
            let count = (void_progress * 20.0) as usize;
            let cx = area.width as f32 / 2.0;
            let cy = area.height as f32 / 2.0;
            let max_dist = cx.max(cy);
            for i in 0..count {
                let angle = i as f32 * std::f32::consts::TAU / 20.0;
                let dist = void_progress * max_dist;
                let x = (cx + angle.cos() * dist).round();
                let y = (cy + angle.sin() * dist * 0.5).round();
                if x >= 0.0 && y >= 0.0 && (x as u16) < area.width && (y as u16) < area.height {
                    buf.set(
                        area.x + x as u16,
                        area.y + y as u16,
                        Cell {
                            symbol: '-',
                            fg: Color::Rgb {
                                r: 0,
                                g: 255,
                                b: 255,
                            },
                            bg: Color::Reset,
                            ..Default::default()
                        },
                    );
                }
            }
            return;
        }

        let content = self.render_destination_preview(destination, area);
        blit(&content, area, buf);
    }
}

fn blit(scratch: &Buffer, area: Rect, buf: &mut Buffer) {
    for y in 0..scratch.height {
        for x in 0..scratch.width {
            buf.set(area.x + x, area.y + y, scratch.get(x, y).clone());
        }
    }
}

impl App for Tardis {
    fn update(&mut self, event: &Event) {
        let Event::Key(k) = event else { return };
        if k.kind != KeyEventKind::Press {
            return;
        }
        if k.code == KeyCode::Char('q') {
            self.quit = true;
            return;
        }
        if self.transitioning_to.is_some() {
            return;
        }
        match self.screen {
            Screen::Hub => match k.code {
                KeyCode::Left => {
                    let from = self.displayed_face_index();
                    self.selected_face = (self.selected_face + FACE_COUNT - 1) % FACE_COUNT;
                    self.face_tween = Some((
                        from,
                        Transition::start(Duration::from_millis(ROTATE_TWEEN_MS)),
                    ));
                }
                KeyCode::Right => {
                    let from = self.displayed_face_index();
                    self.selected_face = (self.selected_face + 1) % FACE_COUNT;
                    self.face_tween = Some((
                        from,
                        Transition::start(Duration::from_millis(ROTATE_TWEEN_MS)),
                    ));
                }
                KeyCode::Enter => {
                    if self.face_tween.is_none() {
                        if let Some(dest) = screen_for_face(self.selected_face) {
                            self.transitioning_to = Some((
                                dest,
                                Transition::start(Duration::from_millis(FLIGHT_TRANSITION_MS)),
                            ));
                        }
                    }
                }
                _ => {}
            },
            Screen::ArtronEnergy => match k.code {
                KeyCode::Esc => self.screen = Screen::Hub,
                KeyCode::Char(' ') => {
                    self.energy += ENERGY_GAIN_PER_HIT;
                    if self.energy >= VENTING_THRESHOLD {
                        for i in 0..8 {
                            let angle = i as f32 * std::f32::consts::TAU / 8.0;
                            self.particles.spawn(Particle {
                                x: 10.0,
                                y: 4.0,
                                vx: angle.cos() * 10.0,
                                vy: angle.sin() * 5.0,
                                symbol: '*',
                                color: Color::Red,
                                lifetime: Duration::from_millis(500),
                                age: Duration::ZERO,
                            });
                        }
                    }
                }
                KeyCode::Char('v') => {
                    self.energy = (self.energy - ENERGY_VENT_AMOUNT).max(0.0);
                    self.vent_flash = Some(Transition::start(Duration::from_millis(VENT_FLASH_MS)));
                }
                _ => {}
            },
            Screen::PsychicPaper | Screen::StarCharts => {
                if k.code == KeyCode::Esc {
                    self.screen = Screen::Hub;
                }
            }
        }
    }

    fn view(&self, area: Rect, buf: &mut LayerStack) {
        if let Some((destination, transition)) = &self.transitioning_to {
            self.render_transition(*destination, area, transition.progress(), buf);
            return;
        }
        match self.screen {
            Screen::Hub => self.render_hub(area, buf),
            Screen::ArtronEnergy => self.render_artron_energy(area, buf),
            Screen::PsychicPaper | Screen::StarCharts => {
                self.render_placeholder(self.screen, area, buf)
            }
        }
    }

    fn should_quit(&self) -> bool {
        self.quit
    }

    fn tick_rate(&self) -> Option<Duration> {
        if self.is_lagging() {
            Some(LAGGING_TICK_INTERVAL)
        } else {
            Some(TICK_INTERVAL)
        }
    }

    fn on_tick(&mut self, elapsed: Duration) {
        self.tick_count += 1;
        if let Some((_, t)) = &mut self.face_tween {
            t.tick(elapsed);
            if t.is_complete() {
                self.face_tween = None;
            }
        }

        self.energy = (self.energy - ENERGY_DECAY_PER_SEC * elapsed.as_secs_f32()).max(0.0);

        if self.is_lagging() {
            self.glitch
                .trigger(Duration::from_millis(GLITCH_DURATION_MS));
        }
        self.glitch.tick(elapsed);

        if let Some(t) = &mut self.vent_flash {
            t.tick(elapsed);
            if t.is_complete() {
                self.vent_flash = None;
            }
        }

        self.particles.update(elapsed);

        if let Some((destination, t)) = &mut self.transitioning_to {
            t.tick(elapsed);
            if t.is_complete() {
                self.screen = *destination;
                self.transitioning_to = None;
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut app = Tardis::new();
    run(&mut app)
}
