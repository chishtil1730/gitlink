use ignore::WalkBuilder;
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::scanner::patterns::PATTERNS;
use crate::scanner::report::Finding;

const MAX_FILE_SIZE: u64 = 2_000_000; // 2MB
const ENTROPY_THRESHOLD: f64 = 4.5;
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

    files
        .par_iter()
        .flat_map(|path| scan_file(path))
        .collect()
}

fn scan_file(path: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();

    if let Ok(metadata) = fs::metadata(path) {
        if metadata.len() > MAX_FILE_SIZE {
            return findings;
        }
    }

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return findings,
    };

    if content.contains('\0') {
        return findings;
    }

    for (line_index, line) in content.lines().enumerate() {
        let line_number = line_index + 1;

        // 1️⃣ Regex detection
        for pattern in PATTERNS.iter() {
            for mat in pattern.regex.find_iter(line) {
                let column_number = mat.start() + 1;

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

        // 2️⃣ Entropy detection (secondary)
        for token in extract_potential_tokens(line) {
            if token.len() >= MIN_SECRET_LENGTH {
                let entropy = shannon_entropy(&token);

                if entropy >= ENTROPY_THRESHOLD {
                    let column_number = line.find(&token).unwrap_or(0) + 1;

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
    }

    findings
}

fn extract_potential_tokens(line: &str) -> Vec<String> {
    line.split(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-' && c != '/' && c != '+')
        .filter(|token| token.len() >= MIN_SECRET_LENGTH)
        .map(|s| s.to_string())
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

use git2::{Repository, ObjectType};
use std::collections::HashSet;

pub fn scan_git_history() -> Vec<Finding> {
    let mut findings = Vec::new();

    let repo = match Repository::discover(".") {
        Ok(r) => r,
        Err(_) => return findings,
    };

    let mut revwalk = repo.revwalk().unwrap();
    revwalk.push_head().unwrap();

    let mut visited_blobs = HashSet::new();

    for oid_result in revwalk {
        let oid = match oid_result {
            Ok(o) => o,
            Err(_) => continue,
        };

        let commit = match repo.find_commit(oid) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let tree = match commit.tree() {
            Ok(t) => t,
            Err(_) => continue,
        };

        let commit_id = commit.id().to_string();

        tree.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
            if entry.kind() == Some(ObjectType::Blob) {
                let blob_oid = entry.id();

                // Avoid scanning same blob multiple times
                if !visited_blobs.insert(blob_oid) {
                    return git2::TreeWalkResult::Ok;
                }

                if let Ok(blob) = repo.find_blob(blob_oid) {
                    if let Ok(content) = std::str::from_utf8(blob.content()) {
                        scan_history_blob(
                            root,
                            entry.name().unwrap_or("unknown"),
                            content,
                            &commit_id,
                            &mut findings,
                        );
                    }
                }
            }

            git2::TreeWalkResult::Ok
        }).unwrap_or(());
    }

    findings
}
fn scan_history_blob(
    root: &str,
    name: &str,
    content: &str,
    commit_id: &str,
    findings: &mut Vec<Finding>,
) {
    let full_path = format!("{}{}", root, name);

    for (line_index, line) in content.lines().enumerate() {
        let line_number = line_index + 1;

        // Regex detection
        for pattern in PATTERNS.iter() {
            for mat in pattern.regex.find_iter(line) {
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
                    column: mat.start() + 1,
                    content: line.to_string(),
                    fingerprint,
                    commit: Some(commit_id.to_string()),
                });
            }
        }

        // Entropy detection
        for token in extract_potential_tokens(line) {
            if token.len() >= MIN_SECRET_LENGTH {
                let entropy = shannon_entropy(&token);

                if entropy >= ENTROPY_THRESHOLD {
                    let fingerprint = generate_fingerprint(
                        &full_path,
                        line_number,
                        line,
                        "High Entropy Secret",
                    );

                    findings.push(Finding {
                        secret_type: "High Entropy Secret".to_string(),
                        file: full_path.clone(),
                        line: line_number,
                        column: line.find(&token).unwrap_or(0) + 1,
                        content: line.to_string(),
                        fingerprint,
                        commit: Some(commit_id.to_string()),
                    });
                }
            }
        }
    }
}

