mod auth;
mod github;

use auth::oauth;
use dialoguer::{theme::ColorfulTheme, Select};
use github::actions_client::{ActionsClient, display_workflow_runs};
use github::client::GitHubClient;
use github::graphql::{self, GraphQLClient};
use github::push_checker::display_push_status;
use github::repo_selector::RepoSelector;
use github::sync_checker::SyncChecker;
use serde::Deserialize;
use std::error::Error;
use std::io::{self, Write};

#[derive(Debug, Deserialize)]
struct GitHubUser {
    login: String,
    name: Option<String>,
    public_repos: u32,
}

#[derive(Debug, Deserialize)]
struct Repo {
    name: String,
    private: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Handle logout command
    let args: Vec<String> = std::env::args().collect();

    if args.len() >= 3 && args[1] == "auth" && args[2] == "logout" {
        auth::token_store::delete_token()?;
        println!("✅ Logged out from GitHub");
        return Ok(());
    }

    // OAuth login with persistent token storage
    use auth::token_store;

    let token = match token_store::load_token() {
        Ok(token) => {
            println!("🔑 Using stored GitHub token");
            token
        }
        Err(_) => {
            println!("🔐 No stored token found. Initiating OAuth flow...");
            let token = oauth::login().await?;
            token_store::save_token(&token)?;
            println!("✅ Token saved securely!");
            token
        }
    };

    // Create GitHub clients
    let gh_client = GitHubClient::new(token.clone());
    let graphql_client = GraphQLClient::new(token.clone());

    // Display main menu
    loop {
        let choice = display_menu()?;

        match choice {
            0 => show_user_activity(&graphql_client).await?,
            1 => show_recent_commits(&graphql_client).await?,
            2 => show_pull_requests(&graphql_client).await?,
            3 => select_and_check_repo(&graphql_client).await?,
            4 => check_multiple_repos(&graphql_client).await?,
            5 => check_push_status(&graphql_client).await?,
            6 => verify_push_possible(&graphql_client).await?,
            7 => show_branches(&graphql_client).await?,
            8 => show_issues_and_actions(&graphql_client, &token).await?,
            9 => show_basic_info(&gh_client).await?,
            10 => {
                println!("👋 Goodbye!");
                break;
            }
            _ => println!("❌ Invalid choice. Please try again."),
        }
    }

    Ok(())
}

fn display_menu() -> Result<usize, Box<dyn Error>> {
    println!("\n{}", "=".repeat(80));
    println!("🚀 GitLink - Your Terminal Git Companion");
    println!("{}", "=".repeat(80));

    let items = vec![
        "📊 Show User Activity & Contributions",
        "💾 Show Recent Commits",
        "🔀 Show Pull Requests",
        "🔍 Select Repository & Check Sync",
        "📦 Check Multiple Repositories Sync",
        "✅ Check if Latest Commit is Pushed to Remote",
        "🚀 Verify if Pushing is Possible",
        "🌿 Show Branches (Local & Remote)",
        "📝 Show Issues & GitHub Actions",
        "👤 Show Basic User Info (REST API)",
        "❌ Quit",
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose an option")
        .items(&items)
        .default(0)
        .interact()?;

    Ok(selection)
}

async fn show_user_activity(client: &GraphQLClient) -> Result<(), Box<dyn Error>> {
    println!("\n📊 Fetching your GitHub activity...");

    let activity = graphql::fetch_user_activity(client).await?;

    println!("\n{}", "=".repeat(80));
    println!("👤 User: {} ({})",
             activity.viewer.name.as_deref().unwrap_or("N/A"),
             activity.viewer.login
    );
    println!("{}", "=".repeat(80));

    let contrib = &activity.viewer.contributions_collection;
    println!("📈 Total Contributions: {}", contrib.contribution_calendar.total_contributions);
    println!("💾 Commits: {}", contrib.total_commit_contributions);
    println!("🔀 Pull Requests: {}", contrib.total_pull_request_contributions);
    println!("📝 Issues: {}", contrib.total_issue_contributions);
    println!("📦 Repositories Created: {}", contrib.total_repository_contributions);

    // Show last 7 days of activity
    println!("\n📅 Last 7 Days Activity:");
    if let Some(last_week) = contrib.contribution_calendar.weeks.last() {
        for day in &last_week.contribution_days {
            let bar = "█".repeat(day.contribution_count.min(20) as usize);
            println!("  {} : {} ({})", day.date, bar, day.contribution_count);
        }
    }

    println!("{}", "=".repeat(80));

    Ok(())
}

async fn show_recent_commits(client: &GraphQLClient) -> Result<(), Box<dyn Error>> {
    let options = vec![
        "Show 3 most recent commits (globally across all repos)",
        "Select a specific repository",
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose option")
        .items(&options)
        .default(0)
        .interact()?;

    match selection {
        0 => {
            // Show TRUE 3 most recent commits globally
            println!("\n💾 Fetching your 3 most recent commits globally...");
            let commits = graphql::fetch_recent_commits(client, 3).await?;

            println!("\n{}", "=".repeat(80));
            println!("3 Most Recent Commits Globally by {}", commits.viewer.login);
            println!("{}", "=".repeat(80));

            let mut all_commits = Vec::new();

            // Collect all commits with repo info
            for repo in &commits.viewer.repositories.nodes {
                if let Some(branch_ref) = &repo.default_branch_ref {
                    for commit in &branch_ref.target.history.nodes {
                        all_commits.push((repo, commit));
                    }
                }
            }

            // Sort by committed date (most recent first) - this gives us TRUE global order
            all_commits.sort_by(|a, b| b.1.committed_date.cmp(&a.1.committed_date));

            // Take only first 3 - these are the TRUE 3 latest commits globally
            for (repo, commit) in all_commits.iter().take(3) {
                println!("\n📦 Repository: {}", repo.name_with_owner);

                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&commit.committed_date) {
                    println!("📝 {}", dt.format("%Y-%m-%d %H:%M:%S"));
                }

                println!("🔑 {}", &commit.oid[..8]);

                let first_line = commit.message.lines().next().unwrap_or(&commit.message);
                println!("💬 {}", first_line);

                if let Some(author) = &commit.author.name {
                    println!("👤 {}", author);
                }

                println!("📊 +{} -{}", commit.additions, commit.deletions);
                println!("{}", "─".repeat(80));
            }

            println!();
        }
        1 => {
            // Select a specific repository
            let selector = RepoSelector::new(client).await?;

            if let Some(repo) = selector.select_repository()? {
                println!("\n💾 Fetching commits from {}...", repo.name_with_owner);

                let commit_data = graphql::fetch_single_repo_commits(
                    client,
                    &repo.owner.login,
                    &repo.name,
                    10
                ).await?;

                println!("\n{}", "=".repeat(80));
                println!("Recent Commits - {}", commit_data.repository.name_with_owner);
                println!("{}", "=".repeat(80));

                if let Some(branch_ref) = &commit_data.repository.default_branch_ref {
                    for commit in &branch_ref.target.history.nodes {
                        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&commit.committed_date) {
                            println!("\n📝 {}", dt.format("%Y-%m-%d %H:%M:%S"));
                        }

                        println!("🔑 {}", &commit.oid[..8]);

                        let first_line = commit.message.lines().next().unwrap_or(&commit.message);
                        println!("💬 {}", first_line);

                        if let Some(author) = &commit.author.name {
                            println!("👤 {}", author);
                        }

                        println!("📊 +{} -{}", commit.additions, commit.deletions);
                        println!("{}", "─".repeat(80));
                    }
                } else {
                    println!("\nNo default branch found for this repository.");
                }

                println!();
            }
        }
        _ => {}
    }

    Ok(())
}

async fn show_pull_requests(client: &GraphQLClient) -> Result<(), Box<dyn Error>> {
    println!("\n🔀 Fetching your pull requests...");

    let states = vec!["Open", "Closed", "Merged"];
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose PR state")
        .items(&states)
        .default(0)
        .interact()?;

    let state = match selection {
        0 => "OPEN",
        1 => "CLOSED",
        2 => "MERGED",
        _ => "OPEN",
    };

    let prs = graphql::fetch_pull_requests(client, state, 10).await?;

    println!("\n{}", "=".repeat(80));
    println!("Pull Requests ({}) - Total: {}", state, prs.viewer.pull_requests.total_count);
    println!("{}", "=".repeat(80));

    for pr in &prs.viewer.pull_requests.nodes {
        println!("\n🔀 #{} - {}", pr.number, pr.title);
        println!("   Repository: {}", pr.repository.name_with_owner);
        println!("   State: {} | Mergeable: {}", pr.state, pr.mergeable);

        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&pr.created_at) {
            println!("   Created: {}", dt.format("%Y-%m-%d"));
        }

        if let Some(reviews) = &pr.reviews {
            println!("   Reviews: {}", reviews.total_count);
            for review in &reviews.nodes {
                if let Some(author) = &review.author {
                    println!("     - {} by {}", review.state, author.login);
                }
            }
        }

        println!("{}", "─".repeat(80));
    }

    Ok(())
}

async fn select_and_check_repo(client: &GraphQLClient) -> Result<(), Box<dyn Error>> {
    let selector = RepoSelector::new(client).await?;

    if let Some(repo) = selector.select_repository()? {
        println!("\n✅ Selected: {}", repo.name_with_owner);

        let sync_checker = SyncChecker::new(GraphQLClient::new(
            auth::token_store::load_token()?
        ));

        sync_checker.display_sync_status(repo).await?;

        let options = vec!["Yes", "No"];
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Show detailed remote info?")
            .items(&options)
            .default(1)
            .interact()?;

        if selection == 0 {
            show_repo_details(client, repo).await?;
        }
    }

    Ok(())
}

async fn check_multiple_repos(client: &GraphQLClient) -> Result<(), Box<dyn Error>> {
    let selector = RepoSelector::new(client).await?;
    let repos = selector.select_multiple()?;

    if !repos.is_empty() {
        println!("\n✅ Selected {} repositories", repos.len());

        let sync_checker = SyncChecker::new(GraphQLClient::new(
            auth::token_store::load_token()?
        ));

        sync_checker.display_multi_sync_status(&repos).await?;
    }

    Ok(())
}

async fn check_push_status(client: &GraphQLClient) -> Result<(), Box<dyn Error>> {
    let selector = RepoSelector::new(client).await?;

    if let Some(repo) = selector.select_repository()? {
        println!("\n🔄 Checking push status for {}...", repo.name_with_owner);

        let branch = if let Some(branch_ref) = &repo.default_branch_ref {
            &branch_ref.name
        } else {
            println!("❌ No default branch found");
            return Ok(());
        };

        let status = client.check_push_status(
            &repo.owner.login,
            &repo.name,
            branch
        ).await?;

        display_push_status(&status);
    }

    Ok(())
}

async fn verify_push_possible(client: &GraphQLClient) -> Result<(), Box<dyn Error>> {
    let selector = RepoSelector::new(client).await?;

    if let Some(repo) = selector.select_repository()? {
        println!("\n🚀 Verifying push possibility for {}...", repo.name_with_owner);

        let branch = if let Some(branch_ref) = &repo.default_branch_ref {
            &branch_ref.name
        } else {
            println!("❌ No default branch found");
            return Ok(());
        };

        let status = client.verify_push_possible(
            &repo.owner.login,
            &repo.name,
            branch
        ).await?;

        display_push_status(&status);

        if status.can_push {
            println!("\n✅ You can safely push to this branch!");
        } else {
            println!("\n⚠️  Action required before pushing:");
            if status.remote_ahead {
                println!("   • Pull remote changes first");
            }
            if status.has_conflicts {
                println!("   • Resolve merge conflicts");
            }
        }
    }

    Ok(())
}

async fn show_branches(client: &GraphQLClient) -> Result<(), Box<dyn Error>> {
    let selector = RepoSelector::new(client).await?;

    if let Some(repo) = selector.select_repository()? {
        println!("\n🌿 Fetching branches for {}...", repo.name_with_owner);

        let branches = graphql::fetch_branches(
            client,
            &repo.owner.login,
            &repo.name
        ).await?;

        println!("\n{}", "=".repeat(80));
        println!("Branches - {} (Total: {})",
                 branches.repository.name_with_owner,
                 branches.repository.refs.total_count);
        println!("{}", "=".repeat(80));

        for branch in &branches.repository.refs.nodes {
            println!("\n🌿 {}", branch.name);
            println!("   Commit: {}", &branch.target.oid[..8]);

            if let Some(date) = &branch.target.committed_date {
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(date) {
                    println!("   Last commit: {}", dt.format("%Y-%m-%d %H:%M:%S"));
                }
            }

            if let Some(author) = &branch.target.author {
                if let Some(name) = &author.name {
                    println!("   Author: {}", name);
                }
            }

            println!("{}", "─".repeat(80));
        }

        println!();
    }

    Ok(())
}

async fn show_issues_and_actions(client: &GraphQLClient, token: &str) -> Result<(), Box<dyn Error>> {
    let options = vec![
        "Show Issues",
        "Show GitHub Actions Workflow Runs",
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose what to view")
        .items(&options)
        .default(0)
        .interact()?;

    match selection {
        0 => show_issues_menu(client).await?,
        1 => show_actions_menu(client, token).await?,
        _ => {}
    }

    Ok(())
}

async fn show_issues_menu(client: &GraphQLClient) -> Result<(), Box<dyn Error>> {
    let scope_options = vec![
        "All my issues across repos",
        "Issues in a specific repository",
    ];

    let scope = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose scope")
        .items(&scope_options)
        .default(0)
        .interact()?;

    let state_options = vec!["Open", "Closed", "Both"];
    let state_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose issue state")
        .items(&state_options)
        .default(0)
        .interact()?;

    let states = match state_selection {
        0 => vec!["OPEN"],
        1 => vec!["CLOSED"],
        2 => vec!["OPEN", "CLOSED"],
        _ => vec!["OPEN"],
    };

    match scope {
        0 => {
            println!("\n📝 Fetching your issues...");
            let issues = graphql::fetch_user_issues(client, &states, 20).await?;

            println!("\n{}", "=".repeat(80));
            println!("Issues - Total: {}", issues.viewer.issues.total_count);
            println!("{}", "=".repeat(80));

            for issue in &issues.viewer.issues.nodes {
                println!("\n📝 #{} - {}", issue.number, issue.title);
                println!("   State: {}", issue.state);

                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&issue.created_at) {
                    println!("   Created: {}", dt.format("%Y-%m-%d"));
                }

                if let Some(author) = &issue.author {
                    println!("   Author: {}", author.login);
                }

                println!("   🔗 {}", issue.url);
                println!("{}", "─".repeat(80));
            }
        }
        1 => {
            let selector = RepoSelector::new(client).await?;

            if let Some(repo) = selector.select_repository()? {
                println!("\n📝 Fetching issues from {}...", repo.name_with_owner);

                let issues = graphql::fetch_issues(
                    client,
                    &repo.owner.login,
                    &repo.name,
                    &states,
                    20
                ).await?;

                println!("\n{}", "=".repeat(80));
                println!("Issues - {} (Total: {})",
                         issues.repository.name_with_owner,
                         issues.repository.issues.total_count);
                println!("{}", "=".repeat(80));

                for issue in &issues.repository.issues.nodes {
                    println!("\n📝 #{} - {}", issue.number, issue.title);
                    println!("   State: {}", issue.state);

                    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&issue.created_at) {
                        println!("   Created: {}", dt.format("%Y-%m-%d"));
                    }

                    if let Some(author) = &issue.author {
                        println!("   Author: {}", author.login);
                    }

                    println!("   🔗 {}", issue.url);
                    println!("{}", "─".repeat(80));
                }
            }
        }
        _ => {}
    }

    Ok(())
}

async fn show_actions_menu(client: &GraphQLClient, token: &str) -> Result<(), Box<dyn Error>> {
    let scope_options = vec![
        "All repositories",
        "Specific repository",
    ];

    let scope = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose scope")
        .items(&scope_options)
        .default(0)
        .interact()?;

    let status_options = vec!["All statuses", "Completed", "In Progress", "Queued"];
    let status_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Filter by workflow status")
        .items(&status_options)
        .default(0)
        .interact()?;

    let status_filter = match status_selection {
        1 => Some("completed"),
        2 => Some("in_progress"),
        3 => Some("queued"),
        _ => None,
    };

    let actions_client = ActionsClient::new(token.to_string());

    match scope {
        0 => {
            println!("\n⚡ Fetching workflow runs from all repos...");

            // Fetch repos first
            let repos_response = graphql::fetch_repositories(client, 20, false).await?;
            let repo_tuples: Vec<(&str, &str)> = repos_response
                .viewer
                .repositories
                .nodes
                .iter()
                .map(|r| (r.owner.login.as_str(), r.name.as_str()))
                .collect();

            let runs = actions_client.fetch_all_workflow_runs(&repo_tuples, status_filter, 5).await?;
            display_workflow_runs(&runs, Some(15));
        }
        1 => {
            let selector = RepoSelector::new(client).await?;

            if let Some(repo) = selector.select_repository()? {
                println!("\n⚡ Fetching workflow runs for {}...", repo.name_with_owner);

                let runs = actions_client.fetch_repo_workflow_runs(
                    &repo.owner.login,
                    &repo.name,
                    status_filter,
                    10
                ).await?;

                display_workflow_runs(&runs.workflow_runs, None);
            }
        }
        _ => {}
    }

    Ok(())
}

async fn show_repo_details(
    client: &GraphQLClient,
    repo: &github::graphql::RepositoryInfo,
) -> Result<(), Box<dyn Error>> {
    println!("\n{}", "=".repeat(80));
    println!("📦 Repository Details: {}", repo.name_with_owner);
    println!("{}", "=".repeat(80));

    println!("🔗 URL: {}", repo.url);
    println!("🔒 Privacy: {}", if repo.is_private { "Private" } else { "Public" });

    if let Some(desc) = &repo.description {
        println!("📝 Description: {}", desc);
    }

    if let Some(branch) = &repo.default_branch_ref {
        println!("\n🌿 Default Branch: {}", branch.name);
        println!("📌 Latest Commit: {}", &branch.target.oid[..8]);

        if let Some(date) = &branch.target.committed_date {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(date) {
                println!("📅 Last Commit: {}", dt.format("%Y-%m-%d %H:%M:%S"));
            }
        }
    }

    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&repo.updated_at) {
        println!("\n🕐 Last Updated: {}", dt.format("%Y-%m-%d %H:%M:%S"));
    }

    println!("\n📥 Clone URLs:");
    println!("  SSH: {}", repo.ssh_url);
    println!("  HTTPS: {}", repo.url);

    println!("{}", "=".repeat(80));

    Ok(())
}

async fn show_basic_info(gh: &GitHubClient) -> Result<(), Box<dyn Error>> {
    println!("\n👤 Fetching basic user info (REST API)...");

    let user = fetch_user(gh).await?;
    println!("\n{}", "=".repeat(80));
    println!("User Info:");
    println!("  Username: {}", user.login);
    println!("  Name: {}", user.name.unwrap_or_else(|| "N/A".into()));
    println!("  Public repos: {}", user.public_repos);
    println!("{}", "=".repeat(80));

    Ok(())
}

async fn fetch_user(
    gh: &GitHubClient,
) -> Result<GitHubUser, reqwest::Error> {
    gh.client()
        .get("https://api.github.com/user")
        .header("Authorization", gh.auth_header())
        .header("User-Agent", "gitlink")
        .send()
        .await?
        .json::<GitHubUser>()
        .await
}