// examples/launcher/main.rs — Portal Nexus: a cross-app launcher that
// composes the three example apps into one shell. Each app is reused in
// place via #[path] inclusion of its `<app>.rs` module (see the
// per-app thin `main.rs` entries).
use std::time::Duration;

use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::style::Color;
use ttui::app::{run, App};
use ttui::buffer::{Buffer, Cell, CellStyle, Intensity, LayerStack};
use ttui::layout::Rect;
use ttui::particles::{Particle, ParticleSystem};
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
const STARFIELD_W: u16 = 250;
const STARFIELD_H: u16 = 80;
const TARGET_STAR_COUNT: usize = 400;
const STAR_LIFETIME_SECS: u64 = 30;
const DIVE_DURATION: Duration = Duration::from_millis(400);
const DIVE_PARTICLE_COUNT: u32 = 16;
const NOMINAL_CENTER_X: f32 = 40.0;
const NOMINAL_CENTER_Y: f32 = 12.0;

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
                style: CellStyle {
                    intensity: if bold {
                        Intensity::Bold
                    } else {
                        Intensity::Normal
                    },
                    ..Default::default()
                },
                alpha: 1.0,
            },
        );
    }
}

/// Spawns one drifting background star at a pseudo-random position and
/// velocity within the fixed virtual starfield space, derived from
/// `seed` (a monotonically-increasing counter, not real randomness —
/// deterministic and dependency-free, matching this codebase's
/// existing hash-based pseudo-random patterns).
fn spawn_star(seed: u64) -> Particle {
    let h1 = seed.wrapping_mul(2_654_435_761);
    let h2 = seed.wrapping_mul(2_246_822_519) ^ 0x9E37_79B9;
    let x = ((h1 ^ (h1 >> 13)) % STARFIELD_W as u64) as f32;
    let y = ((h2 ^ (h2 >> 17)) % STARFIELD_H as u64) as f32;
    let angle = ((h1 >> 16) % 360) as f32 * std::f32::consts::PI / 180.0;
    let speed = 0.3 + ((h2 >> 8) % 71) as f32 / 100.0; // 0.3..1.0 cells/sec
    let brightness = ((h1 >> 24) % 200) as u8;
    let level = 70u8.saturating_add(brightness);
    let symbol = if brightness > 150 {
        '✦'
    } else if brightness > 80 {
        '·'
    } else {
        '.'
    };
    Particle {
        x,
        y,
        vx: angle.cos() * speed,
        vy: angle.sin() * speed,
        symbol,
        color: Color::Rgb {
            r: level,
            g: level,
            b: (level as u16 + 30).min(255) as u8,
        },
        lifetime: Duration::from_secs(STAR_LIFETIME_SECS) + Duration::from_millis(h2 % 30_000),
        age: Duration::ZERO,
    }
}

/// Builds a short-lived particle burst approximating an "into the
/// portal" flourish for launching app `index`. Origin is a fixed
/// offset from a nominal center point, not the portal's real screen
/// position — `apply()` (called from `update()`) has no access to the
/// terminal's actual size, same constraint `spawn_star` works around.
fn spawn_burst(index: usize) -> ParticleSystem {
    let mut ps = ParticleSystem::new();
    let cx = NOMINAL_CENTER_X + (index as f32 - 1.0) * 20.0;
    let cy = NOMINAL_CENTER_Y;
    let accent = PORTALS[index].2;
    for i in 0..DIVE_PARTICLE_COUNT {
        let angle = i as f32 * (std::f32::consts::TAU / DIVE_PARTICLE_COUNT as f32);
        ps.spawn(Particle {
            x: cx,
            y: cy,
            vx: angle.cos() * 25.0,
            vy: angle.sin() * 12.0,
            symbol: '*',
            color: accent,
            lifetime: DIVE_DURATION,
            age: Duration::ZERO,
        });
    }
    ps
}

/// The launcher itself — an `App` that either delegates to the active
/// sub-app or renders the portal nexus.
struct Launcher {
    location: Location,
    active: Option<Box<dyn App>>,
    selected: usize,
    nexus_phase: f32,
    starfield: ParticleSystem,
    star_seed: u64,
    diving: Option<(usize, Transition, ParticleSystem)>,
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
            starfield: ParticleSystem::new(),
            star_seed: 0,
            diving: None,
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
                self.diving = Some((i, Transition::start(DIVE_DURATION), spawn_burst(i)));
                self.returning = None;
            }
            Action::ReturnToNexus => {
                self.active = None;
                self.location = Location::Nexus;
                self.diving = None;
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
            if self.diving.is_some() {
                // Sustained/autorepeat input means on_tick may never fire
                // (App::on_tick only runs when the poll times out with no
                // event), which would otherwise freeze the dive forever.
                // Any keypress mid-dive skips straight to the destination.
                if key.is_some() {
                    if let Some((index, _, _)) = self.diving.take() {
                        self.active = Some(make_app(index));
                        self.location = location_of(index);
                    }
                }
                return;
            }
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
                if let Some((_, transition, burst)) = &self.diving {
                    let fade = 1.0 - transition.progress();
                    nexus::render(
                        self.selected,
                        &self.starfield,
                        self.nexus_phase,
                        fade,
                        area,
                        buf,
                    );
                    let mut scene = Buffer::new(area.width, area.height);
                    burst.render(&mut scene);
                    for y in 0..scene.height {
                        for x in 0..scene.width {
                            let cell = scene.get(x, y);
                            if *cell != Cell::default() {
                                buf.set(area.x + x, area.y + y, cell.clone());
                            }
                        }
                    }
                } else {
                    let fade = self.returning.as_ref().map_or(1.0, |t| t.progress());
                    nexus::render(
                        self.selected,
                        &self.starfield,
                        self.nexus_phase,
                        fade,
                        area,
                        buf,
                    );
                }
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
                self.starfield.update(elapsed);
                while self.starfield.len() < TARGET_STAR_COUNT {
                    self.star_seed = self.star_seed.wrapping_add(1);
                    self.starfield.spawn(spawn_star(self.star_seed));
                }
                if let Some(t) = &mut self.returning {
                    t.tick(elapsed);
                    if t.is_complete() {
                        self.returning = None;
                    }
                }
                if let Some((_, transition, burst)) = &mut self.diving {
                    transition.tick(elapsed);
                    burst.update(elapsed);
                }
                let dive_complete = self
                    .diving
                    .as_ref()
                    .is_some_and(|(_, t, _)| t.is_complete());
                if dive_complete {
                    if let Some((index, _, _)) = self.diving.take() {
                        self.active = Some(make_app(index));
                        self.location = location_of(index);
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
    fn apply_launch_starts_a_dive_apply_return_resets_to_nexus() {
        let mut l = Launcher::new();
        assert_eq!(l.location, Location::Nexus);
        l.apply(Action::Launch(1));
        assert_eq!(
            l.location,
            Location::Nexus,
            "location doesn't change until the dive completes"
        );
        assert!(l.diving.is_some());
        l.apply(Action::ReturnToNexus);
        assert_eq!(l.location, Location::Nexus);
        assert!(l.active.is_none());
        assert!(l.returning.is_some());
    }

    #[test]
    fn launch_starts_a_dive_before_swapping_active_app() {
        let mut l = Launcher::new();
        l.apply(Action::Launch(1));
        assert!(l.diving.is_some());
        assert!(l.active.is_none());
        assert_eq!(
            l.location,
            Location::Nexus,
            "location doesn't change until the dive completes"
        );
    }

    #[test]
    fn dive_completes_into_the_active_app_after_enough_ticks() {
        let mut l = Launcher::new();
        l.apply(Action::Launch(1));
        l.on_tick(DIVE_DURATION + Duration::from_millis(10));
        assert!(l.active.is_some());
        assert_eq!(l.location, Location::Tardis);
        assert!(l.diving.is_none());
    }

    #[test]
    fn any_keypress_during_a_dive_completes_it_immediately() {
        let mut l = Launcher::new();
        l.apply(Action::Launch(1));
        assert!(l.diving.is_some());
        let event = Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('x'),
            crossterm::event::KeyModifiers::NONE,
        ));
        l.update(&event);
        assert!(l.active.is_some());
        assert_eq!(l.location, Location::Tardis);
        assert!(l.diving.is_none());
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
        let starfield = ParticleSystem::new();
        for (w, h) in [(12, 10), (40, 15), (80, 24), (120, 40), (200, 60)] {
            let mut stack = LayerStack::new(w, h);
            let area = Rect {
                x: 0,
                y: 0,
                width: w,
                height: h,
            };
            for sel in 0..APP_COUNT {
                nexus::render(sel, &starfield, 1.23, 1.0, area, &mut stack);
                nexus::render(sel, &starfield, 5.0, 0.4, area, &mut stack);
            }
        }
    }

    #[test]
    fn starfield_tops_up_to_target_count_after_ticking() {
        let mut l = Launcher::new();
        assert_eq!(l.starfield.len(), 0);
        for _ in 0..TARGET_STAR_COUNT {
            l.on_tick(Duration::from_millis(50));
        }
        assert_eq!(l.starfield.len(), TARGET_STAR_COUNT);
    }
}
