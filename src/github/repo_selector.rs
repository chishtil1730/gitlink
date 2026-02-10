use crate::github::graphql::{fetch_repositories, GraphQLClient, RepositoryInfo};
use std::error::Error;
use std::io::{self, Write};

/// Interactive repository selector
pub struct RepoSelector {
    repos: Vec<RepositoryInfo>,
}

impl RepoSelector {
    /// Fetch repositories from GitHub and create selector
    pub async fn new(client: &GraphQLClient) -> Result<Self, Box<dyn Error>> {
        println!("📦 Fetching your repositories from GitHub...");
        let response = fetch_repositories(client, 100, false).await?;

        Ok(Self {
            repos: response.viewer.repositories.nodes,
        })
    }

    /// Display repositories and let user select one
    pub fn select_repository(&self) -> Result<Option<&RepositoryInfo>, Box<dyn Error>> {
        if self.repos.is_empty() {
            println!("No repositories found.");
            return Ok(None);
        }

        println!("\n📂 Your GitHub Repositories:");
        println!("{}", "=".repeat(80));

        for (idx, repo) in self.repos.iter().enumerate() {
            let privacy = if repo.is_private { "🔒 Private" } else { "🌍 Public" };
            let desc = repo
                .description
                .as_deref()
                .unwrap_or("No description");

            println!(
                "{:3}. {} {} - {}",
                idx + 1,
                repo.name_with_owner,
                privacy,
                desc
            );

            // Show last updated
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&repo.updated_at) {
                println!("     └─ Last updated: {}", dt.format("%Y-%m-%d %H:%M"));
            }
        }

        println!("{}", "=".repeat(80));
        print!("\n🎯 Select a repository (1-{}) or 'q' to quit: ", self.repos.len());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.eq_ignore_ascii_case("q") {
            return Ok(None);
        }

        match input.parse::<usize>() {
            Ok(num) if num > 0 && num <= self.repos.len() => {
                Ok(Some(&self.repos[num - 1]))
            }
            _ => {
                println!("❌ Invalid selection. Please try again.");
                self.select_repository()
            }
        }
    }

    /// Select multiple repositories
    pub fn select_multiple(&self) -> Result<Vec<&RepositoryInfo>, Box<dyn Error>> {
        if self.repos.is_empty() {
            println!("No repositories found.");
            return Ok(Vec::new());
        }

        println!("\n📂 Your GitHub Repositories:");
        println!("{}", "=".repeat(80));

        for (idx, repo) in self.repos.iter().enumerate() {
            let privacy = if repo.is_private { "🔒 Private" } else { "🌍 Public" };
            let desc = repo
                .description
                .as_deref()
                .unwrap_or("No description");

            println!(
                "{:3}. {} {} - {}",
                idx + 1,
                repo.name_with_owner,
                privacy,
                desc
            );
        }

        println!("{}", "=".repeat(80));
        println!("\n🎯 Select repositories (comma-separated, e.g., '1,3,5') or 'all': ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.eq_ignore_ascii_case("all") {
            return Ok(self.repos.iter().collect());
        }

        let indices: Result<Vec<usize>, _> = input
            .split(',')
            .map(|s| s.trim().parse::<usize>())
            .collect();

        match indices {
            Ok(nums) => {
                let selected: Vec<&RepositoryInfo> = nums
                    .iter()
                    .filter_map(|&num| {
                        if num > 0 && num <= self.repos.len() {
                            Some(&self.repos[num - 1])
                        } else {
                            None
                        }
                    })
                    .collect();

                if selected.is_empty() {
                    println!("❌ No valid selections. Please try again.");
                    self.select_multiple()
                } else {
                    Ok(selected)
                }
            }
            Err(_) => {
                println!("❌ Invalid input. Please try again.");
                self.select_multiple()
            }
        }
    }

    /// Filter repositories by search term
    pub fn filter_by_name(&self, search: &str) -> Vec<&RepositoryInfo> {
        self.repos
            .iter()
            .filter(|repo| {
                repo.name.to_lowercase().contains(&search.to_lowercase())
                    || repo
                    .description
                    .as_ref()
                    .map(|d| d.to_lowercase().contains(&search.to_lowercase()))
                    .unwrap_or(false)
            })
            .collect()
    }
}
