// examples/smash_crabs/main.rs
use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::style::Color;
use rodio::Source;
use std::time::Duration;
use ttui::app::App;
use ttui::audio::AudioSink;
use ttui::buffer::{Buffer, Cell, CellStyle, Intensity, LayerStack};
use ttui::camera;
use ttui::easing;
use ttui::effects;
use ttui::layout::{Constraint, Direction, Layout, Rect};
use ttui::particles::{Particle, ParticleSystem};
use ttui::theme::{BorderSet, Theme};
use ttui::transition::Transition;
use ttui::widgets::{
    damage_meter::DamageMeter, scuttle_cursor::ScuttleCursor, smash_border::SmashBorder, text::Text,
};

#[path = "boot.rs"]
mod boot;
#[path = "hub.rs"]
mod hub;
#[path = "stage_hazards.rs"]
mod stage_hazards;
#[path = "target_smash.rs"]
mod target_smash;
#[path = "versus.rs"]
mod versus;

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

const TS_TARGETS: [&str; 5] = [
    "Refactor auth module",
    "Fix flaky test",
    "Write release notes",
    "Review PR #42",
    "Update dependencies",
];
const TS_IMPACT_GLYPH: char = '💥';
const KO_HOLD_MS: u64 = 600;
const TS_FADE_MS: u64 = 400;

const RAM_STRESS_AMOUNT: f32 = 22.0;
const RAM_DECAY_PER_SEC: f32 = 6.0;
const RAM_THRESHOLD: f32 = 90.0;
const BOBOMB_FLASH_TICKS: u64 = 6;
const BOBOMB_ART: [&str; 5] = ["  .  ", " /   ", "( o )", "(o o)", " \\_/ "];

const BOOT_FLASH_MS: u64 = 200;
const BOOT_CLAW_MS: u64 = 800;
const BOOT_TITLE_MS: u64 = 600;
const BOOT_FLARE_MS: u64 = 500;
const BOOT_TOTAL_MS: u64 = BOOT_FLASH_MS + BOOT_CLAW_MS + BOOT_TITLE_MS + BOOT_FLARE_MS;
const BOOT_TITLE: &str = "S U P E R S M A S H C L A W S";
const CLAW_OPEN: [&str; 5] = [
    " \\           / ",
    "  \\         /  ",
    "   (         )  ",
    "    \\       /   ",
    "     \\_____/    ",
];
const CLAW_CLOSED: [&str; 5] = [
    "   \\       /    ",
    "    \\     /     ",
    "     (   )      ",
    "      \\ /       ",
    "       X        ",
];

enum TsPhase {
    Impact(Transition),
    Fade(Transition),
}

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
    pub(crate) fn new() -> Self {
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
            "snap" => 110.0,
            _ => return,
        };
        let source = rodio::source::SineWave::new(freq)
            .take_duration(Duration::from_millis(120))
            .amplify(0.2);
        sink.mixer().add(source);
    }
}

pub(crate) struct SmashCrabs {
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
    ts_smashed: [bool; 5],
    ts_selected: usize,
    ts_smashing: Option<(usize, TsPhase)>,
    sh_ram: f32,
    booting: Option<Transition>,
    boot_snap_played: bool,
    quit: bool,
}

impl SmashCrabs {
    pub(crate) fn new() -> Self {
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
            ts_smashed: [false; 5],
            ts_selected: 0,
            ts_smashing: None,
            sh_ram: 20.0,
            booting: Some(Transition::start(Duration::from_millis(BOOT_TOTAL_MS))),
            boot_snap_played: false,
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
            Screen::TargetSmash => {
                let background = self.paint_background(local);
                blit(&background, local, &mut buf);
                let ui = self.paint_ts_ui(local);
                blit(&ui, local, &mut buf);
            }
            Screen::StageHazards => {
                let mut stack = LayerStack::new(area.width, area.height);
                self.render_stage_hazards(local, &mut stack);
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
                        style: CellStyle {
                            intensity: Intensity::Bold,
                            ..Default::default()
                        },
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

fn render_row(buf: &mut Buffer, area: Rect, text: &str, fg: Color) {
    if area.height == 0 {
        return;
    }
    for (i, ch) in text.chars().take(area.width as usize).enumerate() {
        buf.set(
            area.x + i as u16,
            area.y,
            Cell {
                symbol: ch,
                fg,
                bg: Color::Reset,
                ..Default::default()
            },
        );
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
        if self.transitioning_to.is_some() || self.booting.is_some() {
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
                KeyCode::Enter if self.cursor_tween.is_none() => {
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
            Screen::TargetSmash => {
                if self.ts_smashing.is_some() {
                    return;
                }
                let visible = self.ts_visible();
                match k.code {
                    KeyCode::Up => {
                        if !visible.is_empty() {
                            self.ts_selected =
                                (self.ts_selected + visible.len() - 1) % visible.len();
                        }
                    }
                    KeyCode::Down => {
                        if !visible.is_empty() {
                            self.ts_selected = (self.ts_selected + 1) % visible.len();
                        }
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        if let Some(&real_index) = visible.get(self.ts_selected) {
                            self.shake_ticks_remaining = SHAKE_TICKS;
                            self.ts_smashing = Some((
                                real_index,
                                TsPhase::Impact(Transition::start(Duration::from_millis(
                                    KO_HOLD_MS,
                                ))),
                            ));
                        }
                    }
                    KeyCode::Esc => self.screen = Screen::Hub,
                    _ => {}
                }
            }
            Screen::StageHazards => match k.code {
                KeyCode::Char(' ') => {
                    self.sh_ram = (self.sh_ram + RAM_STRESS_AMOUNT).min(100.0);
                }
                KeyCode::Esc => self.screen = Screen::Hub,
                _ => {}
            },
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
            Screen::Versus => {
                buf.push_layer(); // index 1: UI
                buf.push_layer(); // index 2: EFFECTS
                self.render_versus(area, buf);
            }
            Screen::TargetSmash => {
                buf.push_layer(); // index 1: UI
                buf.push_layer(); // index 2: EFFECTS
                self.render_target_smash(area, buf);
            }
            Screen::StageHazards => self.render_stage_hazards(area, buf),
        }
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
            let progress = t.progress();
            let t1 = BOOT_FLASH_MS as f32 / BOOT_TOTAL_MS as f32;
            let t2 = (BOOT_FLASH_MS + BOOT_CLAW_MS) as f32 / BOOT_TOTAL_MS as f32;
            if !self.boot_snap_played && progress >= t1 {
                let claw_sub = ((progress - t1) / (t2 - t1)).clamp(0.0, 1.0);
                if claw_sub >= 0.5 {
                    self.boot_snap_played = true;
                    self.audio.play("snap");
                }
            }
            if t.is_complete() {
                self.booting = None;
            }
        }

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

        self.sh_ram = (self.sh_ram - RAM_DECAY_PER_SEC * elapsed.as_secs_f32()).max(0.0);

        if let Some((real_index, phase)) = &mut self.ts_smashing {
            let real_index = *real_index;
            match phase {
                TsPhase::Impact(t) => {
                    t.tick(elapsed);
                    if t.is_complete() {
                        *phase =
                            TsPhase::Fade(Transition::start(Duration::from_millis(TS_FADE_MS)));
                    }
                }
                TsPhase::Fade(t) => {
                    t.tick(elapsed);
                    if t.is_complete() {
                        self.ts_smashed[real_index] = true;
                        self.ts_smashing = None;
                    }
                }
            }
        }
        if self.ts_smashing.is_none() {
            let visible_len = self.ts_visible().len();
            if visible_len == 0 {
                self.ts_selected = 0;
            } else if self.ts_selected >= visible_len {
                self.ts_selected = visible_len - 1;
            }
        }

        if let Some((destination, t)) = &mut self.transitioning_to {
            t.tick(elapsed);
            if t.is_complete() {
                self.screen = *destination;
                self.transitioning_to = None;
            }
        }
    }
}
