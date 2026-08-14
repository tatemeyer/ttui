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

#[path = "boot.rs"]
mod boot;
#[path = "camera_glitch.rs"]
mod camera_glitch;
#[path = "mascot.rs"]
mod mascot;
#[path = "menu.rs"]
mod menu;
#[path = "mouse_grab.rs"]
mod mouse_grab;
#[path = "particle_vent.rs"]
mod particle_vent;

use camera_glitch::DiagnosticScanState;
use mascot::{GripperMascot, MascotPose};
use mouse_grab::AssemblyLineState;
use particle_vent::OverloadVentState;

const BOOT_MS: u64 = 1200;
const TICK_INTERVAL: Duration = Duration::from_millis(33);

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum VignetteId {
    AssemblyLine,
    OverloadVent,
    DiagnosticScan,
    OverrideSequence,
    Telemetry,
}

#[derive(Clone, Copy, PartialEq)]
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
    mascot: GripperMascot,
    highlighted: usize,
    tile_areas: std::cell::Cell<[Rect; 5]>,
    quit: bool,
    assembly_line: Option<AssemblyLineState>,
    overload_vent: Option<OverloadVentState>,
    diagnostic_scan: Option<DiagnosticScanState>,
}

impl ShowcaseApp {
    pub(crate) fn new() -> Self {
        ShowcaseApp {
            theme: showcase_theme(),
            screen: Screen::Menu,
            booting: Some(Transition::start(Duration::from_millis(BOOT_MS))),
            last_area: std::cell::Cell::new(ZERO_RECT),
            mascot: GripperMascot::new(),
            highlighted: 0,
            tile_areas: std::cell::Cell::new([ZERO_RECT; 5]),
            quit: false,
            assembly_line: None,
            overload_vent: None,
            diagnostic_scan: None,
        }
    }

    fn enter_vignette(&mut self, id: VignetteId) {
        match id {
            VignetteId::AssemblyLine => self.assembly_line = Some(AssemblyLineState::new()),
            VignetteId::OverloadVent => self.overload_vent = Some(OverloadVentState::new()),
            VignetteId::DiagnosticScan => self.diagnostic_scan = Some(DiagnosticScanState::new()),
            _ => {}
        }
        self.screen = Screen::Vignette(id);
    }

    fn exit_vignette(&mut self) {
        self.assembly_line = None;
        self.overload_vent = None;
        self.diagnostic_scan = None;
        self.screen = Screen::Menu;
    }
}

impl App for ShowcaseApp {
    fn update(&mut self, event: &Event) {
        if self.booting.is_some() {
            return;
        }
        let screen = self.screen;
        match screen {
            Screen::Menu => {
                if let Event::Key(k) = event {
                    if k.kind != KeyEventKind::Press {
                        return;
                    }
                    match k.code {
                        KeyCode::Char('q') => self.quit = true,
                        KeyCode::Left => {
                            let prev = self.highlighted;
                            self.highlighted =
                                (self.highlighted + menu::TILES.len() - 1) % menu::TILES.len();
                            if self.highlighted != prev {
                                self.mascot.set_pose(MascotPose::Reacting);
                            }
                        }
                        KeyCode::Right => {
                            let prev = self.highlighted;
                            self.highlighted = (self.highlighted + 1) % menu::TILES.len();
                            if self.highlighted != prev {
                                self.mascot.set_pose(MascotPose::Reacting);
                            }
                        }
                        KeyCode::Enter => {
                            let id = menu::TILES[self.highlighted].0;
                            self.enter_vignette(id);
                        }
                        _ => {}
                    }
                } else if let Event::Mouse(m) = event {
                    if m.kind
                        == crossterm::event::MouseEventKind::Down(
                            crossterm::event::MouseButton::Left,
                        )
                    {
                        for (i, area) in self.tile_areas.get().iter().enumerate() {
                            if area.contains(m.column, m.row) {
                                let id = menu::TILES[i].0;
                                self.enter_vignette(id);
                                return;
                            }
                        }
                    }
                }
            }
            Screen::Vignette(id) => {
                if let Event::Key(k) = event {
                    if k.kind == KeyEventKind::Press && k.code == KeyCode::Esc {
                        self.exit_vignette();
                        return;
                    }
                }
                if id == VignetteId::AssemblyLine {
                    if let (Some(state), Event::Mouse(m)) = (&mut self.assembly_line, event) {
                        if m.kind
                            == crossterm::event::MouseEventKind::Down(
                                crossterm::event::MouseButton::Left,
                            )
                        {
                            state.handle_click(m.column, m.row);
                        }
                    }
                }
                if id == VignetteId::DiagnosticScan {
                    if let (Some(state), Event::Key(k)) = (&mut self.diagnostic_scan, event) {
                        if k.kind == KeyEventKind::Press && k.code == KeyCode::Char(' ') {
                            state.whack();
                        }
                    }
                }
            }
        }
    }

    fn view(&self, area: Rect, buf: &mut LayerStack) {
        self.last_area.set(area);
        if let Some(t) = &self.booting {
            boot::render_boot(area, &self.theme, t.progress(), buf);
            return;
        }
        let mascot_area = Rect {
            x: area.x + area.width.saturating_sub(mascot::MASCOT_WIDTH + 2),
            y: area.y + 1,
            width: mascot::MASCOT_WIDTH,
            height: mascot::MASCOT_HEIGHT,
        };
        match self.screen {
            Screen::Menu => {
                let menu_area = Rect {
                    x: area.x,
                    y: area.y,
                    width: area.width.saturating_sub(mascot::MASCOT_WIDTH + 4),
                    height: area.height,
                };
                let tile_areas = menu::render_menu(menu_area, &self.theme, self.highlighted, buf);
                self.tile_areas.set(tile_areas);
                self.mascot.render(mascot_area, buf);
            }
            Screen::Vignette(VignetteId::AssemblyLine) => {
                if let Some(state) = &self.assembly_line {
                    state.render(area, &self.theme, buf);
                }
                self.mascot.render(mascot_area, buf);
            }
            Screen::Vignette(VignetteId::OverloadVent) => {
                if let Some(state) = &self.overload_vent {
                    state.render(buf);
                }
            }
            Screen::Vignette(VignetteId::DiagnosticScan) => {
                if let Some(state) = &self.diagnostic_scan {
                    state.render(area, &self.theme, buf);
                }
            }
            Screen::Vignette(_) => {}
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
            return;
        }
        self.mascot.tick(elapsed);
        let area = self.last_area.get();
        let screen = self.screen;
        match screen {
            Screen::Menu => {}
            Screen::Vignette(VignetteId::AssemblyLine) => {
                if let Some(state) = &mut self.assembly_line {
                    state.on_tick(elapsed, area);
                    let caught = state.take_caught();
                    let done = state.is_complete();
                    if caught {
                        self.mascot.set_pose(MascotPose::Grabbing);
                    }
                    if done {
                        self.exit_vignette();
                    }
                }
            }
            Screen::Vignette(VignetteId::OverloadVent) => {
                if let Some(state) = &mut self.overload_vent {
                    state.on_tick(elapsed, area);
                    if state.is_complete() {
                        self.exit_vignette();
                    }
                }
            }
            Screen::Vignette(VignetteId::DiagnosticScan) => {
                if let Some(state) = &mut self.diagnostic_scan {
                    state.on_tick(elapsed);
                    if state.is_complete() {
                        self.exit_vignette();
                    }
                }
            }
            Screen::Vignette(_) => {}
        }
    }
}
