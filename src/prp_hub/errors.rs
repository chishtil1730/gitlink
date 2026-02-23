use std::fmt;

#[derive(Debug)]
pub enum PrpError {
    DiscoveryError(String),
    InvalidState { repo: String, reason: String, fix: String },
    DetachedHead(String),
    MergeConflict(String),
    UnmergedPaths(String),
    CommitFailed { repo: String, reason: String },
    PushFailed { repo: String, stderr: String },
    RollbackFailed { repo: String, reason: String },
    NoRepositoriesFound,
}

impl fmt::Display for PrpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrpError::DiscoveryError(msg) => write!(f, "Discovery error: {}", msg),
            PrpError::InvalidState { repo, reason, fix } => {
                write!(
                    f,
                    "\n❌ Repository: {}\n   Error: {}\n   Suggested Fix: {}",
                    repo, reason, fix
                )
            }
            PrpError::DetachedHead(repo) => write!(
                f,
                "\n❌ Repository: {}\n   Error: HEAD is detached\n   Suggested Fix: Run `git checkout <branch>`",
                repo
            ),
            PrpError::MergeConflict(repo) => write!(
                f,
                "\n❌ Repository: {}\n   Error: Merge in progress\n   Suggested Fix: Resolve conflicts and commit, or run `git merge --abort`",
                repo
            ),
            PrpError::UnmergedPaths(repo) => write!(
                f,
                "\n❌ Repository: {}\n   Error: Unresolved merge conflicts\n   Suggested Fix: Resolve all conflicts then `git add`",
                repo
            ),
            PrpError::CommitFailed { repo, reason } => write!(
                f,
                "\n❌ Repository: {}\n   Error: Commit failed — {}\n   Suggested Fix: Check repository state with `git status`",
                repo, reason
            ),
            PrpError::PushFailed { repo, stderr } => write!(
                f,
                "\n❌ Repository: {}\n   Error: Push failed\n   Output: {}\n   Suggested Fix: Check remote connection or run `git pull` first",
                repo, stderr
            ),
            PrpError::RollbackFailed { repo, reason } => write!(
                f,
                "\n❌ Repository: {} — Rollback failed: {}",
                repo, reason
            ),
            PrpError::NoRepositoriesFound => write!(
                f,
                "❌ No git repositories found in the current directory."
            ),
        }
    }
}

impl std::error::Error for PrpError {}