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

/// Perform browser-based OAuth login and return an access token
pub async fn login() -> Result<String, Box<dyn std::error::Error>> {
    let client_id = env::var("GITLINK_CLIENT_ID")
        .expect("GITLINK_CLIENT_ID not set");
    let client_secret = env::var("GITLINK_CLIENT_SECRET")
        .expect("GITLINK_CLIENT_SECRET not set");

    let (tx, rx) = mpsc::channel();

    // Start local callback server
    thread::spawn(move || {
        let server =
            Server::http(CALLBACK_ADDR).expect("Failed to start callback server");

        for request in server.incoming_requests() {
            if request.url().starts_with(CALLBACK_PATH) {
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

    let auth_url = format!(
        "https://github.com/login/oauth/authorize?client_id={}&redirect_uri=http://{}{}&scope=read:user repo",
        client_id, CALLBACK_ADDR, CALLBACK_PATH
    );

    println!("Opening browser for GitHub authorization...");
    println!("If it does not open, visit:\n{}", auth_url);

    that(auth_url)?;

    let code = rx.recv().expect("Failed to receive OAuth code");

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

    Ok(token_response.access_token)
}
