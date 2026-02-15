use regex::Regex;
use once_cell::sync::Lazy;

#[derive(Debug, Clone)]
pub struct SecretPattern {
    pub name: &'static str,
    pub regex: Regex,
}

pub static PATTERNS: Lazy<Vec<SecretPattern>> = Lazy::new(|| {
    vec![
        SecretPattern {
            name: "AWS Access Key",
            regex: Regex::new(r"\bAKIA[0-9A-Z]{16}\b").unwrap(),
        },

        SecretPattern {
            name: "AWS Secret Key",
            regex: Regex::new(
                r#"(?i)(aws_secret_access_key|secret_access_key)\s*[:=]\s*['"]?[A-Za-z0-9/+]{40}['"]?"#
            ).unwrap(),
        },

        SecretPattern {
            name: "Generic API Key / Token",
            regex: Regex::new(
                r#"(?i)\b(api[_-]?key|token|secret|auth[_-]?key)\b\s*[:=]\s*['"]?[A-Za-z0-9_\-]{16,}['"]?"#
            ).unwrap(),
        },

        SecretPattern {
            name: "JWT Token",
            regex: Regex::new(
                r"\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b"
            ).unwrap(),
        },

        SecretPattern {
            name: "Private Key",
            regex: Regex::new(
                r"-----BEGIN (RSA|EC|OPENSSH|DSA) PRIVATE KEY-----"
            ).unwrap(),
        },

        SecretPattern {
            name: "GitHub Token",
            regex: Regex::new(r"\bghp_[A-Za-z0-9]{36}\b").unwrap(),
        },

        SecretPattern {
            name: "Stripe Secret Key",
            regex: Regex::new(r"\bsk_live_[A-Za-z0-9]{24,}\b").unwrap(),
        },
    ]
});
