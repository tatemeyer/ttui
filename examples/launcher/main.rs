// examples/launcher/main.rs — Portal Nexus: a cross-app launcher that
// composes the three example apps into one shell. Each app is reused in
// place via #[path] inclusion of its `<app>.rs` module (see the
// per-app thin `main.rs` entries).
use std::time::Duration;

use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::style::Color;
use ttui::app::{run, App};
use ttui::buffer::{Buffer, Cell, CellStyle, LayerStack};
use ttui::layout::Rect;
use ttui::transition::Transition;

#[path = "../omnitrix/omnitrix.rs"]
mod omnitrix;
#[path = "../smash_crabs/smash_crabs.rs"]
mod smash_crabs;
#[path = "../tardis/tardis.rs"]
mod tardis;

mod nexus;
mod portal;

/// The three launchable apps: display name, tagline, and signature
/// accent color used to tint that app's portal.
pub(crate) const PORTALS: [(&str, &str, Color); 3] = [
    (
        "OMNITRIX",
        "gadget hub",
        Color::Rgb {
            r: 60,
            g: 230,
            b: 90,
        },
    ),
    (
        "TARDIS",
        "hex console",
        Color::Rgb {
            r: 90,
            g: 160,
            b: 250,
        },
    ),
    (
        "SMASH CRABS",
        "arena",
        Color::Rgb {
            r: 245,
            g: 90,
            b: 75,
        },
    ),
];
pub(crate) const APP_COUNT: usize = PORTALS.len();

/// Deep-space background color for the nexus.
pub(crate) const VOID: Color = Color::Rgb { r: 6, g: 8, b: 22 };

const NEXUS_TICK: Duration = Duration::from_millis(50);
const RETURN_FADE_MS: u64 = 350;

/// Which app (or the nexus) is currently front-and-center.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Location {
    Nexus,
    Omnitrix,
    Tardis,
    SmashCrabs,
}

/// What the launcher should do in response to an event.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Action {
    Stay,
    NexusPrev,
    NexusNext,
    Launch(usize),
    ReturnToNexus,
    QuitProcess,
}

/// Pure event router. `key` is the pressed key (if any), `selected` the
/// nexus cursor index, `app_wants_quit` whether the active app just set
/// its own quit flag (only meaningful when not in the nexus).
fn route(
    location: Location,
    key: Option<KeyCode>,
    selected: usize,
    app_wants_quit: bool,
) -> Action {
    match location {
        Location::Nexus => match key {
            Some(KeyCode::Enter) => Action::Launch(selected),
            Some(KeyCode::Char('q')) => Action::QuitProcess,
            Some(KeyCode::Left | KeyCode::Up | KeyCode::BackTab) => Action::NexusPrev,
            Some(KeyCode::Right | KeyCode::Down | KeyCode::Tab) => Action::NexusNext,
            _ => Action::Stay,
        },
        // Inside an app: F12 is the reserved global return; otherwise the
        // app's own quit (its `q`) is reinterpreted as "back to nexus".
        _ => {
            if key == Some(KeyCode::F(12)) || app_wants_quit {
                Action::ReturnToNexus
            } else {
                Action::Stay
            }
        }
    }
}

fn location_of(index: usize) -> Location {
    match index {
        0 => Location::Omnitrix,
        1 => Location::Tardis,
        _ => Location::SmashCrabs,
    }
}

fn make_app(index: usize) -> Box<dyn App> {
    match index {
        0 => Box::new(omnitrix::Omnitrix::new()),
        1 => Box::new(tardis::Tardis::new()),
        _ => Box::new(smash_crabs::SmashCrabs::new()),
    }
}

/// Scales an `Rgb` color's brightness by `f` (other color kinds pass
/// through unchanged).
pub(crate) fn dim_color(c: Color, f: f32) -> Color {
    match c {
        Color::Rgb { r, g, b } => Color::Rgb {
            r: (r as f32 * f) as u8,
            g: (g as f32 * f) as u8,
            b: (b as f32 * f) as u8,
        },
        other => other,
    }
}

/// Draws `s` horizontally centered on row `y` within `area`, on the
/// void background. Clips at the area's right edge.
pub(crate) fn text_center(scene: &mut Buffer, area: Rect, y: u16, s: &str, fg: Color, bold: bool) {
    if y < area.y || y >= area.y + area.height {
        return;
    }
    let len = s.chars().count() as u16;
    let start_x = area.x + area.width.saturating_sub(len) / 2;
    for (i, ch) in s.chars().enumerate() {
        let x = start_x + i as u16;
        if x >= area.x + area.width {
            break;
        }
        scene.set(
            x,
            y,
            Cell {
                symbol: ch,
                fg,
                bg: VOID,
                style: CellStyle { bold, ..Default::default() },
            },
        );
    }
}

/// The launcher itself — an `App` that either delegates to the active
/// sub-app or renders the portal nexus.
struct Launcher {
    location: Location,
    active: Option<Box<dyn App>>,
    selected: usize,
    nexus_phase: f32,
    returning: Option<Transition>,
    quit: bool,
}

impl Launcher {
    fn new() -> Self {
        Launcher {
            location: Location::Nexus,
            active: None,
            selected: 0,
            nexus_phase: 0.0,
            returning: None,
            quit: false,
        }
    }

    fn apply(&mut self, action: Action) {
        match action {
            Action::Stay => {}
            Action::NexusPrev => {
                self.selected = (self.selected + APP_COUNT - 1) % APP_COUNT;
            }
            Action::NexusNext => {
                self.selected = (self.selected + 1) % APP_COUNT;
            }
            Action::Launch(i) => {
                self.active = Some(make_app(i));
                self.location = location_of(i);
                self.returning = None;
            }
            Action::ReturnToNexus => {
                self.active = None;
                self.location = Location::Nexus;
                self.returning = Some(Transition::start(Duration::from_millis(RETURN_FADE_MS)));
            }
            Action::QuitProcess => self.quit = true,
        }
    }
}

impl App for Launcher {
    fn update(&mut self, event: &Event) {
        let key = match event {
            Event::Key(k) if k.kind == KeyEventKind::Press => Some(k.code),
            _ => None,
        };

        if self.location == Location::Nexus {
            let action = route(Location::Nexus, key, self.selected, false);
            self.apply(action);
            return;
        }

        // In an app: intercept the reserved return key before delegating
        // so an app that ignores input mid-boot can still be exited.
        if key == Some(KeyCode::F(12)) {
            self.apply(Action::ReturnToNexus);
            return;
        }
        if let Some(app) = &mut self.active {
            app.update(event);
        }
        let wants = self.active.as_ref().is_some_and(|a| a.should_quit());
        let action = route(self.location, key, self.selected, wants);
        self.apply(action);
    }

    fn view(&self, area: Rect, buf: &mut LayerStack) {
        match &self.active {
            Some(app) => app.view(area, buf),
            None => {
                let fade = self.returning.as_ref().map_or(1.0, |t| t.progress());
                nexus::render(self.selected, self.nexus_phase, fade, area, buf);
            }
        }
    }

    fn should_quit(&self) -> bool {
        self.quit
    }

    fn tick_rate(&self) -> Option<Duration> {
        match &self.active {
            Some(app) => app.tick_rate(),
            None => Some(NEXUS_TICK),
        }
    }

    fn on_tick(&mut self, elapsed: Duration) {
        match &mut self.active {
            Some(app) => app.on_tick(elapsed),
            None => {
                self.nexus_phase += elapsed.as_secs_f32();
                if let Some(t) = &mut self.returning {
                    t.tick(elapsed);
                    if t.is_complete() {
                        self.returning = None;
                    }
                }
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    run(&mut Launcher::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f12_returns_from_any_app() {
        assert_eq!(
            route(Location::Omnitrix, Some(KeyCode::F(12)), 0, false),
            Action::ReturnToNexus
        );
        assert_eq!(
            route(Location::SmashCrabs, Some(KeyCode::F(12)), 2, false),
            Action::ReturnToNexus
        );
    }

    #[test]
    fn app_quit_returns_to_nexus_not_process() {
        assert_eq!(
            route(Location::Tardis, None, 0, true),
            Action::ReturnToNexus
        );
    }

    #[test]
    fn nexus_enter_launches_selected() {
        assert_eq!(
            route(Location::Nexus, Some(KeyCode::Enter), 2, false),
            Action::Launch(2)
        );
    }

    #[test]
    fn nexus_q_quits_process() {
        assert_eq!(
            route(Location::Nexus, Some(KeyCode::Char('q')), 0, false),
            Action::QuitProcess
        );
    }

    #[test]
    fn nexus_arrows_and_tab_move_selection() {
        assert_eq!(
            route(Location::Nexus, Some(KeyCode::Tab), 0, false),
            Action::NexusNext
        );
        assert_eq!(
            route(Location::Nexus, Some(KeyCode::Right), 0, false),
            Action::NexusNext
        );
        assert_eq!(
            route(Location::Nexus, Some(KeyCode::BackTab), 0, false),
            Action::NexusPrev
        );
        assert_eq!(
            route(Location::Nexus, Some(KeyCode::Left), 0, false),
            Action::NexusPrev
        );
    }

    #[test]
    fn f12_in_nexus_does_nothing() {
        assert_eq!(
            route(Location::Nexus, Some(KeyCode::F(12)), 0, false),
            Action::Stay
        );
    }

    #[test]
    fn unrelated_key_in_app_stays() {
        assert_eq!(
            route(Location::Omnitrix, Some(KeyCode::Char('x')), 0, false),
            Action::Stay
        );
    }

    #[test]
    fn apply_launch_and_return_toggle_location() {
        let mut l = Launcher::new();
        assert_eq!(l.location, Location::Nexus);
        l.apply(Action::Launch(1));
        assert_eq!(l.location, Location::Tardis);
        assert!(l.active.is_some());
        l.apply(Action::ReturnToNexus);
        assert_eq!(l.location, Location::Nexus);
        assert!(l.active.is_none());
        assert!(l.returning.is_some());
    }

    #[test]
    fn nexus_selection_wraps() {
        let mut l = Launcher::new();
        l.apply(Action::NexusPrev);
        assert_eq!(l.selected, APP_COUNT - 1);
        l.apply(Action::NexusNext);
        assert_eq!(l.selected, 0);
    }

    #[test]
    fn nexus_render_does_not_panic_across_sizes() {
        for (w, h) in [(12, 10), (40, 15), (80, 24), (120, 40), (200, 60)] {
            let mut stack = LayerStack::new(w, h);
            let area = Rect {
                x: 0,
                y: 0,
                width: w,
                height: h,
            };
            for sel in 0..APP_COUNT {
                nexus::render(sel, 1.23, 1.0, area, &mut stack);
                nexus::render(sel, 5.0, 0.4, area, &mut stack);
            }
        }
    }
}
