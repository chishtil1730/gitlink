use std::time::{Duration, Instant, SystemTime};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::commands::{Command, COMMANDS};
use crate::scanner::report::Finding;
use crate::planner::task::Task;
use crate::planner::history::History;

// ─── Overlay State ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ScanChoice {
    Ignore,
    Keep,
}

pub struct ScannerOverlay {
    pub findings: Vec<Finding>,
    pub current_index: usize,
    pub choice: ScanChoice,
    pub done: bool,
}

impl ScannerOverlay {
    pub fn new(findings: Vec<Finding>) -> Self {
        Self {
            findings,
            current_index: 0,
            choice: ScanChoice::Keep,
            done: false,
        }
    }

    pub fn current(&self) -> Option<&Finding> {
        self.findings.get(self.current_index)
    }

    pub fn total(&self) -> usize {
        self.findings.len()
    }
}

pub struct IgnoreOverlay {
    pub items: Vec<crate::scanner::ignore::IgnoredItem>,
    pub selected: usize,
    pub done: bool,
}

impl IgnoreOverlay {
    pub fn new() -> Self {
        let db = crate::scanner::ignore::load_ignore_db();
        Self {
            items: db.ignored,
            selected: 0,
            done: false,
        }
    }

    pub fn is_clear_all_selected(&self) -> bool {
        self.selected == self.items.len()
    }
}

pub struct PlannerOverlay {
    pub tasks: Vec<Task>,
    pub history: crate::planner::history::History,
    pub selected: usize,
    pub focus: PlannerFocus,
    pub mode: PlannerMode,
    pub input_buf: String,
    pub input_cursor: usize,
    pub input_field: InputField,
    pub scroll: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlannerFocus { List, Detail }

#[derive(Debug, Clone, PartialEq)]
pub enum PlannerMode { Normal, AddingTask, EditingTask, ConfirmDelete }

#[derive(Debug, Clone, PartialEq)]
pub enum InputField { Title, Tags, Description }

impl PlannerOverlay {
    pub fn new() -> Self {
        let task_list = crate::planner::storage::load_tasks();
        Self {
            tasks: task_list.tasks,
            history: History::new(),
            selected: 0,
            focus: PlannerFocus::List,
            mode: PlannerMode::Normal,
            input_buf: String::new(),
            input_cursor: 0,
            input_field: InputField::Title,
            scroll: 0,
        }
    }

    pub fn selected_task(&self) -> Option<&Task> {
        self.tasks.get(self.selected)
    }

    pub fn save(&self) {
        let tl = crate::planner::storage::TaskList { tasks: self.tasks.clone() };
        let _ = crate::planner::storage::save_tasks(&tl);
    }
}

// ─── Generic Info Overlay ─────────────────────────────────────────────────────

pub struct InfoOverlay {
    pub title: String,
    pub lines: Vec<ratatui::text::Line<'static>>,
    pub scroll: usize,
    pub done: bool,
    pub accent: ratatui::style::Color,
}

impl InfoOverlay {
    pub fn new(
        title: impl Into<String>,
        lines: Vec<ratatui::text::Line<'static>>,
        accent: ratatui::style::Color,
    ) -> Self {
        Self { title: title.into(), lines, scroll: 0, done: false, accent }
    }
}

// ─── Auth Overlay ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum AuthStep {
    Menu,
    ShowCode { user_code: String, url: String },
    Polling { user_code: String, url: String, frame: usize },
    Result(String),
}

pub struct AuthOverlay {
    pub step: AuthStep,
    pub selected: usize,
    pub done: bool,
    pub poll_rx: Option<std::sync::mpsc::Receiver<String>>,
}

impl AuthOverlay {
    pub fn new() -> Self {
        Self { step: AuthStep::Menu, selected: 0, done: false, poll_rx: None }
    }
}

// ─── PRP Overlay ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum PrpStep { SelectRepos, EnterMessage, Result }

pub struct PrpOverlay {
    pub repos: Vec<String>,
    pub included: Vec<bool>,
    pub selected: usize,
    pub step: PrpStep,
    pub input_buf: String,
    pub input_cursor: usize,
    pub result_lines: Vec<String>,
    pub done: bool,
}

impl PrpOverlay {
    pub fn new(repos: Vec<String>) -> Self {
        let len = repos.len();
        Self {
            repos,
            included: vec![true; len],
            selected: 0,
            step: PrpStep::SelectRepos,
            input_buf: String::new(),
            input_cursor: 0,
            result_lines: vec![],
            done: false,
        }
    }
}

// ─── Overlay enum ─────────────────────────────────────────────────────────────

pub enum Overlay {
    Scanner(ScannerOverlay),
    Planner(PlannerOverlay),
    Ignore(IgnoreOverlay),
    Info(InfoOverlay),
    Auth(AuthOverlay),
    Prp(PrpOverlay),
}

// ─── Output ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum OutputKind { Success, Error, Info, Command }

#[derive(Debug, Clone)]
pub struct OutputBlock {
    pub kind: OutputKind,
    pub content: String,
}

// ─── App ──────────────────────────────────────────────────────────────────────

pub struct App {
    pub input: String,
    pub cursor_pos: usize,
    pub filtered_commands: Vec<Command>,
    pub selected_index: usize,
    pub outputs: Vec<OutputBlock>,
    pub output_scroll: f32,
    pub is_executing: bool,
    pub show_suggestions: bool,
    pub start_time: SystemTime,
    pub elapsed: f32,
    pub cmd_history: Vec<String>,
    pub history_index: Option<usize>,
    pub overlay: Option<Overlay>,
    pub needs_full_redraw: bool,
    spin_start: Option<Instant>,
    pending_command: Option<String>,
}

impl App {
    pub fn new() -> Self {
        let mut app = Self {
            input: String::new(),
            cursor_pos: 0,
            filtered_commands: vec![],
            selected_index: 0,
            outputs: vec![],
            output_scroll: 0.0,
            is_executing: false,
            show_suggestions: false,
            start_time: SystemTime::now(),
            elapsed: 0.0,
            cmd_history: vec![],
            history_index: None,
            overlay: None,
            needs_full_redraw: false,
            spin_start: None,
            pending_command: None,
        };
        app.outputs.push(OutputBlock {
            kind: OutputKind::Info,
            content: "Welcome to GitLink TUI. Type / to see available commands.".to_string(),
        });
        app
    }

    pub fn on_tick(&mut self) {
        self.elapsed = self.start_time.elapsed().unwrap_or(Duration::ZERO).as_secs_f32();
        // Tick auth overlay so spinner advances and poll result is checked
        if let Some(Overlay::Auth(ref mut ov)) = self.overlay {
            auth_overlay_tick(ov);
            if ov.done {
                self.overlay = None;
                self.needs_full_redraw = true;
            }
        }
    }

    pub fn on_key(&mut self, key: KeyEvent) -> bool {
        if key.kind != KeyEventKind::Press { return false; }

        if self.overlay.is_some() {
            return self.handle_overlay_key(key);
        }

        if self.is_executing { return false; }

        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return true,
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => return true,

            KeyCode::Enter => {
                if self.show_suggestions && !self.filtered_commands.is_empty() {
                    self.accept_suggestion();
                } else {
                    self.submit();
                }
            }

            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.input.remove(self.cursor_pos);
                    self.update_suggestions();
                }
            }

            KeyCode::Left  => { if self.cursor_pos > 0 { self.cursor_pos -= 1; } }
            KeyCode::Right => { if self.cursor_pos < self.input.len() { self.cursor_pos += 1; } }

            KeyCode::Up => {
                if self.show_suggestions && !self.filtered_commands.is_empty() {
                    if self.selected_index > 0 { self.selected_index -= 1; }
                    else { self.selected_index = self.filtered_commands.len() - 1; }
                } else if self.input.is_empty() {
                    self.output_scroll += 1.0;
                } else {
                    self.navigate_history(-1);
                }
            }

            KeyCode::Down => {
                if self.show_suggestions && !self.filtered_commands.is_empty() {
                    self.selected_index = (self.selected_index + 1) % self.filtered_commands.len();
                } else if self.input.is_empty() {
                    if self.output_scroll > 0.0 { self.output_scroll -= 1.0; }
                } else {
                    self.navigate_history(1);
                }
            }

            KeyCode::PageUp   => { self.output_scroll += 5.0; }
            KeyCode::PageDown => { self.output_scroll = (self.output_scroll - 5.0).max(0.0); }

            KeyCode::Tab => {
                if self.show_suggestions && !self.filtered_commands.is_empty() {
                    self.accept_suggestion();
                }
            }

            KeyCode::Char(c) => {
                self.input.insert(self.cursor_pos, c);
                self.cursor_pos += 1;
                self.update_suggestions();
                self.history_index = None;
                self.output_scroll = 0.0;
            }

            _ => {}
        }
        false
    }

    fn handle_overlay_key(&mut self, key: KeyEvent) -> bool {
        match &mut self.overlay {
            Some(Overlay::Scanner(ref mut ov)) => {
                handle_scanner_key(ov, key);
                if let Some(Overlay::Scanner(ref ov)) = self.overlay {
                    if ov.done {
                        self.overlay = None;
                        self.needs_full_redraw = true;
                        self.push_output(OutputBlock { kind: OutputKind::Success, content: "Scan review complete.".to_string() });
                    }
                }
            }
            Some(Overlay::Planner(ref mut ov)) => {
                let close = handle_planner_key(ov, key);
                if close {
                    self.overlay = None;
                    self.needs_full_redraw = true;
                    self.push_output(OutputBlock { kind: OutputKind::Success, content: "Planner closed.".to_string() });
                }
            }
            Some(Overlay::Ignore(ref mut ov)) => {
                handle_ignore_key(ov, key);
                if ov.done {
                    self.overlay = None;
                    self.needs_full_redraw = true;
                }
            }
            Some(Overlay::Info(ref mut ov)) => {
                handle_info_key(ov, key);
                if ov.done {
                    self.overlay = None;
                    self.needs_full_redraw = true;
                }
            }
            Some(Overlay::Auth(ref mut ov)) => {
                let close = handle_auth_key(ov, key);
                if close {
                    self.overlay = None;
                    self.needs_full_redraw = true;
                }
            }
            Some(Overlay::Prp(ref mut ov)) => {
                let close = handle_prp_key(ov, key);
                if close {
                    let msg = if !ov.result_lines.is_empty() {
                        "PRP session complete.".to_string()
                    } else {
                        "PRP session cancelled.".to_string()
                    };
                    self.overlay = None;
                    self.needs_full_redraw = true;
                    self.push_output(OutputBlock { kind: OutputKind::Success, content: msg });
                }
            }
            None => {}
        }
        false
    }

    fn accept_suggestion(&mut self) {
        let cmd = self.filtered_commands[self.selected_index].name.clone();
        self.input = format!("/{}", cmd);
        self.cursor_pos = self.input.len();
        self.show_suggestions = false;
        self.filtered_commands.clear();
        self.selected_index = 0;
    }

    fn update_suggestions(&mut self) {
        if self.input.starts_with('/') {
            let query = self.input[1..].to_lowercase();
            self.filtered_commands = COMMANDS
                .iter()
                .filter(|c| {
                    query.is_empty()
                        || c.name.contains(query.as_str())
                        || c.description.to_lowercase().contains(query.as_str())
                })
                .cloned()
                .collect();
            self.show_suggestions = !self.filtered_commands.is_empty();
            if self.selected_index >= self.filtered_commands.len() { self.selected_index = 0; }
        } else {
            self.show_suggestions = false;
            self.filtered_commands.clear();
            self.selected_index = 0;
        }
    }

    fn submit(&mut self) {
        let raw = self.input.trim().to_string();
        if raw.is_empty() { return; }

        self.outputs.push(OutputBlock { kind: OutputKind::Command, content: raw.clone() });
        self.cmd_history.push(raw.clone());
        self.history_index = None;
        self.input.clear();
        self.cursor_pos = 0;
        self.show_suggestions = false;
        self.filtered_commands.clear();
        self.selected_index = 0;
        self.output_scroll = 0.0;

        if !raw.starts_with('/') {
            self.outputs.push(OutputBlock {
                kind: OutputKind::Error,
                content: "Only slash commands are supported. Type / to see available commands.".to_string(),
            });
            return;
        }

        let after_slash = &raw[1..];
        let cmd_name = after_slash.split_whitespace().next().unwrap_or("").to_string();
        // Match commands by first word (handles "push-check", "show-activity" etc.)
        let known = COMMANDS.iter().any(|c| {
            let c_root = c.name.split_whitespace().next().unwrap_or("");
            c_root == cmd_name
        });
        if !known {
            let suggestion = COMMANDS
                .iter()
                .min_by_key(|c| levenshtein(&c.name, &cmd_name))
                .map(|c| format!(" Did you mean /{} ?", c.name))
                .unwrap_or_default();
            self.outputs.push(OutputBlock {
                kind: OutputKind::Error,
                content: format!("Unknown command: {}.{}", raw, suggestion),
            });
            return;
        }

        self.is_executing = true;
        self.spin_start = Some(Instant::now());
        self.pending_command = Some(raw);
    }

    fn navigate_history(&mut self, direction: i32) {
        if self.cmd_history.is_empty() { return; }
        let len = self.cmd_history.len();
        self.history_index = Some(match self.history_index {
            None => if direction < 0 { len - 1 } else { 0 },
            Some(i) => (i as i32 + direction).max(0).min(len as i32 - 1) as usize,
        });
        if let Some(idx) = self.history_index {
            self.input = self.cmd_history[idx].clone();
            self.cursor_pos = self.input.len();
            self.update_suggestions();
        }
    }

    pub fn spin_elapsed(&self) -> f32 {
        self.spin_start.map(|t| t.elapsed().as_secs_f32()).unwrap_or(0.0)
    }

    pub fn take_pending_command(&mut self) -> Option<String> {
        self.pending_command.take()
    }

    pub fn push_output(&mut self, block: OutputBlock) {
        self.is_executing = false;
        self.spin_start = None;
        self.outputs.push(block);
        self.output_scroll = 0.0;
    }

    pub fn open_scanner_overlay(&mut self, findings: Vec<Finding>) {
        self.is_executing = false;
        if findings.is_empty() {
            self.push_output(OutputBlock {
                kind: OutputKind::Success,
                content: "No secrets found. Working directory is clean.".to_string(),
            });
        } else {
            self.overlay = Some(Overlay::Scanner(ScannerOverlay::new(findings)));
        }
    }

    pub fn open_planner_overlay(&mut self) {
        self.is_executing = false;
        self.overlay = Some(Overlay::Planner(PlannerOverlay::new()));
    }

    pub fn open_ignore_overlay(&mut self) {
        self.is_executing = false;
        let ov = IgnoreOverlay::new();
        if ov.items.is_empty() {
            self.push_output(OutputBlock { kind: OutputKind::Info, content: "No ignored findings to manage.".to_string() });
        } else {
            self.overlay = Some(Overlay::Ignore(ov));
        }
    }

    pub fn open_info_overlay(
        &mut self,
        title: impl Into<String>,
        lines: Vec<ratatui::text::Line<'static>>,
        accent: ratatui::style::Color,
    ) {
        self.is_executing = false;
        self.overlay = Some(Overlay::Info(InfoOverlay::new(title, lines, accent)));
    }

    pub fn open_auth_overlay(&mut self) {
        self.is_executing = false;
        self.overlay = Some(Overlay::Auth(AuthOverlay::new()));
    }

    pub fn open_prp_overlay(&mut self, repos: Vec<String>) {
        self.is_executing = false;
        if repos.is_empty() {
            self.push_output(OutputBlock {
                kind: OutputKind::Error,
                content: "No git repositories found in the current directory.".to_string(),
            });
        } else {
            self.overlay = Some(Overlay::Prp(PrpOverlay::new(repos)));
        }
    }
}

// ─── Scanner key handler ──────────────────────────────────────────────────────

pub fn handle_scanner_key(ov: &mut ScannerOverlay, key: KeyEvent) {
    match key.code {
        KeyCode::Left  | KeyCode::Char('h') => { ov.choice = ScanChoice::Ignore; }
        KeyCode::Right | KeyCode::Char('l') => { ov.choice = ScanChoice::Keep; }
        KeyCode::Up    | KeyCode::Char('k') => { ov.choice = ScanChoice::Keep; }
        KeyCode::Down  | KeyCode::Char('j') => { ov.choice = ScanChoice::Ignore; }
        KeyCode::Enter => {
            if ov.choice == ScanChoice::Ignore {
                if let Some(f) = ov.findings.get(ov.current_index) {
                    let short_id = f.fingerprint[..8.min(f.fingerprint.len())].to_string();
                    let variable = f.content
                        .split('=').next().unwrap_or("").trim()
                        .split(|c: char| !c.is_alphanumeric() && c != '_')
                        .filter(|s| !s.is_empty()).last().unwrap_or("unknown").to_string();
                    let source = if f.commit.is_some() { "history" } else { "working" }.to_string();
                    crate::scanner::ignore::add_ignored(crate::scanner::ignore::IgnoredItem {
                        fingerprint: f.fingerprint.clone(),
                        short_id, variable, source,
                        commit: f.commit.clone(),
                    });
                }
            }
            ov.current_index += 1;
            ov.choice = ScanChoice::Keep;
            if ov.current_index >= ov.findings.len() { ov.done = true; }
        }
        KeyCode::Esc | KeyCode::Char('q') => { ov.done = true; }
        _ => {}
    }
}

// ─── Ignore key handler ───────────────────────────────────────────────────────

pub fn handle_ignore_key(ov: &mut IgnoreOverlay, key: KeyEvent) {
    let total_options = ov.items.len() + 2;
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if ov.selected > 0 { ov.selected -= 1; }
            else { ov.selected = total_options.saturating_sub(1); }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            ov.selected = (ov.selected + 1) % total_options;
        }
        KeyCode::Enter => {
            let len = ov.items.len();
            if ov.selected < len {
                let id_to_remove = ov.items[ov.selected].short_id.clone();
                crate::scanner::ignore::remove_by_short_id(&id_to_remove);
                ov.items = crate::scanner::ignore::load_ignore_db().ignored;
                if ov.items.is_empty() { ov.selected = 0; }
                else if ov.selected >= ov.items.len() { ov.selected = ov.items.len().saturating_sub(1); }
            } else if ov.selected == len {
                crate::scanner::ignore::clear_all_silent();
                ov.items.clear();
                ov.done = true;
            } else {
                ov.done = true;
            }
        }
        KeyCode::Esc | KeyCode::Char('q') => { ov.items.clear(); ov.done = true; }
        _ => {}
    }
}

// ─── Info Overlay key handler ─────────────────────────────────────────────────

pub fn handle_info_key(ov: &mut InfoOverlay, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => { ov.done = true; }
        KeyCode::Up   | KeyCode::Char('k') => { if ov.scroll > 0 { ov.scroll -= 1; } }
        KeyCode::Down | KeyCode::Char('j') => { ov.scroll += 1; }
        KeyCode::PageUp   => { ov.scroll = ov.scroll.saturating_sub(10); }
        KeyCode::PageDown => { ov.scroll += 10; }
        _ => {}
    }
}

// ─── Auth overlay key handler ─────────────────────────────────────────────────

/// Called every tick so the overlay can check for OAuth poll results.
/// Returns true if the overlay should close.
pub fn auth_overlay_tick(ov: &mut AuthOverlay) -> bool {
    // Advance spinner
    if let AuthStep::Polling { ref mut frame, .. } = ov.step {
        *frame = frame.wrapping_add(1);
    }
    // Check if background poll finished
    if let Some(ref rx) = ov.poll_rx {
        if let Ok(msg) = rx.try_recv() {
            ov.poll_rx = None;
            if msg == "__AUTH_SUCCESS__" {
                ov.done = true; // close overlay immediately on success
            } else {
                ov.step = AuthStep::Result(msg); // show error
            }
        }
    }
    false
}

pub fn handle_auth_key(ov: &mut AuthOverlay, key: KeyEvent) -> bool {
    match &ov.step.clone() {
        AuthStep::Menu => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => return true,
            KeyCode::Up   | KeyCode::Char('k') => { if ov.selected > 0 { ov.selected -= 1; } }
            KeyCode::Down | KeyCode::Char('j') => { if ov.selected < 2 { ov.selected += 1; } }
            KeyCode::Enter => {
                let sel = ov.selected;
                match sel {
                    0 => {
                        // Step 1: get device code (fast, blocking ok)
                        let info_result = tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                crate::auth::oauth::request_device_code().await
                            })
                        });
                        match info_result {
                            Err(e) => {
                                ov.step = AuthStep::Result(format!("Error: {}", e));
                            }
                            Ok(info) => {
                                let user_code = info.user_code.clone();
                                let url = info.verification_uri.clone();
                                let (tx, rx) = std::sync::mpsc::channel();
                                tokio::task::spawn(async move {
                                    // Count down 5 seconds before opening browser
                                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                                    let _ = open::that(&info.verification_uri);
                                    let result = crate::auth::oauth::poll_for_token(&info).await;
                                    match result {
                                        Ok(token) => {
                                            let save = crate::auth::token_store::save_token(&token);
                                            let msg = match save {
                                                Ok(_) => "__AUTH_SUCCESS__".to_string(),
                                                Err(e) => format!("Error saving token: {}", e),
                                            };
                                            let _ = tx.send(msg);
                                        }
                                        Err(e) => { let _ = tx.send(format!("Error: {}", e)); }
                                    }
                                });
                                ov.poll_rx = Some(rx);
                                ov.step = AuthStep::ShowCode { user_code, url };
                            }
                        }
                    }
                    1 => {
                        let msg = match crate::auth::token_store::delete_token() {
                            Ok(_) => "Logged out. GitHub token removed.".to_string(),
                            Err(e) => format!("Error: {}", e),
                        };
                        ov.step = AuthStep::Result(msg);
                    }
                    _ => {
                        let msg = match crate::auth::token_store::load_token() {
                            Ok(_) => "Authenticated ✔  — GitHub token is present.".to_string(),
                            Err(_) => "Not authenticated. Run /auth login to connect.".to_string(),
                        };
                        ov.step = AuthStep::Result(msg);
                    }
                }
            }
            _ => {}
        },
        AuthStep::ShowCode { .. } => {
            // Any key advances to polling display (code stays visible until done)
            if let AuthStep::ShowCode { user_code, url } = ov.step.clone() {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        ov.poll_rx = None;
                        return true;
                    }
                    _ => {
                        ov.step = AuthStep::Polling { user_code, url, frame: 0 };
                    }
                }
            }
        }
        AuthStep::Polling { .. } => {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    ov.poll_rx = None;
                    return true;
                }
                _ => {}
            }
        }
        AuthStep::Result(_) => {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => return true,
                _ => {}
            }
        }
    }
    false
}

// ─── PRP overlay key handler ──────────────────────────────────────────────────

pub fn handle_prp_key(ov: &mut PrpOverlay, key: KeyEvent) -> bool {
    match ov.step {
        PrpStep::SelectRepos => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => return true,
            KeyCode::Up   | KeyCode::Char('k') => { if ov.selected > 0 { ov.selected -= 1; } }
            KeyCode::Down | KeyCode::Char('j') => { if ov.selected + 1 < ov.repos.len() { ov.selected += 1; } }
            KeyCode::Char(' ') => {
                if let Some(b) = ov.included.get_mut(ov.selected) { *b = !*b; }
            }
            KeyCode::Enter => {
                if ov.included.iter().any(|&b| b) {
                    ov.step = PrpStep::EnterMessage;
                    ov.input_buf.clear();
                    ov.input_cursor = 0;
                }
            }
            _ => {}
        },
        PrpStep::EnterMessage => match key.code {
            KeyCode::Esc => { ov.step = PrpStep::SelectRepos; }
            KeyCode::Enter => {
                let msg = ov.input_buf.trim().to_string();
                if !msg.is_empty() {
                    let selected_repos: Vec<String> = ov.repos.iter().enumerate()
                        .filter(|(i, _)| ov.included[*i])
                        .map(|(_, n)| n.clone())
                        .collect();
                    ov.result_lines = run_prp_commit(selected_repos, &msg);
                    ov.step = PrpStep::Result;
                }
            }
            KeyCode::Backspace => {
                if ov.input_cursor > 0 {
                    ov.input_cursor -= 1;
                    ov.input_buf.remove(ov.input_cursor);
                }
            }
            KeyCode::Left  => { if ov.input_cursor > 0 { ov.input_cursor -= 1; } }
            KeyCode::Right => { if ov.input_cursor < ov.input_buf.len() { ov.input_cursor += 1; } }
            KeyCode::Char(c) => { ov.input_buf.insert(ov.input_cursor, c); ov.input_cursor += 1; }
            _ => {}
        },
        PrpStep::Result => {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => return true,
                _ => {}
            }
        }
    }
    false
}

fn run_prp_commit(repos: Vec<String>, message: &str) -> Vec<String> {
    let mut lines = vec![
        "🔗 PRP Commit Session".to_string(),
        "────────────────────────────────────".to_string(),
        format!("Commit message: {}", message),
        String::new(),
    ];

    for repo in &repos {
        let _ = std::process::Command::new("git")
            .args(["-C", repo, "add", "-A"])
            .output();
        let commit_out = std::process::Command::new("git")
            .args(["-C", repo, "commit", "-m", message])
            .output();

        match commit_out {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                if out.status.success() {
                    lines.push(format!("  ✔  {}", repo));
                    if !stdout.is_empty() {
                        lines.push(format!("     {}", stdout.lines().next().unwrap_or("")));
                    }
                } else {
                    lines.push(format!("  ✖  {} — {}", repo, stderr.lines().next().unwrap_or("no changes")));
                }
            }
            Err(_) => {
                lines.push(format!("  ✖  {} — failed to run git", repo));
            }
        }
    }

    lines.push(String::new());
    lines.push("Press Enter or Esc to close.".to_string());
    lines
}

// ─── Planner key handler ──────────────────────────────────────────────────────

use std::cell::RefCell;

thread_local! {
    static TASK_SCRATCH: RefCell<(String, String, String)> =
        RefCell::new((String::new(), String::new(), String::new()));
}

pub fn planner_scratch_peek() -> (String, String, String) {
    TASK_SCRATCH.with(|s| {
        let sc = s.borrow();
        (sc.0.clone(), sc.1.clone(), sc.2.clone())
    })
}

pub fn handle_planner_key(ov: &mut PlannerOverlay, key: KeyEvent) -> bool {
    use PlannerMode::*;
    use PlannerFocus::*;

    match ov.mode {
        Normal => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => return true,
            KeyCode::Tab => {
                ov.focus = match ov.focus { List => Detail, Detail => List };
            }
            KeyCode::Up | KeyCode::Char('k') if ov.focus == List => {
                if ov.selected > 0 { ov.selected -= 1; }
                clamp_scroll(ov);
            }
            KeyCode::Down | KeyCode::Char('j') if ov.focus == List => {
                if ov.selected + 1 < ov.tasks.len() { ov.selected += 1; }
                clamp_scroll(ov);
            }
            KeyCode::Char(' ') if ov.focus == List => {
                if let Some(task) = ov.tasks.get_mut(ov.selected) {
                    ov.history.push(crate::planner::history::Action::Toggle { id: task.id.clone() });
                    task.toggle();
                    ov.save();
                }
            }
            KeyCode::Char('a') => {
                ov.mode = AddingTask;
                ov.input_field = InputField::Title;
                ov.input_buf.clear();
                ov.input_cursor = 0;
                TASK_SCRATCH.with(|s| *s.borrow_mut() = (String::new(), String::new(), String::new()));
            }
            KeyCode::Char('e') if ov.focus == List => {
                if let Some(task) = ov.tasks.get(ov.selected) {
                    ov.mode = EditingTask;
                    ov.input_field = InputField::Title;
                    let title = task.title.clone();
                    let tags  = task.tags.join(", ");
                    let desc  = task.description.clone().unwrap_or_default();
                    TASK_SCRATCH.with(|s| *s.borrow_mut() = (title.clone(), tags, desc));
                    ov.input_buf = title;
                    ov.input_cursor = ov.input_buf.len();
                }
            }
            KeyCode::Char('d') if ov.focus == List => {
                if !ov.tasks.is_empty() { ov.mode = ConfirmDelete; }
            }
            KeyCode::Char('u') => {
                ov.history.undo(&mut ov.tasks);
                ov.save();
                if ov.selected >= ov.tasks.len() && !ov.tasks.is_empty() {
                    ov.selected = ov.tasks.len() - 1;
                }
            }
            KeyCode::Char('r') => {
                ov.history.redo(&mut ov.tasks);
                ov.save();
            }
            _ => {}
        },

        AddingTask | EditingTask => match key.code {
            KeyCode::Esc => {
                ov.mode = Normal;
                ov.input_buf.clear();
                ov.input_cursor = 0;
                TASK_SCRATCH.with(|s| *s.borrow_mut() = (String::new(), String::new(), String::new()));
            }
            KeyCode::Enter | KeyCode::Tab => {
                match ov.input_field {
                    InputField::Title => {
                        if !ov.input_buf.trim().is_empty() {
                            let saved = ov.input_buf.clone();
                            TASK_SCRATCH.with(|s| {
                                let mut sc = s.borrow_mut();
                                sc.0 = saved;
                                ov.input_buf = sc.1.clone();
                            });
                            ov.input_cursor = ov.input_buf.len();
                            ov.input_field = InputField::Tags;
                        }
                    }
                    InputField::Tags => {
                        let saved = ov.input_buf.clone();
                        TASK_SCRATCH.with(|s| {
                            let mut sc = s.borrow_mut();
                            sc.1 = saved;
                            ov.input_buf = sc.2.clone();
                        });
                        ov.input_cursor = ov.input_buf.len();
                        ov.input_field = InputField::Description;
                    }
                    InputField::Description => {
                        if key.code == KeyCode::Tab {
                            let saved = ov.input_buf.clone();
                            TASK_SCRATCH.with(|s| {
                                let mut sc = s.borrow_mut();
                                sc.2 = saved;
                                ov.input_buf = sc.0.clone();
                            });
                            ov.input_cursor = ov.input_buf.len();
                            ov.input_field = InputField::Title;
                        } else {
                            let desc_val = ov.input_buf.trim().to_string();
                            TASK_SCRATCH.with(|s| s.borrow_mut().2 = desc_val);
                            TASK_SCRATCH.with(|s| {
                                let sc = s.borrow();
                                let title = sc.0.trim().to_string();
                                if title.is_empty() { return; }
                                let tags: Vec<String> = sc.1.split(',')
                                    .map(|t| t.trim().to_string())
                                    .filter(|t| !t.is_empty()).collect();
                                let description = { let d = sc.2.trim().to_string(); if d.is_empty() { None } else { Some(d) } };
                                if ov.mode == AddingTask {
                                    let mut task = Task::new(title);
                                    task.set_tags(tags);
                                    task.update_description(description);
                                    ov.history.push(crate::planner::history::Action::Add { task: task.clone() });
                                    ov.tasks.push(task);
                                    ov.selected = ov.tasks.len() - 1;
                                } else if let Some(task) = ov.tasks.get_mut(ov.selected) {
                                    let old_title = task.title.clone();
                                    let old_desc  = task.description.clone();
                                    let old_tags  = task.tags.clone();
                                    ov.history.push(crate::planner::history::Action::UpdateTitle { id: task.id.clone(), old_title, new_title: title.clone() });
                                    ov.history.push(crate::planner::history::Action::UpdateDescription { id: task.id.clone(), old_desc, new_desc: description.clone() });
                                    ov.history.push(crate::planner::history::Action::UpdateTags { id: task.id.clone(), old_tags, new_tags: tags.clone() });
                                    task.update_title(title);
                                    task.update_description(description);
                                    task.set_tags(tags);
                                }
                                ov.save();
                            });
                            ov.mode = Normal;
                            ov.input_buf.clear();
                            ov.input_cursor = 0;
                            TASK_SCRATCH.with(|s| *s.borrow_mut() = (String::new(), String::new(), String::new()));
                        }
                    }
                }
            }
            KeyCode::Backspace => {
                if ov.input_cursor > 0 { ov.input_cursor -= 1; ov.input_buf.remove(ov.input_cursor); }
            }
            KeyCode::Left  => { if ov.input_cursor > 0 { ov.input_cursor -= 1; } }
            KeyCode::Right => { if ov.input_cursor < ov.input_buf.len() { ov.input_cursor += 1; } }
            KeyCode::Char(c) => { ov.input_buf.insert(ov.input_cursor, c); ov.input_cursor += 1; }
            _ => {}
        },

        ConfirmDelete => match key.code {
            KeyCode::Char('y') | KeyCode::Enter => {
                if ov.selected < ov.tasks.len() {
                    let task = ov.tasks.remove(ov.selected);
                    ov.history.push(crate::planner::history::Action::Delete { task, index: ov.selected });
                    if ov.selected >= ov.tasks.len() && !ov.tasks.is_empty() {
                        ov.selected = ov.tasks.len() - 1;
                    }
                    ov.save();
                }
                ov.mode = Normal;
            }
            _ => { ov.mode = Normal; }
        },
    }
    false
}

fn clamp_scroll(ov: &mut PlannerOverlay) {
    let visible = 15usize;
    if ov.selected < ov.scroll { ov.scroll = ov.selected; }
    else if ov.selected >= ov.scroll + visible { ov.scroll = ov.selected - visible + 1; }
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 0..=m { dp[i][0] = i; }
    for j in 0..=n { dp[0][j] = j; }
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = if a[i-1] == b[j-1] { dp[i-1][j-1] }
            else { 1 + dp[i-1][j].min(dp[i][j-1]).min(dp[i-1][j-1]) };
        }
    }
    dp[m][n]
}