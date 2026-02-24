use std::time::{Duration, SystemTime};
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

    /// Returns true if the "Clear ALL" option (index == length) is selected
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
pub enum PlannerFocus {
    List,
    Detail,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlannerMode {
    Normal,
    AddingTask,
    EditingTask,
    ConfirmDelete,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputField {
    Title,
    Tags,
    Description,
}

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

pub enum Overlay {
    Scanner(ScannerOverlay),
    Planner(PlannerOverlay),
    Ignore(IgnoreOverlay),
}

// ─── Output ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum OutputKind {
    Success,
    Error,
    Info,
    Command,
}

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
    pub needs_full_redraw: bool,  // triggers terminal.clear() next frame after overlay closes
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
            pending_command: None,
        };
        app.outputs.push(OutputBlock {
            kind: OutputKind::Info,
            content: "Welcome to GitLink TUI. Type / to see available commands.".to_string(),
        });
        app
    }

    pub fn on_tick(&mut self) {
        self.elapsed = self
            .start_time
            .elapsed()
            .unwrap_or(Duration::ZERO)
            .as_secs_f32();
    }

    pub fn on_key(&mut self, key: KeyEvent) -> bool {
        if key.kind != KeyEventKind::Press {
            return false;
        }

        if self.overlay.is_some() {
            return self.handle_overlay_key(key);
        }

        if self.is_executing {
            return false;
        }

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

            KeyCode::Left => {
                if self.cursor_pos > 0 { self.cursor_pos -= 1; }
            }
            KeyCode::Right => {
                if self.cursor_pos < self.input.len() { self.cursor_pos += 1; }
            }

            KeyCode::Up => {
                if self.show_suggestions && !self.filtered_commands.is_empty() {
                    if self.selected_index > 0 {
                        self.selected_index -= 1;
                    } else {
                        self.selected_index = self.filtered_commands.len() - 1;
                    }
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
                    if self.output_scroll > 0.0 {
                        self.output_scroll -= 1.0;
                    }
                } else {
                    self.navigate_history(1);
                }
            }

            KeyCode::PageUp => { self.output_scroll += 5.0; }
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
                        self.push_output(OutputBlock {
                            kind: OutputKind::Success,
                            content: "Scan review complete.".to_string(),
                        });
                    }
                }
            }
            Some(Overlay::Planner(ref mut ov)) => {
                let close = handle_planner_key(ov, key);
                if close {
                    self.overlay = None;
                    self.needs_full_redraw = true;
                    self.push_output(OutputBlock {
                        kind: OutputKind::Success,
                        content: "Planner closed.".to_string(),
                    });
                }
            }
            Some(Overlay::Ignore(ref mut ov)) => {
                handle_ignore_key(ov, key);
                if ov.done {
                    self.overlay = None;
                    self.needs_full_redraw = true;
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
            if self.selected_index >= self.filtered_commands.len() {
                self.selected_index = 0;
            }
        } else {
            self.show_suggestions = false;
            self.filtered_commands.clear();
            self.selected_index = 0;
        }
    }

    fn submit(&mut self) {
        let raw = self.input.trim().to_string();
        if raw.is_empty() { return; }

        self.outputs.push(OutputBlock {
            kind: OutputKind::Command,
            content: raw.clone(),
        });

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

        let cmd_name = raw[1..].split_whitespace().next().unwrap_or("").to_string();
        let known = COMMANDS.iter().any(|c| c.name == cmd_name);
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

    pub fn take_pending_command(&mut self) -> Option<String> {
        self.pending_command.take()
    }

    pub fn push_output(&mut self, block: OutputBlock) {
        self.is_executing = false;
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
            self.push_output(OutputBlock {
                kind: OutputKind::Info,
                content: "No ignored findings to manage.".to_string(),
            });
        } else {
            self.overlay = Some(Overlay::Ignore(ov));
        }
    }
}

// ─── Scanner key handler ──────────────────────────────────────────────────────

pub fn handle_scanner_key(ov: &mut ScannerOverlay, key: KeyEvent) {
    match key.code {
        KeyCode::Left | KeyCode::Char('h') => { ov.choice = ScanChoice::Ignore; }
        KeyCode::Right | KeyCode::Char('l') => { ov.choice = ScanChoice::Keep; }
        KeyCode::Up | KeyCode::Char('k') => { ov.choice = ScanChoice::Keep; }
        KeyCode::Down | KeyCode::Char('j') => { ov.choice = ScanChoice::Ignore; }
        KeyCode::Enter => {
            if ov.choice == ScanChoice::Ignore {
                if let Some(f) = ov.findings.get(ov.current_index) {
                    let short_id = f.fingerprint[..8.min(f.fingerprint.len())].to_string();
                    let variable = f.content
                        .split('=')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .split(|c: char| !c.is_alphanumeric() && c != '_')
                        .filter(|s| !s.is_empty())
                        .last()
                        .unwrap_or("unknown")
                        .to_string();
                    let source = if f.commit.is_some() { "history" } else { "working" }.to_string();
                    crate::scanner::ignore::add_ignored(crate::scanner::ignore::IgnoredItem {
                        fingerprint: f.fingerprint.clone(),
                        short_id,
                        variable,
                        source,
                        commit: f.commit.clone(),
                    });
                }
            }
            ov.current_index += 1;
            ov.choice = ScanChoice::Keep;
            if ov.current_index >= ov.findings.len() {
                ov.done = true;
            }
        }
        KeyCode::Esc | KeyCode::Char('q') => { ov.done = true; }
        _ => {}
    }
}

// ─── Ignore Manager key handler ──────────────────────────────────────────────

pub fn handle_ignore_key(ov: &mut IgnoreOverlay, key: KeyEvent) {
    let total_options = ov.items.len() + 2;

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if ov.selected > 0 {
                ov.selected -= 1;
            } else {
                ov.selected = total_options.saturating_sub(1);
            }
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
                if ov.items.is_empty() {
                    ov.selected = 0;
                } else if ov.selected >= ov.items.len() {
                    ov.selected = ov.items.len().saturating_sub(1);
                }
            } else if ov.selected == len {
                crate::scanner::ignore::clear_all_silent();
                ov.items.clear();
                ov.done = true;
            } else {
                ov.done = true;
            }
        }
        KeyCode::Esc | KeyCode::Char('q') => {
            ov.items.clear();
            ov.done = true;
        }
        _ => {}
    }
}

// ─── Planner key handler ──────────────────────────────────────────────────────

pub fn handle_planner_key(ov: &mut PlannerOverlay, key: KeyEvent) -> bool {
    use PlannerMode::*;
    use PlannerFocus::*;

    match ov.mode {
        Normal => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => return true,
            KeyCode::Tab => {
                ov.focus = match ov.focus {
                    List => Detail,
                    Detail => List,
                };
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
            }
            KeyCode::Char('e') if ov.focus == List => {
                if let Some(task) = ov.tasks.get(ov.selected) {
                    ov.mode = EditingTask;
                    ov.input_field = InputField::Title;
                    ov.input_buf = task.title.clone();
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
            KeyCode::Esc => { ov.mode = Normal; ov.input_buf.clear(); }
            KeyCode::Enter => {
                let title = ov.input_buf.trim().to_string();
                if !title.is_empty() {
                    if ov.mode == AddingTask {
                        let task = Task::new(title);
                        ov.history.push(crate::planner::history::Action::Add { task: task.clone() });
                        ov.tasks.push(task);
                        ov.selected = ov.tasks.len() - 1;
                    } else if let Some(task) = ov.tasks.get_mut(ov.selected) {
                        let old = task.title.clone();
                        ov.history.push(crate::planner::history::Action::UpdateTitle {
                            id: task.id.clone(),
                            old_title: old,
                            new_title: title.clone(),
                        });
                        task.update_title(title);
                    }
                    ov.save();
                }
                ov.mode = Normal;
                ov.input_buf.clear();
            }
            KeyCode::Backspace => {
                if ov.input_cursor > 0 {
                    ov.input_cursor -= 1;
                    ov.input_buf.remove(ov.input_cursor);
                }
            }
            KeyCode::Left => { if ov.input_cursor > 0 { ov.input_cursor -= 1; } }
            KeyCode::Right => { if ov.input_cursor < ov.input_buf.len() { ov.input_cursor += 1; } }
            KeyCode::Char(c) => {
                ov.input_buf.insert(ov.input_cursor, c);
                ov.input_cursor += 1;
            }
            _ => {}
        },
        ConfirmDelete => match key.code {
            KeyCode::Char('y') | KeyCode::Enter => {
                if ov.selected < ov.tasks.len() {
                    let task = ov.tasks.remove(ov.selected);
                    ov.history.push(crate::planner::history::Action::Delete {
                        task,
                        index: ov.selected,
                    });
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
    if ov.selected < ov.scroll {
        ov.scroll = ov.selected;
    } else if ov.selected >= ov.scroll + visible {
        ov.scroll = ov.selected - visible + 1;
    }
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