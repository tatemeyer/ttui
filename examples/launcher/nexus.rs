// examples/launcher/nexus.rs — renders the portal nexus: a starfield
// void, a title, three app portals, and a hint row. Rendered into a
// scratch buffer so a return-fade can dim the whole scene uniformly
// before it is blitted into the live layer stack.
use crossterm::style::Color;
use ttui::buffer::{Buffer, Cell, CellStyle, LayerStack};
use ttui::camera;
use ttui::layout::Rect;
use ttui::particles::ParticleSystem;

use crate::{portal, text_center, PORTALS, VOID};

/// Renders the nexus for `selected`/`phase` into `buf`, dimmed by
/// `fade` (1.0 = fully visible; < 1.0 during the return transition).
pub(crate) fn render(
    selected: usize,
    starfield: &ParticleSystem,
    phase: f32,
    fade: f32,
    area: Rect,
    buf: &mut LayerStack,
) {
    if area.width < 12 || area.height < 10 {
        return;
    }
    let mut scene = Buffer::new(area.width, area.height);
    fill_void(&mut scene);
    starfield.render(&mut scene);
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
    let base_w = slot_w.saturating_sub(2).max(5);
    let base_h = h.saturating_sub(8).clamp(5, 11);
    let focus_w = (base_w + 2).min(slot_w.saturating_sub(1));
    let focus_h = (base_h + 1).min(h.saturating_sub(2));

    for (i, &(name, tagline, accent)) in PORTALS.iter().enumerate() {
        let focused = i == selected;
        let box_w = if focused { focus_w } else { base_w };
        let box_h = if focused { focus_h } else { base_h };
        let slot_x = (i as u16 * slot_w + slot_w.saturating_sub(box_w) / 2).max(1);
        let top = 4 + h.saturating_sub(8).saturating_sub(box_h) / 2;
        let slot = Rect {
            x: slot_x,
            y: top,
            width: box_w,
            height: box_h,
        };
        portal::draw(scene, slot, name, tagline, accent, focused, phase);
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
