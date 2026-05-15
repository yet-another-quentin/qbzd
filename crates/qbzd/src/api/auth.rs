use std::sync::Arc;
use axum::{extract::Query, response::Html};
use axum::Json;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::daemon::DaemonCore;

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum OAuthStatus {
    Idle,
    Pending,
    Success,
    Error { message: String },
}

pub struct OAuthState {
    pub status: OAuthStatus,
}

impl Default for OAuthState {
    fn default() -> Self {
        Self { status: OAuthStatus::Idle }
    }
}

pub type SharedOAuthState = Arc<Mutex<OAuthState>>;

// ── Handlers ──────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct StartQuery {
    /// Base URL reachable by the browser, e.g. "http://localhost:8182".
    /// Defaults to http://<detected-LAN-IP>:<daemon-port>.
    callback_host: Option<String>,
}

#[derive(Serialize)]
pub struct StartResponse {
    pub oauth_url: String,
    pub callback_url: String,
}

pub async fn start(
    daemon: Arc<DaemonCore>,
    Query(q): Query<StartQuery>,
) -> Result<Json<StartResponse>, (axum::http::StatusCode, String)> {
    let client_arc = daemon.core.client();
    let client_guard = client_arc.read().await;
    let client = client_guard
        .as_ref()
        .ok_or((axum::http::StatusCode::SERVICE_UNAVAILABLE, "Qobuz client not ready".into()))?;

    let app_id = client.app_id().await.map_err(|e| {
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("app_id error: {}", e))
    })?;
    drop(client_guard);

    let port = daemon.config.server.port;
    let callback_host = q.callback_host.unwrap_or_else(|| {
        crate::login::detect_lan_ip()
            .map(|ip| format!("http://{}:{}", ip, port))
            .unwrap_or_else(|| format!("http://localhost:{}", port))
    });

    let callback_url = format!("{}/api/auth/oauth/callback", callback_host);
    let oauth_url = format!(
        "https://www.qobuz.com/signin/oauth?ext_app_id={}&redirect_url={}",
        app_id,
        urlencoding::encode(&callback_url),
    );

    *daemon.oauth.lock().await = OAuthState { status: OAuthStatus::Pending };

    Ok(Json(StartResponse { oauth_url, callback_url }))
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    code_autorisation: Option<String>,
    code: Option<String>,
    error: Option<String>,
}

pub async fn callback(
    daemon: Arc<DaemonCore>,
    Query(q): Query<CallbackQuery>,
) -> Html<String> {
    if let Some(err) = q.error {
        let msg = format!("Qobuz returned an error: {}", err);
        set_error(&daemon, msg.clone()).await;
        return Html(error_page(&msg));
    }

    let code = match q.code_autorisation.or(q.code).filter(|c| !c.is_empty()) {
        Some(c) => c,
        None => {
            let msg = "No authorization code received from Qobuz.";
            set_error(&daemon, msg.into()).await;
            return Html(error_page(msg));
        }
    };

    // Exchange code in a background task — return the HTML page immediately.
    let daemon_clone = daemon.clone();
    tokio::spawn(async move {
        match exchange_and_activate(&daemon_clone, &code).await {
            Ok(()) => {
                daemon_clone.oauth.lock().await.status = OAuthStatus::Success;
                log::info!("[auth] OAuth login successful");
            }
            Err(e) => {
                log::error!("[auth] OAuth exchange failed: {}", e);
                set_error(&daemon_clone, e).await;
            }
        }
    });

    Html(success_page())
}

pub async fn status(daemon: Arc<DaemonCore>) -> Json<OAuthStatus> {
    Json(daemon.oauth.lock().await.status.clone())
}

#[derive(Deserialize)]
pub struct TokenBody {
    token: String,
}

#[derive(Serialize)]
pub struct TokenResponse {
    display_name: String,
}

pub async fn set_token(
    daemon: Arc<DaemonCore>,
    axum::Json(body): axum::Json<TokenBody>,
) -> Result<Json<TokenResponse>, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    let err = |msg: &str| (
        axum::http::StatusCode::BAD_REQUEST,
        axum::Json(serde_json::json!({ "error": msg })),
    );

    let session = {
        let client_arc = daemon.core.client();
        let client_guard = client_arc.read().await;
        let client = client_guard.as_ref().ok_or_else(|| err("Qobuz client not ready"))?;
        client
            .login_with_token(&body.token)
            .await
            .map_err(|e| err(&format!("Token validation failed: {}", e)))?
    };

    let display_name = session.display_name.clone();
    let user_id = session.user_id;
    let token = session.user_auth_token.clone();

    tokio::task::spawn_blocking(move || crate::login::save_token_to_keyring(&token))
        .await
        .map_err(|e| err(&format!("spawn_blocking error: {}", e)))?
        .map_err(|e| err(&e))?;

    match crate::session::activate_session(user_id, &daemon.core, &daemon.event_bus).await {
        Ok(user_session) => { *daemon.user.write().await = Some(user_session); }
        Err(e) => log::error!("[auth] Session activation failed: {}", e),
    }

    crate::daemon::start_qconnect_if_needed(&daemon).await;

    Ok(Json(TokenResponse { display_name }))
}

// ── Internals ─────────────────────────────────────────────────────────────────

async fn exchange_and_activate(daemon: &Arc<DaemonCore>, code: &str) -> Result<(), String> {
    // Exchange code → session
    let session = {
        let client_arc = daemon.core.client();
        let client_guard = client_arc.read().await;
        let client = client_guard
            .as_ref()
            .ok_or("Qobuz client not ready")?;
        client
            .login_with_oauth_code(code)
            .await
            .map_err(|e| format!("OAuth exchange failed: {}", e))?
    };

    let user_id = session.user_id;

    // Persist token (keyring/file) — blocking call, must leave the runtime.
    let token = session.user_auth_token.clone();
    tokio::task::spawn_blocking(move || crate::login::save_token_to_keyring(&token))
        .await
        .map_err(|e| format!("spawn_blocking error: {}", e))??;

    // Activate per-user session (same path as auto-login at startup).
    match crate::session::activate_session(user_id, &daemon.core, &daemon.event_bus).await {
        Ok(user_session) => {
            *daemon.user.write().await = Some(user_session);
            log::info!("[auth] User session activated (user_id: {})", user_id);
        }
        Err(e) => log::error!("[auth] Session activation failed: {}", e),
    }

    // Start QConnect now that we have a valid session.
    crate::daemon::start_qconnect_if_needed(daemon).await;

    Ok(())
}

async fn set_error(daemon: &Arc<DaemonCore>, message: String) {
    daemon.oauth.lock().await.status = OAuthStatus::Error { message };
}

// ── HTML pages ────────────────────────────────────────────────────────────────

fn success_page() -> String {
    r#"<!doctype html>
<html><head><meta charset="utf-8"><title>Qobuz Login</title></head>
<body style="font-family:system-ui;text-align:center;padding:60px">
  <h2>&#10003; Login successful</h2>
  <p>You can close this tab and return to the terminal.</p>
</body></html>"#.into()
}

fn error_page(msg: &str) -> String {
    format!(
        r#"<!doctype html>
<html><head><meta charset="utf-8"><title>Qobuz Login</title></head>
<body style="font-family:system-ui;text-align:center;padding:60px">
  <h2>&#10007; Login failed</h2>
  <p>{}</p>
</body></html>"#,
        msg
    )
}
