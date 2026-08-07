// examples/smash_crabs.rs
use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::style::Color;
use std::time::Duration;
use ttui::app::{run, App};
use ttui::buffer::{Cell, LayerStack};
use ttui::easing;
use ttui::layout::{Constraint, Direction, Layout, Rect};
use ttui::theme::{BorderSet, Theme};
use ttui::transition::Transition;
use ttui::widgets::{
    block::Block, scuttle_cursor::ScuttleCursor, smash_border::SmashBorder, text::Text,
};

const BACKGROUND: usize = 0;
const UI: usize = 1;
const EFFECTS: usize = 2;

const TICK_INTERVAL: Duration = Duration::from_millis(33); // ~30 FPS, matches omnitrix
const FLASH_TICKS: u8 = 6; // ~200ms flash at 33ms/tick
const CURSOR_TWEEN_MS: u64 = 150;
const CURSOR_SYMBOL: char = 'C';

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

struct SmashCrabs {
    theme: Theme,
    screen: Screen,
    selected: usize,
    cursor_tween: Option<(f32, Transition)>,
    p1_hp: u8,
    p2_hp: u8,
    flash_ticks_remaining: u8,
    quit: bool,
}

impl SmashCrabs {
    fn new() -> Self {
        SmashCrabs {
            theme: arena_theme(),
            screen: Screen::Hub,
            selected: 0,
            cursor_tween: None,
            p1_hp: 100,
            p2_hp: 100,
            flash_ticks_remaining: 0,
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
        ScuttleCursor::new(CURSOR_SYMBOL).render(cx, cy, self.cursor_tween.is_some(), 0, buf);
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

    fn paint_background(&self, area: Rect, buf: &mut LayerStack) {
        let cell = Cell {
            symbol: ' ',
            fg: self.theme.primary,
            bg: self.theme.background,
            ..Default::default()
        };
        let layer = buf.layer_mut(BACKGROUND);
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                layer.set(x, y, cell.clone());
            }
        }
    }

    fn paint_ui(&self, area: Rect, buf: &mut LayerStack) {
        let panel = Layout::new(Direction::Vertical, vec![Constraint::Fixed(4)]).split(area)[0];
        let panel = Rect {
            width: panel.width.min(20),
            ..panel
        };
        let inner = Block::new()
            .title("Fighters")
            .theme(&self.theme)
            .render(panel, buf.layer_mut(UI));
        let rows = Layout::new(
            Direction::Vertical,
            vec![Constraint::Fixed(1), Constraint::Fixed(1)],
        )
        .split(inner);
        Text::new(&format!("P1: {} HP", self.p1_hp)).render(rows[0], buf.layer_mut(UI));
        Text::new(&format!("P2: {} HP", self.p2_hp)).render(rows[1], buf.layer_mut(UI));
    }

    fn paint_effects(&self, area: Rect, buf: &mut LayerStack) {
        if self.flash_ticks_remaining == 0 {
            return;
        }
        let flash = Cell {
            symbol: '*',
            fg: Color::Black,
            bg: self.theme.accent,
            ..Default::default()
        };
        let w = 7.min(area.width);
        let h = 3.min(area.height);
        let x0 = area.x + (area.width.saturating_sub(w)) / 2;
        let y0 = area.y + (area.height.saturating_sub(h)) / 2;
        let layer = buf.layer_mut(EFFECTS);
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                layer.set(x, y, flash.clone());
            }
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
        match self.screen {
            Screen::Hub => match k.code {
                KeyCode::Left => {
                    let from = self.displayed_cursor_index();
                    self.selected = (self.selected + FIGHTERS.len() - 1) % FIGHTERS.len();
                    self.cursor_tween = Some((
                        from,
                        Transition::start(Duration::from_millis(CURSOR_TWEEN_MS)),
                    ));
                }
                KeyCode::Right => {
                    let from = self.displayed_cursor_index();
                    self.selected = (self.selected + 1) % FIGHTERS.len();
                    self.cursor_tween = Some((
                        from,
                        Transition::start(Duration::from_millis(CURSOR_TWEEN_MS)),
                    ));
                }
                KeyCode::Enter => {
                    if self.cursor_tween.is_none() {
                        self.screen = screen_for_selected(self.selected);
                    }
                }
                _ => {}
            },
            Screen::Versus => {
                if k.code == KeyCode::Esc {
                    self.screen = Screen::Hub;
                } else if k.code == KeyCode::Char(' ') {
                    self.flash_ticks_remaining = FLASH_TICKS;
                    self.p2_hp = self.p2_hp.saturating_sub(10);
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
        match self.screen {
            Screen::Hub => self.render_hub(area, buf),
            Screen::Versus => {
                buf.push_layer(); // index 1: UI
                buf.push_layer(); // index 2: EFFECTS
                self.paint_background(area, buf);
                self.paint_ui(area, buf);
                self.paint_effects(area, buf);
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
    }
}

fn main() -> std::io::Result<()> {
    let mut app = SmashCrabs::new();
    run(&mut app)
}
