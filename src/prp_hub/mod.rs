pub mod types;
pub mod errors;
pub mod discovery;
pub mod state;
pub mod commit;
pub mod push;
pub mod rollback;
pub mod group;

use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};

use crate::prp_hub::{
    commit::commit_all,
    discovery::discover_repositories,
    errors::PrpError,
    group::list_groups,
    push::push_all,
    rollback::rollback_all,
    state::validate_repo,
    types::CommitSession,
};

// NOTE: Add the following to Cargo.toml before building:
//   uuid = { version = "1", features = ["v4"] }
//   walkdir = "2"
use uuid::Uuid;

/// Entry point for `gitlink prp start`
pub fn run_prp_start() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{}", "=".repeat(80));
    println!("🔗 GitLink PRP Hub — Poly-Repo Commit Session");
    println!("{}", "=".repeat(80));

    // ──────────────────────────────────────────────────────
    // 1. Discovery
    // ──────────────────────────────────────────────────────
    println!("\n🔍 Scanning for git repositories...");

    let repos = match discover_repositories(".") {
        Ok(r) => r,
        Err(PrpError::NoRepositoriesFound) => {
            println!("\n❌ No git repositories found in the current directory.");
            return Ok(());
        }
        Err(e) => return Err(Box::new(e)),
    };

    if repos.len() == 1 {
        println!(
            "\n⚠️  Only one repository found ({}). PRP Hub is most useful with multiple repositories.",
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

    println!("\n📂 Discovered {} repository/repositories:\n", repos.len());
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
    // 3. Prompt for commit message
    // ──────────────────────────────────────────────────────
    let commit_message: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Commit message")
        .interact_text()?;

    if commit_message.trim().is_empty() {
        println!("❌ Commit message cannot be empty. Aborting.");
        return Ok(());
    }

    // ──────────────────────────────────────────────────────
    // 4. Generate session group ID
    // ──────────────────────────────────────────────────────
    let group_id = format!("gitlink-{}", Uuid::new_v4());

    println!("\n🆔 Session Group-ID: {}", group_id);

    let mut session = CommitSession::new(group_id.clone(), repos.clone());

    // ──────────────────────────────────────────────────────
    // 5. Commit phase
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
    // 6. Push prompt
    // ──────────────────────────────────────────────────────
    let push_options = vec!["No", "Yes"];
    let push_choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Do you want to push all repositories to remote?")
        .items(&push_options)
        .default(0) // Default: No
        .interact()?;

    if push_choice == 1 {
        // ──────────────────────────────────────────────────
        // 7. Push phase (sequential)
        // ──────────────────────────────────────────────────
        println!("\n🚀 Pushing repositories...\n");

        // Only push repos that were actually committed
        let committed_repos: Vec<_> = repos
            .iter()
            .filter(|r| {
                session
                    .committed
                    .iter()
                    .any(|c| c.path == r.path)
            })
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