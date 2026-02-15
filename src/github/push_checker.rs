use git2::{Repository, StatusOptions};
use serde::{Deserialize, Serialize};
use std::error::Error;

/// Push status information for a branch
#[derive(Debug, Serialize, Deserialize)]
pub struct PushStatus {
    pub can_push: bool,
    pub is_synced: bool,
    pub has_uncommitted_changes: bool,
    pub has_unpushed_commits: bool,
    pub has_conflicts: bool,
    pub remote_ahead: bool,
    pub local_commit: String,
    pub remote_commit: String,
    pub message: String,
}

impl PushStatus {
    fn new() -> Self {
        Self {
            can_push: false,
            is_synced: false,
            has_uncommitted_changes: false,
            has_unpushed_commits: false,
            has_conflicts: false,
            remote_ahead: false,
            local_commit: String::new(),
            remote_commit: String::new(),
            message: String::new(),
        }
    }
}

/// Check push status using local git repository
pub fn check_push_status(branch: &str) -> Result<PushStatus, Box<dyn Error>> {
    let repo = Repository::discover(".")?;
    let mut status = PushStatus::new();

    // ----------------------------------
    // Check uncommitted changes
    // ----------------------------------
    let mut status_opts = StatusOptions::new();
    status_opts.include_untracked(true);

    let statuses = repo.statuses(Some(&mut status_opts))?;
    if !statuses.is_empty() {
        status.has_uncommitted_changes = true;
        status.can_push = false;
        status.message = "Uncommitted changes present".to_string();
    }

    // ----------------------------------
    // Get local HEAD commit
    // ----------------------------------
    let head = repo.head()?;
    let local_oid = head.target().ok_or("No local HEAD")?;
    status.local_commit = local_oid.to_string();

    // ----------------------------------
    // Get remote tracking branch
    // ----------------------------------
    let remote_ref_name = format!("refs/remotes/origin/{}", branch);

    let remote_ref = match repo.find_reference(&remote_ref_name) {
        Ok(r) => r,
        Err(_) => {
            status.message = "Remote tracking branch not found".to_string();
            return Ok(status);
        }
    };

    let remote_oid = remote_ref.target().ok_or("Invalid remote reference")?;
    status.remote_commit = remote_oid.to_string();

    // ----------------------------------
    // Compare commits
    // ----------------------------------
    if local_oid == remote_oid {
        status.is_synced = true;
        status.can_push = true;
        status.message = "Branch is in sync with remote".to_string();
    } else {
        // Determine ahead/behind using merge base
        let base = repo.merge_base(local_oid, remote_oid)?;

        if base == remote_oid {
            // Local ahead
            status.has_unpushed_commits = true;
            status.can_push = true;
            status.is_synced = false;
            status.message = "Local branch is ahead of remote".to_string();
        } else if base == local_oid {
            // Remote ahead
            status.remote_ahead = true;
            status.can_push = false;
            status.is_synced = false;
            status.message = "Remote branch is ahead — pull required".to_string();
        } else {
            // Diverged
            status.has_conflicts = true;
            status.can_push = false;
            status.is_synced = false;
            status.message = "Branch has diverged — merge/rebase required".to_string();
        }
    }

    Ok(status)
}

/// Display push status in a user-friendly format
pub fn display_push_status(status: &PushStatus) {
    println!("\n{}", "=".repeat(80));
    println!("🔄 Push Status");
    println!("{}", "=".repeat(80));

    if status.is_synced {
        println!("✅ {}", status.message);
    } else {
        println!("⚠️  {}", status.message);
    }

    if !status.local_commit.is_empty() {
        println!("📌 Local commit: {}", &status.local_commit[..8]);
    }

    if !status.remote_commit.is_empty() {
        println!("🌐 Remote commit: {}", &status.remote_commit[..8]);
    }

    println!("\n📊 Status Details:");
    println!("  Can push: {}", if status.can_push { "✅ Yes" } else { "❌ No" });
    println!("  In sync: {}", if status.is_synced { "✅ Yes" } else { "⚠️  No" });
    println!("  Uncommitted changes: {}", if status.has_uncommitted_changes { "⚠️  Yes" } else { "✅ No" });
    println!("  Unpushed commits: {}", if status.has_unpushed_commits { "⚠️  Yes" } else { "✅ No" });
    println!("  Remote ahead: {}", if status.remote_ahead { "⚠️  Yes" } else { "✅ No" });
    println!("  Conflicts: {}", if status.has_conflicts { "❌ Yes" } else { "✅ No" });

    println!("{}", "=".repeat(80));
}
