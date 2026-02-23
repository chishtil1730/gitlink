use git2::{Repository, Signature, StatusOptions};

use crate::prp_hub::errors::PrpError;
use crate::prp_hub::types::{CommitSession, RepoCommitResult, RepositoryInfo};

/// Stage all changes and commit in a single repository.
/// Returns None if there is nothing to commit (clean working tree).
pub fn commit_repo(
    info: &RepositoryInfo,
    message: &str,
    group_id: &str,
) -> Result<Option<RepoCommitResult>, PrpError> {
    let repo = Repository::open(&info.path).map_err(|e| PrpError::CommitFailed {
        repo: info.name.clone(),
        reason: e.to_string(),
    })?;

    // Check if there is anything to commit
    let mut status_opts = StatusOptions::new();
    status_opts.include_untracked(true);

    let statuses = repo.statuses(Some(&mut status_opts)).map_err(|e| PrpError::CommitFailed {
        repo: info.name.clone(),
        reason: e.to_string(),
    })?;

    // Skip repos with nothing to commit
    let has_changes = statuses.iter().any(|s| {
        let st = s.status();
        st != git2::Status::CURRENT && !st.contains(git2::Status::IGNORED)
    });

    if !has_changes {
        return Ok(None);
    }

    // Stage all changes (git add .)
    let mut index = repo.index().map_err(|e| PrpError::CommitFailed {
        repo: info.name.clone(),
        reason: format!("Cannot open index: {}", e),
    })?;

    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .map_err(|e| PrpError::CommitFailed {
            repo: info.name.clone(),
            reason: format!("git add failed: {}", e),
        })?;

    index.write().map_err(|e| PrpError::CommitFailed {
        repo: info.name.clone(),
        reason: format!("Cannot write index: {}", e),
    })?;

    let tree_oid = index.write_tree().map_err(|e| PrpError::CommitFailed {
        repo: info.name.clone(),
        reason: format!("Cannot write tree: {}", e),
    })?;

    let tree = repo.find_tree(tree_oid).map_err(|e| PrpError::CommitFailed {
        repo: info.name.clone(),
        reason: format!("Cannot find tree: {}", e),
    })?;

    // Build commit message with Group-ID trailer
    let full_message = format!("{}\n\nGroup-ID: {}", message, group_id);

    // Get signature from repo config (falls back to a placeholder)
    let sig = repo.signature().map_err(|e| PrpError::CommitFailed {
        repo: info.name.clone(),
        reason: format!("Cannot read git config signature: {}", e),
    })?;

    // Get parent commit (HEAD)
    let parent_commit = repo.head().ok().and_then(|h| h.peel_to_commit().ok());

    let parents: Vec<&git2::Commit> = parent_commit.as_ref().map(|c| vec![c]).unwrap_or_default();

    let commit_oid = repo
        .commit(Some("HEAD"), &sig, &sig, &full_message, &tree, &parents)
        .map_err(|e| PrpError::CommitFailed {
            repo: info.name.clone(),
            reason: e.to_string(),
        })?;

    Ok(Some(RepoCommitResult {
        path: info.path.clone(),
        commit_oid,
    }))
}

/// Commit across all repositories in a session.
/// On any failure, stops immediately and returns the error
/// (caller is responsible for rollback of `session.committed`).
pub fn commit_all(
    session: &mut CommitSession,
    message: &str,
) -> Result<(), PrpError> {
    for info in session.repositories.clone().iter() {
        print!("  📝 {} ... ", info.name);

        match commit_repo(info, message, &session.group_id)? {
            Some(result) => {
                println!("✅ committed {}", &result.commit_oid.to_string()[..8]);
                session.committed.push(result);
            }
            None => {
                println!("⏭  nothing to commit, skipped");
            }
        }
    }
    Ok(())
}