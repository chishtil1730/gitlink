use git2::Repository;

use crate::prp_hub::errors::PrpError;
use crate::prp_hub::types::RepoCommitResult;

/// Roll back a single commit using `git reset --soft HEAD~1`.
/// Only touches repos that were committed during this session.
fn rollback_one(result: &RepoCommitResult) -> Result<(), PrpError> {
    let repo = Repository::open(&result.path).map_err(|e| PrpError::RollbackFailed {
        repo: result.path.display().to_string(),
        reason: e.to_string(),
    })?;

    // Find HEAD~1
    let head = repo.head().map_err(|e| PrpError::RollbackFailed {
        repo: result.path.display().to_string(),
        reason: format!("Cannot read HEAD: {}", e),
    })?;

    let head_commit = head.peel_to_commit().map_err(|e| PrpError::RollbackFailed {
        repo: result.path.display().to_string(),
        reason: format!("Cannot peel HEAD to commit: {}", e),
    })?;

    // Verify that HEAD still matches what we committed (safety check)
    if head_commit.id() != result.commit_oid {
        return Err(PrpError::RollbackFailed {
            repo: result.path.display().to_string(),
            reason: format!(
                "HEAD ({}) does not match session commit ({}); skipping rollback for safety",
                &head_commit.id().to_string()[..8],
                &result.commit_oid.to_string()[..8]
            ),
        });
    }

    let parent = head_commit
        .parent(0)
        .map_err(|e| PrpError::RollbackFailed {
            repo: result.path.display().to_string(),
            reason: format!("No parent commit found (initial commit?): {}", e),
        })?;

    // Soft reset to parent
    let obj = repo
        .find_object(parent.id(), None)
        .map_err(|e| PrpError::RollbackFailed {
            repo: result.path.display().to_string(),
            reason: e.to_string(),
        })?;

    repo.reset(&obj, git2::ResetType::Soft, None)
        .map_err(|e| PrpError::RollbackFailed {
            repo: result.path.display().to_string(),
            reason: format!("Reset failed: {}", e),
        })?;

    Ok(())
}

/// Attempt to roll back all commits recorded in this session.
/// Prints status for each repo. Errors are reported but do not stop the loop.
pub fn rollback_all(committed: &[RepoCommitResult]) {
    if committed.is_empty() {
        println!("  ℹ️  Nothing to roll back.");
        return;
    }

    for result in committed.iter().rev() {
        let label = result.path.display().to_string();
        print!("  ↩️  {} ... ", label);

        match rollback_one(result) {
            Ok(()) => println!("✅ rolled back"),
            Err(e) => println!("❌ {}", e),
        }
    }
}