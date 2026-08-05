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
        if let Some(event) = term.next_event(Duration::from_millis(250))? {
            app.update(&event);
            if app.should_quit() {
                break;
            }
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
