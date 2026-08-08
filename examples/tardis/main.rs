// examples/tardis/main.rs — thin standalone entry; the App lives in
// tardis.rs so the launcher example can reuse it via #[path].
#[path = "tardis.rs"]
mod app;

fn main() -> std::io::Result<()> {
    ttui::app::run(&mut app::Tardis::new())
}
