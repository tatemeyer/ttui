// examples/omnitrix/omnitrix.rs — Omnitrix App (reused by the launcher)
use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::style::Color;
use std::time::Duration;
use ttui::app::App;
use ttui::buffer::{Buffer, Cell, LayerStack};
use ttui::camera;
use ttui::easing;
use ttui::layout::Rect;
use ttui::theme::{BorderSet, Theme};
use ttui::transition::Transition;
use ttui::widgets::{
    block::Block, dial::Dial, dna_console::DNAConsole, energy_core::EnergyCore, text::Text,
};

#[path = "boot.rs"]
mod boot;
#[path = "brainstorm.rs"]
mod brainstorm;
#[path = "faceplate.rs"]
mod faceplate;
#[path = "fasttrack.rs"]
mod fasttrack;
#[path = "upgrade.rs"]
mod upgrade;

const TICK_INTERVAL: Duration = Duration::from_millis(33); // ~30 FPS

#[derive(Clone, Copy, PartialEq)]
enum AppMode {
    Faceplate,
    Brainstorm,
    Fasttrack,
    Upgrade,
}

const SAMPLES: [&str; 3] = ["Brainstorm", "Fasttrack", "Upgrade"];

impl AppMode {
    fn from_selected(selected: usize) -> Self {
        match selected {
            0 => AppMode::Brainstorm,
            1 => AppMode::Fasttrack,
            _ => AppMode::Upgrade,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum ChatSpeaker {
    User,
    Agent,
}

const CANNED_PROMPTS: [&str; 3] = [
    "Summarize my inbox",
    "Draft a release note",
    "Explain this stack trace",
];
const BRAINSTORM_THINKING_MS: u64 = 1200;
const PREVIEW_REVEAL_MS: u64 = 400;
const LOCK_ON_MS: u64 = 900;
const COMPLETE_FLASH_MS: u64 = 300;
const RING_POINTS: usize = 8;
const UPGRADE_LOAD_GAIN: f32 = 15.0;
const UPGRADE_LOAD_DECAY_PER_SEC: f32 = 3.0;
const OVERLOAD_THRESHOLD: f32 = 90.0;
const CIRCUIT_NODE_COUNT: u16 = 6;
const BOOT_MS: u64 = 2500;

const HOURGLASS: [&str; 5] = ["┌───┐", " \\ / ", "  X  ", " / \\ ", "└───┘"];

pub(crate) struct Omnitrix {
    pulse_phase: f32,
    quit: bool,
    selected: usize,
    mode: AppMode,
    transitioning_from: Option<(AppMode, Transition)>,
    tick_count: u64,
    chat_log: Vec<(ChatSpeaker, String)>,
    prompt_index: usize,
    thinking: Option<Transition>,
    preview_reveal: Transition,
    targets: Vec<(String, bool)>,
    target_selected: usize,
    lock_on: Option<(usize, Transition)>,
    complete_flash: Option<Transition>,
    load: f32,
    booting: Option<Transition>,
}

impl Omnitrix {
    pub(crate) fn new() -> Self {
        Omnitrix {
            pulse_phase: 0.0,
            quit: false,
            selected: 0,
            mode: AppMode::Faceplate,
            transitioning_from: None,
            tick_count: 0,
            chat_log: Vec::new(),
            prompt_index: 0,
            thinking: None,
            preview_reveal: Transition::start(Duration::from_millis(PREVIEW_REVEAL_MS)),
            targets: vec![
                ("Fix login bug".to_string(), false),
                ("Write tests".to_string(), false),
                ("Ship release".to_string(), false),
            ],
            target_selected: 0,
            lock_on: None,
            complete_flash: None,
            load: 0.0,
            booting: Some(Transition::start(Duration::from_millis(BOOT_MS))),
        }
    }

    fn theme(&self) -> Theme {
        // Breathing pulse: sine wave brightness between a dim and a
        // bright green, matching the Omnitrix vision doc's "Recharge
        // Pulse" description.
        let brightness = (self.pulse_phase.sin() + 1.0) / 2.0;
        let mut primary = Color::Rgb {
            r: 0,
            g: (120.0 + brightness * 135.0) as u8,
            b: (32.0 + brightness * 33.0) as u8,
        };
        if self.mode == AppMode::Upgrade
            && self.load >= OVERLOAD_THRESHOLD
            && self.tick_count.is_multiple_of(2)
        {
            primary = Color::Red;
        }
        Theme {
            background: Color::Black,
            primary,
            secondary: Color::DarkGreen,
            tertiary: Color::Red,
            accent: Color::White,
            border: BorderSet {
                horizontal: '=',
                vertical: '#',
                corner: '+',
            },
            border_bold: brightness > 0.6,
            border_thick: false,
        }
    }

    fn switch_mode(&mut self, next: AppMode) {
        let old = self.mode;
        self.mode = next;
        self.transitioning_from = Some((old, Transition::start(Duration::from_millis(500))));
    }

    fn render_mode_content(&self, mode: AppMode, area: Rect) -> Buffer {
        let mut buf = Buffer::new(area.width, area.height);
        let local = Rect {
            x: 0,
            y: 0,
            width: area.width,
            height: area.height,
        };
        match mode {
            AppMode::Faceplate => self.render_faceplate_content(local, &mut buf),
            AppMode::Brainstorm => self.render_brainstorm_content(local, &mut buf),
            AppMode::Fasttrack => self.render_fasttrack_content(local, &mut buf),
            AppMode::Upgrade => self.render_upgrade_content(local, &mut buf),
        }
        buf
    }

    fn overlay_border_noise(&self, area: Rect, buf: &mut LayerStack) {
        let theme = self.theme();
        for x in area.x..area.x + area.width {
            if (x as u64 + self.tick_count).is_multiple_of(5) {
                buf.set(
                    x,
                    area.y,
                    Cell {
                        symbol: braille_noise(x, area.y, self.tick_count),
                        fg: theme.primary,
                        bg: Color::Reset,
                        ..Default::default()
                    },
                );
                buf.set(
                    x,
                    area.y + area.height - 1,
                    Cell {
                        symbol: braille_noise(x, area.y + area.height - 1, self.tick_count),
                        fg: theme.primary,
                        bg: Color::Reset,
                        ..Default::default()
                    },
                );
            }
        }
        for y in area.y..area.y + area.height {
            if (y as u64 + self.tick_count).is_multiple_of(5) {
                buf.set(
                    area.x,
                    y,
                    Cell {
                        symbol: braille_noise(area.x, y, self.tick_count),
                        fg: theme.primary,
                        bg: Color::Reset,
                        ..Default::default()
                    },
                );
                buf.set(
                    area.x + area.width - 1,
                    y,
                    Cell {
                        symbol: braille_noise(area.x + area.width - 1, y, self.tick_count),
                        fg: theme.primary,
                        bg: Color::Reset,
                        ..Default::default()
                    },
                );
            }
        }
    }

    fn render_transition(&self, old_mode: AppMode, area: Rect, progress: f32, buf: &mut Buffer) {
        if progress < 0.2 {
            for y in 0..area.height {
                for x in 0..area.width {
                    buf.set(
                        area.x + x,
                        area.y + y,
                        Cell {
                            symbol: ' ',
                            fg: Color::Reset,
                            bg: Color::Yellow,
                            ..Default::default()
                        },
                    );
                }
            }
            return;
        }

        let wave = (progress - 0.2) / 0.8;
        let wave_row = (wave * area.height as f32) as u16;
        let old_content = self.render_mode_content(old_mode, area);
        let new_content = self.render_mode_content(self.mode, area);

        for y in 0..area.height {
            for x in 0..area.width {
                let cell = match y.cmp(&wave_row) {
                    std::cmp::Ordering::Less => new_content.get(x, y).clone(),
                    std::cmp::Ordering::Equal => Cell {
                        symbol: braille_noise(x, y, self.tick_count),
                        fg: Color::Reset,
                        bg: Color::Yellow,
                        ..Default::default()
                    },
                    std::cmp::Ordering::Greater => old_content.get(x, y).clone(),
                };
                buf.set(area.x + x, area.y + y, cell);
            }
        }
    }
}

fn braille_noise(x: u16, y: u16, tick: u64) -> char {
    let h = (x as u64).wrapping_mul(374_761_393)
        ^ (y as u64).wrapping_mul(668_265_263)
        ^ tick.wrapping_mul(2_246_822_519);
    let dot_pattern = (h % 256) as u32;
    char::from_u32(0x2800 + dot_pattern).unwrap_or('\u{2800}')
}

fn render_row(buf: &mut Buffer, area: Rect, y: u16, text: &str, fg: Color, bg: Color) {
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
                bg,
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

impl App for Omnitrix {
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
        if self.transitioning_from.is_some() {
            return;
        }
        match self.mode {
            AppMode::Faceplate => match k.code {
                KeyCode::Tab => self.selected = (self.selected + 1) % SAMPLES.len(),
                KeyCode::BackTab => {
                    self.selected = (self.selected + SAMPLES.len() - 1) % SAMPLES.len()
                }
                KeyCode::Enter => self.switch_mode(AppMode::from_selected(self.selected)),
                _ => {}
            },
            AppMode::Brainstorm => {
                if self.thinking.is_some() {
                    return;
                }
                match k.code {
                    KeyCode::Tab => {
                        self.prompt_index = (self.prompt_index + 1) % CANNED_PROMPTS.len();
                        self.preview_reveal =
                            Transition::start(Duration::from_millis(PREVIEW_REVEAL_MS));
                    }
                    KeyCode::BackTab => {
                        self.prompt_index =
                            (self.prompt_index + CANNED_PROMPTS.len() - 1) % CANNED_PROMPTS.len();
                        self.preview_reveal =
                            Transition::start(Duration::from_millis(PREVIEW_REVEAL_MS));
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        self.chat_log.push((
                            ChatSpeaker::User,
                            CANNED_PROMPTS[self.prompt_index].to_string(),
                        ));
                        self.thinking = Some(Transition::start(Duration::from_millis(
                            BRAINSTORM_THINKING_MS,
                        )));
                    }
                    KeyCode::Esc => self.switch_mode(AppMode::Faceplate),
                    _ => {}
                }
            }
            AppMode::Fasttrack => {
                if self.lock_on.is_some() {
                    return;
                }
                let active = self.active_target_indices();
                match k.code {
                    KeyCode::Tab => {
                        if !active.is_empty() {
                            self.target_selected = (self.target_selected + 1) % active.len();
                        }
                    }
                    KeyCode::BackTab => {
                        if !active.is_empty() {
                            self.target_selected =
                                (self.target_selected + active.len() - 1) % active.len();
                        }
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        if let Some(&idx) = active.get(self.target_selected) {
                            self.lock_on =
                                Some((idx, Transition::start(Duration::from_millis(LOCK_ON_MS))));
                        }
                    }
                    KeyCode::Esc => self.switch_mode(AppMode::Faceplate),
                    _ => {}
                }
            }
            AppMode::Upgrade => match k.code {
                KeyCode::Char(' ') => self.load += UPGRADE_LOAD_GAIN,
                KeyCode::Esc => self.switch_mode(AppMode::Faceplate),
                _ => {}
            },
        }
    }

    fn view(&self, area: Rect, buf: &mut LayerStack) {
        if let Some(t) = &self.booting {
            self.render_boot(area, t.progress(), buf);
            return;
        }
        let theme = self.theme();
        let inner = Block::new()
            .title("Omnitrix")
            .theme(&theme)
            .render(area, buf);

        if self.mode == AppMode::Brainstorm && self.thinking.is_some() {
            self.overlay_border_noise(area, buf);
        }

        match &self.transitioning_from {
            None => {
                let content = self.render_mode_content(self.mode, inner);
                blit(&content, inner, buf);
            }
            Some((old_mode, transition)) => {
                self.render_transition(*old_mode, inner, transition.progress(), buf);
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
        let pulse_rate = if self.mode == AppMode::Brainstorm && self.thinking.is_some() {
            3.0
        } else {
            1.0
        };
        self.pulse_phase += elapsed.as_secs_f32() * std::f32::consts::PI * pulse_rate;

        self.tick_count += 1;

        if let Some((_, transition)) = &mut self.transitioning_from {
            transition.tick(elapsed);
            if transition.is_complete() {
                self.transitioning_from = None;
            }
        }

        if let Some(t) = &mut self.thinking {
            t.tick(elapsed);
            if t.is_complete() {
                let prompt = CANNED_PROMPTS[self.prompt_index];
                self.chat_log
                    .push((ChatSpeaker::Agent, format!("{prompt} ... complete.")));
                self.thinking = None;
            }
        }
        self.preview_reveal.tick(elapsed);

        if let Some((idx, t)) = &mut self.lock_on {
            t.tick(elapsed);
            if t.is_complete() {
                self.targets[*idx].1 = true;
                self.complete_flash =
                    Some(Transition::start(Duration::from_millis(COMPLETE_FLASH_MS)));
                self.target_selected = 0;
                self.lock_on = None;
            }
        }
        if let Some(t) = &mut self.complete_flash {
            t.tick(elapsed);
            if t.is_complete() {
                self.complete_flash = None;
            }
        }

        self.load = (self.load - UPGRADE_LOAD_DECAY_PER_SEC * elapsed.as_secs_f32()).max(0.0);

        if let Some(t) = &mut self.booting {
            t.tick(elapsed);
            if t.is_complete() {
                self.booting = None;
            }
        }
    }
}
