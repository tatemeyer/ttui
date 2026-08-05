use crate::buffer::{diff, Buffer};
use crate::layout::Rect;
use crate::terminal::{install_panic_hook, Terminal};
use crossterm::event::Event;
use std::time::Duration;

pub trait App {
    fn update(&mut self, event: &Event);
    fn view(&self, area: Rect, buf: &mut Buffer);
    fn should_quit(&self) -> bool;
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
