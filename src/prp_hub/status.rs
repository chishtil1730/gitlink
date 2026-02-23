use git2::{Repository, Status, StatusOptions};

use crate::prp_hub::types::RepositoryInfo;

#[derive(Debug, Default)]
pub struct RepoStatus {
    pub modified: Vec<String>,
    pub added: Vec<String>,
    pub deleted: Vec<String>,
    pub untracked: Vec<String>,
}

impl RepoStatus {
    pub fn is_empty(&self) -> bool {
        self.modified.is_empty()
            && self.added.is_empty()
            && self.deleted.is_empty()
            && self.untracked.is_empty()
    }

    pub fn total(&self) -> usize {
        self.modified.len() + self.added.len() + self.deleted.len() + self.untracked.len()
    }
}

pub fn collect_status(info: &RepositoryInfo) -> RepoStatus {
    let mut result = RepoStatus::default();

    let repo = match Repository::open(&info.path) {
        Ok(r) => r,
        Err(_) => return result,
    };

    let mut opts = StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);

    let statuses = match repo.statuses(Some(&mut opts)) {
        Ok(s) => s,
        Err(_) => return result,
    };

    for entry in statuses.iter() {
        let path = entry.path().unwrap_or("(unknown)").to_string();
        let st = entry.status();

        if st.contains(Status::IGNORED) || st == Status::CURRENT {
            continue;
        }

        if st.intersects(Status::INDEX_NEW | Status::WT_NEW) {
            result.untracked.push(path.clone());
        }
        if st.intersects(Status::INDEX_MODIFIED | Status::WT_MODIFIED) {
            result.modified.push(path.clone());
        }
        if st.intersects(Status::INDEX_DELETED | Status::WT_DELETED) {
            result.deleted.push(path.clone());
        }
        if st.intersects(Status::INDEX_NEW) && !st.intersects(Status::WT_NEW) {
            // already staged new file — treat as added (staged)
            // remove from untracked if we put it there, list as added
            result.untracked.retain(|p| p != &path);
            if !result.added.contains(&path) {
                result.added.push(path.clone());
            }
        }
    }

    result
}

pub fn display_repo_status(info: &RepositoryInfo, status: &RepoStatus) {
    println!("\n  📁 {}", info.name);
    println!("  {}", "─".repeat(60));

    if !status.modified.is_empty() {
        println!("  🟡 Modified ({}):", status.modified.len());
        for f in &status.modified {
            println!("       {}", f);
        }
    }
    if !status.added.is_empty() {
        println!("  🟢 Added/Staged ({}):", status.added.len());
        for f in &status.added {
            println!("       {}", f);
        }
    }
    if !status.deleted.is_empty() {
        println!("  🔴 Deleted ({}):", status.deleted.len());
        for f in &status.deleted {
            println!("       {}", f);
        }
    }
    if !status.untracked.is_empty() {
        println!("  ⬜ Untracked ({}):", status.untracked.len());
        for f in &status.untracked {
            println!("       {}", f);
        }
    }
}