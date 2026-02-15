use ignore::WalkBuilder;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

use crate::scanner::patterns::PATTERNS;
use crate::scanner::report::Finding;

const MAX_FILE_SIZE: u64 = 2_000_000; // 2MB

/// Shannon Entropy calculation to filter out non-random strings (false positives)
fn calculate_entropy(s: &str) -> f64 {
    let mut counts = [0usize; 256];
    for &b in s.as_bytes() {
        counts[b as usize] += 1;
    }
    let len = s.len() as f64;
    counts.iter().fold(0.0, |acc, &c| {
        if c == 0 { acc }
        else {
            let p = c as f64 / len;
            acc - p * p.log2()
        }
    })
}

pub fn scan_directory(root: &str) -> Vec<Finding> {
    let mut findings = Vec::new();

    let walker = WalkBuilder::new(root)
        .standard_filters(true)
        .hidden(false)
        .build();

    for entry in walker.filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_file() { continue; }

        if let Ok(metadata) = fs::metadata(path) {
            if metadata.len() > MAX_FILE_SIZE { continue; }
        }

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if content.contains('\0') { continue; }

        scan_file(path, &content, &mut findings);
    }
    findings
}

fn scan_file(path: &Path, content: &str, findings: &mut Vec<Finding>) {
    for (line_index, line) in content.lines().enumerate() {
        // --- ADDED: Skip Comment Lines to reduce noise ---
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") || trimmed.starts_with("#") || trimmed.starts_with("*") {
            continue;
        }

        for pattern in PATTERNS.iter() {
            for mat in pattern.regex.find_iter(line) {
                let matched_str = mat.as_str();

                // --- ADDED: Entropy Check ---
                // Secrets are random (high entropy). CSS colors/words are not (low entropy).
                if calculate_entropy(matched_str) < 3.2 {
                    continue;
                }

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
                    content: line.trim().to_string(),
                    fingerprint,
                });
            }
        }
    }
}

fn generate_fingerprint(file: &str, line: usize, content: &str, secret_type: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(file.as_bytes());
    hasher.update(line.to_string().as_bytes());
    hasher.update(content.as_bytes());
    hasher.update(secret_type.as_bytes());
    format!("{:x}", hasher.finalize())
}