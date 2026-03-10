mod app;
mod auth;
mod github;
mod planner;
mod prp_hub;
mod scanner;
mod tui;



//Used async tokio for running the main
#[tokio::main]
async fn main() {
    if let Err(e) = tui::run() {
        eprintln!("GitLink crashed: {}", e);
    }
}