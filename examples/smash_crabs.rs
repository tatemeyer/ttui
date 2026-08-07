// examples/smash_crabs.rs
use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::style::Color;
use rodio::Source;
use std::time::Duration;
use ttui::app::{run, App};
use ttui::audio::AudioSink;
use ttui::buffer::{Buffer, Cell, CellStyle, LayerStack};
use ttui::easing;
use ttui::effects;
use ttui::layout::{Constraint, Direction, Layout, Rect};
use ttui::particles::{Particle, ParticleSystem};
use ttui::theme::{BorderSet, Theme};
use ttui::transition::Transition;
use ttui::widgets::{
    damage_meter::DamageMeter, scuttle_cursor::ScuttleCursor, smash_border::SmashBorder, text::Text,
};

const BACKGROUND: usize = 0;
const UI: usize = 1;
const EFFECTS: usize = 2;

const TICK_INTERVAL: Duration = Duration::from_millis(33); // ~30 FPS, matches omnitrix
const FLASH_TICKS: u8 = 6; // ~200ms flash at 33ms/tick
const CURSOR_TWEEN_MS: u64 = 150;
const CURSOR_SYMBOL: char = 'C';
const SHAKE_TICKS: u8 = 6; // matches FLASH_TICKS's ~200ms feel
const DAMAGE_TWEEN_MS: u64 = 250;
const HIT_DAMAGE: u16 = 17;
const VS_TRANSITION_MS: u64 = 700;

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Hub,
    Versus,
    TargetSmash,
    StageHazards,
}

const FIGHTERS: [&str; 3] = ["Versus Mode", "Target Smash", "Stage Hazards"];

fn screen_for_selected(selected: usize) -> Screen {
    match selected {
        0 => Screen::Versus,
        1 => Screen::TargetSmash,
        _ => Screen::StageHazards,
    }
}

fn arena_theme() -> Theme {
    Theme {
        background: Color::Rgb {
            r: 92,
            g: 64,
            b: 20,
        }, // packed-sand arena floor
        primary: Color::Red,    // crab shell red
        secondary: Color::Cyan, // water
        tertiary: Color::White,
        accent: Color::Yellow,
        border: BorderSet {
            horizontal: '=',
            vertical: '|',
            corner: '+',
        },
        border_bold: false,
        border_thick: false,
    }
}

struct RodioAudioSink {
    sink: Option<rodio::stream::MixerDeviceSink>,
}

impl RodioAudioSink {
    fn new() -> Self {
        match rodio::stream::DeviceSinkBuilder::open_default_sink() {
            Ok(sink) => RodioAudioSink { sink: Some(sink) },
            Err(_) => RodioAudioSink { sink: None },
        }
    }
}

impl AudioSink for RodioAudioSink {
    fn play(&mut self, event_id: &str) {
        let Some(sink) = &self.sink else { return };
        let freq: f32 = match event_id {
            "cursor" => 440.0,
            "select" => 660.0,
            "hit" => 220.0,
            _ => return,
        };
        let source = rodio::source::SineWave::new(freq)
            .take_duration(Duration::from_millis(120))
            .amplify(0.2);
        sink.mixer().add(source);
    }
}

struct SmashCrabs {
    theme: Theme,
    screen: Screen,
    selected: usize,
    cursor_tween: Option<(f32, Transition)>,
    transitioning_to: Option<(Screen, Transition)>,
    p2_damage: u16,
    damage_tween: Option<(f32, Transition)>,
    flash_ticks_remaining: u8,
    shake_ticks_remaining: u8,
    particles: ParticleSystem,
    tick_count: u64,
    audio: RodioAudioSink,
    quit: bool,
}

impl SmashCrabs {
    fn new() -> Self {
        SmashCrabs {
            theme: arena_theme(),
            screen: Screen::Hub,
            selected: 0,
            cursor_tween: None,
            transitioning_to: None,
            p2_damage: 0,
            damage_tween: None,
            flash_ticks_remaining: 0,
            shake_ticks_remaining: 0,
            particles: ParticleSystem::new(),
            tick_count: 0,
            audio: RodioAudioSink::new(),
            quit: false,
        }
    }

    fn displayed_cursor_index(&self) -> f32 {
        match &self.cursor_tween {
            Some((from, t)) => easing::ease_out(*from, self.selected as f32, t.progress()),
            None => self.selected as f32,
        }
    }

    fn hub_panels(area: Rect) -> Vec<Rect> {
        Layout::new(
            Direction::Horizontal,
            vec![Constraint::Fill(1); FIGHTERS.len()],
        )
        .split(area)
    }

    fn cursor_position(&self, area: Rect) -> (f32, f32) {
        let panels = Self::hub_panels(area);
        let centers: Vec<f32> = panels
            .iter()
            .map(|p| p.x as f32 + p.width as f32 / 2.0)
            .collect();
        let index = self.displayed_cursor_index();
        let lo = (index.floor() as usize).min(centers.len() - 1);
        let hi = (lo + 1).min(centers.len() - 1);
        let frac = index - lo as f32;
        let x = easing::lerp(centers[lo], centers[hi], frac);
        let y = area.y as f32 + area.height as f32 - 2.0;
        (x, y)
    }

    fn render_hub(&self, area: Rect, buf: &mut LayerStack) {
        let inner = SmashBorder::new().render(area, &self.theme, buf);
        let panels = Self::hub_panels(inner);
        for (i, panel) in panels.iter().enumerate() {
            let name_row = Rect {
                x: panel.x,
                y: panel.y,
                width: panel.width,
                height: panel.height.min(1),
            };
            Text::new(FIGHTERS[i]).render(name_row, buf);
        }
        let (cx, cy) = self.cursor_position(inner);
        ScuttleCursor::new(CURSOR_SYMBOL).render(
            cx,
            cy,
            self.cursor_tween.is_some(),
            self.tick_count,
            buf,
        );
        let hint_row = Rect {
            x: inner.x,
            y: inner.y + inner.height.saturating_sub(1),
            width: inner.width,
            height: inner.height.saturating_sub(1).min(1),
        };
        Text::new("Left/Right move * Enter select * q quit").render(hint_row, buf);
    }

    fn render_placeholder(&self, screen: Screen, area: Rect, buf: &mut LayerStack) {
        let inner = SmashBorder::new().render(area, &self.theme, buf);
        let name = match screen {
            Screen::TargetSmash => "Target Smash",
            Screen::StageHazards => "Stage Hazards",
            _ => "",
        };
        let name_row = Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: inner.height.min(1),
        };
        let placeholder_row = Rect {
            x: inner.x,
            y: inner.y + 1,
            width: inner.width,
            height: inner.height.saturating_sub(2),
        };
        let hint_row = Rect {
            x: inner.x,
            y: inner.y + inner.height.saturating_sub(1),
            width: inner.width,
            height: inner.height.saturating_sub(1).min(1),
        };
        Text::new(name).render(name_row, buf);
        Text::new("(not yet built)").render(placeholder_row, buf);
        Text::new("Esc back * q quit").render(hint_row, buf);
    }

    fn displayed_p2_damage(&self) -> f32 {
        match &self.damage_tween {
            Some((from, t)) => easing::ease_out(*from, self.p2_damage as f32, t.progress()),
            None => self.p2_damage as f32,
        }
    }

    fn shake_offset(&self) -> (i16, i16) {
        if self.shake_ticks_remaining == 0 {
            return (0, 0);
        }
        let magnitude = (((self.shake_ticks_remaining as i16) + 1) / 2).min(2);
        let dx = if self.shake_ticks_remaining.is_multiple_of(2) {
            magnitude
        } else {
            -magnitude
        };
        let dy = if (self.shake_ticks_remaining / 2).is_multiple_of(2) {
            magnitude
        } else {
            -magnitude
        };
        (dx, dy)
    }

    fn paint_background(&self, area: Rect) -> Buffer {
        let mut buf = Buffer::new(area.width, area.height);
        let cell = Cell {
            symbol: ' ',
            fg: self.theme.primary,
            bg: self.theme.background,
            ..Default::default()
        };
        for y in 0..area.height {
            for x in 0..area.width {
                buf.set(x, y, cell.clone());
            }
        }
        buf
    }

    fn paint_ui(&self, area: Rect) -> Buffer {
        let mut buf = Buffer::new(area.width, area.height);
        let local = Rect {
            x: 0,
            y: 0,
            width: area.width,
            height: area.height,
        };
        let panel = Layout::new(Direction::Vertical, vec![Constraint::Fixed(8)]).split(local)[0];
        let panel = Rect {
            width: panel.width.min(24),
            ..panel
        };
        let inner = SmashBorder::new().render(panel, &self.theme, &mut buf);
        let rows = Layout::new(
            Direction::Vertical,
            vec![Constraint::Fixed(1), Constraint::Fixed(1)],
        )
        .split(inner);
        DamageMeter::new(0).render(rows[0], &mut buf);
        DamageMeter::new(self.displayed_p2_damage().round() as u16).render(rows[1], &mut buf);
        buf
    }

    fn paint_effects(&self, area: Rect) -> Buffer {
        let mut buf = Buffer::new(area.width, area.height);
        if self.flash_ticks_remaining > 0 {
            let flash = Cell {
                symbol: '*',
                fg: Color::Black,
                bg: self.theme.accent,
                ..Default::default()
            };
            let w = 7.min(area.width);
            let h = 3.min(area.height);
            let x0 = (area.width.saturating_sub(w)) / 2;
            let y0 = (area.height.saturating_sub(h)) / 2;
            for y in y0..y0 + h {
                for x in x0..x0 + w {
                    buf.set(x, y, flash.clone());
                }
            }
        }
        self.particles.render(&mut buf);
        buf
    }

    fn render_versus(&self, area: Rect, buf: &mut LayerStack) {
        let (dx, dy) = self.shake_offset();
        let layers: [(usize, Buffer); 3] = [
            (BACKGROUND, self.paint_background(area)),
            (UI, self.paint_ui(area)),
            (EFFECTS, self.paint_effects(area)),
        ];
        for (index, scratch) in layers {
            let final_buf = if dx != 0 || dy != 0 {
                effects::shake(&scratch, dx, dy)
            } else {
                scratch
            };
            blit(&final_buf, area, buf.layer_mut(index));
        }
    }

    fn render_destination_preview(&self, screen: Screen, area: Rect) -> Buffer {
        let mut buf = Buffer::new(area.width, area.height);
        let local = Rect {
            x: 0,
            y: 0,
            width: area.width,
            height: area.height,
        };
        match screen {
            Screen::Versus => {
                let background = self.paint_background(local);
                blit(&background, local, &mut buf);
                let ui = self.paint_ui(local);
                blit(&ui, local, &mut buf);
            }
            Screen::TargetSmash | Screen::StageHazards => {
                let mut stack = LayerStack::new(area.width, area.height);
                self.render_placeholder(screen, local, &mut stack);
                blit(&stack, local, &mut buf);
            }
            Screen::Hub => {
                let mut stack = LayerStack::new(area.width, area.height);
                self.render_hub(local, &mut stack);
                blit(&stack, local, &mut buf);
            }
        }
        buf
    }

    fn render_transition(&self, destination: Screen, area: Rect, progress: f32, buf: &mut Buffer) {
        if progress < 0.4 {
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
            let label = "VS";
            let lx = area.x + area.width.saturating_sub(label.len() as u16) / 2;
            let ly = area.y + area.height / 2;
            for (i, ch) in label.chars().enumerate() {
                buf.set(
                    lx + i as u16,
                    ly,
                    Cell {
                        symbol: ch,
                        fg: Color::White,
                        bg: Color::Black,
                        style: CellStyle { bold: true },
                    },
                );
            }
            return;
        }

        let wipe = (progress - 0.4) / 0.6;
        let content = self.render_destination_preview(destination, area);
        let cx = area.width as f32 / 2.0;
        let cy = area.height as f32 / 2.0;
        let max_radius = ((cx / 2.0).powi(2) + cy.powi(2)).sqrt();
        let radius = wipe * max_radius;
        for y in 0..area.height {
            for x in 0..area.width {
                let dx = (x as f32 - cx) / 2.0;
                let dy = y as f32 - cy;
                let dist = (dx * dx + dy * dy).sqrt();
                let cell = if dist <= radius {
                    content.get(x, y).clone()
                } else {
                    Cell {
                        symbol: ' ',
                        fg: Color::Reset,
                        bg: Color::Black,
                        ..Default::default()
                    }
                };
                buf.set(area.x + x, area.y + y, cell);
            }
        }
    }
}

fn blit(scratch: &Buffer, area: Rect, buf: &mut Buffer) {
    for y in 0..scratch.height {
        for x in 0..scratch.width {
            buf.set(area.x + x, area.y + y, scratch.get(x, y).clone());
        }
    }
}

impl App for SmashCrabs {
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
                    let from = self.displayed_cursor_index();
                    self.selected = (self.selected + FIGHTERS.len() - 1) % FIGHTERS.len();
                    self.cursor_tween = Some((
                        from,
                        Transition::start(Duration::from_millis(CURSOR_TWEEN_MS)),
                    ));
                    self.audio.play("cursor");
                }
                KeyCode::Right => {
                    let from = self.displayed_cursor_index();
                    self.selected = (self.selected + 1) % FIGHTERS.len();
                    self.cursor_tween = Some((
                        from,
                        Transition::start(Duration::from_millis(CURSOR_TWEEN_MS)),
                    ));
                    self.audio.play("cursor");
                }
                KeyCode::Enter => {
                    if self.cursor_tween.is_none() {
                        let destination = screen_for_selected(self.selected);
                        self.transitioning_to = Some((
                            destination,
                            Transition::start(Duration::from_millis(VS_TRANSITION_MS)),
                        ));
                        self.p2_damage = 0;
                        self.damage_tween = None;
                        self.flash_ticks_remaining = 0;
                        self.shake_ticks_remaining = 0;
                        self.particles = ParticleSystem::new();
                        self.audio.play("select");
                    }
                }
                _ => {}
            },
            Screen::Versus => {
                if k.code == KeyCode::Esc {
                    self.screen = Screen::Hub;
                } else if k.code == KeyCode::Char(' ') {
                    self.flash_ticks_remaining = FLASH_TICKS;
                    self.shake_ticks_remaining = SHAKE_TICKS;
                    let from = self.displayed_p2_damage();
                    self.p2_damage += HIT_DAMAGE;
                    self.damage_tween = Some((
                        from,
                        Transition::start(Duration::from_millis(DAMAGE_TWEEN_MS)),
                    ));
                    for i in 0..8 {
                        let angle = i as f32 * std::f32::consts::TAU / 8.0;
                        self.particles.spawn(Particle {
                            x: 10.0,
                            y: 4.0,
                            vx: angle.cos() * 8.0,
                            vy: angle.sin() * 4.0,
                            symbol: '*',
                            color: self.theme.accent,
                            lifetime: Duration::from_millis(400),
                            age: Duration::ZERO,
                        });
                    }
                    self.audio.play("hit");
                }
            }
            Screen::TargetSmash | Screen::StageHazards => {
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
            Screen::Versus => {
                buf.push_layer(); // index 1: UI
                buf.push_layer(); // index 2: EFFECTS
                self.render_versus(area, buf);
            }
            Screen::TargetSmash | Screen::StageHazards => {
                self.render_placeholder(self.screen, area, buf)
            }
        }
    }

    fn should_quit(&self) -> bool {
        self.quit
    }

    fn tick_rate(&self) -> Option<Duration> {
        Some(TICK_INTERVAL)
    }

    fn on_tick(&mut self, elapsed: Duration) {
        if self.flash_ticks_remaining > 0 {
            self.flash_ticks_remaining -= 1;
        }

        if let Some((_, t)) = &mut self.cursor_tween {
            t.tick(elapsed);
            if t.is_complete() {
                self.cursor_tween = None;
            }
        }

        self.tick_count += 1;

        if let Some((_, t)) = &mut self.damage_tween {
            t.tick(elapsed);
            if t.is_complete() {
                self.damage_tween = None;
            }
        }

        if self.shake_ticks_remaining > 0 {
            self.shake_ticks_remaining -= 1;
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
    let mut app = SmashCrabs::new();
    run(&mut app)
}
