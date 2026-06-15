use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

mod ai;
mod brief;
mod collectors;
mod config;
mod models;
mod routes;
mod scheduler;
mod whatsapp;

use config::AppConfig;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub http: reqwest::Client,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "personal_intel_agent=info,tower_http=info".into()),
        )
        .init();

    let config = AppConfig::from_env();
    let http = reqwest::Client::builder()
        .user_agent("personal-intel-agent/0.1")
        .timeout(std::time::Duration::from_secs(45))
        .build()
        .expect("failed to build HTTP client");

    let state = AppState {
        config: Arc::new(config),
        http,
    };

    if state.config.enable_daily_briefing {
        let scheduler_state = state.clone();
        tokio::spawn(async move {
            scheduler::run_daily_scheduler(scheduler_state).await;
        });
    }

    let app = Router::new()
        .route("/", get(routes::health))
        .route("/health", get(routes::health))
        .route("/api/briefing/run", post(routes::run_briefing_now))
        .route("/bridge/incoming", post(routes::whatsapp_incoming))
        .layer(CorsLayer::permissive())
        .with_state(state.clone());

    let addr = format!("0.0.0.0:{}", state.config.port);
    tracing::info!("Personal Intelligence Agent listening on {addr}");
    tracing::info!(
        "Daily briefing enabled: {}",
        state.config.enable_daily_briefing
    );

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
