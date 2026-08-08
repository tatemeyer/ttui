// examples/launcher/nexus.rs — renders the portal nexus: a starfield
// void, a title, three app portals, and a hint row. Rendered into a
// scratch buffer so a return-fade can dim the whole scene uniformly
// before it is blitted into the live layer stack.
use crossterm::style::Color;
use ttui::buffer::{Buffer, Cell, CellStyle, LayerStack};
use ttui::camera;
use ttui::layout::Rect;

use crate::{portal, text_center, PORTALS, VOID};

/// Renders the nexus for `selected`/`phase` into `buf`, dimmed by
/// `fade` (1.0 = fully visible; < 1.0 during the return transition).
pub(crate) fn render(selected: usize, phase: f32, fade: f32, area: Rect, buf: &mut LayerStack) {
    if area.width < 12 || area.height < 10 {
        return;
    }
    let mut scene = Buffer::new(area.width, area.height);
    fill_void(&mut scene);
    starfield(&mut scene, phase);
    header(&mut scene);
    portals(&mut scene, selected, phase);
    footer(&mut scene);

    let scene = if fade < 1.0 {
        camera::dim(&scene, fade.clamp(0.0, 1.0))
    } else {
        scene
    };
    for y in 0..scene.height {
        for x in 0..scene.width {
            buf.set(area.x + x, area.y + y, scene.get(x, y).clone());
        }
    }
}

fn full(scene: &Buffer) -> Rect {
    Rect {
        x: 0,
        y: 0,
        width: scene.width,
        height: scene.height,
    }
}

fn fill_void(scene: &mut Buffer) {
    for y in 0..scene.height {
        for x in 0..scene.width {
            scene.set(
                x,
                y,
                Cell {
                    symbol: ' ',
                    fg: Color::Reset,
                    bg: VOID,
                    style: CellStyle::default(),
                },
            );
        }
    }
}

fn starfield(scene: &mut Buffer, phase: f32) {
    let tick = (phase * 4.0) as u64;
    for y in 0..scene.height {
        for x in 0..scene.width {
            let h = (x as u64).wrapping_mul(73_856_093) ^ (y as u64).wrapping_mul(19_349_663);
            if !h.is_multiple_of(31) {
                continue;
            }
            // Twinkle: brightness drifts with a per-star phase offset.
            let twinkle = (((h >> 5).wrapping_add(tick)) % 8) as f32 / 7.0;
            let level = (70.0 + 150.0 * twinkle) as u8;
            let symbol = if twinkle > 0.75 {
                '✦'
            } else if twinkle > 0.4 {
                '·'
            } else {
                '.'
            };
            scene.set(
                x,
                y,
                Cell {
                    symbol,
                    fg: Color::Rgb {
                        r: level,
                        g: level,
                        b: (level as u16 + 30).min(255) as u8,
                    },
                    bg: VOID,
                    style: CellStyle::default(),
                },
            );
        }
    }
}

fn header(scene: &mut Buffer) {
    let area = full(scene);
    text_center(
        scene,
        area,
        1,
        "◈  P O R T A L   N E X U S  ◈",
        Color::Rgb {
            r: 150,
            g: 220,
            b: 255,
        },
        true,
    );
    text_center(
        scene,
        area,
        2,
        "choose a reality",
        Color::Rgb {
            r: 90,
            g: 120,
            b: 160,
        },
        false,
    );
}

fn portals(scene: &mut Buffer, selected: usize, phase: f32) {
    let w = scene.width;
    let h = scene.height;
    let slot_w = w / 3;
    if slot_w < 6 {
        return;
    }
    let box_w = slot_w.saturating_sub(2).max(5);
    let box_h = h.saturating_sub(8).clamp(5, 11);
    let top = 4 + h.saturating_sub(8).saturating_sub(box_h) / 2;

    for (i, &(name, tagline, accent)) in PORTALS.iter().enumerate() {
        let slot_x = i as u16 * slot_w + slot_w.saturating_sub(box_w) / 2;
        let slot = Rect {
            x: slot_x,
            y: top,
            width: box_w,
            height: box_h,
        };
        portal::draw(scene, slot, name, tagline, accent, i == selected, phase);
    }
}

fn footer(scene: &mut Buffer) {
    let area = full(scene);
    let y = scene.height.saturating_sub(2);
    text_center(
        scene,
        area,
        y,
        "←/→ or Tab: choose    Enter: dive in    F12: back    q: quit",
        Color::Rgb {
            r: 120,
            g: 140,
            b: 170,
        },
        false,
    );
}
