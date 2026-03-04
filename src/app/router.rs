use crate::tui::app::{OutputBlock, OutputKind};

/// Execute a slash command string and return an OutputBlock.
/// This is the single bridge between the TUI and all backend modules.
/// No println! calls are permitted here — all output must be returned as content.
pub fn execute(raw: &str) -> OutputBlock {
    let input = raw.trim_start_matches('/');
    let parts: Vec<&str> = input.split_whitespace().collect();
    let cmd = parts.first().copied().unwrap_or("");
    let sub = parts.get(1).copied().unwrap_or("");

    match cmd {
        // ── Help ──────────────────────────────────────────────────────────────
        "help" => OutputBlock {
            kind: OutputKind::Info,
            content: [
                "Available commands:",
                "  /auth              — Show authentication status",
                "  /auth login        — Login via GitHub OAuth",
                "  /auth logout       — Remove stored token",
                "  /auth status       — Show current auth status",
                "  /scan              — Scan working directory for secrets",
                "  /scan history      — Scan git history for secrets",
                "  /scan ignored      — Manage ignored findings",
                "  /plan              — Open task planner overlay",
                "  /prp               — Start a poly-repo commit session",
                "  /prp list          — List PRP session groups",
                "  /show-activity     — Show GitHub contribution activity",
                "  /commits           — Show 3 most recent commits globally",
                "  /pull-requests     — Show your open pull requests",
                "  /repo-sync         — Check sync status of current local repo",
                "  /multi-sync        — Check sync across your GitHub repos",
                "  /push-check        — Check if latest commit is pushed",
                "  /push-verify       — Verify push is possible + preview",
                "  /branches          — Show branches for current local repo",
                "  /issues            — Show your open issues",
                "  /user-info         — Show basic GitHub user info",
                "  /clear             — Clear output history",
                "  /quit              — Exit GitLink TUI",
                "",
                "  ↑↓ navigate suggestions   Tab autocomplete   Ctrl+C quit",
            ]
                .join("\n"),
        },

        // ── Quit ──────────────────────────────────────────────────────────────
        "quit" => OutputBlock {
            kind: OutputKind::Info,
            content: "Press Ctrl+C to exit GitLink TUI.".to_string(),
        },

        // ── Auth ──────────────────────────────────────────────────────────────
        "auth" => match sub {
            "login" => run_async(async {
                use crate::auth::{oauth, token_store};
                let token = oauth::login().await.map_err(|e| e.to_string())?;
                token_store::save_token(&token).map_err(|e| e.to_string())?;
                Ok("Authenticated with GitHub successfully.".to_string())
            }),
            "logout" => run_sync(|| {
                crate::auth::token_store::delete_token()?;
                Ok("Logged out from GitHub. Token removed.".to_string())
            }),
            "status" | "" => run_sync(|| {
                match crate::auth::token_store::load_token() {
                    Ok(_) => Ok("Authenticated ✔  — GitHub token is present.".to_string()),
                    Err(_) => Err("Not authenticated. Run /auth login to connect.".into()),
                }
            }),
            _ => OutputBlock {
                kind: OutputKind::Error,
                content: format!(
                    "Unknown auth subcommand: '{}'. Try /auth login, /auth logout, or /auth status.",
                    sub
                ),
            },
        },

        // ── Scan ──────────────────────────────────────────────────────────────
        "scan" => {
            let sub_clean = sub.trim();
            match sub_clean {
                "ignored" | "--manage-ignored" => OutputBlock {
                    kind: OutputKind::Info,
                    content: "OPEN_IGNORE_OVERLAY_SIGNAL".to_string(),
                },
                "clear" => run_sync(|| {
                    crate::scanner::ignore::clear_all_silent();
                    Ok("All ignored findings have been cleared.".to_string())
                }),
                "history" => OutputBlock {
                    kind: OutputKind::Info,
                    content: "SCAN_HISTORY_SIGNAL".to_string(),
                },
                "" => OutputBlock {
                    kind: OutputKind::Info,
                    content: "Opening scanner overlay...".to_string(),
                },
                _ => OutputBlock {
                    kind: OutputKind::Error,
                    content: format!(
                        "Unknown scan subcommand: '{}'. Try /scan ignored.",
                        sub_clean
                    ),
                },
            }
        }

        // ── Plan ──────────────────────────────────────────────────────────────
        "plan" => OutputBlock {
            kind: OutputKind::Info,
            content: "Opening planner...".to_string(),
        },

        // ── PRP ───────────────────────────────────────────────────────────────
        "prp" => match sub {
            "list" => run_sync(|| {
                crate::prp_hub::run_prp_list()?;
                Ok("PRP group list complete.".to_string())
            }),
            "" => run_sync(|| {
                crate::prp_hub::run_prp_start()?;
                Ok("PRP session completed.".to_string())
            }),
            _ => OutputBlock {
                kind: OutputKind::Error,
                content: format!("Unknown prp subcommand: '{}'. Try /prp or /prp list.", sub),
            },
        },

        // ── Show Activity ─────────────────────────────────────────────────────
        "show-activity" => run_async(async {
            use crate::auth::token_store;
            use crate::github::graphql::{self, GraphQLClient};

            let token = token_store::load_token()
                .map_err(|_| "Not authenticated. Run /auth login first.".to_string())?;
            let client = GraphQLClient::new(token);
            let activity = graphql::fetch_user_activity(&client)
                .await
                .map_err(|e| e.to_string())?;
            let contrib = &activity.viewer.contributions_collection;

            let recent: Vec<String> = contrib
                .contribution_calendar
                .weeks
                .iter()
                .flat_map(|w| &w.contribution_days)
                .collect::<Vec<_>>()
                .iter()
                .rev()
                .take(3)
                .map(|d| {
                    let bar = "█".repeat(d.contribution_count.min(20) as usize);
                    format!("  {} : {} ({})", d.date, bar, d.contribution_count)
                })
                .collect();

            Ok(format!(
                "GitHub Activity — {} ({})\n\
                 Total contributions : {}\n\
                 Commits             : {}\n\
                 Pull requests       : {}\n\
                 Issues              : {}\n\
                 Repos created       : {}\n\
                 \nLast 3 days:\n{}\n{}",
                activity.viewer.name.as_deref().unwrap_or("N/A"),
                activity.viewer.login,
                contrib.contribution_calendar.total_contributions,
                contrib.total_commit_contributions,
                contrib.total_pull_request_contributions,
                contrib.total_issue_contributions,
                contrib.total_repository_contributions,
                recent.join("\n"),
                {
                    let weeks: Vec<String> = contrib.contribution_calendar.weeks.iter()
                        .map(|w| w.contribution_days.iter()
                            .map(|d| d.contribution_count.to_string())
                            .collect::<Vec<_>>().join(","))
                        .collect();
                    format!("GRID:{}", weeks.join("|"))
                },
            ))
        }),

        // ── Commits ───────────────────────────────────────────────────────────
        "commits" => run_async(async {
            use crate::auth::token_store;
            use crate::github::graphql::{self, GraphQLClient};

            let token = token_store::load_token()
                .map_err(|_| "Not authenticated. Run /auth login first.".to_string())?;
            let client = GraphQLClient::new(token);

            let commits_resp = graphql::fetch_recent_commits(&client, 10)
                .await
                .map_err(|e| e.to_string())?;

            let mut all_commits = Vec::new();
            for repo in &commits_resp.viewer.repositories.nodes {
                if let Some(branch_ref) = &repo.default_branch_ref {
                    for commit in &branch_ref.target.history.nodes {
                        all_commits.push((repo, commit));
                    }
                }
            }
            // Sort by committed date descending — true global order
            all_commits.sort_by(|a, b| b.1.committed_date.cmp(&a.1.committed_date));

            let mut out = format!(
                "3 Most Recent Commits — {}\n{}\n",
                commits_resp.viewer.login,
                "─".repeat(60),
            );

            for (repo, commit) in all_commits.iter().take(3) {
                let date_str = chrono::DateTime::parse_from_rfc3339(&commit.committed_date)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|_| commit.committed_date.clone());

                let first_line = commit.message.lines().next().unwrap_or(&commit.message);
                let author = commit.author.name.as_deref().unwrap_or("unknown");

                out.push_str(&format!(
                    "\n📦 {}\n📝 {}  🔑 {}\n💬 {}\n👤 {}  📊 +{} -{}\n{}\n",
                    repo.name_with_owner,
                    date_str,
                    &commit.oid[..8.min(commit.oid.len())],
                    first_line,
                    author,
                    commit.additions,
                    commit.deletions,
                    "─".repeat(60),
                ));
            }

            Ok(out)
        }),

        // ── Pull Requests ─────────────────────────────────────────────────────
        "pull-requests" => run_async(async {
            use crate::auth::token_store;
            use crate::github::graphql::{self, GraphQLClient};

            let token = token_store::load_token()
                .map_err(|_| "Not authenticated. Run /auth login first.".to_string())?;
            let client = GraphQLClient::new(token);

            let prs = graphql::fetch_pull_requests(&client, "OPEN", 15)
                .await
                .map_err(|e| e.to_string())?;

            let mut out = format!(
                "Open Pull Requests — Total: {}\n{}\n",
                prs.viewer.pull_requests.total_count,
                "─".repeat(60),
            );

            if prs.viewer.pull_requests.nodes.is_empty() {
                out.push_str("\nNo open pull requests found.\n");
                return Ok(out);
            }

            for pr in &prs.viewer.pull_requests.nodes {
                let date_str = chrono::DateTime::parse_from_rfc3339(&pr.created_at)
                    .map(|dt| dt.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|_| pr.created_at.clone());

                out.push_str(&format!(
                    "\n🔀 #{} — {}\n   📦 {}\n   State: {}  Mergeable: {}  Created: {}\n",
                    pr.number,
                    pr.title,
                    pr.repository.name_with_owner,
                    pr.state,
                    pr.mergeable,
                    date_str,
                ));

                if let Some(reviews) = &pr.reviews {
                    if reviews.total_count > 0 {
                        out.push_str(&format!("   Reviews: {}\n", reviews.total_count));
                        for review in &reviews.nodes {
                            if let Some(author) = &review.author {
                                out.push_str(&format!(
                                    "     • {} by {}\n",
                                    review.state, author.login
                                ));
                            }
                        }
                    }
                }

                out.push_str(&format!("{}\n", "─".repeat(60)));
            }

            Ok(out)
        }),

        // ── Repo Sync ─────────────────────────────────────────────────────────
        "repo-sync" => run_async(async {
            use crate::auth::token_store;
            use crate::github::graphql::{self, GraphQLClient};
            use crate::github::sync_checker::{SyncChecker, SyncStatus};

            let token = token_store::load_token()
                .map_err(|_| "Not authenticated. Run /auth login first.".to_string())?;

            let repo_name = detect_local_repo_name()
                .ok_or_else(|| "No git repository found in current directory.".to_string())?;

            let client = GraphQLClient::new(token.clone());

            let repos_resp = graphql::fetch_repositories(&client, 50, true)
                .await
                .map_err(|e| e.to_string())?;

            let repo = repos_resp
                .viewer
                .repositories
                .nodes
                .iter()
                .find(|r| r.name == repo_name || r.name_with_owner.contains(&repo_name))
                .ok_or_else(|| format!("Repository '{}' not found on GitHub.", repo_name))?;

            let sync_checker = SyncChecker::new(GraphQLClient::new(token));
            let status = sync_checker.check_sync(repo, None).await.map_err(|e| e.to_string())?;

            let mut out = format!(
                "Repository Sync — {}\n{}\n\n{} {}\n",
                repo.name_with_owner,
                "─".repeat(60),
                status.emoji(),
                status.description(),
            );

            if let Some(local_path) = sync_checker.find_local_repo(&repo.name) {
                if let Ok(local_info) = sync_checker.get_local_info(&local_path) {
                    out.push_str(&format!(
                        "\n📁 Local Repository\n   Path:   {}\n   Branch: {}\n   Commit: {}\n",
                        local_path.display(),
                        local_info.current_branch,
                        &local_info.latest_commit[..8.min(local_info.latest_commit.len())],
                    ));
                    if local_info.uncommitted_changes {
                        out.push_str("   ⚠️  Uncommitted changes detected\n");
                    }
                }

                match &status {
                    SyncStatus::InSync => {
                        out.push_str("\n✅ SYNCHRONIZED — local and remote are at the same commit.\n");
                    }
                    SyncStatus::LocalAhead { commits } => {
                        out.push_str(&format!(
                            "\n⬆️  LOCAL AHEAD by {} commit(s).\n   Run: git push\n",
                            commits
                        ));
                    }
                    SyncStatus::RemoteAhead { commits } => {
                        out.push_str(&format!(
                            "\n⬇️  REMOTE AHEAD by {} commit(s).\n   Run: git pull\n",
                            commits
                        ));
                    }
                    SyncStatus::Diverged { local_ahead, remote_ahead } => {
                        out.push_str(&format!(
                            "\n🔀 DIVERGED — local +{}, remote +{}.\n   Run: git pull --rebase\n",
                            local_ahead, remote_ahead
                        ));
                    }
                    SyncStatus::BranchMismatch { local_branch, remote_branch } => {
                        out.push_str(&format!(
                            "\n🔄 BRANCH MISMATCH — local: {}, remote default: {}\n",
                            local_branch, remote_branch
                        ));
                    }
                    SyncStatus::NoLocalRepo => {}
                }
            } else {
                out.push_str(&format!(
                    "\n❌ Not cloned locally.\n   Run: git clone {}\n",
                    repo.ssh_url
                ));
            }

            let remote_info = graphql::fetch_repository_sync_info(
                &client,
                &repo.owner.login,
                &repo.name,
            )
                .await
                .map_err(|e| e.to_string())?;

            if let Some(remote_branch) = &remote_info.repository.default_branch_ref {
                let last_updated = remote_branch
                    .target
                    .committed_date
                    .as_deref()
                    .and_then(|d| chrono::DateTime::parse_from_rfc3339(d).ok())
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "N/A".to_string());

                out.push_str(&format!(
                    "\n🌐 Remote Repository\n   Branch:  {}\n   Commit:  {}\n   Updated: {}\n",
                    remote_branch.name,
                    &remote_branch.target.oid[..8.min(remote_branch.target.oid.len())],
                    last_updated,
                ));

                if let Some(history) = &remote_branch.target.history {
                    out.push_str(&format!("   Total commits: {}\n", history.total_count));
                }
            }

            out.push_str(&format!("\n{}\n", "─".repeat(60)));
            Ok(out)
        }),

        // ── Multi Sync ────────────────────────────────────────────────────────
        "multi-sync" => run_async(async {
            use crate::auth::token_store;
            use crate::github::graphql::{self, GraphQLClient};
            use crate::github::sync_checker::SyncChecker;

            let token = token_store::load_token()
                .map_err(|_| "Not authenticated. Run /auth login first.".to_string())?;
            let client = GraphQLClient::new(token.clone());

            let repos_resp = graphql::fetch_repositories(&client, 30, false)
                .await
                .map_err(|e| e.to_string())?;

            let repos = &repos_resp.viewer.repositories.nodes;
            let sync_checker = SyncChecker::new(GraphQLClient::new(token));

            let mut out = format!(
                "Multi-Repo Sync Status — {} repositories\n{}\n",
                repos.len(),
                "─".repeat(60),
            );

            for repo in repos {
                let status = match sync_checker.check_sync(repo, None).await {
                    Ok(s) => s,
                    Err(_) => {
                        out.push_str(&format!(
                            "  ⚠️  {} — error checking sync\n",
                            repo.name_with_owner
                        ));
                        continue;
                    }
                };
                out.push_str(&format!(
                    "  {} {} — {}\n",
                    status.emoji(),
                    repo.name_with_owner,
                    status.description()
                ));
            }

            out.push_str(&format!("\n{}\n", "─".repeat(60)));
            Ok(out)
        }),

        // ── Push Check ────────────────────────────────────────────────────────
        "push-check" => run_sync(|| {
            use crate::github::push_checker::check_push_status;
            use git2::Repository;

            let repo = Repository::discover(".")?;
            let head = repo.head()?;
            let branch = head
                .shorthand()
                .ok_or("Unable to determine current branch")?
                .to_string();

            let status = check_push_status(&branch)?;

            let icon = if status.can_push { "✅" } else { "⚠️ " };
            let mut out = format!(
                "Push Status — branch: {}\n{}\n\n{} {}\n",
                branch,
                "─".repeat(60),
                icon,
                status.message,
            );

            if !status.local_commit.is_empty() {
                out.push_str(&format!(
                    "\n📌 Local commit:  {}\n",
                    &status.local_commit[..8.min(status.local_commit.len())]
                ));
            }
            if !status.remote_commit.is_empty() {
                out.push_str(&format!(
                    "🌐 Remote commit: {}\n",
                    &status.remote_commit[..8.min(status.remote_commit.len())]
                ));
            }

            out.push_str(&format!(
                "\n📊 Details\n\
                 \n   Can push:             {}\
                 \n   In sync:              {}\
                 \n   Uncommitted changes:  {}\
                 \n   Unpushed commits:     {}\
                 \n   Remote ahead:         {}\
                 \n   Conflicts:            {}\n",
                bool_icon(status.can_push),
                bool_icon(status.is_synced),
                bool_warn(!status.has_uncommitted_changes),
                bool_warn(!status.has_unpushed_commits),
                bool_warn(!status.remote_ahead),
                bool_err(!status.has_conflicts),
            ));

            out.push_str(&format!("\n{}\n", "─".repeat(60)));
            Ok(out)
        }),

        // ── Push Verify ───────────────────────────────────────────────────────
        "push-verify" => run_sync(|| {
            use crate::github::push_checker::{check_push_status, generate_push_preview};
            use git2::Repository;

            let repo = Repository::discover(".")?;
            let head = repo.head()?;
            let branch = head
                .shorthand()
                .ok_or("Unable to determine current branch")?
                .to_string();

            let status = check_push_status(&branch)?;

            let icon = if status.can_push { "✅" } else { "⚠️ " };
            let mut out = format!(
                "Push Verify — branch: {}\n{}\n\n{} {}\n",
                branch,
                "─".repeat(60),
                icon,
                status.message,
            );

            out.push_str(&format!(
                "\n📊 Status\n\
                 \n   Can push:             {}\
                 \n   In sync:              {}\
                 \n   Uncommitted changes:  {}\
                 \n   Unpushed commits:     {}\
                 \n   Remote ahead:         {}\
                 \n   Conflicts:            {}\n",
                bool_icon(status.can_push),
                bool_icon(status.is_synced),
                bool_warn(!status.has_uncommitted_changes),
                bool_warn(!status.has_unpushed_commits),
                bool_warn(!status.remote_ahead),
                bool_err(!status.has_conflicts),
            ));

            if status.has_unpushed_commits {
                if let Ok(Some(preview)) = generate_push_preview(&branch) {
                    out.push_str(&format!(
                        "\n🚀 Push Preview — branch: {}\n{}\n   Unpushed commits: {}\n\n",
                        preview.branch,
                        "─".repeat(60),
                        preview.commits.len(),
                    ));

                    for commit in &preview.commits {
                        out.push_str(&format!(
                            "   🔑 {}  💬 {}\n      📊 {} files  +{} -{}\n\n",
                            commit.short_id,
                            commit.message,
                            commit.files_changed,
                            commit.insertions,
                            commit.deletions,
                        ));
                    }

                    out.push_str(&format!(
                        "   {}\n   TOTAL: {} files  +{} -{}\n",
                        "─".repeat(58),
                        preview.total_files,
                        preview.total_insertions,
                        preview.total_deletions,
                    ));
                }
            }

            if status.can_push {
                out.push_str("\n✅ You can safely push to this branch!\n");
            } else {
                out.push_str("\n⚠️  Action required before pushing:\n");
                if status.has_uncommitted_changes {
                    out.push_str("   • Commit your changes first\n");
                }
                if status.remote_ahead {
                    out.push_str("   • Pull remote changes first (git pull)\n");
                }
                if status.has_conflicts {
                    out.push_str("   • Resolve merge conflicts\n");
                }
            }

            out.push_str(&format!("\n{}\n", "─".repeat(60)));
            Ok(out)
        }),

        // ── Branches ─────────────────────────────────────────────────────────
        "branches" => run_async(async {
            use crate::auth::token_store;
            use crate::github::graphql::{self, GraphQLClient};

            let token = token_store::load_token()
                .map_err(|_| "Not authenticated. Run /auth login first.".to_string())?;

            let repo_name = detect_local_repo_name()
                .ok_or_else(|| "No git repository found in current directory.".to_string())?;

            let client = GraphQLClient::new(token);

            let repos_resp = graphql::fetch_repositories(&client, 50, true)
                .await
                .map_err(|e| e.to_string())?;

            let repo = repos_resp
                .viewer
                .repositories
                .nodes
                .iter()
                .find(|r| r.name == repo_name || r.name_with_owner.contains(&repo_name))
                .ok_or_else(|| format!("Repository '{}' not found on GitHub.", repo_name))?;

            let branches = graphql::fetch_branches(&client, &repo.owner.login, &repo.name)
                .await
                .map_err(|e| e.to_string())?;

            let mut out = format!(
                "Branches — {} (Total: {})\n{}\n",
                branches.repository.name_with_owner,
                branches.repository.refs.total_count,
                "─".repeat(60),
            );

            for branch in &branches.repository.refs.nodes {
                let date_str = branch
                    .target
                    .committed_date
                    .as_deref()
                    .and_then(|d| chrono::DateTime::parse_from_rfc3339(d).ok())
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "N/A".to_string());

                let author = branch
                    .target
                    .author
                    .as_ref()
                    .and_then(|a| a.name.as_deref())
                    .unwrap_or("unknown");

                out.push_str(&format!(
                    "\n🌿 {}\n   Commit: {}  Last commit: {}\n   Author: {}\n{}\n",
                    branch.name,
                    &branch.target.oid[..8.min(branch.target.oid.len())],
                    date_str,
                    author,
                    "─".repeat(60),
                ));
            }

            Ok(out)
        }),

        // ── Issues ────────────────────────────────────────────────────────────
        "issues" => run_async(async {
            use crate::auth::token_store;
            use crate::github::graphql::{self, GraphQLClient};

            let token = token_store::load_token()
                .map_err(|_| "Not authenticated. Run /auth login first.".to_string())?;
            let client = GraphQLClient::new(token);

            let issues = graphql::fetch_user_issues(&client, &["OPEN"], 20)
                .await
                .map_err(|e| e.to_string())?;

            let mut out = format!(
                "Open Issues — Total: {}\n{}\n",
                issues.viewer.issues.total_count,
                "─".repeat(60),
            );

            if issues.viewer.issues.nodes.is_empty() {
                out.push_str("\nNo open issues found.\n");
                return Ok(out);
            }

            for issue in &issues.viewer.issues.nodes {
                let date_str = chrono::DateTime::parse_from_rfc3339(&issue.created_at)
                    .map(|dt| dt.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|_| issue.created_at.clone());

                let author = issue
                    .author
                    .as_ref()
                    .map(|a| a.login.as_str())
                    .unwrap_or("unknown");

                out.push_str(&format!(
                    "\n📝 #{} — {}\n   State: {}  Created: {}  Author: {}\n   🔗 {}\n{}\n",
                    issue.number,
                    issue.title,
                    issue.state,
                    date_str,
                    author,
                    issue.url,
                    "─".repeat(60),
                ));
            }

            Ok(out)
        }),

        // ── User Info ─────────────────────────────────────────────────────────
        "user-info" => run_async(async {
            use crate::auth::token_store;
            use crate::github::client::GitHubClient;
            use serde::Deserialize;

            #[derive(Deserialize)]
            struct GitHubUser {
                login: String,
                name: Option<String>,
                public_repos: u32,
                followers: Option<u32>,
                following: Option<u32>,
                company: Option<String>,
                location: Option<String>,
                bio: Option<String>,
                created_at: Option<String>,
            }

            let token = token_store::load_token()
                .map_err(|_| "Not authenticated. Run /auth login first.".to_string())?;

            let gh = GitHubClient::new(token);
            let user: GitHubUser = gh
                .client()
                .get("https://api.github.com/user")
                .header("Authorization", gh.auth_header())
                .header("User-Agent", "gitlink")
                .send()
                .await
                .map_err(|e| e.to_string())?
                .json()
                .await
                .map_err(|e| e.to_string())?;

            let created = user
                .created_at
                .as_deref()
                .and_then(|d| chrono::DateTime::parse_from_rfc3339(d).ok())
                .map(|dt| dt.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "N/A".to_string());

            let mut out = format!("GitHub User Info\n{}\n\n", "─".repeat(60));

            out.push_str(&format!("  👤 Username:     {}\n", user.login));
            out.push_str(&format!("  📛 Name:         {}\n", user.name.as_deref().unwrap_or("N/A")));
            out.push_str(&format!("  📦 Public repos: {}\n", user.public_repos));
            out.push_str(&format!("  👥 Followers:    {}\n", user.followers.unwrap_or(0)));
            out.push_str(&format!("  👣 Following:    {}\n", user.following.unwrap_or(0)));
            out.push_str(&format!("  🏢 Company:      {}\n", user.company.as_deref().unwrap_or("N/A")));
            out.push_str(&format!("  📍 Location:     {}\n", user.location.as_deref().unwrap_or("N/A")));
            out.push_str(&format!("  📅 Member since: {}\n", created));

            if let Some(bio) = &user.bio {
                if !bio.is_empty() {
                    out.push_str(&format!("\n  💬 Bio:\n     {}\n", bio));
                }
            }

            out.push_str(&format!("\n{}\n", "─".repeat(60)));
            Ok(out)
        }),

        // ── Unknown ───────────────────────────────────────────────────────────
        other => OutputBlock {
            kind: OutputKind::Error,
            content: format!("Unknown command: /{}", other),
        },
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn run_sync<F>(f: F) -> OutputBlock
where
    F: FnOnce() -> Result<String, Box<dyn std::error::Error>>,
{
    match f() {
        Ok(msg)  => OutputBlock { kind: OutputKind::Success, content: msg },
        Err(e)   => OutputBlock { kind: OutputKind::Error,   content: e.to_string() },
    }
}

fn run_async<F>(fut: F) -> OutputBlock
where
    F: std::future::Future<Output = Result<String, String>>,
{
    // router::execute is called from spawn_blocking, which runs on a thread
    // with NO existing tokio context. block_in_place / Handle::current() would
    // panic. We build a fresh single-threaded runtime instead.
    let result = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())
        .and_then(|rt| rt.block_on(fut).map_err(|e| e));
    match result {
        Ok(msg)  => OutputBlock { kind: OutputKind::Success, content: msg },
        Err(msg) => OutputBlock { kind: OutputKind::Error,   content: msg },
    }
}

/// Detect the name of the git repository rooted in the current working directory.
fn detect_local_repo_name() -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;

    if output.status.success() {
        let path = String::from_utf8(output.stdout).ok()?;
        std::path::Path::new(path.trim())
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
    } else {
        None
    }
}

fn bool_icon(v: bool) -> &'static str {
    if v { "✅ Yes" } else { "❌ No" }
}

fn bool_warn(ok: bool) -> &'static str {
    if ok { "✅ No" } else { "⚠️  Yes" }
}

fn bool_err(ok: bool) -> &'static str {
    if ok { "✅ No" } else { "❌ Yes" }
}