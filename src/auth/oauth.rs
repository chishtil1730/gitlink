use reqwest::Client;
use serde::Deserialize;
use std::time::{Duration, Instant};
use tokio::time::sleep;

const CLIENT_ID: &str = "Ov23liVaeQzp77a16FuM";

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

pub struct DeviceFlowInfo {
    pub user_code: String,
    pub verification_uri: String,
    pub device_code: String,
    pub interval: u64,
    pub expires_in: u64,
}

/// Step 1: request a device code — no printing, returns data for the TUI overlay.
pub async fn request_device_code() -> Result<DeviceFlowInfo, Box<dyn std::error::Error>> {
    let client = Client::new();
    let resp = client
        .post("https://github.com/login/device/code")
        .header("Accept", "application/json")
        .header("User-Agent", "gitlink")
        .form(&[("client_id", CLIENT_ID), ("scope", "read:user repo")])
        .send()
        .await?;
    let data: DeviceCodeResponse = resp.json().await?;
    Ok(DeviceFlowInfo {
        user_code: data.user_code,
        verification_uri: data.verification_uri,
        device_code: data.device_code,
        interval: data.interval,
        expires_in: data.expires_in,
    })
}

/// Step 2: poll until authorized or expired — no printing.
pub async fn poll_for_token(info: &DeviceFlowInfo) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();
    let mut interval = Duration::from_secs(info.interval);
    let expires_at = Instant::now() + Duration::from_secs(info.expires_in);

    loop {
        if Instant::now() > expires_at {
            return Err("Device code expired. Please try again.".into());
        }
        sleep(interval).await;

        let resp = client
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .header("User-Agent", "gitlink")
            .form(&[
                ("client_id", CLIENT_ID),
                ("device_code", &info.device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await?;

        let text = resp.text().await?;

        if let Ok(t) = serde_json::from_str::<AccessTokenResponse>(&text) {
            return Ok(t.access_token);
        }
        if let Ok(e) = serde_json::from_str::<ErrorResponse>(&text) {
            match e.error.as_str() {
                "authorization_pending" => continue,
                "slow_down" => { interval += Duration::from_secs(5); continue; }
                "expired_token" => return Err("Device code expired.".into()),
                "access_denied"  => return Err("Authorization denied by user.".into()),
                other => return Err(format!("GitHub error: {}", other).into()),
            }
        }
        return Err(format!("Unexpected response: {}", text).into());
    }
}

/// Legacy combined login (kept for router compatibility).
pub async fn login() -> Result<String, Box<dyn std::error::Error>> {
    let info = request_device_code().await?;
    let _ = open::that(&info.verification_uri);
    poll_for_token(&info).await
}