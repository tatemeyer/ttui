// examples/tardis.rs
use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::style::Color;
use rodio::Source;
use std::time::Duration;
use ttui::app::{run, App};
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
    fn new() -> Self {
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
    booting: Option<Transition>,
    tick_count: u64,
    audio: RodioAudioSink,
    psychic_log: Vec<(RelaySpeaker, String)>,
    psychic_prompt_index: usize,
    psychic_send_count: u32,
    psychic_pending: Option<(bool, Transition)>,
    psychic_reveal: Option<Transition>,
    quit: bool,
}

impl Tardis {
    fn new() -> Self {
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

    fn render_psychic_paper(&self, area: Rect, buf: &mut LayerStack) {
        for y in 0..area.height {
            for x in 0..area.width {
                buf.set(
                    area.x + x,
                    area.y + y,
                    Cell {
                        symbol: ' ',
                        fg: Color::Reset,
                        bg: PAPER_COLOR,
                        ..Default::default()
                    },
                );
            }
        }

        let start = self.psychic_log.len().saturating_sub(5);
        let last_index = self.psychic_log.len().saturating_sub(1);
        for (i, (speaker, text)) in self.psychic_log[start..].iter().enumerate() {
            let absolute_index = start + i;
            let prefix = match speaker {
                RelaySpeaker::User => "You: ",
                RelaySpeaker::Agent => "Relay: ",
            };
            let is_latest_agent = *speaker == RelaySpeaker::Agent
                && !self.psychic_log.is_empty()
                && absolute_index == last_index;
            let fg = if is_latest_agent {
                match &self.psychic_reveal {
                    Some(t) => lerp_color(PAPER_COLOR, INK_COLOR, t.progress()),
                    None => INK_COLOR,
                }
            } else {
                INK_COLOR
            };
            render_ink_row(buf, area, i as u16, &format!("{prefix}{text}"), fg);

            if is_latest_agent && self.glitch.is_active() && (i as u16) < area.height {
                let glitch_row = Rect {
                    x: area.x,
                    y: area.y + i as u16,
                    width: area.width,
                    height: 1,
                };
                self.glitch
                    .render(glitch_row, Color::Red, self.tick_count, buf);
            }
        }

        render_ink_row(
            buf,
            area,
            area.height.saturating_sub(2),
            PSYCHIC_PROMPTS[self.psychic_prompt_index],
            INK_COLOR,
        );
        render_ink_row(
            buf,
            area,
            area.height.saturating_sub(1),
            "Tab cycle * Enter send * Esc back * q quit",
            INK_COLOR,
        );
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
            Screen::StarCharts => self.render_placeholder(screen, local, &mut stack),
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

    fn render_police_box(
        &self,
        area: Rect,
        lines: &[&str; 5],
        dx: i16,
        dy: i16,
        buf: &mut LayerStack,
    ) {
        let box_width: i32 = 8;
        let box_height: i32 = 5;
        let x0 = area.x as i32 + (area.width as i32 - box_width) / 2 + dx as i32;
        let y0 = area.y as i32 + (area.height as i32 - box_height) / 2 + dy as i32;
        for (row, line) in lines.iter().enumerate() {
            for (col, ch) in line.chars().enumerate() {
                let px = x0 + col as i32;
                let py = y0 + row as i32;
                if px >= area.x as i32
                    && py >= area.y as i32
                    && (px as u16) < area.x + area.width
                    && (py as u16) < area.y + area.height
                {
                    buf.set(
                        px as u16,
                        py as u16,
                        Cell {
                            symbol: ch,
                            fg: self.theme.tertiary,
                            bg: Color::Reset,
                            ..Default::default()
                        },
                    );
                }
            }
        }
    }

    fn render_boot(&self, area: Rect, progress: f32, buf: &mut LayerStack) {
        for y in 0..area.height {
            for x in 0..area.width {
                buf.set(
                    area.x + x,
                    area.y + y,
                    Cell {
                        symbol: ' ',
                        fg: Color::Reset,
                        bg: Color::Black,
                        ..Default::default()
                    },
                );
            }
        }

        if progress < 0.15 {
            self.render_police_box(area, &POLICE_BOX_CLOSED, 0, 0, buf);
            return;
        }
        if progress < 0.35 {
            let magnitude: i16 = 2;
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
            self.render_police_box(area, &POLICE_BOX_CLOSED, dx, dy, buf);
            return;
        }
        if progress < 0.5 {
            self.render_police_box(area, &POLICE_BOX_OPEN, 0, 0, buf);
            return;
        }
        if progress < 0.65 {
            for y in 0..area.height {
                for x in 0..area.width {
                    buf.set(
                        area.x + x,
                        area.y + y,
                        Cell {
                            symbol: ' ',
                            fg: Color::Reset,
                            bg: Color::Rgb {
                                r: 255,
                                g: 255,
                                b: 255,
                            },
                            ..Default::default()
                        },
                    );
                }
            }
            return;
        }

        let push_progress = ((progress - 0.65) / 0.35).clamp(0.0, 1.0);
        let zoom = easing::ease_out(1.0, 2.2, push_progress);
        let local = Rect {
            x: 0,
            y: 0,
            width: area.width,
            height: area.height,
        };
        let mut hub_stack = LayerStack::new(area.width, area.height);
        self.render_hub(local, &mut hub_stack);
        let cam = Camera::new(
            area.width as f32 / 2.0 * (1.0 - 1.0 / zoom),
            area.height as f32 / 2.0 * (1.0 - 1.0 / zoom),
            zoom,
        );
        let zoomed = camera::viewport(&hub_stack, &cam, area.width, area.height);
        blit(&zoomed, area, buf);
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
                KeyCode::Enter => {
                    if self.face_tween.is_none() {
                        if let Some(dest) = screen_for_face(self.selected_face) {
                            self.transitioning_to = Some((
                                dest,
                                Transition::start(Duration::from_millis(FLIGHT_TRANSITION_MS)),
                            ));
                            self.audio.play("flight");
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
                if k.code == KeyCode::Esc {
                    self.screen = Screen::Hub;
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
            Screen::StarCharts => self.render_placeholder(self.screen, area, buf),
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

fn main() -> std::io::Result<()> {
    let mut app = Tardis::new();
    run(&mut app)
}
