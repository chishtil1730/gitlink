#[derive(Debug, Clone)]
pub struct Command {
    pub name: &'static str,
    pub description: &'static str,
}

pub static COMMANDS: &[Command] = &[
    Command { name: "help",             description: "Show all available commands" },
    Command { name: "auth",             description: "Manage GitHub authentication (login/logout/status)" },
    Command { name: "scan",             description: "Run the secret scanner on current directory" },
    Command { name: "scan history",     description: "Scan git history for exposed secrets" },
    Command { name: "scan ignored",     description: "List all permanently ignored findings" },
    Command { name: "plan",             description: "Open the task planner" },
    Command { name: "prp",              description: "Start a poly-repo commit session" },
    Command { name: "prp list",         description: "List all PRP session groups" },
    Command { name: "show-activity",    description: "Show your GitHub contribution activity" },
    Command { name: "commits",          description: "Show recent commits for a repository" },
    Command { name: "pull-requests",    description: "Show open pull requests" },
    Command { name: "repo-sync",        description: "Select a repository and check sync status" },
    Command { name: "multi-sync",       description: "Check sync status across multiple repositories" },
    Command { name: "push-check",       description: "Check if latest commit is pushed to remote" },
    Command { name: "push-verify",      description: "Verify if pushing is currently possible" },
    Command { name: "branches",         description: "Show local and remote branches" },
    Command { name: "issues",           description: "Show issues and GitHub Actions status" },
    Command { name: "user-info",        description: "Show basic GitHub user info (REST API)" },
    Command { name: "clear",            description: "Clear the output history" },
    Command { name: "quit",             description: "Exit GitLink TUI" },
];