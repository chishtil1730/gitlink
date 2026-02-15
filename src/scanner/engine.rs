use ignore::WalkBuilder;
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

use crate::scanner::patterns::PATTERNS;
use crate::scanner::report::Finding;

const MAX_FILE_SIZE: u64 = 2_000_000; // 2MB

pub fn scan_directory(root: &str) -> Vec<Finding> {
    // Collect file paths first (sequential walk)
    let files: Vec<PathBuf> = WalkBuilder::new(root)
        .standard_filters(true)
        .hidden(false)
        .build()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
        .map(|e| e.into_path())
        .collect();

    // Parallel scan across files
    files
        .par_iter()
        .flat_map(|path| scan_file(path))
        .collect()
}

fn scan_file(path: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Skip large files
    if let Ok(metadata) = fs::metadata(path) {
        if metadata.len() > MAX_FILE_SIZE {
            return findings;
        }
    }

    // Read file as text
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return findings, // skip unreadable/binary
    };

    // Skip binary-like files
    if content.contains('\0') {
        return findings;
    }

    for (line_index, line) in content.lines().enumerate() {
        for pattern in PATTERNS.iter() {
            for mat in pattern.regex.find_iter(line) {
                let line_number = line_index + 1;
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
                });
            }
        }
    }

    findings
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
