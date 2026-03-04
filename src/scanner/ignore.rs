use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::fs::OpenOptions;
use std::path::Path;

const IGNORE_FILE: &str = ".gitlinkignore.json";

pub const IGNORED_DIRS: &[&str] = &[
    ".git", "target", "node_modules", ".idea", ".vscode", "dist", "build",
    "out", ".next", ".nuxt", ".cache", "__pycache__", ".venv", "venv",
    "env", ".mypy_cache", ".pytest_cache", "coverage", ".gradle",
    ".terraform", ".serverless",
];

pub const IGNORED_EXTENSIONS: &[&str] = &[
    "exe", "dll", "so", "dylib", "bin", "o", "a", "class", "jar",
    "png", "jpg", "jpeg", "gif", "pdf", "zip", "tar", "gz",
];

pub const IGNORED_FILES: &[&str] = &[
    "Cargo.lock", "package-lock.json", "yarn.lock", "pnpm-lock.yaml",
];

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IgnoredItem {
    pub fingerprint: String,
    pub short_id: String,
    pub variable: String,
    pub source: String,           // "working" or "history"
    pub commit: Option<String>,   // Only used for history findings
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct IgnoreDatabase {
    pub ignored: Vec<IgnoredItem>,
}

// ─── Data Access ─────────────────────────────────────────────────────────────

pub fn load_ignore_db() -> IgnoreDatabase {
    if Path::new(IGNORE_FILE).exists() {
        match fs::read_to_string(IGNORE_FILE) {
            Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
            Err(_) => IgnoreDatabase::default(),
        }
    } else {
        IgnoreDatabase::default()
    }
}

pub fn save_ignore_db(db: &IgnoreDatabase) {
    if let Ok(json) = serde_json::to_string_pretty(db) {
        let _ = fs::write(IGNORE_FILE, json);
    }
}

// ─── TUI Specific Helpers ────────────────────────────────────────────────────

/// Returns a formatted string of ignored items for the TUI output.
pub fn get_ignored_list_string() -> String {
    let db = load_ignore_db();

    if db.ignored.is_empty() {
        return "No ignored findings.".to_string();
    }

    let mut output = String::from("Ignored findings:\n\n");

    for item in db.ignored {
        let source_info = if item.source == "history" {
            if let Some(commit) = &item.commit {
                format!("(commit {})", &commit[..8.min(commit.len())])
            } else {
                "(history)".to_string()
            }
        } else {
            "(working)".to_string()
        };

        output.push_str(&format!(
            "  [{}] {} {}\n",
            item.short_id,
            item.variable,
            source_info
        ));
    }
    output
}

/// Clears the DB without printing to stdout (prevents TUI artifacts).
pub fn clear_all_silent() {
    save_ignore_db(&IgnoreDatabase::default());
}

// ─── Core Logic ──────────────────────────────────────────────────────────────

pub fn add_ignored(item: IgnoredItem) {
    let mut db = load_ignore_db();

    // Prevent duplicate entries
    if !db.ignored.iter().any(|i| i.fingerprint == item.fingerprint) {
        db.ignored.push(item);
        ensure_gitignore_entry();
        save_ignore_db(&db);
    }
}

pub fn list_ignored() {
    println!("{}", get_ignored_list_string());
}

pub fn clear_all() {
    clear_all_silent();
    println!("All ignored findings cleared.");
}

pub fn remove_by_short_id(short_id: &str) {
    let mut db = load_ignore_db();

    let original_len = db.ignored.len();
    db.ignored.retain(|item| item.short_id != short_id);

    if db.ignored.len() < original_len {
        //println!("Removed [{}].", short_id);
    } else {
        //println!("Short ID [{}] not found.", short_id);
    }

    save_ignore_db(&db);
}

// ─── Git Integration ─────────────────────────────────────────────────────────

pub fn ensure_gitignore_entry() {
    let gitignore_path = ".gitignore";

    // 1. Collect all items we want to ensure are in .gitignore
    let mut entries_to_add = Vec::new();

    // Always include the database file itself
    entries_to_add.push(".gitlinkignore.json");

    // Add directories (suffixed with / for gitignore best practice)
    for dir in IGNORED_DIRS {
        entries_to_add.push(dir);
    }

    // Add specific files
    for file in IGNORED_FILES {
        entries_to_add.push(file);
    }

    // Add extensions (formatted as *.ext)
    let formatted_exts: Vec<String> = IGNORED_EXTENSIONS
        .iter()
        .map(|ext| format!("*.{}", ext))
        .collect();

    // 2. Read existing .gitignore content to avoid duplicates
    let mut existing_lines = Vec::new();
    if let Ok(content) = fs::read_to_string(gitignore_path) {
        existing_lines = content.lines().map(|l| l.trim().to_string()).collect();
    }

    // 3. Filter out what's already there
    let new_entries: Vec<&str> = entries_to_add
        .into_iter()
        .filter(|entry| !existing_lines.contains(&entry.to_string()))
        .collect();

    let new_exts: Vec<&str> = formatted_exts
        .iter()
        .map(|s| s.as_str())
        .filter(|entry| !existing_lines.contains(&entry.to_string()))
        .collect();

    // 4. If there's anything new to add, append it
    if !new_entries.is_empty() || !new_exts.is_empty() {
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(gitignore_path)
        {
            // Add a header for clarity
            let _ = writeln!(file, "\n# GitLink Auto-Ignored Items");

            for entry in new_entries {
                let _ = writeln!(file, "{}", entry);
            }
            for ext in new_exts {
                let _ = writeln!(file, "{}", ext);
            }
        }
    }
}