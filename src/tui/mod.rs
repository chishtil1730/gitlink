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
use ratatui::style::Color;
use ratatui::text::{Line, Span};
use ratatui::style::{Modifier, Style};

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
    let mut pending_result: Option<mpsc::Receiver<OutputBlock>> = None;
    let mut exec_start: Option<std::time::Instant> = None;
    // Track which command spawned the pending async task so we can display it in the right overlay
    let mut pending_cmd_name: Option<String> = None;

    loop {
        if let Some(ref rx) = pending_result {
            if let Ok(output) = rx.try_recv() {
                pending_result = None;
                exec_start = None;

                // Convert successful results for info-overlay commands into overlays
                let cmd = pending_cmd_name.take().unwrap_or_default();
                let is_overlay_cmd = matches!(
                    cmd.as_str(),
                    "show-activity" | "commits" | "pull-requests" | "repo-sync"
                    | "multi-sync" | "push-check" | "push-verify" | "branches"
                    | "issues" | "user-info"
                );

                if is_overlay_cmd && output.kind != OutputKind::Error {
                    let (title, accent) = overlay_meta(&cmd);
                    let lines = text_to_lines(output.content);
                    app.open_info_overlay(title, lines, accent);
                } else {
                    app.push_output(output);
                }
            }
        }

        let spin_elapsed = exec_start.map(|t| t.elapsed().as_secs_f32()).unwrap_or(0.0);

        if app.needs_full_redraw {
            terminal.clear()?;
            app.needs_full_redraw = false;
        }

        terminal.draw(|f| ui::draw(f, app, spin_elapsed))?;

        match events.next()? {
            AppEvent::Tick => { app.on_tick(); }
            AppEvent::Key(key) => {
                if app.on_key(key) {
                    return Ok(());
                }

                if let Some(cmd) = app.take_pending_command() {
                    let trimmed = cmd.trim_start_matches('/');
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    let root = parts.first().copied().unwrap_or("");
                    let sub  = parts.get(1).copied().unwrap_or("");

                    match root {
                        // ── Local / overlay commands ───────────────────────

                        "plan" => { app.open_planner_overlay(); }

                        "auth" => { app.open_auth_overlay(); }

                        "prp" => {
                            if sub == "list" {
                                // Show group list as info overlay
                                let lines = build_prp_list_lines();
                                app.open_info_overlay(
                                    "🔗 PRP Hub — Session Groups",
                                    lines,
                                    Color::Rgb(130, 90, 200),
                                );
                            } else {
                                // Discover repos and open interactive PRP overlay
                                let repos = discover_repo_names();
                                app.open_prp_overlay(repos);
                            }
                        }

                        "scan" => match sub {
                            "ignored" | "--manage-ignored" => { app.open_ignore_overlay(); }
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
                        },

                        "clear" => {
                            app.outputs.clear();
                            app.is_executing = false;
                        }

                        "quit" => { return Ok(()); }

                        // ── Async / info overlays ──────────────────────────

                        "show-activity" => {
                            let (tx, rx) = mpsc::channel();
                            tokio::task::spawn_blocking(move || {
                                let output = router::execute("/show-activity");
                                let _ = tx.send(output);
                            });
                            pending_result = Some(rx);
                            pending_cmd_name = Some("show-activity".to_string());
                            exec_start = Some(std::time::Instant::now());
                        }

                        "commits" => {
                            let (tx, rx) = mpsc::channel::<OutputBlock>();
                            tokio::task::spawn_blocking(move || { let _ = tx.send(router::execute("/commits")); });
                            pending_result = Some(rx);
                            pending_cmd_name = Some("commits".to_string());
                            exec_start = Some(std::time::Instant::now());
                        }

                        "pull-requests" => {
                            let (tx, rx) = mpsc::channel::<OutputBlock>();
                            tokio::task::spawn_blocking(move || { let _ = tx.send(router::execute("/pull-requests")); });
                            pending_result = Some(rx);
                            pending_cmd_name = Some("pull-requests".to_string());
                            exec_start = Some(std::time::Instant::now());
                        }

                        "repo-sync" => {
                            let (tx, rx) = mpsc::channel::<OutputBlock>();
                            tokio::task::spawn_blocking(move || { let _ = tx.send(router::execute("/repo-sync")); });
                            pending_result = Some(rx);
                            pending_cmd_name = Some("repo-sync".to_string());
                            exec_start = Some(std::time::Instant::now());
                        }

                        "multi-sync" => {
                            let (tx, rx) = mpsc::channel::<OutputBlock>();
                            tokio::task::spawn_blocking(move || { let _ = tx.send(router::execute("/multi-sync")); });
                            pending_result = Some(rx);
                            pending_cmd_name = Some("multi-sync".to_string());
                            exec_start = Some(std::time::Instant::now());
                        }

                        "push-check" => {
                            let (tx, rx) = mpsc::channel::<OutputBlock>();
                            tokio::task::spawn_blocking(move || { let _ = tx.send(router::execute("/push-check")); });
                            pending_result = Some(rx);
                            pending_cmd_name = Some("push-check".to_string());
                            exec_start = Some(std::time::Instant::now());
                        }

                        "push-verify" => {
                            let (tx, rx) = mpsc::channel::<OutputBlock>();
                            tokio::task::spawn_blocking(move || { let _ = tx.send(router::execute("/push-verify")); });
                            pending_result = Some(rx);
                            pending_cmd_name = Some("push-verify".to_string());
                            exec_start = Some(std::time::Instant::now());
                        }

                        "branches" => {
                            let (tx, rx) = mpsc::channel::<OutputBlock>();
                            tokio::task::spawn_blocking(move || { let _ = tx.send(router::execute("/branches")); });
                            pending_result = Some(rx);
                            pending_cmd_name = Some("branches".to_string());
                            exec_start = Some(std::time::Instant::now());
                        }

                        "issues" => {
                            let (tx, rx) = mpsc::channel::<OutputBlock>();
                            tokio::task::spawn_blocking(move || { let _ = tx.send(router::execute("/issues")); });
                            pending_result = Some(rx);
                            pending_cmd_name = Some("issues".to_string());
                            exec_start = Some(std::time::Instant::now());
                        }

                        "user-info" => {
                            let (tx, rx) = mpsc::channel::<OutputBlock>();
                            tokio::task::spawn_blocking(move || { let _ = tx.send(router::execute("/user-info")); });
                            pending_result = Some(rx);
                            pending_cmd_name = Some("user-info".to_string());
                            exec_start = Some(std::time::Instant::now());
                        }

                        // ── help → info overlay ────────────────────────────
                        "help" => {
                            let lines = build_help_lines();
                            app.open_info_overlay("📖 GitLink — Help", lines, Color::Rgb(100, 149, 237));
                        }

                        _ => {
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

                // ── Convert a completed async result into an info overlay ──
                // This is done by re-checking pending_result right after it was set;
                // the actual conversion happens at the top of the loop when try_recv fires.
                // We use the OutputBlock content to open an overlay on the *next* tick.
            }
        }

    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn overlay_meta(cmd: &str) -> (String, Color) {
    match cmd {
        "show-activity"  => ("📊 GitHub Activity".to_string(),          Color::Rgb(80,  180, 120)),
        "commits"        => ("💾 Recent Commits".to_string(),            Color::Rgb(100, 149, 237)),
        "pull-requests"  => ("🔀 Pull Requests".to_string(),             Color::Rgb(180, 120, 220)),
        "repo-sync"      => ("🔍 Repository Sync".to_string(),           Color::Rgb(100, 180, 200)),
        "multi-sync"     => ("📦 Multi-Repo Sync".to_string(),           Color::Rgb(100, 180, 200)),
        "push-check"     => ("✅ Push Check".to_string(),                Color::Rgb(80,  180, 120)),
        "push-verify"    => ("🚀 Push Verify".to_string(),               Color::Rgb(80,  180, 120)),
        "branches"       => ("🌿 Branches".to_string(),                  Color::Rgb(120, 200, 100)),
        "issues"         => ("📝 Issues".to_string(),                    Color::Rgb(230, 160, 60)),
        "user-info"      => ("👤 User Info".to_string(),                 Color::Rgb(100, 149, 237)),
        _                => (format!("  {}", cmd),                        Color::Rgb(100, 149, 237)),
    }
}

fn text_to_lines(text: String) -> Vec<Line<'static>> {
    text.lines()
        .map(|l| Line::from(Span::styled(
            l.to_string(),
            Style::default().fg(Color::Rgb(200, 205, 220)),
        )))
        .collect()
}

fn discover_repo_names() -> Vec<String> {
    use crate::prp_hub::discovery::discover_repositories;
    match discover_repositories(".") {
        Ok(repos) => repos.iter().map(|r| r.path.to_string_lossy().to_string()).collect(),
        Err(_) => vec![],
    }
}

fn build_prp_list_lines() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(Span::styled(
            "  PRP Session Groups",
            Style::default().fg(Color::Rgb(200, 200, 220)).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Group history is stored in ~/.gitlink/prp_groups.json",
            Style::default().fg(Color::Rgb(150, 155, 175)),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Use /prp to start a new session.",
            Style::default().fg(Color::Rgb(100, 149, 237)),
        )),
    ]
}

fn build_help_lines() -> Vec<Line<'static>> {
    let entries: &[(&str, &str)] = &[
        ("/auth",           "Manage GitHub authentication — login / logout / status"),
        ("/scan",           "Scan working directory for secrets (interactive overlay)"),
        ("/scan history",   "Scan git history for exposed secrets"),
        ("/scan ignored",   "Manage permanently ignored findings"),
        ("/plan",           "Open the task planner overlay"),
        ("/prp",            "Start an interactive poly-repo commit session"),
        ("/prp list",       "View PRP session groups"),
        ("/show-activity",  "Show your GitHub contribution activity"),
        ("/commits",        "Show recent commits"),
        ("/pull-requests",  "Show open pull requests"),
        ("/repo-sync",      "Select a repository and check sync status"),
        ("/multi-sync",     "Check sync across multiple repositories"),
        ("/push-check",     "Check if latest commit is pushed to remote"),
        ("/push-verify",    "Verify if pushing is currently possible"),
        ("/branches",       "Show local and remote branches"),
        ("/issues",         "Show issues and GitHub Actions status"),
        ("/user-info",      "Show basic GitHub user info"),
        ("/clear",          "Clear the output history"),
        ("/quit",           "Exit GitLink TUI"),
    ];

    let mut lines: Vec<Line<'static>> = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Available Commands",
            Style::default().fg(Color::Rgb(200, 210, 255)).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for (cmd, desc) in entries {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:20}", cmd),
                Style::default().fg(Color::Rgb(100, 149, 237)).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                desc.to_string(),
                Style::default().fg(Color::Rgb(170, 175, 195)),
            ),
        ]));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  ↑↓ ", Style::default().fg(Color::Rgb(100, 149, 237))),
        Span::styled("navigate suggestions   ", Style::default().fg(Color::Rgb(130, 135, 155))),
        Span::styled("Tab ", Style::default().fg(Color::Rgb(100, 149, 237))),
        Span::styled("autocomplete   ", Style::default().fg(Color::Rgb(130, 135, 155))),
        Span::styled("Ctrl+C ", Style::default().fg(Color::Rgb(100, 149, 237))),
        Span::styled("quit", Style::default().fg(Color::Rgb(130, 135, 155))),
    ]));

    lines
}