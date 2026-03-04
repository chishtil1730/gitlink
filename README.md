# GitLink

> A fast, keyboard-driven terminal companion for GitHub — built in Rust.

GitLink brings your GitHub workflow into the terminal with a full TUI interface. Check sync status across repos, scan for leaked secrets, manage tasks, commit across multiple repos at once, and monitor your contribution activity — without leaving your editor.

---

## Features

### Terminal UI (`/tui`)
A rich TUI built with [ratatui](https://github.com/ratatui-org/ratatui). All commands are typed as slash commands with autocomplete, suggestion navigation, and command history.

### GitHub Integration
- **`/show-activity`** — contribution stats and a full 365-day heatmap grid, just like the GitHub profile page
- **`/commits`** — your 3 most recent commits globally, sorted across all repos
- **`/pull-requests`** — open pull requests with state, mergeability, and review summary
- **`/repo-sync`** — sync status of your current local repo against its GitHub remote
- **`/multi-sync`** — check sync status across all your GitHub repos with fuzzy search and batch selection
- **`/branches`** — local and remote branches with last commit info
- **`/issues`** — your open issues across GitHub
- **`/user-info`** — profile info via the REST API

### Secret Scanner
- **`/scan`** — scans your working directory for leaked secrets
- **`/scan history`** — scans your full git commit history
- **`/scan ignored`** — manage permanently ignored findings

Detects: AWS Access Keys, AWS Secret Keys, GitHub Tokens, JWT Tokens, Stripe Secret Keys, generic API keys/tokens, and PEM private keys. Uses both regex pattern matching and Shannon entropy analysis. Intelligently skips build artifacts, lock files, compiled binaries, and other known false-positive sources.

### Poly-Repo Hub (`/prp`)
Commit and push to multiple git repositories in a single session. Discovers all repos in the current directory, validates their state, shows a diff preview, takes a single commit message, and applies it atomically — with automatic rollback on failure.

- **`/prp`** — start an interactive poly-repo commit session
- **`/prp list`** — view past session groups

### Task Planner (`/plan`)
A built-in task manager overlay. Add, edit, delete, and complete tasks with titles, tags, and descriptions. Full undo/redo history. Tasks persist locally.

### Authentication
- **`/auth`** — GitHub OAuth device flow (no PAT needed — just open a URL and approve)
- **`/auth logout`** — remove stored token
- **`/auth status`** — check current auth state

---

## Installation

### cargo install
```sh
cargo install gitlink
```

### Pre-built binaries
Download the latest binary for your platform from the [Releases](https://github.com/YOUR_USERNAME/gitlink/releases) page.

| Platform | File |
|---|---|
| Windows x64 | `gitlink-windows-x86_64.exe` |
| Linux x64 | `gitlink-linux-x86_64` |
| macOS x64 | `gitlink-macos-x86_64` |
| macOS ARM (M1/M2/M3) | `gitlink-macos-arm64` |

### Homebrew (macOS)
```sh
brew tap YOUR_USERNAME/tap
brew install gitlink
```

### Scoop (Windows)
```sh
scoop bucket add gitlink https://github.com/YOUR_USERNAME/scoop-bucket
scoop install gitlink
```

---

## Building from source

**Requirements:** Rust 1.75+, a GitHub OAuth App client ID.

```sh
git clone https://github.com/YOUR_USERNAME/gitlink
cd gitlink

# Set your OAuth App client ID (create one at github.com/settings/developers)
export GITLINK_CLIENT_ID=your_client_id   # Linux/macOS
$env:GITLINK_CLIENT_ID="your_client_id"   # Windows PowerShell

cargo build --release
./target/release/gitlink tui
```

---

## Usage

```sh
# launch the TUI
gitlink            
```

Once inside the TUI, type `/` to see all available commands with descriptions. Use `↑↓` or `Tab` to navigate and autocomplete suggestions.

### All commands

| Command | Description |
|---|---|
| `/auth` | Manage GitHub authentication |
| `/scan` | Scan working directory for secrets |
| `/scan history` | Scan git commit history for secrets |
| `/scan ignored` | Manage ignored findings |
| `/plan` | Open the task planner |
| `/prp` | Start a poly-repo commit session |
| `/prp list` | List PRP session groups |
| `/show-activity` | GitHub contribution stats + 365-day heatmap |
| `/commits` | 3 most recent commits globally |
| `/pull-requests` | Open pull requests |
| `/repo-sync` | Sync status of current local repo |
| `/multi-sync` | Sync status across all your repos |
| `/push-check` | Check if latest commit is pushed |
| `/push-verify` | Verify push is possible + preview unpushed commits |
| `/branches` | Local and remote branches |
| `/issues` | Open issues |
| `/user-info` | GitHub profile info |
| `/clear` | Clear output history |
| `/quit` | Exit |

### TUI keyboard shortcuts

| Key | Action |
|---|---|
| `/` | Start typing a command |
| `↑` `↓` | Navigate suggestions / scroll output |
| `Tab` | Autocomplete selected suggestion |
| `Enter` | Run command |
| `Ctrl+C` | Quit |
| `Esc` | Close overlay |
| `↑` `↓` (in overlay) | Navigate list |
| `Space` | Toggle selection (multi-sync, prp) |
| `/` (in search) | Open search |
| `//` (in search) | Close search |

---

## Configuration

GitLink stores data in the following locations:

| File | Purpose |
|---|---|
| `.gitlinkignore.json` | Permanently ignored scanner findings (project root) |
| `~/.gitlink/tasks.json` | Task planner data |
| `~/.gitlink/prp_groups.json` | PRP session history |
| System keychain / credential store | GitHub OAuth token |

The secret scanner respects `.gitignore` and additionally skips `target/`, `node_modules/`, build artifacts, lock files, binary files, and compiler-generated files automatically.

---

## OAuth App Setup

GitLink uses GitHub's device OAuth flow — no personal access token required.

1. Go to [github.com/settings/developers](https://github.com/settings/developers) → **OAuth Apps** → **New OAuth App**
2. Set **Application name** to `GitLink` (or anything you like)
3. Set **Homepage URL** to your repo URL
4. Leave **Authorization callback URL** blank (device flow doesn't use it)
5. Copy the **Client ID** and set it as `GITLINK_CLIENT_ID` at build time

Users authenticate by running `/auth` in the TUI — they'll see a code to enter at `github.com/login/device`. No token is ever stored in plaintext.

---

## License

MIT — see [LICENSE](LICENSE).
