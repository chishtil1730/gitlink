use crate::github::graphql::{fetch_repositories, GraphQLClient, RepositoryInfo};
use dialoguer::{theme::ColorfulTheme, Input, MultiSelect, Select};
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

    /// Display repositories and let user select one using arrow keys
    /// Shows 5 repos at a time with search functionality
    pub fn select_repository(&self) -> Result<Option<&RepositoryInfo>, Box<dyn Error>> {
        if self.repos.is_empty() {
            println!("No repositories found.");
            return Ok(None);
        }

        println!("\n📂 Your GitHub Repositories:");
        println!("{}", "=".repeat(80));
        println!("💡 Tip: Type '/' to search for repositories");
        println!("{}", "=".repeat(80));

        // Use a Vec of indices or clone the data to keep types consistent
        let mut filtered_repos = self.repos.clone();
        let mut page = 0;
        const PAGE_SIZE: usize = 5;

        loop {
            // Calculate pagination
            let total_pages = (filtered_repos.len() + PAGE_SIZE - 1) / PAGE_SIZE;
            let start_idx = page * PAGE_SIZE;
            let end_idx = std::cmp::min(start_idx + PAGE_SIZE, filtered_repos.len());

            if filtered_repos.is_empty() {
                println!("No repositories match your search.");
                print!("\nPress Enter to clear search or type '/' to search again: ");
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;

                if input.trim() == "/" {
                    continue;
                } else {
                    filtered_repos = self.repos.clone();
                    page = 0;
                    continue;
                }
            }

            // Create display items for current page
            let mut items: Vec<String> = filtered_repos[start_idx..end_idx]
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

            // Add navigation options
            if page > 0 {
                items.insert(0, "⬆️  Previous page".to_string());
            }
            if end_idx < filtered_repos.len() {
                items.push("⬇️  Next page".to_string());
            }
            items.push("🔍 Search repositories".to_string());
            items.push("❌ Cancel".to_string());

            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt(format!("Page {}/{} - Select a repository", page + 1, total_pages))
                .items(&items)
                .default(0)
                .interact()?;

            // Calculate actual selection accounting for navigation items
            let has_prev = page > 0;
            let has_next = end_idx < filtered_repos.len();
            let prev_offset = if has_prev { 1 } else { 0 };

            // Logic for navigation and search
            if has_prev && selection == 0 {
                page = page.saturating_sub(1);
                continue;
            } else if has_next && selection == (if has_prev { items.len() - 3 } else { items.len() - 3 + 1 }) {
                // Adjusting index check for "Next page" based on whether "Previous" exists
                page += 1;
                continue;
            } else if selection == items.len() - 2 {
                // Search
                let search_term: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Search repositories")
                    .allow_empty(true)
                    .interact_text()?;

                if !search_term.is_empty() {
                    // FIX: We now store owned objects in filtered_repos
                    filtered_repos = self.filter_by_name(&search_term);
                    page = 0;
                }
                continue;
            } else if selection == items.len() - 1 {
                // Cancel
                return Ok(None);
            } else {
                // Actual repository selection
                let repo_idx = start_idx + selection - prev_offset;
                let selected_repo = &filtered_repos[repo_idx];

                // Return reference to the original repo in self.repos
                return Ok(self.repos.iter().find(|r| r.name_with_owner == selected_repo.name_with_owner));
            }
        }
    }

    /// Select multiple repositories using arrow keys and space to toggle
    pub fn select_multiple(&self) -> Result<Vec<&RepositoryInfo>, Box<dyn Error>> {
        if self.repos.is_empty() {
            println!("No repositories found.");
            return Ok(Vec::new());
        }

        println!("\n📂 Your GitHub Repositories:");
        let items: Vec<String> = self
            .repos
            .iter()
            .map(|repo| {
                let privacy = if repo.is_private { "🔒" } else { "🌍" };
                let desc = repo.description.as_deref().unwrap_or("No description");
                format!("{} {} - {}", privacy, repo.name_with_owner, desc)
            })
            .collect();

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

    /// Filter repositories by search term - Now returns owned Vec to match filtered_repos type
    pub fn filter_by_name(&self, search: &str) -> Vec<RepositoryInfo> {
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
            .cloned() // Changed to .cloned() to return Vec<RepositoryInfo>
            .collect()
    }
}