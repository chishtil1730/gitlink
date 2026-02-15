use ignore::WalkBuilder;
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use git2::{ObjectType, Repository};

use crate::scanner::patterns::PATTERNS;
use crate::scanner::report::Finding;

const MAX_FILE_SIZE: u64 = 2_000_000; // 2MB
const ENTROPY_THRESHOLD: f64 = 4.3;
const MIN_SECRET_LENGTH: usize = 20;

pub fn scan_directory(root: &str) -> Vec<Finding> {
    let files: Vec<PathBuf> = WalkBuilder::new(root)
        .standard_filters(true)
        .hidden(false)
        .build()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
        .map(|e| e.into_path())
        .collect();

    files.par_iter().flat_map(|path| scan_file(path)).collect()
}

fn scan_file(path: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    // Skip large files
    if let Ok(metadata) = fs::metadata(path) {
        if metadata.len() > MAX_FILE_SIZE {
            return findings;
        }
    }

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return findings,
    };

    // Skip binary-like files
    if content.contains('\0') {
        return findings;
    }

    for (line_index, line) in content.lines().enumerate() {
        let line_number = line_index + 1;

        // ==================================================
        // 1️⃣ REGEX DETECTION (FIXED: uses captures_iter)
        // ==================================================
        for pattern in PATTERNS.iter() {
            for caps in pattern.regex.captures_iter(line) {
                let secret_match = caps.get(2).or_else(|| caps.get(0));
                let secret = match secret_match {
                    Some(m) => m.as_str(),
                    None => continue,
                };

                let column_number = secret_match.unwrap().start() + 1;

                let dedup_key =
                    format!("{}:{}:{}:{}", path.display(), line_number, pattern.name, secret);

                if !seen.insert(dedup_key) {
                    continue;
                }

                let fingerprint = generate_fingerprint(
                    &path.display().to_string(),
                    line_number,
                    line,
                    pattern.name,
                );

                findings.push(Finding {
                    secret_type: pattern.name.to_string(),
                    file: path.display().to_string(),
                    line: line_number,
                    column: column_number,
                    content: line.trim_end().to_string(),
                    fingerprint,
                    commit: None,
                });
            }
        }

        // ==================================================
        // 2️⃣ ENTROPY DETECTION (Improved)
        // ==================================================
        for token in extract_potential_tokens(line) {
            if token.len() < MIN_SECRET_LENGTH {
                continue;
            }

            // Skip obvious variable names
            if token.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
                continue;
            }

            let entropy = shannon_entropy(&token);

            if entropy >= ENTROPY_THRESHOLD {
                let column_number = line.find(&token).unwrap_or(0) + 1;

                let dedup_key =
                    format!("{}:{}:entropy:{}", path.display(), line_number, token);

                if !seen.insert(dedup_key) {
                    continue;
                }

                let fingerprint = generate_fingerprint(
                    &path.display().to_string(),
                    line_number,
                    line,
                    "High Entropy Secret",
                );

                findings.push(Finding {
                    secret_type: "High Entropy Secret".to_string(),
                    file: path.display().to_string(),
                    line: line_number,
                    column: column_number,
                    content: line.trim_end().to_string(),
                    fingerprint,
                    commit: None,
                });
            }
        }
    }

    findings
}

fn extract_potential_tokens(line: &str) -> Vec<String> {
    line.split_whitespace()
        .flat_map(|segment| {
            segment
                .trim_matches(|c: char| {
                    !c.is_ascii_alphanumeric() && c != '_' && c != '-' && c != '/' && c != '+'
                })
                .split(|c: char| {
                    !c.is_ascii_alphanumeric() && c != '_' && c != '-' && c != '/' && c != '+'
                })
                .filter(|token| token.len() >= MIN_SECRET_LENGTH)
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        })
        .collect()
}

fn shannon_entropy(input: &str) -> f64 {
    let mut freq: HashMap<char, usize> = HashMap::new();

    for c in input.chars() {
        *freq.entry(c).or_insert(0) += 1;
    }

    let len = input.len() as f64;

    freq.values()
        .map(|&count| {
            let p = count as f64 / len;
            -p * p.log2()
        })
        .sum()
}

fn generate_fingerprint(
    file: &str,
    line: usize,
    content: &str,
    secret_type: &str,
) -> String {
    let mut hasher = Sha256::new();

    hasher.update(file.as_bytes());
    hasher.update(line.to_string().as_bytes());
    hasher.update(content.as_bytes());
    hasher.update(secret_type.as_bytes());

    format!("{:x}", hasher.finalize())
}

//
// ===============================
// 🔥 GIT HISTORY SCANNING (FIXED)
// ===============================
//

use git2::{DiffOptions};
use chrono::{Utc, Duration};

pub fn scan_git_history(since_days: Option<i64>) -> Vec<Finding> {
    let mut findings = Vec::new();

    let repo = match Repository::discover(".") {
        Ok(r) => r,
        Err(_) => return findings,
    };

    let mut revwalk = repo.revwalk().unwrap();
    revwalk.push_head().unwrap();

    let mut seen: HashSet<String> = HashSet::new();

    // Compute cutoff timestamp if --since provided
    let cutoff_timestamp = since_days.map(|days| {
        (Utc::now() - Duration::days(days)).timestamp()
    });

    for oid in revwalk.flatten() {
        let commit = match repo.find_commit(oid) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Skip commits older than cutoff
        if let Some(cutoff) = cutoff_timestamp {
            if commit.time().seconds() < cutoff {
                continue;
            }
        }

        if commit.parent_count() == 0 {
            continue;
        }

        let parent = match commit.parent(0) {
            Ok(p) => p,
            Err(_) => continue,
        };

        let parent_tree = match parent.tree() {
            Ok(t) => t,
            Err(_) => continue,
        };

        let commit_tree = match commit.tree() {
            Ok(t) => t,
            Err(_) => continue,
        };

        let mut diff_opts = DiffOptions::new();
        diff_opts.include_unmodified(false);

        let diff = match repo.diff_tree_to_tree(
            Some(&parent_tree),
            Some(&commit_tree),
            Some(&mut diff_opts),
        ) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let commit_id = commit.id().to_string();

        diff.foreach(
            &mut |_, _| true,
            None,
            None,
            Some(&mut |delta, _, line| {

                if line.origin() != '+' {
                    return true;
                }

                let content = match std::str::from_utf8(line.content()) {
                    Ok(c) => c,
                    Err(_) => return true,
                };

                let file = delta.new_file().path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                let line_number = line.new_lineno().unwrap_or(0) as usize;

                scan_history_line(
                    &file,
                    content,
                    line_number,
                    &commit_id,
                    &mut findings,
                    &mut seen,
                );

                true
            }),
        ).ok();
    }

    findings
}



//helper func
fn scan_history_line(
    file: &str,
    line: &str,
    line_number: usize,
    commit_id: &str,
    findings: &mut Vec<Finding>,
    seen: &mut HashSet<String>,
) {
    for pattern in PATTERNS.iter() {
        for mat in pattern.regex.find_iter(line) {

            let fingerprint = generate_fingerprint(
                file,
                line_number,
                line,
                pattern.name,
            );

            let dedup_key = format!("{}:{}:{}", file, line_number, pattern.name);

            if seen.insert(dedup_key) {
                findings.push(Finding {
                    secret_type: pattern.name.to_string(),
                    file: file.to_string(),
                    line: line_number,
                    column: mat.start() + 1,
                    content: line.trim().to_string(),
                    fingerprint,
                    commit: Some(commit_id.to_string()),
                });
            }
        }
    }

    // Entropy detection
    for token in extract_potential_tokens(line) {
        if token.len() >= MIN_SECRET_LENGTH {

            if token.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
                continue;
            }

            let entropy = shannon_entropy(&token);

            if entropy >= ENTROPY_THRESHOLD {

                let fingerprint = generate_fingerprint(
                    file,
                    line_number,
                    line,
                    "High Entropy Secret",
                );

                let dedup_key = format!("{}:{}:entropy", file, line_number);

                if seen.insert(dedup_key) {
                    findings.push(Finding {
                        secret_type: "High Entropy Secret".to_string(),
                        file: file.to_string(),
                        line: line_number,
                        column: line.find(&token).unwrap_or(0) + 1,
                        content: line.trim().to_string(),
                        fingerprint,
                        commit: Some(commit_id.to_string()),
                    });
                }
            }
        }
    }
}


fn scan_history_blob(
    root: &str,
    name: &str,
    content: &str,
    commit_id: &str,
    findings: &mut Vec<Finding>,
) {
    let full_path = format!("{}{}", root, name);
    let mut seen = HashSet::new();

    for (line_index, line) in content.lines().enumerate() {
        let line_number = line_index + 1;

        for pattern in PATTERNS.iter() {
            for caps in pattern.regex.captures_iter(line) {
                let secret_match = caps.get(2).or_else(|| caps.get(0));
                let secret = match secret_match {
                    Some(m) => m.as_str(),
                    None => continue,
                };

                let dedup_key =
                    format!("{}:{}:{}:{}", full_path, line_number, pattern.name, secret);

                if !seen.insert(dedup_key) {
                    continue;
                }

                let fingerprint = generate_fingerprint(
                    &full_path,
                    line_number,
                    line,
                    pattern.name,
                );

                findings.push(Finding {
                    secret_type: pattern.name.to_string(),
                    file: full_path.clone(),
                    line: line_number,
                    column: secret_match.unwrap().start() + 1,
                    content: line.to_string(),
                    fingerprint,
                    commit: Some(commit_id.to_string()),
                });
            }
        }
    }
}
