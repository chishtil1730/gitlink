use std::path::PathBuf;
use std::collections::HashSet;

use walkdir::WalkDir;

use crate::prp_hub::errors::PrpError;
use crate::prp_hub::types::RepositoryInfo;

/// Recursively discover all git repositories under `root`, including root itself.
/// Nested repositories are treated independently.
pub fn discover_repositories(root: &str) -> Result<Vec<RepositoryInfo>, PrpError> {
    let root_path = std::fs::canonicalize(root)
        .map_err(|e| PrpError::DiscoveryError(e.to_string()))?;

    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut repos: Vec<RepositoryInfo> = Vec::new();

    // WalkDir will traverse all subdirectories; we look for .git entries.
    // When we find a `.git` directory, its parent is a repo root.
    for entry in WalkDir::new(&root_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // We're looking for a `.git` file or directory
        if path.file_name().map(|n| n == ".git").unwrap_or(false) {
            if let Some(repo_root) = path.parent() {
                let canonical = match std::fs::canonicalize(repo_root) {
                    Ok(p) => p,
                    Err(_) => continue,
                };

                if seen.contains(&canonical) {
                    continue;
                }
                seen.insert(canonical.clone());

                let name = canonical
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| canonical.to_string_lossy().into_owned());

                repos.push(RepositoryInfo {
                    name,
                    path: canonical,
                });
            }
        }
    }

    if repos.is_empty() {
        return Err(PrpError::NoRepositoriesFound);
    }

    // Sort for deterministic output
    repos.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(repos)
}