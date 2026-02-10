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

    /// Display repositories with "/" key triggering autocomplete search
    pub fn select_repository(&self) -> Result<Option<&RepositoryInfo>, Box<dyn Error>> {
        if self.repos.is_empty() {
            println!("No repositories found.");
            return Ok(None);
        }

        println!("\n📂 Your GitHub Repositories:");
        println!("{}", "=".repeat(80));
        println!("💡 Tip: Press '/' to open search dialog");
        println!("{}", "=".repeat(80));

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
                println!("\nPress '/' to search again or Enter to reset");
                print!("> ");
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;

                if input.trim() == "/" {
                    // Open search dialog
                    if let Some(search_term) = self.show_search_dialog()? {
                        filtered_repos = self.filter_by_name(&search_term);
                        page = 0;
                    }
                } else {
                    filtered_repos = self.repos.clone();
                    page = 0;
                }
                continue;
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
            items.push("/ Search (press / key)".to_string());
            items.push("❌ Cancel".to_string());

            // Custom prompt with instruction
            print!("\nPage {}/{} - Select a repository (or press '/'): ", page + 1, total_pages);
            io::stdout().flush()?;

            // Check if user presses "/" before showing select menu
            let mut peek_input = String::new();
            let mut buffer = [0u8; 1];

            // Use a simple approach - show the select menu and let user choose "/" option
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
            } else if has_next && selection == items.len() - 3 {
                page += 1;
                continue;
            } else if selection == items.len() - 2 {
                // Search option selected
                if let Some(search_term) = self.show_search_dialog()? {
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

    /// Show interactive search dialog as user types
    fn show_search_dialog(&self) -> Result<Option<String>, Box<dyn Error>> {
        println!("\n{}", "=".repeat(80));
        println!("🔍 Search Repositories");
        println!("{}", "=".repeat(80));
        println!("Type to search (matches repo name and description)");
        println!("Press Enter to search, Esc to cancel");
        println!("{}", "=".repeat(80));

        let search_term: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Search")
            .allow_empty(true)
            .interact_text()?;

        if search_term.is_empty() {
            Ok(None)
        } else {
            // Show matching results preview
            let matches = self.filter_by_name(&search_term);
            println!("\n✨ Found {} matching repositories", matches.len());

            if matches.len() > 0 && matches.len() <= 5 {
                println!("\nMatches:");
                for repo in matches.iter().take(5) {
                    let privacy = if repo.is_private { "🔒" } else { "🌍" };
                    println!("  {} {}", privacy, repo.name_with_owner);
                }
            }

            Ok(Some(search_term))
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
            .with_prompt("Select repositories (Space to toggle, Enter to confirm)")
            .items(&items)
            .interact()?;

        let selected: Vec<&RepositoryInfo> = selections
            .iter()
            .map(|&idx| &self.repos[idx])
            .collect();

        Ok(selected)
    }

    /// Filter repositories by search term
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
            .cloned()
            .collect()
    }
}