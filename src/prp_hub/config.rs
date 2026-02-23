use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const CONFIG_PATH: &str = ".gitlink/prp_config.json";

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PrpConfig {
    pub excluded_repos: Vec<PathBuf>,
}

fn config_path() -> PathBuf {
    PathBuf::from(CONFIG_PATH)
}

pub fn load_config() -> PrpConfig {
    let path = config_path();
    if !path.exists() {
        return PrpConfig::default();
    }
    let raw = match std::fs::read_to_string(&path) {
        Ok(r) => r,
        Err(_) => return PrpConfig::default(),
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn save_config(config: &PrpConfig) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(config) {
        let _ = std::fs::write(&path, json);
    }
}

pub fn is_excluded(config: &PrpConfig, path: &Path) -> bool {
    config.excluded_repos.iter().any(|e| e == path)
}

pub fn exclude_repo(config: &mut PrpConfig, path: PathBuf) {
    if !is_excluded(config, &path) {
        config.excluded_repos.push(path);
    }
}

pub fn include_repo(config: &mut PrpConfig, path: &Path) {
    config.excluded_repos.retain(|e| e != path);
}