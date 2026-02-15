use open::that;
use reqwest::Client;
use serde::Deserialize;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tokio::select;
use tokio::io::{self, AsyncBufReadExt};

// Client ID is baked in at compile time
const CLIENT_ID: &str = env!("GITLINK_CLIENT_ID");

#[derive(Deserialize, Debug)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Deserialize, Debug)]
struct AccessTokenResponse {
    access_token: String,
}

#[derive(Deserialize, Debug)]
struct ErrorResponse {
    error: String,
}

pub async fn login() -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();

    println!("🔐 Initiating GitHub Device Flow authentication...\n");

    // Step 1: Request device + user code
    let device_response = client
        .post("https://github.com/login/device/code")
        .header("Accept", "application/json")
        .header("User-Agent", "gitlink")
        .form(&[
            ("client_id", CLIENT_ID),
            ("scope", "read:user repo"),
        ])
        .send()
        .await?;

    let device_data: DeviceCodeResponse = device_response.json().await?;

    // Step 2: Display code clearly
    println!("\n========================================");
    println!("📋 Your verification code: {}", device_data.user_code);
    println!("🌐 Verification URL: {}", device_data.verification_uri);
    println!("========================================\n");

    println!("Press Enter to open browser...");
    println!("Opening browser in 8 seconds...\n");

    let mut reader = io::BufReader::new(io::stdin());
    let mut input = String::new();

    // Countdown task
    let countdown = async {
        for i in (1..=8).rev() {
            println!("Opening browser in {}...", i);
            sleep(Duration::from_secs(1)).await;
        }
    };

    // Wait for Enter
    let enter_pressed = async {
        let _ = reader.read_line(&mut input).await;
    };

    // Whichever happens first: countdown finishes OR Enter pressed
    select! {
        _ = countdown => {},
        _ = enter_pressed => {
            println!("Opening browser...");
        }
    }

    // Open browser
    that(&device_data.verification_uri)?;

    println!("\nWaiting for authorization...\n");

    // Step 3: Poll for access token
    let mut interval = Duration::from_secs(device_data.interval);
    let expires_at = Instant::now() + Duration::from_secs(device_data.expires_in);

    loop {
        if Instant::now() > expires_at {
            return Err("Device code expired. Please try again.".into());
        }

        sleep(interval).await;

        let token_response = client
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .header("User-Agent", "gitlink")
            .form(&[
                ("client_id", CLIENT_ID),
                ("device_code", &device_data.device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await?;

        let text = token_response.text().await?;

        // Try parsing access token
        if let Ok(token_res) = serde_json::from_str::<AccessTokenResponse>(&text) {
            println!("✅ Authorization successful!");
            return Ok(token_res.access_token);
        }

        // Try parsing error
        if let Ok(error_res) = serde_json::from_str::<ErrorResponse>(&text) {
            match error_res.error.as_str() {
                "authorization_pending" => {
                    print!("⏳ ");
                    std::io::Write::flush(&mut std::io::stdout())?;
                    continue;
                }
                "slow_down" => {
                    interval += Duration::from_secs(5);
                    continue;
                }
                "expired_token" => {
                    return Err("Device code expired. Please try again.".into());
                }
                "access_denied" => {
                    return Err("Authorization was denied by the user.".into());
                }
                _ => {
                    return Err(format!("GitHub returned an error: {}", error_res.error).into());
                }
            }
        }

        return Err(format!("Unexpected response from GitHub: {}", text).into());
    }
}
