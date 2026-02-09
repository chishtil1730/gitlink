use open::that;
use reqwest::Client;
use serde::Deserialize;
use std::env;
use std::sync::mpsc;
use std::thread;
use tiny_http::{Response, Server};

const CALLBACK_ADDR: &str = "127.0.0.1:7878";
const CALLBACK_PATH: &str = "/callback";

#[derive(Deserialize)]
struct AccessTokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct GitHubUser {
    login: String,
}



#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1️⃣ Load credentials
    let client_id = env::var("GITLINK_CLIENT_ID")
        .expect("GITLINK_CLIENT_ID not set");
    let client_secret = env::var("GITLINK_CLIENT_SECRET")
        .expect("GITLINK_CLIENT_SECRET not set");

    // 2️⃣ Channel to receive OAuth code
    let (tx, rx) = mpsc::channel();

    // 3️⃣ Start local callback server
    thread::spawn(move || {
        let server =
            Server::http(CALLBACK_ADDR).expect("Failed to start callback server");

        for request in server.incoming_requests() {
            if request.url().starts_with(CALLBACK_PATH) {
                // FIX: own the URL string
                let url = request.url().to_string();

                let code = url
                    .split("code=")
                    .nth(1)
                    .and_then(|s| s.split('&').next())
                    .expect("No code in callback");

                let response = Response::from_string(
                    "Authorization complete. You can close this window.",
                );

                let _ = request.respond(response);

                let _ = tx.send(code.to_string());
                break;
            }
        }
    });

    // 4️⃣ Open browser to GitHub OAuth
    let auth_url = format!(
        "https://github.com/login/oauth/authorize?client_id={}&redirect_uri=http://{}{}&scope=read:user repo",
        client_id, CALLBACK_ADDR, CALLBACK_PATH
    );

    println!("Opening browser for GitHub authorization...");
    println!("If it does not open, visit:\n{}", auth_url);

    that(auth_url)?;

    // 5️⃣ Wait for authorization code
    let code = rx.recv().expect("Failed to receive OAuth code");

    // 6️⃣ Exchange code for access token
    let client = Client::new();

    let token_response: AccessTokenResponse = client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("code", code.as_str()),
        ])
        .send()
        .await?
        .json()
        .await?;

    let token = token_response.access_token;

    // 7️⃣ Fetch GitHub user (dummy data)
    let user: GitHubUser = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "gitlink")
        .send()
        .await?
        .json()
        .await?;

    println!("Authenticated as: {}", user.login);

    Ok(())
}
