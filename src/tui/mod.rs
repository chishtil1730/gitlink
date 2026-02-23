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
                    let root = trimmed.split_whitespace().next().unwrap_or("");

                    // Special: /plan exits TUI temporarily, runs planner, re-enters
                    if root == "plan" {
                        app.is_executing = false;
                        disable_raw_mode()?;
                        execute!(io::stdout(), LeaveAlternateScreen)?;
                        terminal.show_cursor()?;

                        let plan_result = crate::planner::ui::run_planner();

                        enable_raw_mode()?;
                        execute!(io::stdout(), EnterAlternateScreen)?;
                        terminal.hide_cursor()?;
                        terminal.clear()?;

                        let msg = match plan_result {
                            Ok(_) => OutputBlock {
                                kind: OutputKind::Success,
                                content: "Planner session ended. Back in GitLink TUI.".to_string(),
                            },
                            Err(e) => OutputBlock {
                                kind: OutputKind::Error,
                                content: format!("Planner error: {}", e),
                            },
                        };
                        app.push_output(msg);
                        continue;
                    }

                    // Special: /clear
                    let output = router::execute(&cmd);
                    if output.kind == OutputKind::Info && output.content == "__CLEAR__" {
                        app.outputs.clear();
                        app.is_executing = false;
                    } else {
                        app.push_output(output);
                    }
                }
            }
        }
    }
}