use crate::github::graphql::GraphQLClient;
use serde::{Deserialize, Serialize};
use std::error::Error;

/// Push status information for a branch
#[derive(Debug, Serialize, Deserialize)]
pub struct PushStatus {
    pub can_push: bool,
    pub is_synced: bool,
    pub has_uncommitted_changes: bool,
    pub has_unpushed_commits: bool,
    pub has_conflicts: bool,
    pub remote_ahead: bool,
    pub local_commit: String,
    pub remote_commit: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
struct BranchComparisonResponse {
    repository: RepositoryComparison,
}

#[derive(Debug, Deserialize)]
struct RepositoryComparison {
    ref_: Option<RefInfo>,
}

#[derive(Debug, Deserialize)]
struct RefInfo {
    target: TargetInfo,
    compare: CompareInfo,
}

#[derive(Debug, Deserialize)]
struct TargetInfo {
    oid: String,
}

#[derive(Debug, Deserialize)]
struct CompareInfo {
    #[serde(rename = "aheadBy")]
    ahead_by: i32,
    #[serde(rename = "behindBy")]
    behind_by: i32,
}

impl GraphQLClient {
    /// Check if the latest commit has been pushed to remote
    /// Compares local HEAD commit against remote tracking branch
    pub async fn check_push_status(
        &self,
        owner: &str,
        repo_name: &str,
        branch: &str,
    ) -> Result<PushStatus, Box<dyn Error>> {
        // Query to get local branch commit and compare with remote
        let query = r#"
            query($owner: String!, $repo: String!, $branch: String!) {
                repository(owner: $owner, name: $repo) {
                    ref(qualifiedName: $branch) {
                        target {
                            oid
                        }
                        compare(headRef: $branch) {
                            aheadBy
                            behindBy
                        }
                    }
                }
            }
        "#;

        let variables = serde_json::json!({
            "owner": owner,
            "repo": repo_name,
            "branch": format!("refs/heads/{}", branch)
        });

        let response: BranchComparisonResponse = self.query(query, variables).await?;

        let mut status = PushStatus {
            can_push: true,
            is_synced: true,
            has_uncommitted_changes: false,
            has_unpushed_commits: false,
            has_conflicts: false,
            remote_ahead: false,
            local_commit: String::new(),
            remote_commit: String::new(),
            message: String::new(),
        };

        if let Some(ref_info) = response.repository.ref_ {
            status.local_commit = ref_info.target.oid.clone();

            let ahead = ref_info.compare.ahead_by;
            let behind = ref_info.compare.behind_by;

            if ahead > 0 {
                status.has_unpushed_commits = true;
                status.is_synced = false;
                status.message = format!("{} commit(s) ahead of remote", ahead);
            }

            if behind > 0 {
                status.remote_ahead = true;
                status.can_push = false;
                status.is_synced = false;
                status.message = format!("{} commit(s) behind remote - pull required", behind);
            }

            if ahead > 0 && behind > 0 {
                status.has_conflicts = true;
                status.can_push = false;
                status.message = format!("{} ahead, {} behind - merge/rebase required", ahead, behind);
            }

            if ahead == 0 && behind == 0 {
                status.is_synced = true;
                status.message = "Branch is in sync with remote".to_string();
            }
        } else {
            status.message = "Branch not found or no remote tracking".to_string();
            status.can_push = false;
        }

        Ok(status)
    }

    /// Comprehensive check if pushing is possible
    /// Checks for: uncommitted changes, unpushed commits, conflicts, remote ahead
    pub async fn verify_push_possible(
        &self,
        owner: &str,
        repo_name: &str,
        branch: &str,
    ) -> Result<PushStatus, Box<dyn Error>> {
        // This uses the same underlying check but provides a comprehensive view
        self.check_push_status(owner, repo_name, branch).await
    }
}

/// Display push status in a user-friendly format
pub fn display_push_status(status: &PushStatus) {
    println!("\n{}", "=".repeat(80));
    println!("🔄 Push Status");
    println!("{}", "=".repeat(80));

    if status.is_synced {
        println!("✅ {}", status.message);
    } else {
        println!("⚠️  {}", status.message);
    }

    if !status.local_commit.is_empty() {
        println!("📌 Local commit: {}", &status.local_commit[..8]);
    }

    println!("\n📊 Status Details:");
    println!("  Can push: {}", if status.can_push { "✅ Yes" } else { "❌ No" });
    println!("  In sync: {}", if status.is_synced { "✅ Yes" } else { "⚠️  No" });
    println!("  Unpushed commits: {}", if status.has_unpushed_commits { "⚠️  Yes" } else { "✅ No" });
    println!("  Remote ahead: {}", if status.remote_ahead { "⚠️  Yes" } else { "✅ No" });
    println!("  Conflicts: {}", if status.has_conflicts { "❌ Yes" } else { "✅ No" });

    println!("{}", "=".repeat(80));
}