use super::*;

impl Falcon {
    pub(crate) fn render_boot(&self, area: Rect, progress: f32, buf: &mut LayerStack) {
        let (windshield, console) = Self::windshield_console_split(area);

        if progress < 0.1 {
            let cx = area.x + area.width / 2;
            let cy = area.y + area.height / 2;
            buf.set(
                cx,
                cy,
                Cell {
                    symbol: '*',
                    fg: self.theme.primary,
                    bg: Color::Reset,
                    alpha: 1.0,
                    ..Default::default()
                },
            );
            return;
        }

        if progress < 0.4 {
            // Windshield power-on: the starfield is already fully present
            // (render_windshield always draws it in full), while the
            // canopy wireframe reveals a growing prefix of CANOPY_EDGES —
            // ceil() so the very first edge appears immediately at the
            // start of this phase rather than needing a full step of
            // `wave` to show anything.
            let wave = (progress - 0.1) / 0.3;
            let edges_shown = ((wave * 12.0).ceil() as usize).min(12);
            self.render_windshield(windshield, buf, edges_shown, false);
            return;
        }

        if progress < 0.85 {
            // Windshield is fully revealed and stays visible above the
            // console strip while its panels reveal underneath.
            self.render_windshield(windshield, buf, 12, true);

            let wave = (progress - 0.4) / 0.45;
            let panels_shown = ((wave * 3.0).ceil() as usize).min(3);
            let slots = Self::panel_slots(console);
            let mut newest: Option<(usize, Rect)> = None;
            for (i, kind) in PANELS.iter().enumerate().take(panels_shown) {
                let panel_box = Self::panel_box(slots[i], false);
                let inner = CockpitPanel::new(false).render(panel_box, &self.theme, buf);
                Text::new(kind.name()).render(inner, buf);
                newest = Some((i, inner));
            }
            // The most-recently-revealed panel gets a static burst that
            // decays over its own reveal window, rather than staying
            // pinned at full intensity: a freshly-triggered GlitchBuffer
            // rendered without ticking always renders at full intensity
            // (100% cell coverage), which would fully blank the panel
            // instead of overlaying it — so we manually tick the burst
            // forward by how far into its own reveal slice `wave` is.
            if let Some((newest_index, inner)) = newest {
                // How far into this panel's own reveal slot we are, in
                // [0.0, 1.0] — each panel occupies a 1/3-wide slice of
                // `wave`; this normalizes position-within-slice so the
                // burst can decay smoothly across it rather than staying
                // pinned at full intensity the whole time.
                let local_wave = ((wave * 3.0) - newest_index as f32).clamp(0.0, 1.0);
                let mut burst = GlitchBuffer::new();
                let burst_duration = Duration::from_millis(300);
                burst.trigger(burst_duration);
                burst.tick(Duration::from_secs_f32(
                    local_wave * burst_duration.as_secs_f32(),
                ));
                let overlay = buf.push_layer();
                burst.render(inner, self.theme.tertiary, self.tick_count, overlay);
            }
            return;
        }

        // Ramp from `BOOT_FADE_FLOOR`, not from zero. `lerp_color` at
        // fade 0 returns the background outright, so a tick landing near
        // the 0.85 boundary rendered the whole cockpit at under 16%
        // brightness for one frame — a visible dip *downward* from the
        // panel-reveal phase that immediately precedes it, which is
        // already drawn at roughly 88% of the live dashboard's
        // brightness (measured: mean luma 13.2 against 15.05). Starting
        // at that same level makes the boundary continuous and leaves
        // the last sliver of brightening as the intended flourish (#117).
        let fade =
            BOOT_FADE_FLOOR + (1.0 - BOOT_FADE_FLOOR) * ((progress - 0.85) / 0.15).clamp(0.0, 1.0);
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
