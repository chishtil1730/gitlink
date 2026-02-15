use regex::Regex;
use once_cell::sync::Lazy;

#[derive(Debug, Clone)]
pub struct SecretPattern {
    pub name: &'static str,
    pub regex: Regex,
}

pub static PATTERNS: Lazy<Vec<SecretPattern>> = Lazy::new(|| {
    vec![
        // ------------------------------------------------------------
        // AWS Access Key ID (Safe & precise)
        // ------------------------------------------------------------
        SecretPattern {
            name: "AWS Access Key",
            regex: Regex::new(r"\bAKIA[0-9A-Z]{16}\b").unwrap(),
        },

        // ------------------------------------------------------------
        // AWS Secret Access Key (Requires assignment context)
        // ------------------------------------------------------------
        SecretPattern {
            name: "AWS Secret Key",
            regex: Regex::new(
                r#"(?i)\b(aws_secret_access_key|secret_access_key)\b\s*[:=]\s*['"]([A-Za-z0-9/+]{40})['"]"#
            ).unwrap(),
        },

        // ------------------------------------------------------------
        // Generic API Keys / Tokens (Hardened)
        // ------------------------------------------------------------
        // Fixes:
        // - Detects api_key, api_key2, api_key_prod
        // - Requires quoted assignment
        // - Enforces minimum 20 chars
        // - Avoids matching random type names
        // ------------------------------------------------------------
        SecretPattern {
            name: "Generic API Key / Token",
            regex: Regex::new(
                r#"(?i)\b(api[_-]?key\w*|token\w*|secret\w*|auth[_-]?key\w*)\b\s*[:=]\s*['"]([A-Za-z0-9_\-]{20,})['"]"#
            ).unwrap(),
        },

        // ------------------------------------------------------------
        // JWT Tokens
        // ------------------------------------------------------------
        SecretPattern {
            name: "JWT Token",
            regex: Regex::new(
                r"\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b"
            ).unwrap(),
        },

        // ------------------------------------------------------------
        // Private Keys (PEM format)
        // ------------------------------------------------------------
        SecretPattern {
            name: "Private Key",
            regex: Regex::new(
                r"-----BEGIN (RSA|EC|OPENSSH|DSA) PRIVATE KEY-----"
            ).unwrap(),
        },

        // ------------------------------------------------------------
        // GitHub Personal Access Token
        // ------------------------------------------------------------
        SecretPattern {
            name: "GitHub Token",
            regex: Regex::new(r"\bghp_[A-Za-z0-9]{36}\b").unwrap(),
        },

        // ------------------------------------------------------------
        // Stripe Live Secret Key
        // ------------------------------------------------------------
        SecretPattern {
            name: "Stripe Secret Key",
            regex: Regex::new(r"\bsk_live_[A-Za-z0-9]{24,}\b").unwrap(),
        },
    ]
});
