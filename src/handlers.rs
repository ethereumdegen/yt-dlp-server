use axum::{
    Json,
    body::Body,
    extract::State,
    http::{StatusCode, header},
    response::Response,
};
use serde::{Deserialize, Serialize};
use tokio_util::io::ReaderStream;

use crate::config::Config;
use crate::ytdlp;

type AppError = (StatusCode, String);

fn internal(e: anyhow::Error) -> AppError {
    tracing::error!("{e:#}");
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

// --- Health ---

#[derive(Serialize)]
pub struct HealthResponse {
    status: String,
    yt_dlp_version: String,
}

pub async fn health(State(config): State<Config>) -> Result<Json<HealthResponse>, AppError> {
    let version = ytdlp::version(&config).await.map_err(internal)?;
    Ok(Json(HealthResponse {
        status: "ok".into(),
        yt_dlp_version: version,
    }))
}

// --- Info ---

#[derive(Deserialize)]
pub struct InfoRequest {
    url: String,
}

pub async fn info(
    State(config): State<Config>,
    Json(req): Json<InfoRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let info = ytdlp::get_info(&config, &req.url).await.map_err(internal)?;
    Ok(Json(info))
}

// --- Audio ---

#[derive(Deserialize)]
pub struct AudioRequest {
    url: String,
    #[serde(default = "default_audio_format")]
    format: String,
    #[serde(default = "default_audio_quality")]
    quality: String,
}

fn default_audio_format() -> String {
    "m4a".into()
}
fn default_audio_quality() -> String {
    "5".into()
}

pub async fn audio(
    State(config): State<Config>,
    Json(req): Json<AudioRequest>,
) -> Result<Response, AppError> {
    // Check duration first
    let info = ytdlp::get_info(&config, &req.url).await.map_err(internal)?;
    let _title = info
        .get("title")
        .and_then(|t| t.as_str())
        .unwrap_or("audio");
    let video_id = info
        .get("id")
        .and_then(|t| t.as_str())
        .unwrap_or("unknown");

    let tmp_dir = tempfile::tempdir().map_err(|e| internal(e.into()))?;
    let file_path = ytdlp::download_audio(&config, &req.url, &req.format, &req.quality, tmp_dir.path())
        .await
        .map_err(internal)?;

    let content_type = match req.format.as_str() {
        "mp3" => "audio/mpeg",
        "m4a" => "audio/mp4",
        "opus" => "audio/opus",
        "wav" => "audio/wav",
        _ => "application/octet-stream",
    };

    let ext = file_path
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_else(|| req.format.clone());
    let filename = format!("{video_id}.{ext}");

    let file = tokio::fs::File::open(&file_path)
        .await
        .map_err(|e| internal(e.into()))?;
    let stream = ReaderStream::new(file);

    // Keep tmp_dir alive by moving it into a task that waits for the response to finish.
    // Actually, we need to keep it alive for the duration of the stream.
    // We'll leak the TempDir handle so it gets cleaned up when dropped after streaming.
    let tmp_dir = std::sync::Arc::new(tmp_dir);
    let tmp_dir_clone = tmp_dir.clone();

    // Spawn cleanup task
    tokio::spawn(async move {
        // Keep reference alive for 5 minutes max, then clean up
        tokio::time::sleep(std::time::Duration::from_secs(300)).await;
        drop(tmp_dir_clone);
    });

    let body = Body::from_stream(stream);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{filename}\""),
        )
        .body(body)
        .unwrap())
}

// --- Subtitles ---

#[derive(Deserialize)]
pub struct SubtitleRequest {
    url: String,
    #[serde(default = "default_lang")]
    lang: String,
}

fn default_lang() -> String {
    "en".into()
}

#[derive(Serialize)]
pub struct SubtitleResponse {
    subtitles: Vec<SubtitleEntry>,
    auto_captions: bool,
}

#[derive(Serialize)]
pub struct SubtitleEntry {
    lang: String,
    text: String,
}

pub async fn subtitles(
    State(config): State<Config>,
    Json(req): Json<SubtitleRequest>,
) -> Result<Json<SubtitleResponse>, AppError> {
    let tmp_dir = tempfile::tempdir().map_err(|e| internal(e.into()))?;
    let result = ytdlp::get_subtitles(&config, &req.url, &req.lang, tmp_dir.path())
        .await
        .map_err(internal)?;

    Ok(Json(SubtitleResponse {
        auto_captions: result.auto_captions,
        subtitles: vec![SubtitleEntry {
            lang: result.lang,
            text: result.text,
        }],
    }))
}
