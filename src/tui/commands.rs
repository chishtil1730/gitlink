#[derive(Debug, Clone)]
pub struct Command {
    pub name: &'static str,
    pub description: &'static str,
}

pub static COMMANDS: &[Command] = &[
    Command { name: "help",          description: "Show all available commands" },
    Command { name: "auth",          description: "Manage GitHub authentication (login/logout/status)" },
    Command { name: "auth login",    description: "Authenticate with GitHub via OAuth" },
    Command { name: "auth logout",   description: "Remove stored GitHub token" },
    Command { name: "auth status",   description: "Show current authentication status" },
    Command { name: "scan",          description: "Run the secret scanner on current directory" },
    Command { name: "scan history",  description: "Scan git history for exposed secrets" },
    Command { name: "scan ignored",  description: "List all permanently ignored findings" },
    Command { name: "plan",          description: "Open the task planner (exits TUI temporarily)" },
    Command { name: "prp",           description: "Start a poly-repo commit session" },
    Command { name: "prp list",      description: "List all PRP session groups" },
    Command { name: "show-activity", description: "Show your GitHub contribution activity" },
    Command { name: "clear",         description: "Clear the output history" },
    Command { name: "quit",          description: "Exit GitLink TUI" },
];