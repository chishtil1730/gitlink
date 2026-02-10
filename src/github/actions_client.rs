use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

const GITHUB_API_BASE: &str = "https://api.github.com";

/// GitHub Actions client for REST API
pub struct ActionsClient {
    client: Client,
    token: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WorkflowRunsResponse {
    pub total_count: i32,
    pub workflow_runs: Vec<WorkflowRun>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WorkflowRun {
    pub id: u64,
    pub name: String,
    pub head_branch: String,
    pub head_sha: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub html_url: String,
    pub repository: RunRepository,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RunRepository {
    pub full_name: String,
}

impl ActionsClient {
    pub fn new(token: String) -> Self {
        Self {
            client: Client::new(),
            token,
        }
    }

    /// Fetch workflow runs for a specific repository
    pub async fn fetch_repo_workflow_runs(
        &self,
        owner: &str,
        repo: &str,
        status: Option<&str>,
        per_page: i32,
    ) -> Result<WorkflowRunsResponse, Box<dyn Error>> {
        let mut url = format!(
            "{}/repos/{}/{}/actions/runs?per_page={}",
            GITHUB_API_BASE, owner, repo, per_page
        );

        if let Some(s) = status {
            url.push_str(&format!("&status={}", s));
        }

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("User-Agent", "gitlink")
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("GitHub API error: {}", response.status()).into());
        }

        let runs: WorkflowRunsResponse = response.json().await?;
        Ok(runs)
    }

    /// Fetch workflow runs across all accessible repositories
    pub async fn fetch_all_workflow_runs(
        &self,
        repos: &[(&str, &str)], // Vec of (owner, repo) tuples
        status: Option<&str>,
        per_page: i32,
    ) -> Result<Vec<WorkflowRun>, Box<dyn Error>> {
        let mut all_runs = Vec::new();

        for (owner, repo) in repos {
            match self.fetch_repo_workflow_runs(owner, repo, status, per_page).await {
                Ok(response) => {
                    all_runs.extend(response.workflow_runs);
                }
                Err(e) => {
                    eprintln!("⚠️  Error fetching runs for {}/{}: {}", owner, repo, e);
                }
            }
        }

        // Sort by created_at (most recent first)
        all_runs.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(all_runs)
    }
}

/// Display workflow runs in a user-friendly format
pub fn display_workflow_runs(runs: &[WorkflowRun], limit: Option<usize>) {
    let display_runs = if let Some(l) = limit {
        &runs[..std::cmp::min(l, runs.len())]
    } else {
        runs
    };

    println!("\n{}", "=".repeat(80));
    println!("⚡ GitHub Actions Workflow Runs");
    println!("{}", "=".repeat(80));

    if display_runs.is_empty() {
        println!("No workflow runs found.");
        return;
    }

    for run in display_runs {
        let status_icon = match run.conclusion.as_deref() {
            Some("success") => "✅",
            Some("failure") => "❌",
            Some("cancelled") => "🚫",
            Some("skipped") => "⏭️",
            _ => match run.status.as_str() {
                "in_progress" => "🔄",
                "queued" => "⏳",
                _ => "❓",
            },
        };

        println!("\n{} {}", status_icon, run.name);
        println!("   Repository: {}", run.repository.full_name);
        println!("   Branch: {}", run.head_branch);
        println!("   Status: {}", run.status);

        if let Some(conclusion) = &run.conclusion {
            println!("   Conclusion: {}", conclusion);
        }

        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&run.created_at) {
            println!("   Created: {}", dt.format("%Y-%m-%d %H:%M:%S"));
        }

        println!("   🔗 {}", run.html_url);
        println!("{}", "─".repeat(80));
    }

    if limit.is_some() && runs.len() > display_runs.len() {
        println!("\n... and {} more runs", runs.len() - display_runs.len());
    }

    println!();
}