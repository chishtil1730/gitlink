use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Finding {
    pub secret_type: String,
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub content: String,
    pub fingerprint: String,
    pub commit: Option<String>, // 👈 required
}
