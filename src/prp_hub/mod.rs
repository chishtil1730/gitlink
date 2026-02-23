pub mod types;
pub mod errors;
pub mod discovery;
pub mod state;
pub mod commit;
pub mod push;
pub mod rollback;
pub mod group;
pub mod status;
pub mod config;

use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};

use crate::prp_hub::{
    commit::commit_all,
    config::{is_excluded, load_config},
    discovery::discover_repositories,
    errors::PrpError,
    group::list_groups,
    push::push_all,
    rollback::rollback_all,
    state::validate_repo,
    status::{collect_status, display_repo_status},
    types::CommitSession,
};

// NOTE: Cargo.toml dependencies required:
//   uuid = { version = "1", features = ["v4"] }
//   walkdir = "2"
//   serde_json = "1"
use uuid::Uuid;

/// Entry point for `gitlink prp start`
pub fn run_prp_start() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{}", "=".repeat(80));
    println!("🔗 GitLink PRP Hub — Poly-Repo Commit Session");
    println!("{}", "=".repeat(80));

    // ──────────────────────────────────────────────────────
    // 1. Discovery — filter out excluded repos
    // ──────────────────────────────────────────────────────
    println!("\n🔍 Scanning for git repositories...");

    let all_repos = match discover_repositories(".") {
        Ok(r) => r,
        Err(PrpError::NoRepositoriesFound) => {
            println!("\n❌ No git repositories found in the current directory.");
            return Ok(());
        }
        Err(e) => return Err(Box::new(e)),
    };

    let config = load_config();
    let repos: Vec<_> = all_repos
        .into_iter()
        .filter(|r| !is_excluded(&config, &r.path))
        .collect();

    if repos.is_empty() {
        println!("\n❌ No active repositories (all are excluded). Use `gitlink prp list` to re-add repos.");
        return Ok(());
    }

    if repos.len() == 1 {
        println!(
            "\n⚠️  Only one active repository ({}). PRP Hub is most useful with multiple repositories.",
            repos[0].name
        );
        let proceed = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Continue anyway?")
            .default(false)
            .interact()?;
        if !proceed {
            return Ok(());
        }
    }

    println!("\n📂 {} active repository/repositories:\n", repos.len());
    for (i, r) in repos.iter().enumerate() {
        println!("  {}. {} ({})", i + 1, r.name, r.path.display());
    }

    // ──────────────────────────────────────────────────────
    // 2. State validation (fail fast)
    // ──────────────────────────────────────────────────────
    println!("\n🔎 Validating repository states...");

    let mut validation_errors = Vec::new();
    for repo in &repos {
        if let Err(e) = validate_repo(repo) {
            validation_errors.push(e);
        }
    }

    if !validation_errors.is_empty() {
        println!("\n❌ One or more repositories failed validation:\n");
        for e in &validation_errors {
            println!("{}\n", e);
        }
        println!("Aborting. Fix the issues above and try again.");
        return Ok(());
    }

    println!("✅ All repositories are in a valid state.");

    // ──────────────────────────────────────────────────────
    // 3. Collect and display working tree status
    // ──────────────────────────────────────────────────────
    println!("\n📊 Working tree status:\n");

    let mut any_changes = false;
    let mut repo_statuses = Vec::new();

    for repo in &repos {
        let st = collect_status(repo);
        if !st.is_empty() {
            display_repo_status(repo, &st);
            any_changes = true;
        } else {
            println!("\n  📁 {} — nothing to commit", repo.name);
        }
        repo_statuses.push((repo, st));
    }

    if !any_changes {
        println!("\nℹ️  Nothing to commit in any repository.");
        return Ok(());
    }

    // ──────────────────────────────────────────────────────
    // 4. Confirmation gate
    // ──────────────────────────────────────────────────────
    println!();
    let confirmed = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Do you want to commit these changes?")
        .default(false)
        .interact()?;

    if !confirmed {
        println!("\nℹ️  Aborted. No commits were made.");
        return Ok(());
    }

    // ──────────────────────────────────────────────────────
    // 5. Prompt for commit message
    // ──────────────────────────────────────────────────────
    let commit_message: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Commit message")
        .interact_text()?;

    if commit_message.trim().is_empty() {
        println!("❌ Commit message cannot be empty. Aborting.");
        return Ok(());
    }

    // ──────────────────────────────────────────────────────
    // 6. Generate session group ID
    // ──────────────────────────────────────────────────────
    let group_id = format!("gitlink-{}", Uuid::new_v4());
    println!("\n🆔 Session Group-ID: {}", group_id);

    let mut session = CommitSession::new(group_id.clone(), repos.clone());

    // ──────────────────────────────────────────────────────
    // 7. Commit phase
    // ──────────────────────────────────────────────────────
    println!("\n📝 Committing in all repositories...\n");

    if let Err(e) = commit_all(&mut session, commit_message.trim()) {
        println!("\n❌ Commit failed: {}", e);
        println!("\n🔄 Rolling back committed repositories...\n");
        rollback_all(&session.committed);
        println!("\n✅ Rollback complete. No partial commits remain.");
        return Ok(());
    }

    if session.committed.is_empty() {
        println!("\nℹ️  Nothing to commit in any repository.");
        return Ok(());
    }

    println!(
        "\n✅ Successfully committed in {} repository/repositories.",
        session.committed.len()
    );

    // ──────────────────────────────────────────────────────
    // 8. Push prompt
    // ──────────────────────────────────────────────────────
    let push_options = vec!["No", "Yes"];
    let push_choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Do you want to push all repositories to remote?")
        .items(&push_options)
        .default(0)
        .interact()?;

    if push_choice == 1 {
        println!("\n🚀 Pushing repositories...\n");

        let committed_repos: Vec<_> = repos
            .iter()
            .filter(|r| session.committed.iter().any(|c| c.path == r.path))
            .cloned()
            .collect();

        if let Err(e) = push_all(&committed_repos) {
            println!("\n❌ Push failed: {}", e);
            println!("\n🔄 Rolling back all committed repositories...\n");
            rollback_all(&session.committed);
            println!("\n✅ Rollback complete.");
            return Ok(());
        }

        println!("\n✅ All repositories pushed successfully.");
    } else {
        println!("\nℹ️  Push skipped. Your commits are local.");
    }

    println!("\n{}", "=".repeat(80));
    println!("🎉 PRP Session complete. Group-ID: {}", group_id);
    println!("{}", "=".repeat(80));

    Ok(())
}

/// Entry point for `gitlink prp list`
pub fn run_prp_list() -> Result<(), Box<dyn std::error::Error>> {
    list_groups()
}