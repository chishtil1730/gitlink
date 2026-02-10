mod auth;
mod github;

use auth::oauth;
use dialoguer::{theme::ColorfulTheme, Select};
use github::client::GitHubClient;
use github::graphql::{self, GraphQLClient};
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
            5 => show_basic_info(&gh_client).await?,
            6 => {
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
    println!("\n💾 Fetching your recent commits...");

    let commits = graphql::fetch_recent_commits(client, 5).await?;

    println!("\n{}", "=".repeat(80));
    println!("Recent Commits by {}", commits.viewer.login);
    println!("{}", "=".repeat(80));

    for repo in &commits.viewer.repositories.nodes {
        if let Some(branch_ref) = &repo.default_branch_ref {
            println!("\n📦 Repository: {}", repo.name_with_owner);
            println!("{}", "─".repeat(80));

            for commit in &branch_ref.target.history.nodes {
                // Parse and format the date
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&commit.committed_date) {
                    println!("\n  📝 {}", dt.format("%Y-%m-%d %H:%M:%S"));
                }

                println!("  🔑 {}", &commit.oid[..8]);

                // Show first line of commit message
                let first_line = commit.message.lines().next().unwrap_or(&commit.message);
                println!("  💬 {}", first_line);

                if let Some(author) = &commit.author.name {
                    println!("  👤 {}", author);
                }

                println!("  📊 +{} -{}", commit.additions, commit.deletions);
            }
        } else {
            println!("\n📦 Repository: {} (no default branch)", repo.name_with_owner);
        }
    }

    println!("\n{}", "=".repeat(80));

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

        // Create sync checker and check the selected repo
        let sync_checker = SyncChecker::new(GraphQLClient::new(
            auth::token_store::load_token()?
        ));

        sync_checker.display_sync_status(repo).await?;

        // Offer to show more details
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