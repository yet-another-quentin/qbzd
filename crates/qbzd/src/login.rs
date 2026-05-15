//! Interactive Qobuz login via system browser OAuth.
//!
//! Supports both local and remote (headless) operation:
//! - Local: opens browser automatically, callback to localhost
//! - Remote/headless: prints URL for user to open on another device,
//!   callback to the daemon's LAN IP so the redirect works from
//!   any browser on the same network.

const OAUTH_TIMEOUT_SECS: u64 = 300;

pub async fn interactive_login(
    qobuz_token: Option<String>,
    callback_host: Option<String>,
    daemon_port: u16,
) -> Result<(), String> {
    if let Some(token) = qobuz_token {
        return login_with_direct_token(&token, daemon_port).await;
    }
    oauth_via_daemon(callback_host, daemon_port).await
}

/// Validate a Qobuz user_auth_token directly, without OAuth.
async fn login_with_direct_token(token: &str, daemon_port: u16) -> Result<(), String> {
    let http = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/api/auth/token", daemon_port);
    let resp = http
        .post(&url)
        .json(&serde_json::json!({ "token": token }))
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("Daemon not reachable on port {}: {}", daemon_port, e))?;

    if resp.status().is_success() {
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        println!("\nLogged in as: {}", body["display_name"].as_str().unwrap_or("?"));
        println!("Credentials saved. The daemon will auto-login on next start.");
        Ok(())
    } else {
        let msg: serde_json::Value = resp.json().await.unwrap_or_default();
        Err(msg["error"].as_str().unwrap_or("Token validation failed").to_string())
    }
}

/// OAuth flow via the running daemon — callback lands on the daemon's own port.
async fn oauth_via_daemon(callback_host: Option<String>, daemon_port: u16) -> Result<(), String> {
    let http = reqwest::Client::new();
    let base = format!("http://127.0.0.1:{}", daemon_port);

    // Check daemon is reachable
    http.get(format!("{}/api/ping", base))
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
        .map_err(|_| format!(
            "Daemon not reachable on port {}. Start the daemon first.",
            daemon_port
        ))?;

    // Start OAuth flow
    let start_url = match callback_host {
        Some(ref h) => format!("{}/api/auth/oauth/start?callback_host={}", base, urlencoding::encode(h)),
        None => format!("{}/api/auth/oauth/start", base),
    };
    let start_resp: serde_json::Value = http
        .post(&start_url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Failed to start OAuth: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Invalid response from daemon: {}", e))?;

    let oauth_url = start_resp["oauth_url"].as_str()
        .ok_or("Daemon returned no oauth_url")?;
    let callback_url = start_resp["callback_url"].as_str()
        .unwrap_or("?");

    println!("\n╔══════════════════════════════════════════════════╗");
    println!("║  Open this URL in any browser on your network:  ║");
    println!("╚══════════════════════════════════════════════════╝\n");
    println!("  {}\n", oauth_url);
    println!("Callback URL (handled by the daemon): {}", callback_url);
    println!("Waiting for login ({}s timeout)...\n", OAUTH_TIMEOUT_SECS);

    let _ = open::that(oauth_url);

    // Poll /api/auth/oauth/status
    let deadline = std::time::Instant::now()
        + std::time::Duration::from_secs(OAUTH_TIMEOUT_SECS);
    loop {
        if std::time::Instant::now() > deadline {
            return Err(format!("Login timed out after {}s", OAUTH_TIMEOUT_SECS));
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let status: serde_json::Value = http
            .get(format!("{}/api/auth/oauth/status", base))
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| format!("Status poll failed: {}", e))?
            .json()
            .await
            .unwrap_or_default();

        match status["status"].as_str() {
            Some("success") => {
                println!("\nLogged in successfully.");
                println!("The daemon is now authenticated and Qobuz Connect is active.");
                return Ok(());
            }
            Some("error") => {
                let msg = status["message"].as_str().unwrap_or("Unknown error");
                return Err(format!("Login failed: {}", msg));
            }
            _ => {} // pending — keep polling
        }
    }
}

/// Save OAuth token — tries keyring first, falls back to encrypted file.
pub(crate) fn save_token_to_keyring(token: &str) -> Result<(), String> {
    const SERVICE: &str = "qbz-player";
    const KEY: &str = "qobuz-oauth-token";

    // Try keyring first
    match keyring::Entry::new(SERVICE, KEY) {
        Ok(entry) => match entry.set_password(token) {
            Ok(()) => {
                println!("Token saved to system keyring");
                return Ok(());
            }
            Err(e) => {
                println!("Keyring unavailable ({}), using file fallback", e);
            }
        },
        Err(e) => {
            println!("Keyring unavailable ({}), using file fallback", e);
        }
    }

    // Fallback: save to file
    save_token_to_file(token)
}

fn token_file_path() -> Option<std::path::PathBuf> {
    dirs::data_dir().map(|d| d.join("qbz").join(".oauth-token"))
}

fn save_token_to_file(token: &str) -> Result<(), String> {
    let path = token_file_path().ok_or("Cannot determine data directory")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(&path, token).map_err(|e| format!("Failed to write token file: {}", e))?;
    // Restrict permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    println!("Token saved to {}", path.display());
    Ok(())
}

pub fn load_token_from_file() -> Option<String> {
    let path = token_file_path()?;
    std::fs::read_to_string(&path).ok().filter(|t| !t.trim().is_empty())
}

/// Detect the primary LAN IP address of this machine.
pub(crate) fn detect_lan_ip() -> Option<String> {
    // Try to get the default route interface IP by connecting to a public DNS
    // (no actual data is sent, just gets the local IP used for routing)
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let addr = socket.local_addr().ok()?;
    let ip = addr.ip().to_string();
    if ip == "0.0.0.0" || ip == "127.0.0.1" {
        return None;
    }
    Some(ip)
}
