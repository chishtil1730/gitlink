use super::task::Task;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const TASKS_FILE: &str = ".gitlink-tasks.json";
const ARCHIVED_FILE: &str = ".gitlink-tasks-archive.json";
const GITIGNORE_FILE: &str = ".gitignore";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskList {
    pub tasks: Vec<Task>,
}

impl TaskList {
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }
}

pub fn ensure_gitignore() {
    let gitignore_path = PathBuf::from(GITIGNORE_FILE);

    let mut content = if gitignore_path.exists() {
        fs::read_to_string(&gitignore_path).unwrap_or_default()
    } else {
        String::new()
    };

    let entries = [TASKS_FILE, ARCHIVED_FILE];
    let mut modified = false;

    for entry in &entries {
        if !content.lines().any(|line| line.trim() == *entry) {
            if !content.is_empty() && !content.ends_with('\n') {
                content.push('\n');
            }
            content.push_str(entry);
            content.push('\n');
            modified = true;
        }
    }

    if modified {
        if let Err(e) = fs::write(&gitignore_path, content) {
            eprintln!("⚠️  Warning: Could not update .gitignore: {}", e);
        }
    }
}

pub fn load_tasks() -> TaskList {
    ensure_gitignore();

    let path = PathBuf::from(TASKS_FILE);

    if !path.exists() {
        return TaskList::new();
    }

    match fs::read_to_string(&path) {
        Ok(content) => {
            serde_json::from_str(&content).unwrap_or_else(|_| TaskList::new())
        }
        Err(_) => TaskList::new(),
    }
}

pub fn save_tasks(task_list: &TaskList) -> Result<(), Box<dyn std::error::Error>> {
    ensure_gitignore();

    let json = serde_json::to_string_pretty(task_list)?;
    fs::write(TASKS_FILE, json)?;
    Ok(())
}

pub fn load_archive() -> TaskList {
    ensure_gitignore();

    let path = PathBuf::from(ARCHIVED_FILE);

    if !path.exists() {
        return TaskList::new();
    }

    match fs::read_to_string(&path) {
        Ok(content) => {
            serde_json::from_str(&content).unwrap_or_else(|_| TaskList::new())
        }
        Err(_) => TaskList::new(),
    }
}

pub fn save_archive(task_list: &TaskList) -> Result<(), Box<dyn std::error::Error>> {
    ensure_gitignore();

    let json = serde_json::to_string_pretty(task_list)?;
    fs::write(ARCHIVED_FILE, json)?;
    Ok(())
}