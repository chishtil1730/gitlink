use git2::{Repository, RepositoryState};

use crate::prp_hub::errors::PrpError;
use crate::prp_hub::types::RepositoryInfo;

/// Validate repository state before committing.
/// Returns Ok(()) if repo is ready, or a descriptive PrpError otherwise.
pub fn validate_repo(info: &RepositoryInfo) -> Result<(), PrpError> {
    let repo = Repository::open(&info.path).map_err(|e| PrpError::InvalidState {
        repo: info.name.clone(),
        reason: format!("Cannot open repository: {}", e),
        fix: "Ensure the path is a valid git repository".to_string(),
    })?;

    // 1. HEAD must not be detached
    if repo.head_detached().unwrap_or(true) {
        return Err(PrpError::DetachedHead(info.name.clone()));
    }

    // 2. Repo state must be clean (no merge/rebase/cherry-pick in progress)
    match repo.state() {
        RepositoryState::Clean => {}
        RepositoryState::Merge => return Err(PrpError::MergeConflict(info.name.clone())),
        RepositoryState::Rebase
        | RepositoryState::RebaseInteractive
        | RepositoryState::RebaseMerge => {
            return Err(PrpError::InvalidState {
                repo: info.name.clone(),
                reason: "Rebase in progress".to_string(),
                fix: "Complete or abort with `git rebase --abort`".to_string(),
            })
        }
        RepositoryState::CherryPick | RepositoryState::CherryPickSequence => {
            return Err(PrpError::InvalidState {
                repo: info.name.clone(),
                reason: "Cherry-pick in progress".to_string(),
                fix: "Complete or abort with `git cherry-pick --abort`".to_string(),
            })
        }
        other => {
            return Err(PrpError::InvalidState {
                repo: info.name.clone(),
                reason: format!("Repository in unexpected state: {:?}", other),
                fix: "Run `git status` to inspect the repository".to_string(),
            })
        }
    }

    // 3. No unresolved conflicts
    let statuses = repo.statuses(None).map_err(|e| PrpError::InvalidState {
        repo: info.name.clone(),
        reason: format!("Cannot read status: {}", e),
        fix: "Run `git status` to inspect".to_string(),
    })?;

    let has_conflicts = statuses
        .iter()
        .any(|s| s.status().contains(git2::Status::CONFLICTED));

    if has_conflicts {
        return Err(PrpError::UnmergedPaths(info.name.clone()));
    }

    // 4. Upstream must exist for current branch
    let head = repo.head().map_err(|e| PrpError::InvalidState {
        repo: info.name.clone(),
        reason: format!("Cannot read HEAD: {}", e),
        fix: "Ensure you are on a branch".to_string(),
    })?;

    let branch_name = head.shorthand().unwrap_or("").to_string();
    let remote_ref = format!("refs/remotes/origin/{}", branch_name);

    if repo.find_reference(&remote_ref).is_err() {
        return Err(PrpError::InvalidState {
            repo: info.name.clone(),
            reason: format!(
                "No upstream tracking branch found for '{}'",
                branch_name
            ),
            fix: format!(
                "Run `git push --set-upstream origin {}` first",
                branch_name
            ),
        });
    }

    Ok(())
}