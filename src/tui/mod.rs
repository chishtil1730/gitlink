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
    // Multi-sync overlay: separate channel for repo list fetch and sync results
    let mut pending_multi_sync: Option<mpsc::Receiver<MultiSyncMsg>> = None;

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
                    | "push-check" | "push-verify" | "branches"
                    | "issues" | "user-info"
                );

                if is_overlay_cmd && output.kind != OutputKind::Error {
                    let (title, accent) = overlay_meta(&cmd);
                    let lines = build_overlay_lines(&cmd, output.content);
                    app.open_info_overlay(title, lines, accent);
                } else {
                    app.push_output(output);
                }
            }
        }

        // Poll multi-sync background task
        if let Some(ref rx) = pending_multi_sync {
            if let Ok(msg) = rx.try_recv() {
                pending_multi_sync = None;
                if let Some(crate::tui::app::Overlay::MultiSync(ref mut ov)) = app.overlay {
                    match msg {
                        MultiSyncMsg::RepoList(repos) => {
                            ov.repos = repos;
                            ov.step = crate::tui::app::MultiSyncStep::SelectRepos;
                            app.is_executing = false;
                        }
                        MultiSyncMsg::SyncResults(lines) => {
                            ov.result_lines = lines;
                            ov.step = crate::tui::app::MultiSyncStep::Results;
                            ov.scroll = 0;
                            app.is_executing = false;
                        }
                        MultiSyncMsg::Error(msg) => {
                            ov.done = true;
                            app.is_executing = false;
                            app.push_output(crate::tui::app::OutputBlock {
                                kind: crate::tui::app::OutputKind::Error,
                                content: msg,
                            });
                        }
                    }
                }
                app.needs_full_redraw = true;
            }
        }

        // If multi-sync is in Running state, kick off the sync task
        let should_run_sync = if let Some(crate::tui::app::Overlay::MultiSync(ref ov)) = app.overlay {
            ov.step == crate::tui::app::MultiSyncStep::Running && pending_multi_sync.is_none()
        } else { false };

        if should_run_sync {
            let selected: Vec<String> = if let Some(crate::tui::app::Overlay::MultiSync(ref ov)) = app.overlay {
                ov.repos.iter().filter(|r| r.selected).map(|r| r.name_with_owner.clone()).collect()
            } else { vec![] };

            let (tx, rx) = mpsc::channel::<MultiSyncMsg>();
            pending_multi_sync = Some(rx);
            std::thread::spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run_multi_sync_check(selected)
                }));
                match result {
                    Ok(lines) => { let _ = tx.send(MultiSyncMsg::SyncResults(lines)); }
                    Err(_)    => { let _ = tx.send(MultiSyncMsg::Error("Sync check failed unexpectedly.".to_string())); }
                }
            });
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

                        "plan" => {
                            app.outputs.push(crate::tui::app::OutputBlock { kind: crate::tui::app::OutputKind::Info, content: "Opening task planner…".to_string() });
                            app.open_planner_overlay();
                        }

                        "auth" => {
                            app.outputs.push(crate::tui::app::OutputBlock { kind: crate::tui::app::OutputKind::Info, content: "Opening GitHub auth…".to_string() });
                            app.open_auth_overlay();
                        }

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
                                app.outputs.push(crate::tui::app::OutputBlock { kind: crate::tui::app::OutputKind::Info, content: "Opening poly-repo commit session…".to_string() });
                                let repos = discover_repo_names();
                                app.open_prp_overlay(repos);
                            }
                        }

                        "scan" => match sub {
                            "ignored" | "--manage-ignored" => {
                                app.outputs.push(crate::tui::app::OutputBlock { kind: crate::tui::app::OutputKind::Info, content: "Opening ignored findings…".to_string() });
                                app.open_ignore_overlay();
                            }
                            "history" => {
                                app.outputs.push(crate::tui::app::OutputBlock { kind: crate::tui::app::OutputKind::Info, content: "Scanning git history…".to_string() });
                                let mut f = crate::scanner::engine::scan_git_history(None);
                                let db = crate::scanner::ignore::load_ignore_db();
                                f.retain(|x| !db.ignored.iter().any(|i| i.fingerprint == x.fingerprint));
                                app.open_scanner_overlay(f);
                            }
                            _ => {
                                app.outputs.push(crate::tui::app::OutputBlock { kind: crate::tui::app::OutputKind::Info, content: "Scanning working directory…".to_string() });
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
                            std::thread::spawn(move || {
                                let output = router::execute("/show-activity");
                                let _ = tx.send(output);
                            });
                            pending_result = Some(rx);
                            pending_cmd_name = Some("show-activity".to_string());
                            exec_start = Some(std::time::Instant::now());
                        }

                        "commits" => {
                            let (tx, rx) = mpsc::channel::<OutputBlock>();
                            std::thread::spawn(move || {
                                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| router::execute("/commits")));
                                let output = result.unwrap_or_else(|_| crate::tui::app::OutputBlock {
                                    kind: crate::tui::app::OutputKind::Error,
                                    content: "Command failed unexpectedly.".to_string(),
                                });
                                let _ = tx.send(output);
                            });
                            pending_result = Some(rx);
                            pending_cmd_name = Some("commits".to_string());
                            exec_start = Some(std::time::Instant::now());
                        }

                        "pull-requests" => {
                            let (tx, rx) = mpsc::channel::<OutputBlock>();
                            std::thread::spawn(move || {
                                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| router::execute("/pull-requests")));
                                let output = result.unwrap_or_else(|_| crate::tui::app::OutputBlock {
                                    kind: crate::tui::app::OutputKind::Error,
                                    content: "Command failed unexpectedly.".to_string(),
                                });
                                let _ = tx.send(output);
                            });
                            pending_result = Some(rx);
                            pending_cmd_name = Some("pull-requests".to_string());
                            exec_start = Some(std::time::Instant::now());
                        }

                        "repo-sync" => {
                            let (tx, rx) = mpsc::channel::<OutputBlock>();
                            std::thread::spawn(move || {
                                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| router::execute("/repo-sync")));
                                let output = result.unwrap_or_else(|_| crate::tui::app::OutputBlock {
                                    kind: crate::tui::app::OutputKind::Error,
                                    content: "Command failed unexpectedly.".to_string(),
                                });
                                let _ = tx.send(output);
                            });
                            pending_result = Some(rx);
                            pending_cmd_name = Some("repo-sync".to_string());
                            exec_start = Some(std::time::Instant::now());
                        }

                        "multi-sync" => {
                            app.open_multi_sync_overlay();
                            app.outputs.push(crate::tui::app::OutputBlock {
                                kind: crate::tui::app::OutputKind::Info,
                                content: "Opening multi-repo sync…".to_string(),
                            });
                            let (tx, rx) = mpsc::channel::<MultiSyncMsg>();
                            pending_multi_sync = Some(rx);
                            std::thread::spawn(move || {
                                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(fetch_repos_for_multi_sync));
                                match result {
                                    Ok(Ok(repos))  => { let _ = tx.send(MultiSyncMsg::RepoList(repos)); }
                                    Ok(Err(e))     => { let _ = tx.send(MultiSyncMsg::Error(e)); }
                                    Err(_)         => { let _ = tx.send(MultiSyncMsg::Error("Failed to fetch repositories.".to_string())); }
                                }
                            });
                        }

                        "push-check" => {
                            let (tx, rx) = mpsc::channel::<OutputBlock>();
                            std::thread::spawn(move || {
                                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| router::execute("/push-check")));
                                let output = result.unwrap_or_else(|_| crate::tui::app::OutputBlock {
                                    kind: crate::tui::app::OutputKind::Error,
                                    content: "Command failed unexpectedly.".to_string(),
                                });
                                let _ = tx.send(output);
                            });
                            pending_result = Some(rx);
                            pending_cmd_name = Some("push-check".to_string());
                            exec_start = Some(std::time::Instant::now());
                        }

                        "push-verify" => {
                            let (tx, rx) = mpsc::channel::<OutputBlock>();
                            std::thread::spawn(move || {
                                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| router::execute("/push-verify")));
                                let output = result.unwrap_or_else(|_| crate::tui::app::OutputBlock {
                                    kind: crate::tui::app::OutputKind::Error,
                                    content: "Command failed unexpectedly.".to_string(),
                                });
                                let _ = tx.send(output);
                            });
                            pending_result = Some(rx);
                            pending_cmd_name = Some("push-verify".to_string());
                            exec_start = Some(std::time::Instant::now());
                        }

                        "branches" => {
                            let (tx, rx) = mpsc::channel::<OutputBlock>();
                            std::thread::spawn(move || {
                                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| router::execute("/branches")));
                                let output = result.unwrap_or_else(|_| crate::tui::app::OutputBlock {
                                    kind: crate::tui::app::OutputKind::Error,
                                    content: "Command failed unexpectedly.".to_string(),
                                });
                                let _ = tx.send(output);
                            });
                            pending_result = Some(rx);
                            pending_cmd_name = Some("branches".to_string());
                            exec_start = Some(std::time::Instant::now());
                        }

                        "issues" => {
                            let (tx, rx) = mpsc::channel::<OutputBlock>();
                            std::thread::spawn(move || {
                                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| router::execute("/issues")));
                                let output = result.unwrap_or_else(|_| crate::tui::app::OutputBlock {
                                    kind: crate::tui::app::OutputKind::Error,
                                    content: "Command failed unexpectedly.".to_string(),
                                });
                                let _ = tx.send(output);
                            });
                            pending_result = Some(rx);
                            pending_cmd_name = Some("issues".to_string());
                            exec_start = Some(std::time::Instant::now());
                        }

                        "user-info" => {
                            let (tx, rx) = mpsc::channel::<OutputBlock>();
                            std::thread::spawn(move || {
                                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| router::execute("/user-info")));
                                let output = result.unwrap_or_else(|_| crate::tui::app::OutputBlock {
                                    kind: crate::tui::app::OutputKind::Error,
                                    content: "Command failed unexpectedly.".to_string(),
                                });
                                let _ = tx.send(output);
                            });
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
                            std::thread::spawn(move || {
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

fn build_overlay_lines(cmd: &str, content: String) -> Vec<Line<'static>> {
    match cmd {
        "show-activity" => build_activity_lines(content),
        "commits"       => build_commits_lines(content),
        "pull-requests" => build_prs_lines(content),
        "repo-sync"     => build_reposync_lines(content),
        "multi-sync"    => build_multisync_lines(content),
        "push-check"    => build_push_lines(content),
        "push-verify"   => build_push_lines(content),
        "branches"      => build_branches_lines(content),
        "issues"        => build_issues_lines(content),
        "user-info"     => build_userinfo_lines(content),
        _               => build_generic_lines(content),
    }
}

// ── Palette ──────────────────────────────────────────────────────────────────
const C_HEADER:  Color = Color::Rgb(210, 218, 255); // bold section title
const C_LABEL:   Color = Color::Rgb(100, 110, 150); // key / label
const C_VALUE:   Color = Color::Rgb(218, 224, 245); // value text
const C_DIM:     Color = Color::Rgb(50,  55,  72);  // divider / ghost
const C_GREEN:   Color = Color::Rgb(80,  210, 130); // success / positive
const C_YELLOW:  Color = Color::Rgb(230, 180, 60);  // warning
const C_RED:     Color = Color::Rgb(220, 80,  80);  // error
const C_BLUE:    Color = Color::Rgb(100, 155, 245); // accent
const C_PURPLE:  Color = Color::Rgb(160, 120, 240); // commit hash / branch
const C_TEAL:    Color = Color::Rgb(80,  195, 200); // dates
const C_BODY:    Color = Color::Rgb(185, 192, 218); // default body

// ── Shared helpers ────────────────────────────────────────────────────────────

fn sep() -> Line<'static> {
    Line::from(Span::styled(
        "  ─────────────────────────────────────────────────────────",
        Style::default().fg(C_DIM),
    ))
}

fn empty() -> Line<'static> { Line::from("") }

fn header(text: impl Into<String>) -> Line<'static> {
    Line::from(Span::styled(
        text.into(),
        Style::default().fg(C_HEADER).add_modifier(Modifier::BOLD),
    ))
}

fn kv(key: impl Into<String>, value: impl Into<String>, vcolor: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {:22}", key.into()), Style::default().fg(C_LABEL)),
        Span::styled(value.into(), Style::default().fg(vcolor).add_modifier(Modifier::BOLD)),
    ])
}

fn status_icon(ok: bool) -> (&'static str, Color) {
    if ok { ("✔", C_GREEN) } else { ("✖", C_RED) }
}

fn warn_icon(problem: bool) -> (&'static str, Color) {
    if problem { ("⚠", C_YELLOW) } else { ("✔", C_GREEN) }
}

// ── show-activity ─────────────────────────────────────────────────────────────

fn build_activity_lines(content: String) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = vec![empty()];
    let mut section = "";

    for raw in content.lines() {
        let l = raw.trim_end().to_string();
        let t = l.trim();

        if t.is_empty() { lines.push(empty()); continue; }

        // "GitHub Activity — Name (login)"
        if t.starts_with("GitHub Activity") {
            let parts: Vec<&str> = t.splitn(3, " — ").collect();
            let who = parts.get(1).copied().unwrap_or(t);
            lines.push(Line::from(vec![
                Span::styled("  ◈  ", Style::default().fg(C_GREEN).add_modifier(Modifier::BOLD)),
                Span::styled(who.to_string(), Style::default().fg(C_HEADER).add_modifier(Modifier::BOLD)),
            ]));
            lines.push(sep());
            lines.push(empty());
            section = "stats";
            continue;
        }

        // Stats section: "Total contributions : 455"
        if section == "stats" && t.contains(" : ") {
            let parts: Vec<&str> = t.splitn(2, " : ").collect();
            let key = parts[0].trim();
            let val = parts.get(1).copied().unwrap_or("").trim();
            let (vcolor, icon) = match key {
                "Total contributions" => (C_GREEN,  "⬡"),
                "Commits"             => (C_BLUE,   "◎"),
                "Pull requests"       => (C_PURPLE, "⎇"),
                "Issues"              => (C_YELLOW, "⊙"),
                "Repos created"       => (C_TEAL,   "▣"),
                _                     => (C_VALUE,  "·"),
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", icon), Style::default().fg(vcolor)),
                Span::styled(format!("{:22}", key), Style::default().fg(C_LABEL)),
                Span::styled(val.to_string(), Style::default().fg(vcolor).add_modifier(Modifier::BOLD)),
            ]));
            continue;
        }

        // "Last 3 days:" header
        if t == "Last 3 days:" {
            lines.push(empty());
            lines.push(sep());
            lines.push(Line::from(Span::styled(
                "  Last 3 days",
                Style::default().fg(C_HEADER).add_modifier(Modifier::BOLD),
            )));
            lines.push(empty());
            section = "days";
            continue;
        }

        // Day bars: "  2026-03-04 : ███ (3)"
        if section == "days" && t.contains(" : ") {
            let parts: Vec<&str> = t.splitn(2, " : ").collect();
            let date = parts[0].trim();
            let rest = parts.get(1).copied().unwrap_or("").trim();
            // rest = "███ (3)" — split off the count in parens
            let (bar_part, count_part) = if let Some(p) = rest.rfind('(') {
                (rest[..p].trim_end(), &rest[p..])
            } else {
                (rest, "")
            };
            // Colour bar by density: more blocks = brighter green
            let block_count = bar_part.chars().filter(|&c| c == '█').count();
            let bar_color = if block_count >= 10 { C_GREEN }
            else if block_count >= 4 { Color::Rgb(60, 180, 110) }
            else if block_count >= 1 { Color::Rgb(40, 150, 90) }
            else { C_DIM };
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", date), Style::default().fg(C_TEAL)),
                Span::styled(bar_part.to_string(), Style::default().fg(bar_color).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {}", count_part), Style::default().fg(C_LABEL)),
            ]));
            continue;
        }

        // ── 365-day contribution grid ────────────────────────────────────
        if t.starts_with("GRID:") {
            let data = &t["GRID:".len()..];
            // Parse weeks: each week is comma-separated daily counts
            let weeks: Vec<Vec<i32>> = data.split('|')
                .map(|w| w.split(',')
                    .filter_map(|n| n.parse::<i32>().ok())
                    .collect())
                .collect();

            if !weeks.is_empty() {
                // Find max for relative scaling
                let max_val = weeks.iter().flat_map(|w| w.iter()).copied().max().unwrap_or(1).max(1);

                lines.push(empty());
                lines.push(sep());
                lines.push(Line::from(Span::styled(
                    "  Contribution Activity — Last 365 days",
                    Style::default().fg(C_HEADER).add_modifier(Modifier::BOLD),
                )));
                lines.push(empty());

                // GitHub-style grid: 7 rows (Sun–Sat), 53 cols (weeks)
                // Each cell is a block char coloured by heat level
                let day_labels = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
                // Heat palette: 0 = empty, 1-4 = increasing green shades
                let heat_colors = [
                    Color::Rgb(22, 27, 34),     // 0  — no contributions (dark bg)
                    Color::Rgb(14, 68, 41),     // 1  — very low
                    Color::Rgb(0, 109, 50),     // 2  — low
                    Color::Rgb(38, 166, 65),    // 3  — medium
                    Color::Rgb(57, 211, 83),    // 4  — high
                ];
                let heat_char = "█";

                for row in 0..7usize {
                    let mut spans: Vec<Span<'static>> = vec![
                        Span::styled(
                            format!("  {:3} ", day_labels[row]),
                            Style::default().fg(C_LABEL),
                        ),
                    ];
                    for week in &weeks {
                        let count = week.get(row).copied().unwrap_or(0);
                        let level = if count == 0 { 0 }
                        else if count <= max_val / 4 { 1 }
                        else if count <= max_val / 2 { 2 }
                        else if count <= (max_val * 3) / 4 { 3 }
                        else { 4 };
                        spans.push(Span::styled(
                            heat_char,
                            Style::default().fg(heat_colors[level]),
                        ));
                        spans.push(Span::raw(" "));
                    }
                    lines.push(Line::from(spans));
                }

                // Month labels row — find first day of each month
                let mut month_spans: Vec<Span<'static>> = vec![
                    Span::styled("       ", Style::default()),
                ];
                let month_names = ["Jan","Feb","Mar","Apr","May","Jun",
                    "Jul","Aug","Sep","Oct","Nov","Dec"];
                let mut last_month: i32 = -1;
                for (wi, week) in weeks.iter().enumerate() {
                    // We don't have dates here, estimate from position
                    // Use a placeholder space — just mark every ~4 weeks
                    let _ = week;
                    let approx_month = ((wi as i32 * 12) / 53).min(11);
                    if approx_month != last_month {
                        last_month = approx_month;
                        let label = month_names[approx_month as usize];
                        month_spans.push(Span::styled(
                            label.to_string(),
                            Style::default().fg(C_LABEL),
                        ));
                        // pad to align: label is 3 chars, each cell is 2 chars wide
                        // so skip next cell
                    } else {
                        month_spans.push(Span::raw("  "));
                    }
                }
                lines.push(empty());
                lines.push(Line::from(month_spans));

                // Legend
                lines.push(empty());
                lines.push(Line::from(vec![
                    Span::styled("  Less ", Style::default().fg(C_LABEL)),
                    Span::styled("█", Style::default().fg(heat_colors[0])),
                    Span::raw(" "),
                    Span::styled("█", Style::default().fg(heat_colors[1])),
                    Span::raw(" "),
                    Span::styled("█", Style::default().fg(heat_colors[2])),
                    Span::raw(" "),
                    Span::styled("█", Style::default().fg(heat_colors[3])),
                    Span::raw(" "),
                    Span::styled("█", Style::default().fg(heat_colors[4])),
                    Span::styled(" More", Style::default().fg(C_LABEL)),
                ]));
            }
            continue;
        }

        lines.push(Line::from(Span::styled(l, Style::default().fg(C_BODY))));
    }
    lines.push(empty());
    lines
}

// ── commits ───────────────────────────────────────────────────────────────────

fn build_commits_lines(content: String) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = vec![empty()];
    for raw in content.lines() {
        let l = raw.trim_end().to_string();
        let t = l.trim();
        if t.is_empty() { lines.push(empty()); continue; }
        if t.chars().all(|c| c == '─') { lines.push(sep()); continue; }

        // "3 Most Recent Commits — login"
        if t.starts_with("3 Most Recent") {
            let parts: Vec<&str> = t.splitn(2, " — ").collect();
            lines.push(header(format!("  {}", parts[0])));
            if let Some(who) = parts.get(1) {
                lines.push(Line::from(vec![
                    Span::styled("  @", Style::default().fg(C_LABEL)),
                    Span::styled(who.to_string(), Style::default().fg(C_BLUE)),
                ]));
            }
            lines.push(sep());
            continue;
        }

        // "📦 owner/repo"
        if t.starts_with("📦") {
            let repo = t.trim_start_matches("📦").trim();
            lines.push(empty());
            lines.push(Line::from(vec![
                Span::styled("  📦 ", Style::default().fg(C_TEAL)),
                Span::styled(repo.to_string(), Style::default().fg(C_HEADER).add_modifier(Modifier::BOLD)),
            ]));
            continue;
        }

        // "📝 2026-03-04  🔑 abc1234"
        if t.starts_with("📝") {
            // split on  🔑
            let rest = t.trim_start_matches("📝").trim();
            let (date_part, hash_part) = if let Some(idx) = rest.find("🔑") {
                let d = rest[..idx].trim();
                let h = rest[idx..].trim_start_matches("🔑").trim();
                (d, h)
            } else {
                (rest, "")
            };
            lines.push(Line::from(vec![
                Span::styled("    ", Style::default()),
                Span::styled(date_part.to_string(), Style::default().fg(C_TEAL)),
                Span::styled("  #", Style::default().fg(C_DIM)),
                Span::styled(hash_part.to_string(), Style::default().fg(C_PURPLE)),
            ]));
            continue;
        }

        // "💬 message"
        if t.starts_with("💬") {
            let msg = t.trim_start_matches("💬").trim();
            lines.push(Line::from(vec![
                Span::styled("    💬 ", Style::default().fg(C_LABEL)),
                Span::styled(msg.to_string(), Style::default().fg(C_VALUE)),
            ]));
            continue;
        }

        // "👤 author  📊 +N -N"
        if t.starts_with("👤") {
            let rest = t.trim_start_matches("👤").trim();
            let (author, stats) = if let Some(idx) = rest.find("📊") {
                (rest[..idx].trim(), rest[idx..].trim())
            } else {
                (rest, "")
            };
            let mut spans = vec![
                Span::styled("    ", Style::default()),
                Span::styled(author.to_string(), Style::default().fg(C_LABEL)),
            ];
            if !stats.is_empty() {
                let s = stats.trim_start_matches("📊").trim();
                spans.push(Span::styled("   ".to_string(), Style::default()));
                // colour insertions green, deletions red
                for part in s.split_whitespace() {
                    if part.starts_with('+') {
                        spans.push(Span::styled(format!("{} ", part), Style::default().fg(C_GREEN)));
                    } else if part.starts_with('-') {
                        spans.push(Span::styled(format!("{} ", part), Style::default().fg(C_RED)));
                    } else {
                        spans.push(Span::styled(format!("{} ", part), Style::default().fg(C_LABEL)));
                    }
                }
            }
            lines.push(Line::from(spans));
            continue;
        }

        lines.push(Line::from(Span::styled(l, Style::default().fg(C_BODY))));
    }
    lines.push(empty());
    lines
}

// ── pull-requests ─────────────────────────────────────────────────────────────

fn build_prs_lines(content: String) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = vec![empty()];
    for raw in content.lines() {
        let l = raw.trim_end().to_string();
        let t = l.trim();
        if t.is_empty() { lines.push(empty()); continue; }
        if t.chars().all(|c| c == '─') { lines.push(sep()); continue; }

        // Header "Open Pull Requests — Total: N"
        if t.starts_with("Open Pull Requests") {
            let parts: Vec<&str> = t.splitn(2, " — ").collect();
            lines.push(header(format!("  {}", parts[0])));
            if let Some(total) = parts.get(1) {
                let n = total.trim_start_matches("Total: ");
                lines.push(Line::from(vec![
                    Span::styled("  Total  ", Style::default().fg(C_LABEL)),
                    Span::styled(n.to_string(), Style::default().fg(C_PURPLE).add_modifier(Modifier::BOLD)),
                ]));
            }
            lines.push(sep());
            continue;
        }

        // "🔀 #N — title"
        if t.starts_with("🔀") {
            let rest = t.trim_start_matches("🔀").trim();
            let (num, title) = if let Some(idx) = rest.find(" — ") {
                (rest[..idx].trim(), rest[idx + " — ".len()..].trim())
            } else {
                (rest, "")
            };
            lines.push(empty());
            lines.push(Line::from(vec![
                Span::styled("  🔀 ", Style::default().fg(C_PURPLE)),
                Span::styled(num.to_string(), Style::default().fg(C_LABEL)),
                Span::styled("  ", Style::default()),
                Span::styled(title.to_string(), Style::default().fg(C_HEADER).add_modifier(Modifier::BOLD)),
            ]));
            continue;
        }

        // "   📦 repo"
        if t.starts_with("📦") {
            let repo = t.trim_start_matches("📦").trim();
            lines.push(Line::from(vec![
                Span::styled("     📦 ", Style::default().fg(C_TEAL)),
                Span::styled(repo.to_string(), Style::default().fg(C_VALUE)),
            ]));
            continue;
        }

        // "   State: X  Mergeable: Y  Created: Z"
        if t.starts_with("State:") || l.trim_start().starts_with("State:") {
            for part in t.split("  ") {
                let p = part.trim();
                if let Some(c) = p.find(':') {
                    let k = &p[..c];
                    let v = p[c+1..].trim();
                    let vcolor = match k {
                        "State" => if v == "OPEN" { C_GREEN } else { C_LABEL },
                        "Mergeable" => if v == "MERGEABLE" { C_GREEN } else { C_YELLOW },
                        _ => C_VALUE,
                    };
                    lines.push(Line::from(vec![
                        Span::styled(format!("     {:12}", format!("{}:", k)), Style::default().fg(C_LABEL)),
                        Span::styled(v.to_string(), Style::default().fg(vcolor).add_modifier(Modifier::BOLD)),
                    ]));
                }
            }
            continue;
        }

        // Reviews
        if t.starts_with("Reviews:") {
            lines.push(Line::from(vec![
                Span::styled("     Reviews  ", Style::default().fg(C_LABEL)),
                Span::styled(t.trim_start_matches("Reviews:").trim().to_string(), Style::default().fg(C_VALUE)),
            ]));
            continue;
        }

        if t.starts_with('•') {
            lines.push(Line::from(Span::styled(format!("       {}", t), Style::default().fg(C_LABEL))));
            continue;
        }

        lines.push(Line::from(Span::styled(l, Style::default().fg(C_BODY))));
    }
    lines.push(empty());
    lines
}

// ── repo-sync ─────────────────────────────────────────────────────────────────

fn build_reposync_lines(content: String) -> Vec<Line<'static>> {
    build_generic_lines(content)
}

// ── multi-sync ────────────────────────────────────────────────────────────────

fn build_multisync_lines(content: String) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = vec![empty()];
    for raw in content.lines() {
        let l = raw.trim_end().to_string();
        let t = l.trim();
        if t.is_empty() { lines.push(empty()); continue; }
        if t.chars().all(|c| c == '─') { lines.push(sep()); continue; }

        // Header
        if t.starts_with("Multi-Repo") {
            lines.push(header(format!("  {}", t)));
            lines.push(sep());
            continue;
        }

        // "  ✅ / ⬆️ / ⬇️ etc.  owner/repo — description"
        if t.starts_with("✅") || t.starts_with("⬆") || t.starts_with("⬇")
            || t.starts_with("🔀") || t.starts_with("❌") || t.starts_with("🔄") || t.starts_with("⚠") {
            let rest = t;
            // Find repo name vs description split " — "
            if let Some(idx) = rest.find(" — ") {
                let left = rest[..idx].trim();
                let desc = rest[idx + " — ".len()..].trim();
                let (icon, repo) = {
                    let mut chars = left.chars();
                    let mut icon_end = 0;
                    for c in left.chars() {
                        if c.is_ascii() { break; }
                        icon_end += c.len_utf8();
                    }
                    (left[..icon_end].trim(), left[icon_end..].trim())
                };
                let desc_color = if desc.contains("In sync") { C_GREEN }
                else if desc.contains("ahead") { C_YELLOW }
                else if desc.contains("Diverged") { C_RED }
                else { C_BODY };
                lines.push(Line::from(vec![
                    Span::styled(format!("  {} ", icon), Style::default().fg(C_VALUE)),
                    Span::styled(format!("{:35}", repo), Style::default().fg(C_VALUE)),
                    Span::styled(desc.to_string(), Style::default().fg(desc_color)),
                ]));
            } else {
                lines.push(Line::from(Span::styled(l, Style::default().fg(C_BODY))));
            }
            continue;
        }

        lines.push(Line::from(Span::styled(l, Style::default().fg(C_BODY))));
    }
    lines.push(empty());
    lines
}

// ── push-check / push-verify ──────────────────────────────────────────────────

fn build_push_lines(content: String) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = vec![empty()];
    for raw in content.lines() {
        let l = raw.trim_end().to_string();
        let t = l.trim();
        if t.is_empty() { lines.push(empty()); continue; }
        if t.chars().all(|c| c == '─') { lines.push(sep()); continue; }

        // "Push Status — branch: main" or "Push Verify — branch: main"
        if t.starts_with("Push Status") || t.starts_with("Push Verify") {
            let parts: Vec<&str> = t.splitn(2, " — ").collect();
            lines.push(header(format!("  {}", parts[0])));
            if let Some(branch_part) = parts.get(1) {
                let branch = branch_part.trim_start_matches("branch: ");
                lines.push(Line::from(vec![
                    Span::styled("  branch  ", Style::default().fg(C_LABEL)),
                    Span::styled(branch.to_string(), Style::default().fg(C_PURPLE).add_modifier(Modifier::BOLD)),
                ]));
            }
            lines.push(sep());
            continue;
        }

        // "✅ message" or "⚠️  message"
        if t.starts_with("✅") || t.starts_with("⚠") {
            let (icon, color) = if t.starts_with("✅") { ("✅", C_GREEN) } else { ("⚠", C_YELLOW) };
            let msg = t.trim_start_matches(icon).trim().trim_start_matches('️').trim().trim_start_matches(' ');
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", icon), Style::default().fg(color)),
                Span::styled(msg.to_string(), Style::default().fg(color).add_modifier(Modifier::BOLD)),
            ]));
            continue;
        }

        // "📌 Local commit:  abc123"
        if t.starts_with("📌") || t.starts_with("🌐") {
            let rest = t.trim_start_matches("📌").trim().trim_start_matches("🌐").trim();
            if let Some(c) = rest.find(':') {
                let k = rest[..c].trim();
                let v = rest[c+1..].trim();
                lines.push(Line::from(vec![
                    Span::styled(format!("  {:18}", format!("{}:", k)), Style::default().fg(C_LABEL)),
                    Span::styled(v.to_string(), Style::default().fg(C_PURPLE).add_modifier(Modifier::BOLD)),
                ]));
            }
            continue;
        }

        // "📊 Details" section header
        if t.starts_with("📊") {
            lines.push(empty());
            lines.push(Line::from(Span::styled(
                "  Details",
                Style::default().fg(C_HEADER).add_modifier(Modifier::BOLD),
            )));
            continue;
        }

        // "   Can push:  ✅ Yes" style detail rows
        if l.starts_with("   ") && t.contains(':') {
            let c = t.find(':').unwrap();
            let k = t[..c].trim();
            let v = t[c+1..].trim();
            let (vcolor, bold) = if v.contains("Yes") || v.contains("No ") || v.starts_with('✅') {
                if v.contains("✅") { (C_GREEN, true) }
                else if v.contains("⚠") { (C_YELLOW, true) }
                else if v.contains("❌") { (C_RED, true) }
                else { (C_VALUE, false) }
            } else { (C_VALUE, false) };
            let style = if bold { Style::default().fg(vcolor).add_modifier(Modifier::BOLD) }
            else { Style::default().fg(vcolor) };
            lines.push(Line::from(vec![
                Span::styled(format!("  {:26}", format!("{}:", k)), Style::default().fg(C_LABEL)),
                Span::styled(v.to_string(), style),
            ]));
            continue;
        }

        // "🚀 Push Preview"
        if t.starts_with("🚀") {
            lines.push(empty());
            lines.push(sep());
            lines.push(header(format!("  {}", t.trim_start_matches("🚀").trim())));
            continue;
        }

        // "   🔑 abc  💬 message"
        if t.starts_with("🔑") || (l.starts_with("   ") && t.starts_with("🔑")) {
            // hash and message on same line
            let rest = t.trim_start_matches("🔑").trim();
            let (hash, msg) = if let Some(i) = rest.find("💬") {
                (rest[..i].trim(), rest[i..].trim_start_matches("💬").trim())
            } else {
                (rest, "")
            };
            lines.push(Line::from(vec![
                Span::styled("  #", Style::default().fg(C_DIM)),
                Span::styled(hash.to_string(), Style::default().fg(C_PURPLE)),
                Span::styled("  ", Style::default()),
                Span::styled(msg.to_string(), Style::default().fg(C_VALUE)),
            ]));
            continue;
        }

        // "   📊 N files  +X -Y"
        if (l.starts_with("      ") && t.starts_with("📊")) || t.starts_with("   📊") {
            let rest = t.trim_start_matches("📊").trim();
            let mut spans = vec![Span::styled("    ".to_string(), Style::default())];
            for part in rest.split_whitespace() {
                if part.starts_with('+') {
                    spans.push(Span::styled(format!("{} ", part), Style::default().fg(C_GREEN)));
                } else if part.starts_with('-') {
                    spans.push(Span::styled(format!("{} ", part), Style::default().fg(C_RED)));
                } else {
                    spans.push(Span::styled(format!("{} ", part), Style::default().fg(C_LABEL)));
                }
            }
            lines.push(Line::from(spans));
            continue;
        }

        // Action bullets
        if t.starts_with('•') {
            lines.push(Line::from(Span::styled(format!("    {}", t), Style::default().fg(C_YELLOW))));
            continue;
        }

        // "✅ You can safely push" / "⚠️  Action required"
        if t.starts_with("✅ You") || t.starts_with("⚠️") {
            let color = if t.starts_with("✅") { C_GREEN } else { C_YELLOW };
            lines.push(empty());
            lines.push(Line::from(Span::styled(format!("  {}", t), Style::default().fg(color).add_modifier(Modifier::BOLD))));
            continue;
        }

        // "   TOTAL:"
        if t.starts_with("TOTAL:") {
            lines.push(Line::from(Span::styled(format!("  {}", t), Style::default().fg(C_HEADER).add_modifier(Modifier::BOLD))));
            continue;
        }

        lines.push(Line::from(Span::styled(l, Style::default().fg(C_BODY))));
    }
    lines.push(empty());
    lines
}

// ── branches ─────────────────────────────────────────────────────────────────

fn build_branches_lines(content: String) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = vec![empty()];
    for raw in content.lines() {
        let l = raw.trim_end().to_string();
        let t = l.trim();
        if t.is_empty() { lines.push(empty()); continue; }
        if t.chars().all(|c| c == '─') { lines.push(sep()); continue; }

        // "Branches — repo (Total: N)"
        if t.starts_with("Branches") {
            let parts: Vec<&str> = t.splitn(3, " — ").collect();
            lines.push(header(format!("  Branches")));
            if let Some(repo) = parts.get(1) {
                let (name, total) = if let Some(i) = repo.find(" (Total:") {
                    (repo[..i].trim(), repo[i..].trim_start_matches(" (Total:").trim_end_matches(')').trim())
                } else {
                    (*repo, "")
                };
                lines.push(Line::from(vec![
                    Span::styled("  📦 ", Style::default().fg(C_TEAL)),
                    Span::styled(name.to_string(), Style::default().fg(C_VALUE)),
                    Span::styled(format!("  ({} branches)", total), Style::default().fg(C_LABEL)),
                ]));
            }
            lines.push(sep());
            continue;
        }

        // "🌿 branch-name"
        if t.starts_with("🌿") {
            let name = t.trim_start_matches("🌿").trim();
            lines.push(empty());
            lines.push(Line::from(vec![
                Span::styled("  🌿 ", Style::default().fg(C_GREEN)),
                Span::styled(name.to_string(), Style::default().fg(C_HEADER).add_modifier(Modifier::BOLD)),
            ]));
            continue;
        }

        // "   Commit: abc  Last commit: date"
        if l.starts_with("   ") && t.contains("Commit:") {
            for part in t.split("  ") {
                let p = part.trim();
                if let Some(c) = p.find(':') {
                    let k = p[..c].trim();
                    let v = p[c+1..].trim();
                    let (icon, vcolor) = if k == "Commit" { ("#", C_PURPLE) } else { ("", C_TEAL) };
                    lines.push(Line::from(vec![
                        Span::styled(format!("     {:16}", format!("{}:", k)), Style::default().fg(C_LABEL)),
                        Span::styled(icon.to_string(), Style::default().fg(C_DIM)),
                        Span::styled(v.to_string(), Style::default().fg(vcolor)),
                    ]));
                }
            }
            continue;
        }

        // "   Author: name"
        if l.starts_with("   ") && t.starts_with("Author:") {
            let v = t.trim_start_matches("Author:").trim();
            lines.push(Line::from(vec![
                Span::styled("     Author:          ", Style::default().fg(C_LABEL)),
                Span::styled(v.to_string(), Style::default().fg(C_VALUE)),
            ]));
            continue;
        }

        lines.push(Line::from(Span::styled(l, Style::default().fg(C_BODY))));
    }
    lines.push(empty());
    lines
}

// ── issues ────────────────────────────────────────────────────────────────────

fn build_issues_lines(content: String) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = vec![empty()];
    for raw in content.lines() {
        let l = raw.trim_end().to_string();
        let t = l.trim();
        if t.is_empty() { lines.push(empty()); continue; }
        if t.chars().all(|c| c == '─') { lines.push(sep()); continue; }

        // "Open Issues — Total: N"
        if t.starts_with("Open Issues") {
            lines.push(header("  Issues".to_string()));
            let n = t.splitn(2, "Total: ").nth(1).unwrap_or("0");
            lines.push(Line::from(vec![
                Span::styled("  Total open  ", Style::default().fg(C_LABEL)),
                Span::styled(n.to_string(), Style::default().fg(C_YELLOW).add_modifier(Modifier::BOLD)),
            ]));
            lines.push(sep());
            continue;
        }

        // "📝 #N — title"
        if t.starts_with("📝") {
            let rest = t.trim_start_matches("📝").trim();
            let (num, title) = if let Some(i) = rest.find(" — ") {
                (rest[..i].trim(), rest[i + " — ".len()..].trim())
            } else {
                (rest, "")
            };
            lines.push(empty());
            lines.push(Line::from(vec![
                Span::styled("  📝 ", Style::default().fg(C_YELLOW)),
                Span::styled(num.to_string(), Style::default().fg(C_LABEL)),
                Span::styled("  ", Style::default()),
                Span::styled(title.to_string(), Style::default().fg(C_HEADER).add_modifier(Modifier::BOLD)),
            ]));
            continue;
        }

        // "   State: X  Created: Y  Author: Z"
        if l.starts_with("   ") && (t.starts_with("State:") || t.contains("Created:") || t.contains("Author:")) {
            for part in t.split("  ") {
                let p = part.trim();
                if let Some(c) = p.find(':') {
                    let k = p[..c].trim();
                    let v = p[c+1..].trim();
                    let vcolor = match k {
                        "State" => if v == "OPEN" { C_GREEN } else { C_LABEL },
                        "Created" => C_TEAL,
                        "Author" => C_BLUE,
                        _ => C_VALUE,
                    };
                    lines.push(Line::from(vec![
                        Span::styled(format!("     {:10}", format!("{}:", k)), Style::default().fg(C_LABEL)),
                        Span::styled(v.to_string(), Style::default().fg(vcolor)),
                    ]));
                }
            }
            continue;
        }

        // "   🔗 url"
        if t.starts_with("🔗") {
            let url = t.trim_start_matches("🔗").trim();
            lines.push(Line::from(vec![
                Span::styled("     🔗 ", Style::default().fg(C_DIM)),
                Span::styled(url.to_string(), Style::default().fg(C_BLUE).add_modifier(Modifier::UNDERLINED)),
            ]));
            continue;
        }

        lines.push(Line::from(Span::styled(l, Style::default().fg(C_BODY))));
    }
    lines.push(empty());
    lines
}

// ── user-info ─────────────────────────────────────────────────────────────────

fn build_userinfo_lines(content: String) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = vec![empty()];
    for raw in content.lines() {
        let l = raw.trim_end().to_string();
        let t = l.trim();
        if t.is_empty() { lines.push(empty()); continue; }
        if t.chars().all(|c| c == '─') { lines.push(sep()); continue; }

        if t == "GitHub User Info" {
            lines.push(header("  GitHub User Info".to_string()));
            lines.push(sep());
            continue;
        }

        // "  👤 Username:     value"  — emoji + key + value
        if l.starts_with("  ") && !t.is_empty() {
            // find first non-ascii char = emoji
            let first = t.chars().next().unwrap_or(' ');
            if !first.is_ascii() {
                let mut icon_end = 0;
                for c in t.chars() {
                    if c.is_ascii() { break; }
                    icon_end += c.len_utf8();
                }
                let icon = &t[..icon_end];
                let rest = t[icon_end..].trim();
                if let Some(c) = rest.find(':') {
                    let k = rest[..c].trim();
                    let v = rest[c+1..].trim();
                    let vcolor = match k {
                        "Username"     => C_BLUE,
                        "Name"         => C_HEADER,
                        "Public repos" => C_PURPLE,
                        "Followers"    => C_GREEN,
                        "Following"    => C_GREEN,
                        "Member since" => C_TEAL,
                        _              => C_VALUE,
                    };
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {} ", icon), Style::default().fg(Color::Rgb(200, 210, 255))),
                        Span::styled(format!("{:16}", format!("{}:", k)), Style::default().fg(C_LABEL)),
                        Span::styled(v.to_string(), Style::default().fg(vcolor).add_modifier(Modifier::BOLD)),
                    ]));
                    continue;
                }
            }
        }

        // Bio section "💬 Bio:" header and content
        if t.starts_with("💬") || t.starts_with("Bio:") {
            lines.push(empty());
            lines.push(Line::from(Span::styled(
                "  Bio",
                Style::default().fg(C_LABEL).add_modifier(Modifier::BOLD),
            )));
            continue;
        }

        if l.starts_with("     ") {
            lines.push(Line::from(Span::styled(
                format!("    {}", t),
                Style::default().fg(C_VALUE),
            )));
            continue;
        }

        lines.push(Line::from(Span::styled(l, Style::default().fg(C_BODY))));
    }
    lines.push(empty());
    lines
}

// ── generic fallback ─────────────────────────────────────────────────────────

fn build_generic_lines(content: String) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = vec![empty()];
    for raw in content.lines() {
        let l = raw.trim_end().to_string();
        let t = l.trim();
        if t.is_empty() { lines.push(empty()); continue; }
        if t.chars().all(|c| c == '─' || c == '═') { lines.push(sep()); continue; }

        let first = t.chars().next().unwrap_or(' ');
        let is_emoji = !first.is_ascii();
        let is_header_line = !l.starts_with(' ') && !is_emoji;
        let is_kv = (l.starts_with("  ") || l.starts_with("   ")) && t.contains(':') && !is_emoji;

        if is_header_line {
            lines.push(header(format!("  {}", t)));
            continue;
        }
        if is_kv {
            if let Some(c) = t.find(':') {
                let indent: String = l.chars().take_while(|c| *c == ' ').collect();
                let k = &t[..c+1];
                let v = t[c+1..].trim();
                let vcolor = if v.contains("✅") || v == "Yes" { C_GREEN }
                else if v.contains("⚠") || v.contains("No") { C_YELLOW }
                else if v.contains("❌") { C_RED }
                else { C_VALUE };
                lines.push(Line::from(vec![
                    Span::raw(indent),
                    Span::styled(format!("{:26} ", k), Style::default().fg(C_LABEL)),
                    Span::styled(v.to_string(), Style::default().fg(vcolor).add_modifier(Modifier::BOLD)),
                ]));
                continue;
            }
        }
        if is_emoji {
            let mut icon_end = 0;
            for c in t.chars() { if c.is_ascii() { break; } icon_end += c.len_utf8(); }
            let icon = &t[..icon_end];
            let rest = t[icon_end..].trim_start();
            let indent: String = l.chars().take_while(|c| *c == ' ').collect();
            if let Some(c) = rest.find(':') {
                lines.push(Line::from(vec![
                    Span::raw(indent),
                    Span::styled(format!("{} ", icon), Style::default().fg(C_VALUE)),
                    Span::styled(format!("{}: ", &rest[..c]), Style::default().fg(C_LABEL)),
                    Span::styled(rest[c+1..].trim().to_string(), Style::default().fg(C_VALUE).add_modifier(Modifier::BOLD)),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::raw(indent),
                    Span::styled(format!("{} ", icon), Style::default().fg(C_VALUE)),
                    Span::styled(rest.to_string(), Style::default().fg(C_VALUE)),
                ]));
            }
            continue;
        }
        if t.starts_with('•') || t.starts_with("✅") || t.starts_with("⚠") || t.starts_with("❌") {
            let color = if t.starts_with("✅") { C_GREEN }
            else if t.starts_with("⚠") { C_YELLOW }
            else if t.starts_with("❌") { C_RED }
            else { C_BODY };
            lines.push(Line::from(Span::styled(format!("  {}", t), Style::default().fg(color))));
            continue;
        }
        lines.push(Line::from(Span::styled(l, Style::default().fg(C_BODY))));
    }
    lines.push(empty());
    lines
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
// ─── MultiSync message type ───────────────────────────────────────────────────

enum MultiSyncMsg {
    RepoList(Vec<crate::tui::app::MultiSyncRepo>),
    SyncResults(Vec<(String, ratatui::style::Color)>),
    Error(String),
}

// Fetches all GitHub repos and returns them as MultiSyncRepo entries.
// Runs in spawn_blocking so uses a fresh tokio runtime.
fn fetch_repos_for_multi_sync() -> Result<Vec<crate::tui::app::MultiSyncRepo>, String> {
    use crate::auth::token_store;
    use crate::github::graphql::{self, GraphQLClient};
    use crate::tui::app::MultiSyncRepo;

    let token = token_store::load_token()
        .map_err(|_| "Not authenticated. Run /auth login first.".to_string())?;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;

    let repos_resp = rt.block_on(graphql::fetch_repositories(
        &GraphQLClient::new(token),
        100,
        true,
    )).map_err(|e| e.to_string())?;

    Ok(repos_resp.viewer.repositories.nodes.into_iter().map(|r| MultiSyncRepo {
        name_with_owner: r.name_with_owner,
        description: r.description.unwrap_or_default(),
        is_private: r.is_private,
        selected: false,
    }).collect())
}

// Checks sync for each selected repo name_with_owner and returns colored result lines.
fn run_multi_sync_check(repo_names: Vec<String>) -> Vec<(String, ratatui::style::Color)> {
    use crate::auth::token_store;
    use crate::github::graphql::{self, GraphQLClient};
    use crate::github::sync_checker::{SyncChecker, SyncStatus};
    use ratatui::style::Color;

    let mut lines: Vec<(String, Color)> = Vec::new();

    let token = match token_store::load_token() {
        Ok(t) => t,
        Err(_) => {
            lines.push(("Not authenticated.".to_string(), Color::Rgb(220, 80, 80)));
            return lines;
        }
    };

    let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(rt) => rt,
        Err(e) => {
            lines.push((format!("Runtime error: {}", e), Color::Rgb(220, 80, 80)));
            return lines;
        }
    };

    let client = GraphQLClient::new(token.clone());
    let repos_resp = match rt.block_on(graphql::fetch_repositories(&client, 100, true)) {
        Ok(r) => r,
        Err(e) => {
            lines.push((format!("Failed to fetch repos: {}", e), Color::Rgb(220, 80, 80)));
            return lines;
        }
    };

    let all_repos = repos_resp.viewer.repositories.nodes;
    let checker = SyncChecker::new(GraphQLClient::new(token));

    lines.push(("".to_string(), Color::Reset));
    lines.push((
        format!("  Sync results — {} repositories", repo_names.len()),
        Color::Rgb(180, 190, 255),
    ));
    lines.push((
        "  ──────────────────────────────────────────────────────────".to_string(),
        Color::Rgb(45, 50, 68),
    ));
    lines.push(("".to_string(), Color::Reset));

    for name in &repo_names {
        let repo = match all_repos.iter().find(|r| &r.name_with_owner == name) {
            Some(r) => r,
            None => {
                lines.push((format!("  ❓ {} — not found", name), Color::Rgb(150, 150, 170)));
                continue;
            }
        };

        let status = match rt.block_on(checker.check_sync(repo, None)) {
            Ok(s) => s,
            Err(e) => {
                lines.push((
                    format!("  ⚠  {} — error: {}", name, e),
                    Color::Rgb(230, 180, 60),
                ));
                continue;
            }
        };

        let (icon, color, desc) = match &status {
            SyncStatus::InSync =>
                ("✔", Color::Rgb(80, 210, 130), "In sync".to_string()),
            SyncStatus::LocalAhead { commits } =>
                ("⬆", Color::Rgb(230, 180, 60), format!("Local ahead by {}", commits)),
            SyncStatus::RemoteAhead { commits } =>
                ("⬇", Color::Rgb(100, 155, 245), format!("Remote ahead by {}", commits)),
            SyncStatus::Diverged { local_ahead, remote_ahead } =>
                ("⇅", Color::Rgb(220, 80, 80), format!("Diverged +{} -{}", local_ahead, remote_ahead)),
            SyncStatus::NoLocalRepo =>
                ("○", Color::Rgb(90, 100, 130), "Not cloned locally".to_string()),
            SyncStatus::BranchMismatch { local_branch, remote_branch } =>
                ("↔", Color::Rgb(160, 120, 240), format!("Branch mismatch: {} ↔ {}", local_branch, remote_branch)),
        };

        // Pad repo name
        let padded = format!("{:<40}", name);
        lines.push((format!("  {}  {}  {}", icon, padded, desc), color));
    }

    lines.push(("".to_string(), Color::Reset));
    lines.push((
        "  ──────────────────────────────────────────────────────────".to_string(),
        Color::Rgb(45, 50, 68),
    ));
    lines.push(("".to_string(), Color::Reset));
    lines.push((
        "  Press Esc or q to close".to_string(),
        Color::Rgb(70, 78, 100),
    ));

    lines
}