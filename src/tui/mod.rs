pub mod app;
pub mod commands;
pub mod components;
pub mod event;
pub mod ui;

use std::io;
use std::time::Duration;

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::app::router;
use app::{App, OutputBlock, OutputKind};
use event::{AppEvent, EventHandler};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let events = EventHandler::new(Duration::from_millis(16));

    let result = run_loop(&mut terminal, &mut app, &events);

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    events: &EventHandler,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        // One-shot hard terminal clear to flush ghost border chars left by
        // ratatui's double-buffer differ when an overlay closes.
        if app.needs_full_redraw {
            terminal.clear()?;
            app.needs_full_redraw = false;
        }

        terminal.draw(|f| ui::draw(f, app))?;

        match events.next()? {
            AppEvent::Tick => {
                app.on_tick();
            }
            AppEvent::Key(key) => {
                if app.on_key(key) {
                    return Ok(());
                }

                if let Some(cmd) = app.take_pending_command() {
                    let trimmed = cmd.trim_start_matches('/');
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    let root = parts.first().copied().unwrap_or("");
                    let sub = parts.get(1).copied().unwrap_or("");

                    match root {
                        // ── /plan → open planner overlay ──────────────────
                        "plan" => {
                            app.open_planner_overlay();
                        }

                        // ── /scan → run scan or manage ignored ──────────────────────────
                        "scan" => {
                            match sub {
                                "ignored" | "--manage-ignored" => {
                                    app.open_ignore_overlay();
                                }
                                "history" => {
                                    let mut f = crate::scanner::engine::scan_git_history(None);
                                    let db = crate::scanner::ignore::load_ignore_db();
                                    f.retain(|x| !db.ignored.iter().any(|i| i.fingerprint == x.fingerprint));
                                    app.open_scanner_overlay(f);
                                }
                                _ => {
                                    let mut f = crate::scanner::engine::scan_directory(".");
                                    let db = crate::scanner::ignore::load_ignore_db();
                                    f.retain(|x| !db.ignored.iter().any(|i| i.fingerprint == x.fingerprint));
                                    app.open_scanner_overlay(f);
                                }
                            }
                        }

                        // ── /clear ─────────────────────────────────────────
                        "clear" => {
                            app.outputs.clear();
                            app.is_executing = false;
                        }

                        // ── everything else → router ───────────────────────
                        _ => {
                            // Draw once now so the spinner is visible while the
                            // command blocks (router::execute may block_on async work)
                            terminal.draw(|f| ui::draw(f, app))?;
                            let output = router::execute(&cmd);
                            app.push_output(output);
                        }
                    }
                }
            }
        }
    }
}