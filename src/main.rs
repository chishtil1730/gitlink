mod app;
mod auth;
mod github;
mod planner;
mod prp_hub;
mod scanner;
mod tui;

fn main() {
    if let Err(e) = tui::run() {
        eprintln!("GitLink crashed: {}", e);
    }
}