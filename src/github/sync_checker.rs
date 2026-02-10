use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::github::graphql::{fetch_repository_sync_info, GraphQLClient, RepositoryInfo};

#[derive(Debug)]
pub enum SyncStatus {
    InSync,
    LocalAhead { commits: i32 },
    RemoteAhead { commits: i32 },
    Diverged { local_ahead: i32, remote_ahead: i32 },
    NoLocalRepo,
    BranchMismatch { local_branch: String, remote_branch: String },
}

impl SyncStatus {
    pub fn emoji(&self) -> &str {
        match self {
            SyncStatus::InSync => "✅",
            SyncStatus::LocalAhead { .. } => "⬆️",
            SyncStatus::RemoteAhead { .. } => "⬇️",
            SyncStatus::Diverged { .. } => "🔀",
            SyncStatus::NoLocalRepo => "❌",
            SyncStatus::BranchMismatch { .. } => "🔄",
        }
    }

    pub fn description(&self) -> String {
        match self {
            SyncStatus::InSync => "In sync with remote".to_string(),
            SyncStatus::LocalAhead { commits } => {
                format!("Local is {} commit(s) ahead", commits)
            }
            SyncStatus::RemoteAhead { commits } => {
                format!("Remote is {} commit(s) ahead", commits)
            }
            SyncStatus::Diverged { local_ahead, remote_ahead } => {
                format!(
                    "Diverged: {} ahead, {} behind",
                    local_ahead, remote_ahead
                )
            }
            SyncStatus::NoLocalRepo => "Not cloned locally".to_string(),
            SyncStatus::BranchMismatch { local_branch, remote_branch } => {
                format!(
                    "Branch mismatch: local={}, remote={}",
                    local_branch, remote_branch
                )
            }
        }
    }
}

pub struct SyncChecker {
    client: GraphQLClient,
}

impl SyncChecker {
    pub fn new(client: GraphQLClient) -> Self {
        Self { client }
    }

    /// Check if a repository exists locally
    pub fn find_local_repo(&self, repo_name: &str) -> Option<PathBuf> {
        // Check common locations
        let common_paths = vec![
            PathBuf::from("."),
            PathBuf::from(format!("../{}", repo_name)),
            PathBuf::from(format!("../../{}", repo_name)),
            dirs::home_dir()?.join("projects").join(repo_name),
            dirs::home_dir()?.join("dev").join(repo_name),
            dirs::home_dir()?.join("code").join(repo_name),
        ];

        for path in common_paths {
            if path.join(".git").exists() {
                if let Some(name) = path.file_name() {
                    if name.to_string_lossy() == repo_name {
                        return Some(path);
                    }
                }
            }
        }

        None
    }

    /// Get local repository information
    pub fn get_local_info(&self, path: &Path) -> Result<LocalRepoInfo, Box<dyn Error>> {
        let current_branch = self.get_current_branch(path)?;
        let latest_commit = self.get_latest_commit(path, &current_branch)?;
        let uncommitted_changes = self.has_uncommitted_changes(path)?;

        Ok(LocalRepoInfo {
            path: path.to_path_buf(),
            current_branch,
            latest_commit,
            uncommitted_changes,
        })
    }

    fn get_current_branch(&self, path: &Path) -> Result<String, Box<dyn Error>> {
        let output = Command::new("git")
            .arg("-C")
            .arg(path)
            .arg("branch")
            .arg("--show-current")
            .output()?;

        if output.status.success() {
            Ok(String::from_utf8(output.stdout)?.trim().to_string())
        } else {
            Err("Failed to get current branch".into())
        }
    }

    fn get_latest_commit(&self, path: &Path, branch: &str) -> Result<String, Box<dyn Error>> {
        let output = Command::new("git")
            .arg("-C")
            .arg(path)
            .arg("rev-parse")
            .arg(branch)
            .output()?;

        if output.status.success() {
            Ok(String::from_utf8(output.stdout)?.trim().to_string())
        } else {
            Err("Failed to get latest commit".into())
        }
    }

    fn has_uncommitted_changes(&self, path: &Path) -> Result<bool, Box<dyn Error>> {
        let output = Command::new("git")
            .arg("-C")
            .arg(path)
            .arg("status")
            .arg("--porcelain")
            .output()?;

        Ok(!output.stdout.is_empty())
    }

    fn count_commits_between(
        &self,
        path: &Path,
        from: &str,
        to: &str,
    ) -> Result<i32, Box<dyn Error>> {
        let output = Command::new("git")
            .arg("-C")
            .arg(path)
            .arg("rev-list")
            .arg("--count")
            .arg(format!("{}..{}", from, to))
            .output()?;

        if output.status.success() {
            let count = String::from_utf8(output.stdout)?.trim().parse()?;
            Ok(count)
        } else {
            Ok(0)
        }
    }

    /// Check sync status between local and remote
    pub async fn check_sync(
        &self,
        repo: &RepositoryInfo,
        local_path: Option<&Path>,
    ) -> Result<SyncStatus, Box<dyn Error>> {
        // If no local path provided, search for it
        let local_path = match local_path {
            Some(p) => Some(p.to_path_buf()),
            None => self.find_local_repo(&repo.name),
        };

        let local_path = match local_path {
            Some(p) => p,
            None => return Ok(SyncStatus::NoLocalRepo),
        };

        // Get local repo info
        let local_info = self.get_local_info(&local_path)?;

        // Get remote repo info via GraphQL
        let remote_info = fetch_repository_sync_info(
            &self.client,
            &repo.owner.login,
            &repo.name,
        )
            .await?;

        // Get the default branch info
        let remote_branch = match &remote_info.repository.default_branch_ref {
            Some(branch_ref) => branch_ref,
            None => return Err("Remote repository has no default branch".into()),
        };

        // Check if branches match
        if local_info.current_branch != remote_branch.name {
            return Ok(SyncStatus::BranchMismatch {
                local_branch: local_info.current_branch.clone(),
                remote_branch: remote_branch.name.clone(),
            });
        }

        // Fetch latest from remote to ensure accurate comparison
        let _ = Command::new("git")
            .arg("-C")
            .arg(&local_path)
            .arg("fetch")
            .arg("origin")
            .arg(&local_info.current_branch)
            .output();

        let remote_branch_ref = format!("origin/{}", local_info.current_branch);

        // Compare commits
        let local_ahead = self.count_commits_between(
            &local_path,
            &remote_branch_ref,
            &local_info.current_branch,
        )?;

        let remote_ahead = self.count_commits_between(
            &local_path,
            &local_info.current_branch,
            &remote_branch_ref,
        )?;

        match (local_ahead, remote_ahead) {
            (0, 0) => Ok(SyncStatus::InSync),
            (n, 0) if n > 0 => Ok(SyncStatus::LocalAhead { commits: n }),
            (0, n) if n > 0 => Ok(SyncStatus::RemoteAhead { commits: n }),
            (local, remote) => Ok(SyncStatus::Diverged {
                local_ahead: local,
                remote_ahead: remote,
            }),
        }
    }

    /// Display sync status for a repository
    pub async fn display_sync_status(
        &self,
        repo: &RepositoryInfo,
    ) -> Result<(), Box<dyn Error>> {
        println!("\n🔍 Checking sync status for: {}", repo.name_with_owner);
        println!("{}", "─".repeat(80));

        let status = self.check_sync(repo, None).await?;

        println!("{} {}", status.emoji(), status.description());

        // If local repo exists, show more details
        if let Some(local_path) = self.find_local_repo(&repo.name) {
            let local_info = self.get_local_info(&local_path)?;

            println!("\n📁 Local path: {}", local_path.display());
            println!("🌿 Current branch: {}", local_info.current_branch);
            println!("📝 Latest commit: {}", &local_info.latest_commit[..8]);

            if local_info.uncommitted_changes {
                println!("⚠️  You have uncommitted changes");
            }
        }

        println!("{}", "─".repeat(80));

        Ok(())
    }

    /// Display sync status for multiple repositories
    pub async fn display_multi_sync_status(
        &self,
        repos: &[&RepositoryInfo],
    ) -> Result<(), Box<dyn Error>> {
        println!("\n🔍 Checking sync status for {} repositories...", repos.len());
        println!("{}", "=".repeat(80));

        for repo in repos {
            let status = self.check_sync(repo, None).await?;
            println!(
                "{} {} - {}",
                status.emoji(),
                repo.name_with_owner,
                status.description()
            );
        }

        println!("{}", "=".repeat(80));
        Ok(())
    }
}

#[derive(Debug)]
pub struct LocalRepoInfo {
    pub path: PathBuf,
    pub current_branch: String,
    pub latest_commit: String,
    pub uncommitted_changes: bool,
}