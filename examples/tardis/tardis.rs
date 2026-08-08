// examples/tardis/main.rs
use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::style::Color;
use rodio::Source;
use std::time::Duration;
use ttui::app::App;
use ttui::audio::AudioSink;
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

#[path = "artron_energy.rs"]
mod artron_energy;
#[path = "boot.rs"]
mod boot;
#[path = "hub.rs"]
mod hub;
#[path = "psychic_paper.rs"]
mod psychic_paper;
#[path = "star_charts.rs"]
mod star_charts;

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
const BOOT_MS: u64 = 3000;

const POLICE_BOX_CLOSED: [&str; 5] = ["+------+", "|POLICE|", "|BOX   |", "|[DOOR]|", "+------+"];
const POLICE_BOX_OPEN: [&str; 5] = ["+------+", "|POLICE|", "|BOX   |", "|[    ]|", "+------+"];

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

#[derive(Clone, Copy, PartialEq)]
enum RelaySpeaker {
    User,
    Agent,
}

const PSYCHIC_PROMPTS: [&str; 3] = [
    "Status of the away team",
    "Translate this inscription",
    "Locate the temporal anomaly",
];
const PSYCHIC_THINKING_MS: u64 = 800;
const PSYCHIC_REVEAL_MS: u64 = 800;
const PSYCHIC_GLITCH_EVERY: u32 = 3;
const PSYCHIC_GLITCH_DURATION_MS: u64 = 600;
const TIMELINE: [&str; 5] = [
    "Draft proposal",
    "Review PR",
    "Deploy hotfix",
    "Write docs",
    "Plan sprint",
];
const TEMPORAL_SHIFT_MS: u64 = 400;
const CLOUD_GLYPHS: [char; 4] = ['?', '~', '·', '#'];
const PAPER_COLOR: Color = Color::Rgb {
    r: 230,
    g: 225,
    b: 210,
};
const INK_COLOR: Color = Color::Rgb {
    r: 20,
    g: 20,
    b: 40,
};

struct RodioAudioSink {
    sink: Option<rodio::stream::MixerDeviceSink>,
}

impl RodioAudioSink {
    pub(crate) fn new() -> Self {
        match rodio::stream::DeviceSinkBuilder::open_default_sink() {
            Ok(sink) => {
                let hum = rodio::source::SineWave::new(80.0)
                    .take_duration(Duration::from_secs(2))
                    .amplify(0.05)
                    .repeat_infinite();
                sink.mixer().add(hum);
                RodioAudioSink { sink: Some(sink) }
            }
            Err(_) => RodioAudioSink { sink: None },
        }
    }
}

impl AudioSink for RodioAudioSink {
    fn play(&mut self, event_id: &str) {
        let Some(sink) = &self.sink else { return };
        let freq: f32 = match event_id {
            "boot" => 100.0,
            "flight" => 300.0,
            "vent" => 500.0,
            "glitch" => 700.0,
            _ => return,
        };
        let source = rodio::source::SineWave::new(freq)
            .take_duration(Duration::from_millis(200))
            .amplify(0.15);
        sink.mixer().add(source);
    }
}

pub(crate) struct Tardis {
    theme: Theme,
    screen: Screen,
    selected_face: usize,
    face_tween: Option<(f32, Transition)>,
    energy: f32,
    vent_flash: Option<Transition>,
    glitch: GlitchBuffer,
    particles: ParticleSystem,
    transitioning_to: Option<(Screen, Transition)>,
    booting: Option<Transition>,
    tick_count: u64,
    audio: RodioAudioSink,
    psychic_log: Vec<(RelaySpeaker, String)>,
    psychic_prompt_index: usize,
    psychic_send_count: u32,
    psychic_pending: Option<(bool, Transition)>,
    psychic_reveal: Option<Transition>,
    present_index: usize,
    temporal_shift: Option<Transition>,
    quit: bool,
}

impl Tardis {
    pub(crate) fn new() -> Self {
        let mut tardis = Tardis {
            theme: tardis_theme(),
            screen: Screen::Hub,
            selected_face: 0,
            face_tween: None,
            energy: 0.0,
            vent_flash: None,
            glitch: GlitchBuffer::new(),
            particles: ParticleSystem::new(),
            transitioning_to: None,
            booting: Some(Transition::start(Duration::from_millis(BOOT_MS))),
            tick_count: 0,
            audio: RodioAudioSink::new(),
            psychic_log: Vec::new(),
            psychic_prompt_index: 0,
            psychic_send_count: 0,
            psychic_pending: None,
            psychic_reveal: None,
            present_index: 2,
            temporal_shift: None,
            quit: false,
        };
        tardis.audio.play("boot");
        tardis
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
            Screen::PsychicPaper => self.render_psychic_paper(local, &mut stack),
            Screen::StarCharts => self.render_star_charts(local, &mut stack),
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

fn lerp_color(from: Color, to: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    match (from, to) {
        (
            Color::Rgb {
                r: r1,
                g: g1,
                b: b1,
            },
            Color::Rgb {
                r: r2,
                g: g2,
                b: b2,
            },
        ) => Color::Rgb {
            r: (r1 as f32 + (r2 as f32 - r1 as f32) * t) as u8,
            g: (g1 as f32 + (g2 as f32 - g1 as f32) * t) as u8,
            b: (b1 as f32 + (b2 as f32 - b1 as f32) * t) as u8,
        },
        _ => to,
    }
}

fn render_ink_row(buf: &mut LayerStack, area: Rect, y: u16, text: &str, fg: Color) {
    if y >= area.height {
        return;
    }
    for (i, ch) in text.chars().take(area.width as usize).enumerate() {
        buf.set(
            area.x + i as u16,
            area.y + y,
            Cell {
                symbol: ch,
                fg,
                bg: PAPER_COLOR,
                ..Default::default()
            },
        );
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
        if self.booting.is_some() {
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
                KeyCode::Enter if self.face_tween.is_none() => {
                    if let Some(dest) = screen_for_face(self.selected_face) {
                        self.transitioning_to = Some((
                            dest,
                            Transition::start(Duration::from_millis(FLIGHT_TRANSITION_MS)),
                        ));
                        self.audio.play("flight");
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
                    self.audio.play("vent");
                }
                _ => {}
            },
            Screen::PsychicPaper => {
                if self.psychic_pending.is_some() {
                    return;
                }
                match k.code {
                    KeyCode::Tab => {
                        self.psychic_prompt_index =
                            (self.psychic_prompt_index + 1) % PSYCHIC_PROMPTS.len();
                    }
                    KeyCode::BackTab => {
                        self.psychic_prompt_index =
                            (self.psychic_prompt_index + PSYCHIC_PROMPTS.len() - 1)
                                % PSYCHIC_PROMPTS.len();
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        self.psychic_log.push((
                            RelaySpeaker::User,
                            PSYCHIC_PROMPTS[self.psychic_prompt_index].to_string(),
                        ));
                        self.psychic_send_count += 1;
                        let will_glitch =
                            self.psychic_send_count.is_multiple_of(PSYCHIC_GLITCH_EVERY);
                        self.psychic_pending = Some((
                            will_glitch,
                            Transition::start(Duration::from_millis(PSYCHIC_THINKING_MS)),
                        ));
                    }
                    KeyCode::Esc => self.screen = Screen::Hub,
                    _ => {}
                }
            }
            Screen::StarCharts => {
                if self.temporal_shift.is_some() {
                    return;
                }
                match k.code {
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        self.present_index = (self.present_index + 1) % TIMELINE.len();
                        self.temporal_shift =
                            Some(Transition::start(Duration::from_millis(TEMPORAL_SHIFT_MS)));
                    }
                    KeyCode::Esc => self.screen = Screen::Hub,
                    _ => {}
                }
            }
        }
    }

    fn view(&self, area: Rect, buf: &mut LayerStack) {
        if let Some(t) = &self.booting {
            self.render_boot(area, t.progress(), buf);
            return;
        }
        if let Some((destination, transition)) = &self.transitioning_to {
            self.render_transition(*destination, area, transition.progress(), buf);
            return;
        }
        match self.screen {
            Screen::Hub => self.render_hub(area, buf),
            Screen::ArtronEnergy => self.render_artron_energy(area, buf),
            Screen::PsychicPaper => self.render_psychic_paper(area, buf),
            Screen::StarCharts => self.render_star_charts(area, buf),
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

        if let Some((will_glitch, t)) = &mut self.psychic_pending {
            t.tick(elapsed);
            if t.is_complete() {
                if *will_glitch {
                    self.psychic_log
                        .push((RelaySpeaker::Agent, "...signal lost...".to_string()));
                    self.glitch
                        .trigger(Duration::from_millis(PSYCHIC_GLITCH_DURATION_MS));
                    self.audio.play("glitch");
                } else {
                    let prompt = PSYCHIC_PROMPTS[self.psychic_prompt_index];
                    self.psychic_log
                        .push((RelaySpeaker::Agent, format!("{prompt} — relay confirmed.")));
                    self.psychic_reveal =
                        Some(Transition::start(Duration::from_millis(PSYCHIC_REVEAL_MS)));
                }
                self.psychic_pending = None;
            }
        }
        if let Some(t) = &mut self.psychic_reveal {
            t.tick(elapsed);
            if t.is_complete() {
                self.psychic_reveal = None;
            }
        }

        if let Some(t) = &mut self.temporal_shift {
            t.tick(elapsed);
            if t.is_complete() {
                self.temporal_shift = None;
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

        if let Some(t) = &mut self.booting {
            t.tick(elapsed);
            if t.is_complete() {
                self.booting = None;
            }
        }
    }
}
