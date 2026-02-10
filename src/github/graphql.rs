use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

const GITHUB_GRAPHQL_ENDPOINT: &str = "https://api.github.com/graphql";

/// GraphQL client wrapper for GitHub API
pub struct GraphQLClient {
    client: Client,
    token: String,
}

impl GraphQLClient {
    pub fn new(token: String) -> Self {
        Self {
            client: Client::new(),
            token,
        }
    }

    /// Execute a GraphQL query
    pub async fn query<T: for<'de> Deserialize<'de>>(
        &self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<T, Box<dyn Error>> {
        let body = serde_json::json!({
            "query": query,
            "variables": variables
        });

        let response = self
            .client
            .post(GITHUB_GRAPHQL_ENDPOINT)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("User-Agent", "gitlink")
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        let response_text = response.text().await?;

        if !status.is_success() {
            return Err(format!("GraphQL request failed: {}", response_text).into());
        }

        let graphql_response: GraphQLResponse<T> = serde_json::from_str(&response_text)?;

        if let Some(errors) = graphql_response.errors {
            return Err(format!("GraphQL errors: {:?}", errors).into());
        }

        graphql_response
            .data
            .ok_or_else(|| "No data in GraphQL response".into())
    }
}

#[derive(Debug, Deserialize)]
struct GraphQLResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Deserialize)]
struct GraphQLError {
    message: String,
    #[serde(rename = "type")]
    error_type: Option<String>,
    path: Option<Vec<String>>,
}

// ============================================================================
// User Activity Queries
// ============================================================================

#[derive(Debug, Deserialize, Serialize)]
pub struct UserActivity {
    pub viewer: Viewer,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Viewer {
    pub login: String,
    pub name: Option<String>,
    #[serde(rename = "contributionsCollection")]
    pub contributions_collection: ContributionsCollection,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ContributionsCollection {
    #[serde(rename = "totalCommitContributions")]
    pub total_commit_contributions: i32,
    #[serde(rename = "totalPullRequestContributions")]
    pub total_pull_request_contributions: i32,
    #[serde(rename = "totalIssueContributions")]
    pub total_issue_contributions: i32,
    #[serde(rename = "totalRepositoryContributions")]
    pub total_repository_contributions: i32,
    #[serde(rename = "contributionCalendar")]
    pub contribution_calendar: ContributionCalendar,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ContributionCalendar {
    #[serde(rename = "totalContributions")]
    pub total_contributions: i32,
    pub weeks: Vec<ContributionWeek>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ContributionWeek {
    #[serde(rename = "contributionDays")]
    pub contribution_days: Vec<ContributionDay>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ContributionDay {
    pub date: String,
    #[serde(rename = "contributionCount")]
    pub contribution_count: i32,
}

pub async fn fetch_user_activity(
    client: &GraphQLClient,
) -> Result<UserActivity, Box<dyn Error>> {
    let query = r#"
        query {
            viewer {
                login
                name
                contributionsCollection {
                    totalCommitContributions
                    totalPullRequestContributions
                    totalIssueContributions
                    totalRepositoryContributions
                    contributionCalendar {
                        totalContributions
                        weeks {
                            contributionDays {
                                date
                                contributionCount
                            }
                        }
                    }
                }
            }
        }
    "#;

    client.query(query, serde_json::json!({})).await
}

// ============================================================================
// Recent Commits Query
// ============================================================================

#[derive(Debug, Deserialize, Serialize)]
pub struct UserCommitsResponse {
    pub viewer: ViewerCommits,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ViewerCommits {
    pub login: String,
    pub repositories: RepositoriesWithCommits,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RepositoriesWithCommits {
    pub nodes: Vec<RepositoryWithCommits>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RepositoryWithCommits {
    pub name: String,
    #[serde(rename = "nameWithOwner")]
    pub name_with_owner: String,
    #[serde(rename = "defaultBranchRef")]
    pub default_branch_ref: Option<BranchWithCommits>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BranchWithCommits {
    pub target: CommitTarget,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CommitTarget {
    pub history: CommitHistory,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CommitHistory {
    pub nodes: Vec<Commit>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Commit {
    pub message: String,
    #[serde(rename = "committedDate")]
    pub committed_date: String,
    pub oid: String,
    pub additions: i32,
    pub deletions: i32,
    pub author: CommitAuthor,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CommitAuthor {
    pub name: Option<String>,
    pub email: Option<String>,
    pub date: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Repository {
    pub name: String,
    #[serde(rename = "nameWithOwner")]
    pub name_with_owner: String,
}

pub async fn fetch_recent_commits(
    client: &GraphQLClient,
    limit: i32,
) -> Result<UserCommitsResponse, Box<dyn Error>> {
    let query = r#"
        query($limit: Int!) {
            viewer {
                login
                repositories(
                    first: 10,
                    orderBy: {field: PUSHED_AT, direction: DESC},
                    ownerAffiliations: [OWNER, COLLABORATOR]
                ) {
                    nodes {
                        name
                        nameWithOwner
                        defaultBranchRef {
                            target {
                                ... on Commit {
                                    history(first: $limit) {
                                        nodes {
                                            message
                                            committedDate
                                            oid
                                            additions
                                            deletions
                                            author {
                                                name
                                                email
                                                date
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    "#;

    let variables = serde_json::json!({
        "limit": limit
    });

    client.query(query, variables).await
}

// ============================================================================
// Pull Requests Query
// ============================================================================

#[derive(Debug, Deserialize, Serialize)]
pub struct PullRequestsResponse {
    pub viewer: ViewerPRs,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ViewerPRs {
    pub login: String,
    #[serde(rename = "pullRequests")]
    pub pull_requests: PullRequestConnection,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PullRequestConnection {
    pub nodes: Vec<PullRequest>,
    #[serde(rename = "totalCount")]
    pub total_count: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PullRequest {
    pub title: String,
    pub number: i32,
    pub state: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    pub repository: Repository,
    pub author: Option<Author>,
    pub reviews: Option<ReviewConnection>,
    #[serde(rename = "mergeable")]
    pub mergeable: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Author {
    pub login: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReviewConnection {
    #[serde(rename = "totalCount")]
    pub total_count: i32,
    pub nodes: Vec<Review>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Review {
    pub state: String,
    pub author: Option<Author>,
}

pub async fn fetch_pull_requests(
    client: &GraphQLClient,
    state: &str, // "OPEN", "CLOSED", "MERGED"
    limit: i32,
) -> Result<PullRequestsResponse, Box<dyn Error>> {
    let query = r#"
        query($states: [PullRequestState!], $limit: Int!) {
            viewer {
                login
                pullRequests(first: $limit, states: $states, orderBy: {field: UPDATED_AT, direction: DESC}) {
                    totalCount
                    nodes {
                        title
                        number
                        state
                        createdAt
                        updatedAt
                        mergeable
                        repository {
                            name
                            nameWithOwner
                        }
                        author {
                            login
                        }
                        reviews(first: 5) {
                            totalCount
                            nodes {
                                state
                                author {
                                    login
                                }
                            }
                        }
                    }
                }
            }
        }
    "#;

    let variables = serde_json::json!({
        "states": [state],
        "limit": limit
    });

    client.query(query, variables).await
}

// ============================================================================
// Repository List Query (for selection)
// ============================================================================

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RepositoriesResponse {
    pub viewer: ViewerRepos,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ViewerRepos {
    pub login: String,
    pub repositories: RepositoryConnection,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RepositoryConnection {
    pub nodes: Vec<RepositoryInfo>,
    #[serde(rename = "totalCount")]
    pub total_count: i32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RepositoryInfo {
    pub name: String,
    #[serde(rename = "nameWithOwner")]
    pub name_with_owner: String,
    pub description: Option<String>,
    #[serde(rename = "isPrivate")]
    pub is_private: bool,
    #[serde(rename = "defaultBranchRef")]
    pub default_branch_ref: Option<BranchRef>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    pub url: String,
    #[serde(rename = "sshUrl")]
    pub ssh_url: String,
    pub owner: Owner,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BranchRef {
    pub name: String,
    pub target: Target,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Target {
    pub oid: String,
    #[serde(rename = "committedDate")]
    pub committed_date: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Owner {
    pub login: String,
}

pub async fn fetch_repositories(
    client: &GraphQLClient,
    limit: i32,
    include_forks: bool,
) -> Result<RepositoriesResponse, Box<dyn Error>> {
    let query = r#"
        query($limit: Int!, $isFork: Boolean) {
            viewer {
                login
                repositories(
                    first: $limit,
                    orderBy: {field: UPDATED_AT, direction: DESC},
                    isFork: $isFork,
                    ownerAffiliations: [OWNER, COLLABORATOR]
                ) {
                    totalCount
                    nodes {
                        name
                        nameWithOwner
                        description
                        isPrivate
                        url
                        sshUrl
                        updatedAt
                        owner {
                            login
                        }
                        defaultBranchRef {
                            name
                            target {
                                oid
                                ... on Commit {
                                    committedDate
                                }
                            }
                        }
                    }
                }
            }
        }
    "#;

    let variables = serde_json::json!({
        "limit": limit,
        "isFork": if include_forks { serde_json::Value::Null } else { serde_json::json!(false) }
    });

    client.query(query, variables).await
}

// ============================================================================
// Repository Sync Check Query
// ============================================================================

#[derive(Debug, Deserialize, Serialize)]
pub struct RepoSyncResponse {
    pub repository: RepositorySync,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RepositorySync {
    pub name: String,
    #[serde(rename = "nameWithOwner")]
    pub name_with_owner: String,
    #[serde(rename = "defaultBranchRef")]
    pub default_branch_ref: Option<BranchRefSync>,
    pub refs: Option<RefsConnection>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BranchRefSync {
    pub name: String,
    pub target: TargetSync,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TargetSync {
    pub oid: String,
    #[serde(rename = "committedDate")]
    pub committed_date: Option<String>,
    pub history: Option<CommitHistoryCount>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CommitHistoryCount {
    #[serde(rename = "totalCount")]
    pub total_count: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RefsConnection {
    pub nodes: Vec<BranchRefSync>,
}

pub async fn fetch_repository_sync_info(
    client: &GraphQLClient,
    owner: &str,
    repo_name: &str,
) -> Result<RepoSyncResponse, Box<dyn Error>> {
    let query = r#"
        query($owner: String!, $name: String!) {
            repository(owner: $owner, name: $name) {
                name
                nameWithOwner
                defaultBranchRef {
                    name
                    target {
                        oid
                        ... on Commit {
                            committedDate
                            history {
                                totalCount
                            }
                        }
                    }
                }
                refs(refPrefix: "refs/heads/", first: 10) {
                    nodes {
                        name
                        target {
                            oid
                            ... on Commit {
                                committedDate
                            }
                        }
                    }
                }
            }
        }
    "#;

    let variables = serde_json::json!({
        "owner": owner,
        "name": repo_name
    });

    client.query(query, variables).await
}