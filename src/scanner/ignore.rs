use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const IGNORE_FILE: &str = ".gitlinkignore.json";

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
    let db = load_ignore_db();

    if db.ignored.is_empty() {
        println!("No ignored findings.");
        return;
    }

    println!("Ignored findings:\n");

    for item in db.ignored {
        if item.source == "history" {
            if let Some(commit) = &item.commit {
                println!(
                    "[{}] {} (commit {})",
                    item.short_id,
                    item.variable,
                    &commit[..8]
                );
            } else {
                println!(
                    "[{}] {} (history)",
                    item.short_id,
                    item.variable
                );
            }
        } else {
            println!(
                "[{}] {} (working)",
                item.short_id,
                item.variable
            );
        }
    }
}

pub fn clear_all() {
    save_ignore_db(&IgnoreDatabase::default());
    println!("All ignored findings cleared.");
}

pub fn remove_by_short_id(short_id: &str) {
    let mut db = load_ignore_db();

    let original_len = db.ignored.len();
    db.ignored.retain(|item| item.short_id != short_id);

    if db.ignored.len() < original_len {
        println!("Removed [{}].", short_id);
    } else {
        println!("Short ID [{}] not found.", short_id);
    }

    save_ignore_db(&db);
}

//to ensure .gitlink_ignore.json is ignored by git
use std::fs::{OpenOptions};
use std::io::{Write, Read};

pub fn ensure_gitignore_entry() {
    let gitignore_path = ".gitignore";
    let entry = ".gitlinkignore.json";

    // Read existing content if file exists
    let mut existing = String::new();

    if let Ok(mut file) = fs::File::open(gitignore_path) {
        let _ = file.read_to_string(&mut existing);
    }

    // If already present → do nothing
    if existing.lines().any(|line| line.trim() == entry) {
        return;
    }

    // Otherwise append
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(gitignore_path)
    {
        let _ = writeln!(file, "\n# GitLink ignore database\n{}", entry);
    }
}
