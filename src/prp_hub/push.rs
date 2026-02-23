use std::process::Command;

use crate::prp_hub::errors::PrpError;
use crate::prp_hub::types::RepositoryInfo;

/// Push a single repository using a shell `git push`.
/// Shell is used intentionally for better SSH/credential-helper compatibility.
pub fn push_repo(info: &RepositoryInfo) -> Result<(), PrpError> {
    let output = Command::new("git")
        .arg("push")
        .current_dir(&info.path)
        .output()
        .map_err(|e| PrpError::PushFailed {
            repo: info.name.clone(),
            stderr: e.to_string(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(PrpError::PushFailed {
            repo: info.name.clone(),
            stderr,
        });
    }

    Ok(())
}

/// Push all repositories sequentially. Stops on first failure.
/// Returns the name of the repo that failed, if any.
pub fn push_all(repos: &[RepositoryInfo]) -> Result<(), PrpError> {
    for info in repos {
        print!("  🚀 {} ... ", info.name);
        push_repo(info)?;
        println!("✅ pushed");
    }
    Ok(())
}