use std::thread;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct DeviceCodeInfo {
    pub user_code: String,
    pub verification_uri: String,
    pub device_code: String,
    pub interval: u64,
    pub expires_in: u64,
}

#[derive(Debug)]
pub enum AuthStatus {
    Pending,
    Expired,
    Authorized { access_token: String },
    Error(String),
}

pub fn request_device_code(server_url: &str) -> Result<DeviceCodeInfo, String> {
    let url = format!("{}/api/auth/device", server_url.trim_end_matches('/'));
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(&url)
        .json(&serde_json::json!({ "client_name": "SyncNotes Desktop" }))
        .send()
        .map_err(|e| format!("Failed to connect: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Server returned {}", resp.status()));
    }

    let data: serde_json::Value = resp
        .json()
        .map_err(|e| format!("Invalid response: {}", e))?;

    Ok(DeviceCodeInfo {
        user_code: data["user_code"]
            .as_str()
            .unwrap_or("????-????")
            .to_string(),
        verification_uri: data["verification_uri"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        device_code: data["device_code"].as_str().unwrap_or("").to_string(),
        interval: data["interval"].as_u64().unwrap_or(5),
        expires_in: data["expires_in"].as_u64().unwrap_or(600),
    })
}

pub fn poll_auth_status(
    server_url: &str,
    device_code: &str,
    interval: u64,
    expires_in: u64,
    on_status: &dyn Fn(AuthStatus),
) {
    let url = format!(
        "{}/api/auth/device/status?device_code={}",
        server_url.trim_end_matches('/'),
        device_code
    );
    let client = reqwest::blocking::Client::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(expires_in);

    while std::time::Instant::now() < deadline {
        thread::sleep(Duration::from_secs(interval));

        match client.get(&url).send() {
            Ok(resp) => {
                if resp.status() == 200 {
                    let data: serde_json::Value =
                        resp.json().unwrap_or(serde_json::Value::Null);
                    if data["status"] == "authorized" {
                        if let Some(token) = data["access_token"].as_str() {
                            on_status(AuthStatus::Authorized {
                                access_token: token.to_string(),
                            });
                            return;
                        }
                    }
                    on_status(AuthStatus::Pending);
                } else if resp.status() == 400 {
                    on_status(AuthStatus::Expired);
                    return;
                } else {
                    on_status(AuthStatus::Error(format!("HTTP {}", resp.status())));
                    return;
                }
            }
            Err(e) => {
                on_status(AuthStatus::Error(format!("Connection failed: {}", e)));
                return;
            }
        }
    }

    on_status(AuthStatus::Expired);
}

pub fn verify_token(server_url: &str, token: &str) -> bool {
    let url = format!(
        "{}/api/auth/verify",
        server_url.trim_end_matches('/')
    );
    let client = reqwest::blocking::Client::new();
    match client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
    {
        Ok(resp) => resp.status() == 200,
        Err(_) => false,
    }
}
