mod auth;

use auth::oauth;
use reqwest::Client;
use serde::Deserialize;

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
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // OAuth login
    let token = oauth::login().await?;

    let client = Client::new();

    let user = fetch_user(&client, &token).await?;
    println!(
        "\nUser Info:\n- Username: {}\n- Name: {}\n- Public repos: {}\n",
        user.login,
        user.name.unwrap_or_else(|| "N/A".into()),
        user.public_repos
    );

    let repos = fetch_repos(&client, &token).await?;
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
    client: &Client,
    token: &str,
) -> Result<GitHubUser, reqwest::Error> {
    client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "gitlink")
        .send()
        .await?
        .json::<GitHubUser>()
        .await
}

async fn fetch_repos(
    client: &Client,
    token: &str,
) -> Result<Vec<Repo>, reqwest::Error> {
    client
        .get("https://api.github.com/user/repos")
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "gitlink")
        .send()
        .await?
        .json::<Vec<Repo>>()
        .await
}
