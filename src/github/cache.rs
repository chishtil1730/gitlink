use std::fs;
use std::path::PathBuf;
use std::time::{Duration};

fn cache_dir() -> PathBuf {
    let mut dir = dirs::cache_dir().expect("No cache dir found");
    dir.push("gitlink");
    fs::create_dir_all(&dir).ok();
    dir
}

pub fn cache_path(key: &str) -> PathBuf {
    let mut path = cache_dir();
    path.push(format!("{}.json", key));
    path
}

pub fn is_cache_valid(path: &PathBuf, ttl: Duration) -> bool {
    if let Ok(metadata) = fs::metadata(path) {
        if let Ok(modified) = metadata.modified() {
            return modified.elapsed().unwrap_or(ttl) < ttl;
        }
    }
    false
}
