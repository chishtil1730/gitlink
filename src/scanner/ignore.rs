use serde::{Serialize, Deserialize};
use std::fs;

const IGNORE_FILE: &str = ".gitlinkignore.json";

#[derive(Serialize, Deserialize, Default)]
pub struct IgnoreDatabase {
    pub ignored: Vec<String>,
}

pub fn load_ignore_db() -> IgnoreDatabase {
    if let Ok(data) = fs::read_to_string(IGNORE_FILE) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        IgnoreDatabase::default()
    }
}

pub fn save_ignore_db(db: &IgnoreDatabase) {
    if let Ok(json) = serde_json::to_string_pretty(db) {
        let _ = fs::write(IGNORE_FILE, json);
    }
}
