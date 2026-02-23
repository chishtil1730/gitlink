use std::time::{Duration, SystemTime};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::commands::{Command, COMMANDS};

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

pub struct App {
    pub input: String,
    pub cursor_pos: usize,

    pub filtered_commands: Vec<Command>,
    pub selected_index: usize,

    pub outputs: Vec<OutputBlock>,
    pub output_scroll: u16,

    pub is_executing: bool,
    pub show_suggestions: bool,

    pub start_time: SystemTime,
    pub elapsed: f32,

    // Command history navigation
    pub cmd_history: Vec<String>,
    pub history_index: Option<usize>,

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
            output_scroll: 0,
            is_executing: false,
            show_suggestions: false,
            start_time: SystemTime::now(),
            elapsed: 0.0,
            cmd_history: vec![],
            history_index: None,
            pending_command: None,
        };
        // Welcome message
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

    /// Returns true if the app should quit.
    pub fn on_key(&mut self, key: KeyEvent) -> bool {
        if self.is_executing {
            return false;
        }

        match key.code {
            // Quit
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => return true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return true,

            // Submit
            KeyCode::Enter => self.submit(),

            // Backspace
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.input.remove(self.cursor_pos);
                    self.update_suggestions();
                }
            }

            // Cursor movement
            KeyCode::Left => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor_pos < self.input.len() {
                    self.cursor_pos += 1;
                }
            }

            // Suggestion navigation
            KeyCode::Up => {
                if self.show_suggestions && !self.filtered_commands.is_empty() {
                    if self.selected_index > 0 {
                        self.selected_index -= 1;
                    } else {
                        self.selected_index = self.filtered_commands.len() - 1;
                    }
                } else {
                    self.navigate_history(-1);
                }
            }
            KeyCode::Down => {
                if self.show_suggestions && !self.filtered_commands.is_empty() {
                    self.selected_index =
                        (self.selected_index + 1) % self.filtered_commands.len();
                } else {
                    self.navigate_history(1);
                }
            }

            // Tab: autocomplete selected suggestion
            KeyCode::Tab => {
                if self.show_suggestions && !self.filtered_commands.is_empty() {
                    let cmd = self.filtered_commands[self.selected_index].name.clone();
                    self.input = format!("/{}", cmd);
                    self.cursor_pos = self.input.len();
                    self.show_suggestions = false;
                }
            }

            // Scroll output
            KeyCode::PageUp => {
                self.output_scroll = self.output_scroll.saturating_sub(3);
            }
            KeyCode::PageDown => {
                self.output_scroll = self.output_scroll.saturating_add(3);
            }

            // Character input
            KeyCode::Char(c) => {
                self.input.insert(self.cursor_pos, c);
                self.cursor_pos += 1;
                self.update_suggestions();
                // Reset history navigation on new input
                self.history_index = None;
            }

            _ => {}
        }
        false
    }

    fn update_suggestions(&mut self) {
        if self.input.starts_with('/') {
            let query = self.input[1..].to_lowercase();
            self.filtered_commands = COMMANDS
                .iter()
                .filter(|c| c.name.contains(query.as_str()) || c.description.to_lowercase().contains(query.as_str()))
                .cloned()
                .collect();
            self.show_suggestions = !self.filtered_commands.is_empty() || self.input == "/";
            if self.input == "/" {
                self.filtered_commands = COMMANDS.to_vec();
            }
            // Clamp selection
            if self.selected_index >= self.filtered_commands.len() {
                self.selected_index = 0;
            }
        } else {
            self.show_suggestions = false;
            self.filtered_commands.clear();
        }
    }

    fn submit(&mut self) {
        let raw = self.input.trim().to_string();
        if raw.is_empty() {
            return;
        }

        // Push the command echo to output
        self.outputs.push(OutputBlock {
            kind: OutputKind::Command,
            content: raw.clone(),
        });

        self.cmd_history.push(raw.clone());
        self.history_index = None;

        self.input.clear();
        self.cursor_pos = 0;
        self.show_suggestions = false;
        self.output_scroll = 0;

        // Validate it's a slash command
        if !raw.starts_with('/') {
            self.outputs.push(OutputBlock {
                kind: OutputKind::Error,
                content: format!(
                    "Only slash commands are supported. Try typing /help to see available commands."
                ),
            });
            return;
        }

        // Find closest match if unknown
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
        if self.cmd_history.is_empty() {
            return;
        }
        let len = self.cmd_history.len();
        self.history_index = Some(match self.history_index {
            None => {
                if direction < 0 { len - 1 } else { 0 }
            }
            Some(i) => {
                let next = i as i32 + direction;
                next.max(0).min(len as i32 - 1) as usize
            }
        });
        if let Some(idx) = self.history_index {
            self.input = self.cmd_history[idx].clone();
            self.cursor_pos = self.input.len();
        }
    }

    pub fn take_pending_command(&mut self) -> Option<String> {
        self.pending_command.take()
    }

    pub fn push_output(&mut self, block: OutputBlock) {
        self.is_executing = false;
        self.outputs.push(block);
        // Auto-scroll to bottom
        self.output_scroll = 0;
    }
}

/// Simple Levenshtein distance for closest-match suggestions
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let m = a.len();
    let n = b.len();
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 0..=m { dp[i][0] = i; }
    for j in 0..=n { dp[0][j] = j; }
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = if a[i-1] == b[j-1] {
                dp[i-1][j-1]
            } else {
                1 + dp[i-1][j].min(dp[i][j-1]).min(dp[i-1][j-1])
            };
        }
    }
    dp[m][n]
}