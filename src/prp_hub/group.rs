use std::collections::HashMap;
use std::process::Command;

use crate::prp_hub::discovery::discover_repositories;

#[derive(Debug)]
pub struct CommitInfo {
    pub repo_name: String,
    pub short_sha: String,
    pub first_line: String,
}

/// Parse `git log` output for a repo and extract commits bearing a Group-ID trailer.
/// Returns a map of group_id -> Vec<CommitInfo>.
fn extract_groups_from_repo(
    repo_name: &str,
    repo_path: &std::path::Path,
) -> HashMap<String, Vec<CommitInfo>> {
    let mut map: HashMap<String, Vec<CommitInfo>> = HashMap::new();

    // git log --format="%H%n%B%n---COMMIT_END---"
    let output = Command::new("git")
        .args(["log", "--format=%H%n%B%n---COMMIT_END---"])
        .current_dir(repo_path)
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return map,
    };

    let raw = String::from_utf8_lossy(&output.stdout);

    for block in raw.split("---COMMIT_END---") {
        let block = block.trim();
        if block.is_empty() {
            continue;
        }

        let mut lines = block.lines();

        // First line is the full SHA
        let sha = match lines.next() {
            Some(s) => s.trim().to_string(),
            None => continue,
        };

        if sha.len() < 8 {
            continue;
        }

        let rest: Vec<&str> = lines.collect();
        let body = rest.join("\n");

        // Find Group-ID trailer
        let group_id = body.lines().find_map(|l| {
            let l = l.trim();
            if l.starts_with("Group-ID:") {
                Some(l.trim_start_matches("Group-ID:").trim().to_string())
            } else {
                None
            }
        });

        let group_id = match group_id {
            Some(g) => g,
            None => continue,
        };

        // First non-empty body line is the commit subject
        let first_line = body
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("(no message)")
            .to_string();

        let short_sha = sha[..8].to_string();

        map.entry(group_id).or_default().push(CommitInfo {
            repo_name: repo_name.to_string(),
            short_sha,
            first_line,
        });
    }

    map
}

/// Scan all repos from CWD and print a grouped view of commits sharing Group-IDs.
pub fn list_groups() -> Result<(), Box<dyn std::error::Error>> {
    let repos = discover_repositories(".")?;

    println!("\n🔗 Scanning {} repositories for linked commit groups...\n", repos.len());

    // Merge all per-repo maps
    let mut global: HashMap<String, Vec<CommitInfo>> = HashMap::new();

    for repo in &repos {
        let local = extract_groups_from_repo(&repo.name, &repo.path);
        for (gid, commits) in local {
            global.entry(gid).or_default().extend(commits);
        }
    }

    // Only show groups that appear in more than one repo (true poly-repo groups)
    // or all groups — show all, but flag single-repo ones
    if global.is_empty() {
        println!("ℹ️  No commits with Group-ID trailers found.");
        return Ok(());
    }

    // Sort group IDs for deterministic output
    let mut group_ids: Vec<String> = global.keys().cloned().collect();
    group_ids.sort();

    for gid in &group_ids {
        let commits = &global[gid];
        let repo_count = {
            let mut names: Vec<&str> = commits.iter().map(|c| c.repo_name.as_str()).collect();
            names.dedup();
            names.len()
        };

        println!("{}", "=".repeat(80));
        println!("📦 Group-ID: {}", gid);
        println!("   Spans {} repo(s)", repo_count);
        println!("{}", "-".repeat(80));

        for c in commits {
            println!(
                "   [{repo}]  {sha}  {msg}",
                repo = c.repo_name,
                sha = c.short_sha,
                msg = c.first_line
            );
        }

        println!();
    }

    println!("{}", "=".repeat(80));
    println!("Total groups found: {}", group_ids.len());

    Ok(())
}