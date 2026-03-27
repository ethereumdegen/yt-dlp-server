use axum::{
    Json,
    body::Body,
    extract::State,
    http::{StatusCode, header},
    response::Response,
};
use axum_extra::extract::Multipart;
use base64::Engine;
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

// --- Audio Split ---

#[derive(Serialize)]
pub struct SplitResponse {
    pub chunks: Vec<ChunkEntry>,
}

#[derive(Serialize)]
pub struct ChunkEntry {
    pub index: u32,
    pub filename: String,
    pub size: u64,
    pub data: String, // base64
}

/// Accepts multipart: `file` (audio bytes) + optional `segment_seconds` (default 600).
/// Returns JSON with base64-encoded chunks.
pub async fn audio_split(
    mut multipart: Multipart,
) -> Result<Json<SplitResponse>, AppError> {
    let mut file_bytes: Option<(String, Vec<u8>)> = None;
    let mut segment_seconds: u32 = 600;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("multipart error: {e}")))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                let filename = field.file_name().unwrap_or("audio.m4a").to_string();
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| (StatusCode::BAD_REQUEST, format!("failed to read file: {e}")))?;
                file_bytes = Some((filename, data.to_vec()));
            }
            "segment_seconds" => {
                let val = field.text().await.unwrap_or_default();
                segment_seconds = val.parse().unwrap_or(600);
            }
            _ => {}
        }
    }

    let (filename, bytes) = file_bytes
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "missing 'file' field".into()))?;

    let tmp_dir = tempfile::tempdir().map_err(|e| internal(e.into()))?;

    // Write uploaded file to temp dir
    let ext = std::path::Path::new(&filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("m4a");
    let input_path = tmp_dir.path().join(format!("input.{ext}"));
    tokio::fs::write(&input_path, &bytes)
        .await
        .map_err(|e| internal(e.into()))?;

    // Create a separate dir for chunks so we don't pick up the input file
    let chunks_dir = tmp_dir.path().join("chunks");
    tokio::fs::create_dir(&chunks_dir)
        .await
        .map_err(|e| internal(e.into()))?;

    let chunk_paths = ytdlp::split_audio(&input_path, segment_seconds, &chunks_dir)
        .await
        .map_err(internal)?;

    let b64 = base64::engine::general_purpose::STANDARD;
    let mut chunks = Vec::with_capacity(chunk_paths.len());

    for (i, path) in chunk_paths.iter().enumerate() {
        let data = tokio::fs::read(path)
            .await
            .map_err(|e| internal(e.into()))?;
        let size = data.len() as u64;
        let fname = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| format!("chunk_{i:03}.{ext}"));

        chunks.push(ChunkEntry {
            index: i as u32,
            filename: fname,
            size,
            data: b64.encode(&data),
        });
    }

    Ok(Json(SplitResponse { chunks }))
}
