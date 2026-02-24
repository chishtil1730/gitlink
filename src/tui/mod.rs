pub mod app;
pub mod commands;
pub mod components;
pub mod event;
pub mod ui;

use std::io;
use std::sync::mpsc;
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
    // When a router command is running in the background, its result arrives here.
    let mut pending_result: Option<mpsc::Receiver<OutputBlock>> = None;
    // Tracks when the current execution started so the spinner counts from 0.
    let mut exec_start: Option<std::time::Instant> = None;

    loop {
        // ── Poll for a finished background command ────────────────────────────
        if let Some(ref rx) = pending_result {
            if let Ok(output) = rx.try_recv() {
                app.push_output(output);
                pending_result = None;
                exec_start = None;
            }
        }

        // Compute how long the current command has been running (0.0 if idle).
        let spin_elapsed = exec_start
            .map(|t| t.elapsed().as_secs_f32())
            .unwrap_or(0.0);

        // One-shot hard terminal clear to flush ghost border chars left by
        // ratatui's double-buffer differ when an overlay closes.
        if app.needs_full_redraw {
            terminal.clear()?;
            app.needs_full_redraw = false;
        }

        terminal.draw(|f| ui::draw(f, app, spin_elapsed))?;

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

                        // ── everything else → router (non-blocking) ────────
                        _ => {
                            // Spawn router::execute on a blocking thread so the
                            // main loop keeps ticking and the spinner animates.
                            let (tx, rx) = mpsc::channel();
                            let cmd_owned = cmd.clone();
                            tokio::task::spawn_blocking(move || {
                                let output = router::execute(&cmd_owned);
                                let _ = tx.send(output);
                            });
                            pending_result = Some(rx);
                            exec_start = Some(std::time::Instant::now());
                        }
                    }
                }
            }
        }
    }
}