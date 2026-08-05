use crate::buffer::{diff, Buffer};
use crate::layout::Rect;
use crate::terminal::{install_panic_hook, Terminal};
use crossterm::event::Event;
use std::time::Duration;

pub trait App {
    fn update(&mut self, event: &Event);
    fn view(&self, area: Rect, buf: &mut Buffer);
    fn should_quit(&self) -> bool;

    fn tick_rate(&self) -> Option<Duration> {
        None
    }

    fn on_tick(&mut self, _elapsed: Duration) {}
}

pub fn run<A: App>(app: &mut A) -> std::io::Result<()> {
    install_panic_hook();
    let mut term = Terminal::new()?;

    let (w, h) = term.size()?;
    let mut prev = Buffer::new(w, h);
    let mut next = Buffer::new(w, h);
    app.view(
        Rect {
            x: 0,
            y: 0,
            width: w,
            height: h,
        },
        &mut next,
    );
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
            }
            None => {
                // Poll timed out with no input. If the app has opted into a
                // tick rate, this timeout IS the tick — call on_tick and
                // redraw. If it hasn't (tick_rate() is None), do nothing,
                // exactly like today: redraw only ever happens as a direct
                // consequence of an input event.
                if let Some(tick_rate) = app.tick_rate() {
                    app.on_tick(tick_rate);
                    should_redraw = true;
                }
            }
        }

        if should_redraw {
            let (w, h) = term.size()?;
            if (w, h) != (prev.width, prev.height) {
                prev = Buffer::new(w, h); // force full redraw on resize
            }
            let mut next = Buffer::new(w, h);
            app.view(
                Rect {
                    x: 0,
                    y: 0,
                    width: w,
                    height: h,
                },
                &mut next,
            );
            term.draw_diff(&diff(&prev, &next))?;
            prev = next;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::Buffer;
    use crate::layout::Rect;

    struct Dummy;

    impl App for Dummy {
        fn update(&mut self, _event: &Event) {}
        fn view(&self, _area: Rect, _buf: &mut Buffer) {}
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
