mod auth;
mod github;

use auth::oauth;
use github::client::GitHubClient;
use serde::Deserialize;
use std::error::Error;

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
    // OAuth login
    let token = oauth::login().await?;

    // Create GitHub client abstraction
    let gh = GitHubClient::new(token);

    // Fetch and print data
    get_raw_info(&gh).await?;

    Ok(())
}

async fn get_raw_info(gh: &GitHubClient) -> Result<(), Box<dyn Error>> {
    let user = fetch_user(gh).await?;
    println!(
        "\nUser Info:\n- Username: {}\n- Name: {}\n- Public repos: {}\n",
        user.login,
        user.name.unwrap_or_else(|| "N/A".into()),
        user.public_repos
    );

    let repos = fetch_repos(gh).await?;
    println!("Repositories:");
    for repo in repos {
        println!(
            "- {} ({})",
            repo.name,
            if repo.private { "private" } else { "public" }
        );
    }

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

async fn fetch_repos(
    gh: &GitHubClient,
) -> Result<Vec<Repo>, reqwest::Error> {
    gh.client()
        .get("https://api.github.com/user/repos")
        .header("Authorization", gh.auth_header())
        .header("User-Agent", "gitlink")
        .send()
        .await?
        .json::<Vec<Repo>>()
        .await
}
