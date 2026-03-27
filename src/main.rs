mod config;
mod handlers;
mod ytdlp;

use axum::{Router, routing::{get, post}};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = config::Config::from_env();
    let addr = format!("0.0.0.0:{}", config.port);

    let app = Router::new()
        .route("/health", get(handlers::health))
        .route("/api/v1/info", post(handlers::info))
        .route("/api/v1/audio", post(handlers::audio))
        .route("/api/v1/subtitles", post(handlers::subtitles))
        .route("/api/v1/audio/split", post(handlers::audio_split))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(config.clone());

    tracing::info!("yt-dlp-rs listening on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
