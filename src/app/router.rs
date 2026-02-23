use crate::tui::app::{OutputBlock, OutputKind};

/// Execute a slash command string and return an OutputBlock.
/// This is the only place the TUI layer reaches into the backend.
pub fn execute(raw: &str) -> OutputBlock {
    // Strip leading slash and split into parts
    let input = raw.trim_start_matches('/');
    let parts: Vec<&str> = input.split_whitespace().collect();
    let cmd = parts.first().copied().unwrap_or("");
    let sub = parts.get(1).copied().unwrap_or("");

    match cmd {
        // ── Help ──────────────────────────────────────────────────
        "help" => OutputBlock {
            kind: OutputKind::Info,
            content: [
                "Available commands:",
                "  /auth              — Manage GitHub authentication",
                "  /auth login        — Login via OAuth",
                "  /auth logout       — Remove stored token",
                "  /auth status       — Show auth status",
                "  /scan              — Scan working directory for secrets",
                "  /scan history      — Scan git history for secrets",
                "  /scan ignored      — List ignored findings",
                "  /plan              — Open the task planner",
                "  /prp               — Start a poly-repo commit session",
                "  /prp list          — List PRP session groups",
                "  /show-activity     — Show GitHub contribution activity",
                "  /clear             — Clear output history",
                "  /quit              — Exit GitLink TUI",
                "",
                "  Use ↑↓ to navigate suggestions, Tab to autocomplete.",
                "  Ctrl+C or Ctrl+Q to quit.",
            ]
                .join("\n"),
        },

        // ── Quit ─────────────────────────────────────────────────
        "quit" => {
            // Signal quit by returning a special marker the loop can check.
            // For now we surface a message; the quit path is handled in on_key via Ctrl+C.
            OutputBlock {
                kind: OutputKind::Info,
                content: "Press Ctrl+C to exit GitLink TUI.".to_string(),
            }
        }

        // ── Clear ────────────────────────────────────────────────
        "clear" => {
            // Clearing is handled specially: the router returns a marker kind.
            // ui loop checks for this and drains app.outputs.
            OutputBlock {
                kind: OutputKind::Info,
                content: "__CLEAR__".to_string(),
            }
        }

        // ── Auth ─────────────────────────────────────────────────
        "auth" => match sub {
            "login" => run_blocking(|| {
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(async {
                    use crate::auth::{oauth, token_store};
                    let token = oauth::login().await?;
                    token_store::save_token(&token)?;
                    Ok::<_, Box<dyn std::error::Error>>(
                        "Authenticated with GitHub successfully.".to_string(),
                    )
                })
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
                content: format!("Unknown auth subcommand: '{}'. Try /auth login, /auth logout, or /auth status.", sub),
            },
        },

        // ── Scan ─────────────────────────────────────────────────
        "scan" => match sub {
            "history" => run_sync(|| {
                let mut findings = crate::scanner::engine::scan_git_history(None);
                let ignore_db = crate::scanner::ignore::load_ignore_db();
                findings.retain(|f| {
                    !ignore_db.ignored.iter().any(|i| i.fingerprint == f.fingerprint)
                });
                if findings.is_empty() {
                    return Ok("No secrets found in git history.".to_string());
                }
                let lines: Vec<String> = findings
                    .iter()
                    .map(|f| {
                        format!(
                            "  {}:{}  [{}]  {}",
                            f.file,
                            f.line,
                            f.secret_type,
                            f.content.trim()
                        )
                    })
                    .collect();
                Ok(format!(
                    "Found {} secret(s) in git history:\n{}",
                    findings.len(),
                    lines.join("\n")
                ))
            }),

            "ignored" => run_sync(|| {
                let db = crate::scanner::ignore::load_ignore_db();
                if db.ignored.is_empty() {
                    return Ok("No ignored findings.".to_string());
                }
                let lines: Vec<String> = db
                    .ignored
                    .iter()
                    .map(|i| format!("  [{}] {} ({})", i.short_id, i.variable, i.source))
                    .collect();
                Ok(format!("Ignored findings:\n{}", lines.join("\n")))
            }),

            "" => run_sync(|| {
                let mut findings = crate::scanner::engine::scan_directory(".");
                let ignore_db = crate::scanner::ignore::load_ignore_db();
                findings.retain(|f| {
                    !ignore_db.ignored.iter().any(|i| i.fingerprint == f.fingerprint)
                });
                if findings.is_empty() {
                    return Ok("No secrets found in working directory.".to_string());
                }
                let lines: Vec<String> = findings
                    .iter()
                    .map(|f| {
                        format!(
                            "  {}:{}  [{}]  {}",
                            f.file,
                            f.line,
                            f.secret_type,
                            f.content.trim()
                        )
                    })
                    .collect();
                Ok(format!(
                    "Found {} secret(s):\n{}",
                    findings.len(),
                    lines.join("\n")
                ))
            }),

            _ => OutputBlock {
                kind: OutputKind::Error,
                content: format!(
                    "Unknown scan subcommand: '{}'. Try /scan, /scan history, or /scan ignored.",
                    sub
                ),
            },
        },

        // ── Plan ─────────────────────────────────────────────────
        "plan" => OutputBlock {
            kind: OutputKind::Info,
            content: "Launching planner... (exits TUI temporarily, press Q inside planner to return)".to_string(),
        },

        // ── PRP ──────────────────────────────────────────────────
        "prp" => match sub {
            "list" => run_sync(|| {
                // Capture stdout from run_prp_list
                crate::prp_hub::run_prp_list()?;
                Ok("PRP group list printed above.".to_string())
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

        // ── Show Activity ────────────────────────────────────────
        "show-activity" => run_blocking(|| {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                use crate::auth::token_store;
                use crate::github::graphql::{self, GraphQLClient};

                let token = token_store::load_token()
                    .map_err(|_| "Not authenticated. Run /auth login first.")?;
                let client = GraphQLClient::new(token);
                let activity = graphql::fetch_user_activity(&client).await?;
                let contrib = &activity.viewer.contributions_collection;

                let all_days: Vec<_> = contrib
                    .contribution_calendar
                    .weeks
                    .iter()
                    .flat_map(|w| &w.contribution_days)
                    .collect();

                let recent: Vec<String> = all_days
                    .iter()
                    .rev()
                    .take(3)
                    .map(|d| {
                        let bar = "█".repeat(d.contribution_count.min(20) as usize);
                        format!("  {} : {} ({})", d.date, bar, d.contribution_count)
                    })
                    .collect();

                Ok::<_, Box<dyn std::error::Error>>(format!(
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
            })
        }),

        // ── Unknown ───────────────────────────────────────────────
        other => OutputBlock {
            kind: OutputKind::Error,
            content: format!("Unknown command: /{}", other),
        },
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Run a sync closure, wrapping Ok → Success and Err → Error.
fn run_sync<F>(f: F) -> OutputBlock
where
    F: FnOnce() -> Result<String, Box<dyn std::error::Error>>,
{
    match f() {
        Ok(msg) => OutputBlock { kind: OutputKind::Success, content: msg },
        Err(e) => OutputBlock { kind: OutputKind::Error, content: e.to_string() },
    }
}

/// Run a blocking async closure (creates a temporary Tokio runtime).
fn run_blocking<F>(f: F) -> OutputBlock
where
    F: FnOnce() -> Result<String, Box<dyn std::error::Error>>,
{
    run_sync(f)
}