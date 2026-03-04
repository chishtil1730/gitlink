mod app;
mod auth;
mod github;
mod planner;
mod prp_hub;
mod scanner;
mod tui;
///added action for mac os binary
fn main() {
    if let Err(e) = tui::run() {
        eprintln!("GitLink crashed: {}", e);
    }
}