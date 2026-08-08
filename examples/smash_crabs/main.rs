// examples/smash_crabs/main.rs — thin standalone entry; the App lives in
// smash_crabs.rs so the launcher example can reuse it via #[path].
#[path = "smash_crabs.rs"]
mod app;

fn main() -> std::io::Result<()> {
    ttui::app::run(&mut app::SmashCrabs::new())
}
