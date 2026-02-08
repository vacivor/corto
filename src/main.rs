use std::net::SocketAddr;
use std::net::UdpSocket;

use crate::app::AppState;
use crate::config::config::AppConfig;
use crate::services::short_url_service::ShortUrlService;
use tracing_subscriber::EnvFilter;

mod app;
mod common;
mod config;
mod db;
mod handlers;
mod models;
mod problem;
mod routes;
mod services;
mod utils;

#[tokio::main]
async fn main() {
    let app_config = config::config::load_configuration().expect("Failed to load configuration");
    init_tracing(&app_config);
    let state = build_state(&app_config).await;
    let app = routes::routes(state);

    let socket_addr = build_socket_addr(&app_config);
    let listener = tokio::net::TcpListener::bind(socket_addr)
        .await
        .expect("failed to bind server listener");

    log_server_addresses(socket_addr);

    axum::serve(listener, app)
        .await
        .expect("server failed to run");
}

fn normalize_base_url(config: &AppConfig) -> Option<String> {
    let base_url = config.server.base_url.as_deref()?.trim();
    if base_url.is_empty() {
        return None;
    }
    Some(base_url.trim_end_matches('/').to_string())
}

async fn build_state(config: &AppConfig) -> AppState {
    let db = db::init_db(&config.datasource).await;
    AppState {
        short_url_service: ShortUrlService::new(db),
        base_url: normalize_base_url(config),
    }
}

fn build_socket_addr(config: &AppConfig) -> SocketAddr {
    let host = config.server.host.as_deref().unwrap_or("0.0.0.0");
    let port = config.server.port;
    format!("{}:{}", host, port)
        .parse()
        .expect("invalid server host or port")
}

fn init_tracing(config: &AppConfig) {
    let level = config.logging.level.as_str();
    let filter = EnvFilter::try_new(level)
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();
}

fn log_server_addresses(socket_addr: SocketAddr) {
    let local_ip = match get_local_ip() {
        Some(ip) => ip,
        None => "unknown".to_string(),
    };

    tracing::info!(
        "server running at http://127.0.0.1:{} and http://{}:{}",
        socket_addr.port(),
        local_ip,
        socket_addr.port(),
    );
    tracing::info!("bound on http://{}", socket_addr);
}

fn get_local_ip() -> Option<String> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let addr = socket.local_addr().ok()?;
    Some(addr.ip().to_string())
}
