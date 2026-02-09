use reqwest::Client;

pub struct GitHubClient {
    client: Client,
    token: String,
}

impl GitHubClient {
    pub fn new(token: String) -> Self {
        Self {
            client: Client::new(),
            token,
        }
    }

    pub fn auth_header(&self) -> String {
        // Correcting to ensure "Bearer" or "token" is used as per GitHub's specific API requirements
        format!("Bearer {}", self.token)
    }

    pub fn client(&self) -> &Client {
        &self.client
    }
}