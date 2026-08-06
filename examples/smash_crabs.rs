// examples/smash_crabs.rs
use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::style::Color;
use std::time::Duration;
use ttui::app::{run, App};
use ttui::buffer::{Cell, LayerStack};
use ttui::layout::{Constraint, Direction, Layout, Rect};
use ttui::theme::{BorderSet, Theme};
use ttui::widgets::{block::Block, text::Text};

const BACKGROUND: usize = 0;
const UI: usize = 1;
const EFFECTS: usize = 2;

const TICK_INTERVAL: Duration = Duration::from_millis(33); // ~30 FPS, matches omnitrix
const FLASH_TICKS: u8 = 6; // ~200ms flash at 33ms/tick

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
    p1_hp: u8,
    p2_hp: u8,
    flash_ticks_remaining: u8,
    quit: bool,
}

impl SmashCrabs {
    fn new() -> Self {
        SmashCrabs {
            theme: arena_theme(),
            p1_hp: 100,
            p2_hp: 100,
            flash_ticks_remaining: 0,
            quit: false,
        }
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
        match k.code {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Char(' ') => {
                self.flash_ticks_remaining = FLASH_TICKS;
                self.p2_hp = self.p2_hp.saturating_sub(10);
            }
            _ => {}
        }
    }

    fn view(&self, area: Rect, buf: &mut LayerStack) {
        buf.push_layer(); // index 1: UI
        buf.push_layer(); // index 2: EFFECTS
        self.paint_background(area, buf);
        self.paint_ui(area, buf);
        self.paint_effects(area, buf);
    }

    fn should_quit(&self) -> bool {
        self.quit
    }

    fn tick_rate(&self) -> Option<Duration> {
        Some(TICK_INTERVAL)
    }

    fn on_tick(&mut self, _elapsed: Duration) {
        if self.flash_ticks_remaining > 0 {
            self.flash_ticks_remaining -= 1;
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut app = SmashCrabs::new();
    run(&mut app)
}
