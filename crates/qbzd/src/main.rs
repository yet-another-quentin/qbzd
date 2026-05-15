mod api;
mod config;
mod daemon;
mod adapter;
mod login;
mod qconnect;
mod resources;
mod session;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "qbzd", about = "QBZ headless music daemon")]
#[command(version)]
struct Cli {
    /// HTTP port
    #[arg(short, long, default_value_t = 8182)]
    port: u16,

    /// Bind address
    #[arg(short, long, default_value = "0.0.0.0")]
    bind: String,

    /// Data directory
    #[arg(short, long)]
    data_dir: Option<String>,

    /// Config file path
    #[arg(short, long)]
    config: Option<String>,

    /// Auth token (auto-generated if not provided)
    #[arg(long)]
    token: Option<String>,

    /// Log level
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Print a new token and exit
    #[arg(long)]
    generate_token: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate with Qobuz via OAuth (daemon must be running).
    Login {
        /// Direct Qobuz user_auth_token — skips the browser OAuth flow.
        #[arg(long)]
        token: Option<String>,
        /// Public base URL used as OAuth callback, e.g. "http://192.168.1.10:8182".
        /// Defaults to http://<LAN-IP>:<port>. Use "http://localhost:8182" in Docker.
        #[arg(long)]
        callback_host: Option<String>,
    },
    /// Show daemon status
    Status,
    /// Show or regenerate API token
    Token,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Init logging
    let level = cli.log_level.parse().unwrap_or(log::LevelFilter::Info);
    env_logger::Builder::new()
        .filter_level(level)
        .filter_module("zbus", log::LevelFilter::Warn)
        .filter_module("tracing", log::LevelFilter::Warn)
        .filter_module("mio", log::LevelFilter::Warn)
        .filter_module("hyper", log::LevelFilter::Warn)
        .filter_module("reqwest", log::LevelFilter::Warn)
        .filter_module("symphonia", log::LevelFilter::Warn)
        .filter_module("tokio_tungstenite", log::LevelFilter::Warn)
        .filter_module("tungstenite", log::LevelFilter::Warn)
        .format_timestamp_millis()
        .init();

    log::info!("qbzd starting...");

    // Load config (TOML file -> env vars -> CLI overrides)
    let mut cfg = config::DaemonConfig::load(cli.config.as_deref());

    // CLI overrides
    cfg.server.port = cli.port;
    cfg.server.bind = cli.bind.clone();
    if let Some(ref dir) = cli.data_dir {
        cfg.data.dir = dir.clone();
    }
    if let Some(ref token) = cli.token {
        cfg.server.token = token.clone();
    }

    // Generate token and exit
    if cli.generate_token {
        let token = config::generate_token();
        println!("{}", token);
        return;
    }

    // Auto-detect resources
    resources::auto_detect_cache_config(&mut cfg);

    // Handle subcommands
    match cli.command {
        Some(Commands::Login { token, callback_host }) => {
            if let Err(e) = login::interactive_login(token, callback_host, cfg.server.port).await {
                eprintln!("Login failed: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Status) => {
            let port = cfg.server.port;
            match reqwest::Client::new()
                .get(format!("http://127.0.0.1:{}/api/status", port))
                .timeout(std::time::Duration::from_secs(3))
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    let body: serde_json::Value = resp.json().await.unwrap_or_default();
                    println!("qbzd running on port {}", port);
                    println!("{}", serde_json::to_string_pretty(&body).unwrap_or_default());
                }
                Ok(resp) => {
                    println!("qbzd responded with HTTP {}", resp.status());
                }
                Err(_) => {
                    println!("qbzd not running (no response on port {})", port);
                }
            }
        }
        Some(Commands::Token) => {
            if cfg.server.token == "auto" {
                let token = config::generate_token();
                println!("Generated token: {}", token);
            } else {
                println!("Current token: {}", cfg.server.token);
            }
        }
        None => {
            // Main daemon mode
            if let Err(e) = daemon::run(cfg).await {
                log::error!("Daemon exited with error: {}", e);
                std::process::exit(1);
            }
        }
    }
}
