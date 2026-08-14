// showcase/main.rs — thin standalone entry; the App lives in
// showcase.rs, matching the convention every themed example app uses.
#[path = "showcase.rs"]
mod app;

fn main() -> std::io::Result<()> {
    ttui::app::run(&mut app::ShowcaseApp::new())
}
