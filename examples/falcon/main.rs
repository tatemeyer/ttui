// examples/falcon/main.rs — thin standalone entry; the App lives in
// falcon.rs so the launcher example can reuse it via #[path], same
// convention as every other themed app.
#[path = "falcon.rs"]
mod app;

fn main() -> std::io::Result<()> {
    ttui::app::run(&mut app::Falcon::new())
}
