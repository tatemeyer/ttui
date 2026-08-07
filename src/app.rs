//! Terminal event loop: polls input, dispatches to an `App`, and
//! redraws only the cells that changed since the last frame.

use crate::buffer::{diff, Buffer, LayerStack};
use crate::layout::Rect;
use crate::terminal::{install_panic_hook, Terminal};
use crossterm::event::Event;
use std::time::Duration;

/// An interactive terminal app: reacts to input, renders into a
/// `LayerStack`, and optionally ticks on a fixed interval.
pub trait App {
    /// Handle one input event, mutating app state.
    fn update(&mut self, event: &Event);
    /// Render current state into `buf` for the given terminal `area`.
    fn view(&self, area: Rect, buf: &mut LayerStack);
    /// Whether `run` should exit its loop after this update.
    fn should_quit(&self) -> bool;

    /// Poll timeout used as this app's animation tick rate — `None`
    /// (the default) means the app never ticks on its own.
    fn tick_rate(&self) -> Option<Duration> {
        None
    }

    /// Called once per tick when `tick_rate` is `Some`, with the real
    /// elapsed time since the previous tick or input event.
    fn on_tick(&mut self, _elapsed: Duration) {}
}

/// Runs `app`'s event loop until it requests to quit: enables raw
/// mode, polls for input/ticks, and diff-redraws the terminal.
pub fn run<A: App>(app: &mut A) -> std::io::Result<()> {
    install_panic_hook();
    let mut term = Terminal::new()?;

    let (w, h) = term.size()?;
    let mut last_tick_at = std::time::Instant::now();
    let mut prev = Buffer::new(w, h);
    let mut stack = LayerStack::new(w, h);
    app.view(
        Rect {
            x: 0,
            y: 0,
            width: w,
            height: h,
        },
        &mut stack,
    );
    let next = stack.composite();
    term.draw_diff(&diff(&prev, &next))?;
    prev = next;

    loop {
        let poll_timeout = app.tick_rate().unwrap_or(Duration::from_millis(250));
        let mut should_redraw = false;

        match term.next_event(poll_timeout)? {
            Some(event) => {
                app.update(&event);
                if app.should_quit() {
                    break;
                }
                should_redraw = true;
                // Reset the tick tracker on every input event too, so a
                // burst of rapid typing followed by a tick doesn't report
                // one huge elapsed jump.
                last_tick_at = std::time::Instant::now();
            }
            None => {
                // Poll timed out with no input. If the app has opted into a
                // tick rate, this timeout IS the tick — call on_tick and
                // redraw. If it hasn't (tick_rate() is None), do nothing,
                // exactly like today: redraw only ever happens as a direct
                // consequence of an input event.
                if app.tick_rate().is_some() {
                    let now = std::time::Instant::now();
                    let elapsed = now.duration_since(last_tick_at);
                    last_tick_at = now;
                    app.on_tick(elapsed);
                    should_redraw = true;
                }
            }
        }

        if should_redraw {
            let (w, h) = term.size()?;
            if (w, h) != (prev.width, prev.height) {
                prev = Buffer::new(w, h); // force full redraw on resize
            }
            let mut stack = LayerStack::new(w, h);
            app.view(
                Rect {
                    x: 0,
                    y: 0,
                    width: w,
                    height: h,
                },
                &mut stack,
            );
            let next = stack.composite();
            term.draw_diff(&diff(&prev, &next))?;
            prev = next;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::Rect;

    struct Dummy;

    impl App for Dummy {
        fn update(&mut self, _event: &Event) {}
        fn view(&self, _area: Rect, _buf: &mut LayerStack) {}
        fn should_quit(&self) -> bool {
            false
        }
    }

    #[test]
    fn tick_rate_defaults_to_none() {
        let dummy = Dummy;
        assert_eq!(dummy.tick_rate(), None);
    }

    #[test]
    fn on_tick_default_is_a_no_op() {
        let mut dummy = Dummy;
        dummy.on_tick(Duration::from_millis(16));
        assert!(!dummy.should_quit());
    }
}
