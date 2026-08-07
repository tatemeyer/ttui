use super::*;

impl SmashCrabs {
    pub(crate) fn paint_ui(&self, area: Rect) -> Buffer {
        let mut buf = Buffer::new(area.width, area.height);
        let local = Rect {
            x: 0,
            y: 0,
            width: area.width,
            height: area.height,
        };
        let panel = Layout::new(Direction::Vertical, vec![Constraint::Fixed(8)]).split(local)[0];
        let panel = Rect {
            width: panel.width.min(24),
            ..panel
        };
        let inner = SmashBorder::new().render(panel, &self.theme, &mut buf);
        let rows = Layout::new(
            Direction::Vertical,
            vec![Constraint::Fixed(1), Constraint::Fixed(1)],
        )
        .split(inner);
        DamageMeter::new(0).render(rows[0], &mut buf);
        DamageMeter::new(self.displayed_p2_damage().round() as u16).render(rows[1], &mut buf);
        buf
    }

    pub(crate) fn paint_effects(&self, area: Rect) -> Buffer {
        let mut buf = Buffer::new(area.width, area.height);
        if self.flash_ticks_remaining > 0 {
            let flash = Cell {
                symbol: '*',
                fg: Color::Black,
                bg: self.theme.accent,
                ..Default::default()
            };
            let w = 7.min(area.width);
            let h = 3.min(area.height);
            let x0 = (area.width.saturating_sub(w)) / 2;
            let y0 = (area.height.saturating_sub(h)) / 2;
            for y in y0..y0 + h {
                for x in x0..x0 + w {
                    buf.set(x, y, flash.clone());
                }
            }
        }
        self.particles.render(&mut buf);
        buf
    }

    pub(crate) fn render_versus(&self, area: Rect, buf: &mut LayerStack) {
        let (dx, dy) = self.shake_offset();
        let layers: [(usize, Buffer); 3] = [
            (BACKGROUND, self.paint_background(area)),
            (UI, self.paint_ui(area)),
            (EFFECTS, self.paint_effects(area)),
        ];
        for (index, scratch) in layers {
            let final_buf = if dx != 0 || dy != 0 {
                effects::shake(&scratch, dx, dy)
            } else {
                scratch
            };
            blit(&final_buf, area, buf.layer_mut(index));
        }
    }
}
