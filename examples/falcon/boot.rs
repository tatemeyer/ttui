use super::*;

impl Falcon {
    pub(crate) fn render_boot(&self, area: Rect, progress: f32, buf: &mut LayerStack) {
        if progress < 0.1 {
            let cx = area.x + area.width / 2;
            let cy = area.y + area.height / 2;
            buf.set(
                cx,
                cy,
                Cell {
                    symbol: '\u{2022}', // '•'
                    fg: self.theme.primary,
                    bg: Color::Reset,
                    alpha: 1.0,
                    ..Default::default()
                },
            );
            return;
        }

        if progress < 0.7 {
            let wave = (progress - 0.1) / 0.6;
            let panels_shown = ((wave * 3.0) as usize).min(3);
            let slots = Self::panel_slots(area);
            let mut newest_inner = None;
            for (i, kind) in PANELS.iter().enumerate().take(panels_shown) {
                let panel_box = Self::panel_box(slots[i], false);
                let inner = CockpitPanel::new(false).render(panel_box, &self.theme, buf);
                Text::new(kind.name()).render(inner, buf);
                newest_inner = Some(inner);
            }
            // The most-recently-revealed panel gets a static burst: a
            // freshly-triggered GlitchBuffer rendered in the same frame
            // (never ticked) always renders at full intensity, so this
            // panel flashes static until the next one takes over as
            // "newest" — the "brief static burst" the design spec calls
            // for at each panel's reveal moment.
            if let Some(inner) = newest_inner {
                let mut burst = GlitchBuffer::new();
                burst.trigger(Duration::from_millis(300));
                buf.push_layer();
                burst.render(inner, self.theme.tertiary, self.tick_count, buf);
            }
            return;
        }

        let fade = ((progress - 0.7) / 0.3).clamp(0.0, 1.0);
        // Render into an isolated scratch LayerStack, not `buf` directly:
        // render_dashboard pushes its own glitch/particle layer, and if we
        // dimmed cells directly on `buf` afterward we'd only be rewriting
        // its base layer — the un-dimmed glitch/particle layer would still
        // be there for app.rs's own final composite() to blend back in on
        // top, undoing the fade. Compositing the scratch stack down to a
        // flat Buffer first, then writing dimmed cells onto `buf` (which
        // stays single-layer throughout this branch), avoids that leak.
        let mut scratch = LayerStack::new(area.width, area.height);
        self.render_dashboard(
            Rect {
                x: 0,
                y: 0,
                width: area.width,
                height: area.height,
            },
            &mut scratch,
        );
        let composited = scratch.composite();
        for y in 0..area.height {
            for x in 0..area.width {
                let real = composited.get(x, y);
                let dimmed = Cell {
                    symbol: real.symbol,
                    fg: ttui::easing::lerp_color(self.theme.background, real.fg, fade),
                    bg: ttui::easing::lerp_color(self.theme.background, real.bg, fade),
                    style: real.style,
                    alpha: 1.0,
                };
                buf.set(area.x + x, area.y + y, dimmed);
            }
        }
    }
}
