use std::collections::HashMap;
use std::process::Command;

use dialoguer::{theme::ColorfulTheme, Select};

use crate::prp_hub::config::{exclude_repo, include_repo, is_excluded, load_config, save_config};
use crate::prp_hub::discovery::discover_repositories;

#[derive(Debug)]
pub struct CommitInfo {
    pub repo_name: String,
    pub short_sha: String,
    pub first_line: String,
}

fn extract_groups_from_repo(
    repo_name: &str,
    repo_path: &std::path::Path,
) -> HashMap<String, Vec<CommitInfo>> {
    let mut map: HashMap<String, Vec<CommitInfo>> = HashMap::new();

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

        let sha = match lines.next() {
            Some(s) => s.trim().to_string(),
            None => continue,
        };

        if sha.len() < 8 {
            continue;
        }

        let rest: Vec<&str> = lines.collect();
        let body = rest.join("\n");

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

/// `gitlink prp list` — shows commit groups and lets user manage repo inclusion.
pub fn list_groups() -> Result<(), Box<dyn std::error::Error>> {
    let all_repos = discover_repositories(".")?;
    let mut config = load_config();

    loop {
        println!("\n{}", "=".repeat(80));
        println!("🔗 GitLink PRP Hub — Repository Sync Membership");
        println!("{}", "=".repeat(80));

        let active: Vec<_> = all_repos
            .iter()
            .filter(|r| !is_excluded(&config, &r.path))
            .collect();

        let excluded: Vec<_> = all_repos
            .iter()
            .filter(|r| is_excluded(&config, &r.path))
            .collect();

        println!("\n✅ Active repos ({}):", active.len());
        for r in &active {
            println!("   • {} ({})", r.name, r.path.display());
        }

        if !excluded.is_empty() {
            println!("\n🚫 Excluded repos ({}):", excluded.len());
            for r in &excluded {
                println!("   • {} ({})", r.name, r.path.display());
            }
        }

        let mut menu_items: Vec<String> = Vec::new();
        menu_items.push("📋 Show commit groups".to_string());

        for r in &active {
            menu_items.push(format!("🚫 Exclude '{}' from sync", r.name));
        }
        for r in &excluded {
            menu_items.push(format!("✅ Re-add '{}' to sync", r.name));
        }
        menu_items.push("← Quit".to_string());

        let choice = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Choose an action")
            .items(&menu_items)
            .default(0)
            .interact()?;

        if choice == 0 {
            show_commit_groups(&active)?;
            continue;
        }

        let back_index = menu_items.len() - 1;
        if choice == back_index {
            break;
        }

        if choice >= 1 && choice <= active.len() {
            let repo = active[choice - 1];
            exclude_repo(&mut config, repo.path.clone());
            save_config(&config);
            println!("\n🚫 '{}' excluded from PRP sync.", repo.name);
            continue;
        }

        let readd_start = active.len() + 1;
        if choice >= readd_start && choice < back_index {
            let repo = excluded[choice - readd_start];
            include_repo(&mut config, &repo.path);
            save_config(&config);
            println!("\n✅ '{}' re-added to PRP sync.", repo.name);
        }
    }

    Ok(())
}

fn show_commit_groups(
    repos: &[&crate::prp_hub::types::RepositoryInfo],
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔍 Scanning {} active repositories for linked commit groups...\n", repos.len());

    let mut global: HashMap<String, Vec<CommitInfo>> = HashMap::new();

    for repo in repos {
        let local = extract_groups_from_repo(&repo.name, &repo.path);
        for (gid, commits) in local {
            global.entry(gid).or_default().extend(commits);
        }
    }

    if global.is_empty() {
        println!("ℹ️  No commits with Group-ID trailers found.");
        return Ok(());
    }

    let mut group_ids: Vec<String> = global.keys().cloned().collect();
    group_ids.sort();

    for gid in &group_ids {
        let commits = &global[gid];
        let mut seen_repos: Vec<&str> = commits.iter().map(|c| c.repo_name.as_str()).collect();
        seen_repos.dedup();
        let repo_count = seen_repos.len();

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