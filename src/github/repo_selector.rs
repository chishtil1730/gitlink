use crate::github::graphql::{fetch_repositories, GraphQLClient, RepositoryInfo};
use dialoguer::{theme::ColorfulTheme, MultiSelect, Select};
use std::error::Error;

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

    /// Display repositories and let user select one using arrow keys
    pub fn select_repository(&self) -> Result<Option<&RepositoryInfo>, Box<dyn Error>> {
        if self.repos.is_empty() {
            println!("No repositories found.");
            return Ok(None);
        }

        println!("\n📂 Your GitHub Repositories:");
        println!("{}", "=".repeat(80));

        // Create display items for the menu
        let items: Vec<String> = self
            .repos
            .iter()
            .map(|repo| {
                let privacy = if repo.is_private { "🔒" } else { "🌍" };
                let desc = repo
                    .description
                    .as_deref()
                    .unwrap_or("No description");

                format!("{} {} - {}", privacy, repo.name_with_owner, desc)
            })
            .collect();

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select a repository (ESC to cancel)")
            .items(&items)
            .default(0)
            .interact_opt()?;

        match selection {
            Some(idx) => Ok(Some(&self.repos[idx])),
            None => Ok(None),
        }
    }

    /// Select multiple repositories using arrow keys and space to toggle
    pub fn select_multiple(&self) -> Result<Vec<&RepositoryInfo>, Box<dyn Error>> {
        if self.repos.is_empty() {
            println!("No repositories found.");
            return Ok(Vec::new());
        }

        println!("\n📂 Your GitHub Repositories:");
        println!("{}", "=".repeat(80));

        // Create display items for the menu
        let items: Vec<String> = self
            .repos
            .iter()
            .map(|repo| {
                let privacy = if repo.is_private { "🔒" } else { "🌍" };
                let desc = repo
                    .description
                    .as_deref()
                    .unwrap_or("No description");

                format!("{} {} - {}", privacy, repo.name_with_owner, desc)
            })
            .collect();

        println!("Use arrow keys to navigate, SPACE to select/deselect, ENTER to confirm");

        let selections = MultiSelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Select repositories")
            .items(&items)
            .interact()?;

        let selected: Vec<&RepositoryInfo> = selections
            .iter()
            .map(|&idx| &self.repos[idx])
            .collect();

        Ok(selected)
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