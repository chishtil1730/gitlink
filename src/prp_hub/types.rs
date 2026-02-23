use std::path::PathBuf;
use std::time::SystemTime;
use git2::Oid;

/// Basic info about a discovered repository
#[derive(Debug, Clone)]
pub struct RepositoryInfo {
    pub name: String,
    pub path: PathBuf,
}

/// A single commit recorded during this session
#[derive(Debug, Clone)]
pub struct RepoCommitResult {
    pub path: PathBuf,
    pub commit_oid: Oid,
}

/// The entire session: tracks what was discovered, committed, and rolled back
#[derive(Debug)]
pub struct CommitSession {
    /// e.g. "gitlink-550e8400-e29b-41d4-a716-446655440000"
    pub group_id: String,
    pub repositories: Vec<RepositoryInfo>,
    pub committed: Vec<RepoCommitResult>,
    pub started_at: SystemTime,
}

impl CommitSession {
    pub fn new(group_id: String, repositories: Vec<RepositoryInfo>) -> Self {
        Self {
            group_id,
            repositories,
            committed: Vec::new(),
            started_at: SystemTime::now(),
        }
    }
}