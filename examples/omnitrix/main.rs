// examples/omnitrix/main.rs — thin standalone entry; the App lives in
// omnitrix.rs so the launcher example can reuse it via #[path].
#[path = "omnitrix.rs"]
mod app;

fn main() -> std::io::Result<()> {
    ttui::app::run(&mut app::Omnitrix::new())
}
