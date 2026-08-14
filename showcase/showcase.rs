//! showcase — the flagship demo reel. A robot mascot hosts a tile
//! menu of 5 auto-playing vignettes, each showcasing one TTUI
//! technique (mouse, particles, camera+glitch, chord input, data-viz).

use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::style::Color;
use std::time::Duration;
use ttui::app::App;
use ttui::buffer::{CellStyle, LayerStack};
use ttui::layout::Rect;
use ttui::theme::{BorderSet, Theme};
use ttui::transition::Transition;

const BOOT_MS: u64 = 1200;
const TICK_INTERVAL: Duration = Duration::from_millis(33);

#[derive(Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub(crate) enum VignetteId {
    AssemblyLine,
    OverloadVent,
    DiagnosticScan,
    OverrideSequence,
    Telemetry,
}

#[derive(Clone, Copy, PartialEq)]
#[allow(dead_code)]
enum Screen {
    Menu,
    Vignette(VignetteId),
}

fn showcase_theme() -> Theme {
    Theme {
        background: Color::Rgb { r: 8, g: 8, b: 10 },
        primary: Color::Rgb {
            r: 138,
            g: 143,
            b: 152,
        },
        secondary: Color::Rgb {
            r: 199,
            g: 203,
            b: 209,
        },
        tertiary: Color::Rgb {
            r: 255,
            g: 60,
            b: 60,
        },
        accent: Color::Rgb {
            r: 255,
            g: 140,
            b: 66,
        },
        primary_end: None,
        border: BorderSet::default(),
        border_style: CellStyle::default(),
        border_thick: false,
    }
}

const ZERO_RECT: Rect = Rect {
    x: 0,
    y: 0,
    width: 0,
    height: 0,
};

pub(crate) struct ShowcaseApp {
    theme: Theme,
    screen: Screen,
    booting: Option<Transition>,
    last_area: std::cell::Cell<Rect>,
    quit: bool,
}

impl ShowcaseApp {
    pub(crate) fn new() -> Self {
        ShowcaseApp {
            theme: showcase_theme(),
            screen: Screen::Menu,
            booting: Some(Transition::start(Duration::from_millis(BOOT_MS))),
            last_area: std::cell::Cell::new(ZERO_RECT),
            quit: false,
        }
    }
}

impl App for ShowcaseApp {
    fn update(&mut self, event: &Event) {
        if self.booting.is_some() {
            return;
        }
        if self.screen == Screen::Menu {
            if let Event::Key(k) = event {
                if k.kind == KeyEventKind::Press && k.code == KeyCode::Char('q') {
                    self.quit = true;
                }
            }
        }
    }

    fn view(&self, area: Rect, _buf: &mut LayerStack) {
        self.last_area.set(area);
        if let Some(t) = &self.booting {
            let _ = t.progress();
            return;
        }
        if self.screen == Screen::Menu {
            // Task 3 replaces this with the real tile menu.
            let _ = &self.theme;
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
            if t.is_complete() {
                self.booting = None;
            }
        }
    }
}
