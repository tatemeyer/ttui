use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::style::Color;
use std::time::Duration;
use ttui::app::App;
use ttui::buffer::{Cell, LayerStack};
use ttui::layout::{Constraint, Direction, Layout, Rect};
use ttui::theme::{BorderSet, Theme};
use ttui::widgets::{cockpit_panel::CockpitPanel, text::Text};

const TICK_INTERVAL: Duration = Duration::from_millis(33); // ~30 FPS, matches every other app

#[derive(Clone, Copy, PartialEq)]
enum PanelKind {
    Hyperdrive,
    Sensors,
    Weapons,
}

const PANELS: [PanelKind; 3] = [
    PanelKind::Hyperdrive,
    PanelKind::Sensors,
    PanelKind::Weapons,
];

impl PanelKind {
    fn name(&self) -> &'static str {
        match self {
            PanelKind::Hyperdrive => "Hyperdrive",
            PanelKind::Sensors => "Sensors",
            PanelKind::Weapons => "Weapons",
        }
    }
}

fn falcon_theme() -> Theme {
    Theme {
        background: Color::Rgb { r: 10, g: 10, b: 8 },
        primary: Color::Rgb {
            r: 255,
            g: 176,
            b: 0,
        },
        secondary: Color::Rgb {
            r: 76,
            g: 187,
            b: 23,
        },
        tertiary: Color::Rgb {
            r: 255,
            g: 49,
            b: 49,
        },
        accent: Color::Rgb {
            r: 255,
            g: 215,
            b: 0,
        },
        primary_end: None,
        border: BorderSet::default(),
        border_bold: false,
        border_thick: false,
    }
}

pub(crate) struct Falcon {
    theme: Theme,
    focused: usize,
    // Task 4 adds `last_area`/`glitches`/`particles`/`tick_count` here.
    // Task 5 adds `booting` here.
    quit: bool,
}

impl Falcon {
    pub(crate) fn new() -> Self {
        Falcon {
            theme: falcon_theme(),
            focused: 0,
            quit: false,
        }
    }

    fn panel_slots(area: Rect) -> [Rect; 3] {
        let slots = Layout::new(Direction::Horizontal, vec![Constraint::Fill(1); 3]).split(area);
        [slots[0], slots[1], slots[2]]
    }

    fn panel_box(slot: Rect, focused: bool) -> Rect {
        let base_w = slot.width.saturating_sub(2).max(8);
        let base_h = slot.height.saturating_sub(4).clamp(4, 10);
        let focus_w = (base_w + 4).min(slot.width.saturating_sub(1));
        let focus_h = (base_h + 2).min(slot.height.saturating_sub(1));
        let box_w = if focused { focus_w } else { base_w };
        let box_h = if focused { focus_h } else { base_h };
        Rect {
            x: slot.x + slot.width.saturating_sub(box_w) / 2,
            y: slot.y + slot.height.saturating_sub(box_h) / 2,
            width: box_w,
            height: box_h,
        }
    }

    fn render_dashboard(&self, area: Rect, buf: &mut LayerStack) {
        let bg = Cell {
            symbol: ' ',
            fg: self.theme.primary,
            bg: self.theme.background,
            alpha: 1.0,
            ..Default::default()
        };
        for y in 0..area.height {
            for x in 0..area.width {
                buf.set(area.x + x, area.y + y, bg.clone());
            }
        }

        let slots = Self::panel_slots(area);
        for (i, kind) in PANELS.iter().enumerate() {
            let focused = i == self.focused;
            let panel_box = Self::panel_box(slots[i], focused);
            let inner = CockpitPanel::new(focused).render(panel_box, &self.theme, buf);
            Text::new(kind.name()).render(inner, buf);
            if inner.height > 1 {
                let hint = Rect {
                    x: inner.x,
                    y: inner.y + 1,
                    width: inner.width,
                    height: 1,
                };
                Text::new("(not yet built)").render(hint, buf);
            }
        }
    }
}

impl App for Falcon {
    fn update(&mut self, event: &Event) {
        let Event::Key(k) = event else { return };
        if k.kind != KeyEventKind::Press {
            return;
        }
        if k.code == KeyCode::Char('q') {
            self.quit = true;
            return;
        }
        match k.code {
            KeyCode::Tab => self.focused = (self.focused + 1) % PANELS.len(),
            KeyCode::BackTab => self.focused = (self.focused + PANELS.len() - 1) % PANELS.len(),
            _ => {}
        }
    }

    fn view(&self, area: Rect, buf: &mut LayerStack) {
        self.render_dashboard(area, buf);
    }

    fn should_quit(&self) -> bool {
        self.quit
    }

    fn tick_rate(&self) -> Option<Duration> {
        Some(TICK_INTERVAL)
    }
}
