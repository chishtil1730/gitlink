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

use serde::{ Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct Repo {
    name: String,
    private: bool,
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    //Deleting the token that is persisted:
    let args: Vec<String> = std::env::args().collect();

    if args.len() >= 3 && args[1] == "auth" && args[2] == "logout" {
        auth::token_store::delete_token()?;
        println!("Logged out from GitHub");
        return Ok(());
    }


    // OAuth login
    //Persistent token by auth/token_store
    use auth::token_store;

    let token = match token_store::load_token() {
        Ok(token) => {
            println!("Using stored GitHub token");
            token
        }
        Err(_) => {
            let token = oauth::login().await?;
            token_store::save_token(&token)?;
            token
        }
    };


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

use std::fs;
use std::time::Duration;
use crate::github::cache;

async fn fetch_repos(
    gh: &GitHubClient,
) -> Result<Vec<Repo>, Box<dyn std::error::Error>> {
    let cache_file = cache::cache_path("repos");
    let ttl = Duration::from_secs(60);

    if cache::is_cache_valid(&cache_file, ttl) {
        let data = fs::read_to_string(&cache_file)?;
        let repos = serde_json::from_str(&data)?;
        return Ok(repos);
    }

    let response = gh
        .client()
        .get("https://api.github.com/user/repos?per_page=100")
        .header("Authorization", gh.auth_header())
        .header("User-Agent", "gitlink")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?;

    let repos = response.json::<Vec<Repo>>().await?;

    fs::write(&cache_file, serde_json::to_string(&repos)?)?;

    Ok(repos)
}

