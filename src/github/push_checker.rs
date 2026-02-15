use git2::{Repository, StatusOptions};
use serde::{Deserialize, Serialize};
use std::error::Error;


///Push preview

#[derive(Debug)]
pub struct PushPreview {
    pub branch: String,
    pub commits: Vec<PreviewCommit>,
    pub total_files: usize,
    pub total_insertions: usize,
    pub total_deletions: usize,
}

#[derive(Debug)]
pub struct PreviewCommit {
    pub short_id: String,
    pub message: String,
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
}


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

//push preview func

pub fn generate_push_preview(branch: &str) -> Result<Option<PushPreview>, Box<dyn Error>> {
    let repo = Repository::discover(".")?;

    let head = repo.head()?;
    let local_oid = head.target().ok_or("No local HEAD")?;

    let remote_ref_name = format!("refs/remotes/origin/{}", branch);
    let remote_ref = match repo.find_reference(&remote_ref_name) {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };

    let remote_oid = remote_ref.target().ok_or("Invalid remote reference")?;

    if local_oid == remote_oid {
        return Ok(None); // Nothing to push
    }

    let mut revwalk = repo.revwalk()?;
    revwalk.push(local_oid)?;
    revwalk.hide(remote_oid)?;

    let mut commits = Vec::new();
    let mut total_files = 0;
    let mut total_insertions = 0;
    let mut total_deletions = 0;

    for oid_result in revwalk {
        let oid = oid_result?;
        let commit = repo.find_commit(oid)?;

        let short_id = commit.id().to_string()[..8].to_string();
        let message = commit.summary().unwrap_or("No message").to_string();

        let tree = commit.tree()?;

        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };

        let diff = repo.diff_tree_to_tree(
            parent_tree.as_ref(),
            Some(&tree),
            None,
        )?;

        let stats = diff.stats()?;

        let files_changed = stats.files_changed();
        let insertions = stats.insertions();
        let deletions = stats.deletions();

        total_files += files_changed;
        total_insertions += insertions;
        total_deletions += deletions;

        commits.push(PreviewCommit {
            short_id,
            message,
            files_changed,
            insertions,
            deletions,
        });
    }

    commits.reverse(); // Oldest first (cleaner output)

    Ok(Some(PushPreview {
        branch: branch.to_string(),
        commits,
        total_files,
        total_insertions,
        total_deletions,
    }))
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

pub fn display_push_preview(preview: &PushPreview) {
    println!("\n{}", "=".repeat(80));
    println!("🚀 Push Preview");
    println!("{}", "=".repeat(80));

    println!("Branch: {}", preview.branch);
    println!("Unpushed commits: {}\n", preview.commits.len());

    for commit in &preview.commits {
        println!(
            "{}  {}",
            commit.short_id,
            commit.message
        );

        println!(
            "   Files: {}  +{}  -{}",
            commit.files_changed,
            commit.insertions,
            commit.deletions
        );
        println!();
    }

    println!("{}", "-".repeat(80));
    println!("TOTAL");
    println!(
        "Files: {}  Insertions: +{}  Deletions: -{}",
        preview.total_files,
        preview.total_insertions,
        preview.total_deletions
    );

    println!("{}", "=".repeat(80));
}
