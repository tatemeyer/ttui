// examples/tardis.rs
use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::style::Color;
use std::time::Duration;
use ttui::app::{run, App};
use ttui::buffer::{Buffer, LayerStack};
use ttui::camera::{self, Camera};
use ttui::easing;
use ttui::layout::Rect;
use ttui::theme::{BorderSet, Theme};
use ttui::transition::Transition;
use ttui::widgets::{roundel::Roundel, text::Text, time_rotor::TimeRotor};

const TICK_INTERVAL: Duration = Duration::from_millis(33);
const FACE_COUNT: usize = 6;
const FACE_NAMES: [&str; 6] = [
    "Psychic Paper",
    "Auxiliary Roundel Bay",
    "Star Charts",
    "Auxiliary Roundel Bay",
    "Artron Energy",
    "Auxiliary Roundel Bay",
];
const ROTATE_TWEEN_MS: u64 = 200;
const DIM_FACTORS: [f32; 4] = [0.0, 0.35, 0.65, 0.85];

fn tardis_theme() -> Theme {
    Theme {
        background: Color::Rgb { r: 0, g: 0, b: 0 },
        primary: Color::Rgb {
            r: 0,
            g: 255,
            b: 20,
        },
        secondary: Color::Rgb {
            r: 184,
            g: 115,
            b: 51,
        },
        tertiary: Color::Rgb {
            r: 0,
            g: 255,
            b: 255,
        },
        accent: Color::Rgb {
            r: 255,
            g: 191,
            b: 0,
        },
        border: BorderSet {
            horizontal: '=',
            vertical: '#',
            corner: '+',
        },
        border_bold: false,
        border_thick: false,
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Hub,
    PsychicPaper,
    StarCharts,
    ArtronEnergy,
}

fn screen_for_face(face: usize) -> Option<Screen> {
    match face {
        0 => Some(Screen::PsychicPaper),
        2 => Some(Screen::StarCharts),
        4 => Some(Screen::ArtronEnergy),
        _ => None,
    }
}

fn hex_distance(a: usize, b: usize) -> usize {
    let diff = a.abs_diff(b);
    diff.min(FACE_COUNT - diff)
}

struct Tardis {
    theme: Theme,
    screen: Screen,
    selected_face: usize,
    face_tween: Option<(f32, Transition)>,
    tick_count: u64,
    quit: bool,
}

impl Tardis {
    fn new() -> Self {
        Tardis {
            theme: tardis_theme(),
            screen: Screen::Hub,
            selected_face: 0,
            face_tween: None,
            tick_count: 0,
            quit: false,
        }
    }

    fn displayed_face_index(&self) -> f32 {
        match &self.face_tween {
            Some((from, t)) => easing::ease_out(*from, self.selected_face as f32, t.progress()),
            None => self.selected_face as f32,
        }
    }

    fn time_rotor_speed(&self) -> f32 {
        1.0
    }

    fn render_face_content(&self, face: usize, area: Rect, buf: &mut Buffer) {
        let name_row = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height.min(1),
        };
        Text::new(FACE_NAMES[face]).render(name_row, buf);
        if screen_for_face(face).is_none() {
            for i in 0..3u16 {
                let rx = area.x + (area.width / 4) * (i + 1);
                let ry = area.y + area.height / 2;
                let pulse = ((self.tick_count as f32 * 0.05 + i as f32).sin() + 1.0) / 2.0;
                Roundel::new(pulse, self.theme.tertiary).render(
                    Rect {
                        x: rx,
                        y: ry,
                        width: 1,
                        height: 1,
                    },
                    buf,
                );
            }
        }
    }

    fn render_hub(&self, area: Rect, buf: &mut LayerStack) {
        let vw = area.width;
        let vh = area.height;
        let mut virtual_buf = Buffer::new(vw * FACE_COUNT as u16, vh);
        for face in 0..FACE_COUNT {
            let face_area = Rect {
                x: face as u16 * vw,
                y: 0,
                width: vw,
                height: vh,
            };
            self.render_face_content(face, face_area, &mut virtual_buf);
            let factor = DIM_FACTORS[hex_distance(face, self.selected_face)];
            if factor > 0.0 {
                let face_camera = Camera::new(face_area.x as f32, face_area.y as f32, 1.0);
                let cropped = camera::viewport(&virtual_buf, &face_camera, vw, vh);
                let dimmed = camera::dim(&cropped, factor);
                blit(&dimmed, face_area, &mut virtual_buf);
            }
        }
        let cam = Camera::new(self.displayed_face_index() * vw as f32, 0.0, 1.0);
        let view = camera::viewport(&virtual_buf, &cam, vw, vh);
        blit(&view, area, buf);

        let rotor_area = Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width,
            height: area.height.saturating_sub(2),
        };
        TimeRotor::new(self.time_rotor_speed()).render(rotor_area, self.tick_count, buf);

        let hint_row = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(1),
            width: area.width,
            height: area.height.saturating_sub(1).min(1),
        };
        Text::new("Left/Right rotate * Enter select * q quit").render(hint_row, buf);
    }

    fn render_placeholder(&self, screen: Screen, area: Rect, buf: &mut LayerStack) {
        let name = match screen {
            Screen::PsychicPaper => "Psychic Paper",
            Screen::StarCharts => "Star Charts",
            Screen::ArtronEnergy => "Artron Energy",
            Screen::Hub => "",
        };
        let name_row = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height.min(1),
        };
        let placeholder_row = Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width,
            height: area.height.saturating_sub(2),
        };
        let hint_row = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(1),
            width: area.width,
            height: area.height.saturating_sub(1).min(1),
        };
        Text::new(name).render(name_row, buf);
        Text::new("(not yet built)").render(placeholder_row, buf);
        Text::new("Esc back * q quit").render(hint_row, buf);
    }
}

fn blit(scratch: &Buffer, area: Rect, buf: &mut Buffer) {
    for y in 0..scratch.height {
        for x in 0..scratch.width {
            buf.set(area.x + x, area.y + y, scratch.get(x, y).clone());
        }
    }
}

impl App for Tardis {
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
                    let from = self.displayed_face_index();
                    self.selected_face = (self.selected_face + FACE_COUNT - 1) % FACE_COUNT;
                    self.face_tween = Some((
                        from,
                        Transition::start(Duration::from_millis(ROTATE_TWEEN_MS)),
                    ));
                }
                KeyCode::Right => {
                    let from = self.displayed_face_index();
                    self.selected_face = (self.selected_face + 1) % FACE_COUNT;
                    self.face_tween = Some((
                        from,
                        Transition::start(Duration::from_millis(ROTATE_TWEEN_MS)),
                    ));
                }
                KeyCode::Enter => {
                    if self.face_tween.is_none() {
                        if let Some(dest) = screen_for_face(self.selected_face) {
                            self.screen = dest;
                        }
                    }
                }
                _ => {}
            },
            _ => {
                if k.code == KeyCode::Esc {
                    self.screen = Screen::Hub;
                }
            }
        }
    }

    fn view(&self, area: Rect, buf: &mut LayerStack) {
        match self.screen {
            Screen::Hub => self.render_hub(area, buf),
            Screen::PsychicPaper | Screen::StarCharts | Screen::ArtronEnergy => {
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
        self.tick_count += 1;
        if let Some((_, t)) = &mut self.face_tween {
            t.tick(elapsed);
            if t.is_complete() {
                self.face_tween = None;
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut app = Tardis::new();
    run(&mut app)
}
