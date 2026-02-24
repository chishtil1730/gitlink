use crate::tui::app::{OutputBlock, OutputKind};

/// Execute a slash command string and return an OutputBlock.
/// This is the single bridge between the TUI and all backend modules.
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
                "  /scan              — Scan working directory for secrets (interactive)",
                "  /scan history      — Scan git history for secrets",
                "  /scan ignored      — List ignored findings",
                "  /plan              — Open task planner overlay",
                "  /prp               — Start a poly-repo commit session",
                "  /prp list          — List PRP session groups",
                "  /show-activity     — Show GitHub contribution activity",
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

        // ── Scan (ignored listing only — active scan handled in tui/mod.rs) ───
        // ── Scan ──────────────────────────────────────────────────────────────
        // ── Scan ──────────────────────────────────────────────────────────────
        "scan" => match sub {
            // 1. List ignored findings via the TUI
            "ignored" => {
                let content = crate::scanner::ignore::get_ignored_list_string();
                OutputBlock {
                    kind: OutputKind::Info,
                    content,
                }
            },

            // 2. Clear the ignore database
            "clear" => run_sync(|| {
                crate::scanner::ignore::clear_all_silent(); // We'll add this silent version below
                Ok("All ignored findings have been cleared.".to_string())
            }),

            // 3. Base command: Intercepted by TUI to open interactive overlay
            "" => OutputBlock {
                kind: OutputKind::Info,
                content: "Opening scanner overlay...".to_string(),
            },

            // 4. Trigger for history scanning
            "history" => OutputBlock {
                kind: OutputKind::Info,
                content: "SCAN_HISTORY_SIGNAL".to_string(),
            },

            _ => OutputBlock {
                kind: OutputKind::Error,
                content: format!(
                    "Unknown scan subcommand: '{}'. Try /scan, /scan ignored, or /scan clear.",
                    sub
                ),
            },
        },

        // ── Plan (intercepted in tui/mod.rs) ──────────────────────────────────
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
                 \nLast 3 days:\n{}",
                activity.viewer.name.as_deref().unwrap_or("N/A"),
                activity.viewer.login,
                contrib.contribution_calendar.total_contributions,
                contrib.total_commit_contributions,
                contrib.total_pull_request_contributions,
                contrib.total_issue_contributions,
                contrib.total_repository_contributions,
                recent.join("\n"),
            ))
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
        Ok(msg) => OutputBlock { kind: OutputKind::Success, content: msg },
        Err(e)  => OutputBlock { kind: OutputKind::Error,   content: e.to_string() },
    }
}

/// Run an async future on the current tokio runtime without spawning a new one.
/// All errors must be mapped to String before reaching this function to avoid
/// the Send + Sync bound that Box<dyn Error> cannot satisfy inside async blocks.
fn run_async<F>(fut: F) -> OutputBlock
where
    F: std::future::Future<Output = Result<String, String>>,
{
    let result = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(fut)
    });
    match result {
        Ok(msg)  => OutputBlock { kind: OutputKind::Success, content: msg },
        Err(msg) => OutputBlock { kind: OutputKind::Error,   content: msg },
    }
}