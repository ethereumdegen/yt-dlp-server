use crate::config::Config;
use anyhow::{Context, Result, bail};
use std::path::Path;
use tokio::process::Command;

/// Build base args including cookie flags if configured.
fn cookie_args(config: &Config) -> Vec<String> {
    let mut args = Vec::new();
    if let Some(ref path) = config.cookies_path {
        args.push("--cookies".into());
        args.push(path.clone());
    }
    if let Some(ref browser) = config.cookies_browser {
        args.push("--cookies-from-browser".into());
        args.push(browser.clone());
    }
    args
}

/// Get yt-dlp version string.
pub async fn version(config: &Config) -> Result<String> {
    let output = Command::new(&config.ytdlp_path)
        .arg("--version")
        .output()
        .await
        .context("failed to run yt-dlp")?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Dump video metadata as JSON.
pub async fn get_info(config: &Config, url: &str) -> Result<serde_json::Value> {
    let mut cmd = Command::new(&config.ytdlp_path);
    cmd.args(["--dump-json", "--no-warnings"]);
    cmd.args(cookie_args(config));
    cmd.arg(url);

    let output = cmd.output().await.context("failed to run yt-dlp")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("yt-dlp info failed: {stderr}");
    }

    let info: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("failed to parse yt-dlp JSON")?;

    // Check duration limit
    if let Some(duration) = info.get("duration").and_then(|d| d.as_f64()) {
        if duration > config.max_duration as f64 {
            bail!(
                "video duration {duration:.0}s exceeds limit of {}s",
                config.max_duration
            );
        }
    }

    Ok(info)
}

/// Download audio to a temp directory, return the path to the downloaded file.
pub async fn download_audio(
    config: &Config,
    url: &str,
    format: &str,
    quality: &str,
    tmp_dir: &Path,
) -> Result<std::path::PathBuf> {
    let output_template = tmp_dir.join("%(id)s.%(ext)s");

    let mut cmd = Command::new(&config.ytdlp_path);
    cmd.args([
        "-x",
        "--audio-format",
        format,
        "--audio-quality",
        quality,
        "--no-warnings",
        "-o",
    ]);
    cmd.arg(&output_template);
    cmd.args(cookie_args(config));
    cmd.arg(url);

    let output = cmd.output().await.context("failed to run yt-dlp")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("yt-dlp audio download failed: {stderr}");
    }

    // Find the downloaded file
    let mut entries = tokio::fs::read_dir(tmp_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() {
            return Ok(path);
        }
    }

    bail!("no audio file found after download")
}

/// Download subtitles, return the subtitle text content.
pub async fn get_subtitles(
    config: &Config,
    url: &str,
    lang: &str,
    tmp_dir: &Path,
) -> Result<SubtitleResult> {
    let output_template = tmp_dir.join("%(id)s.%(ext)s");

    let mut cmd = Command::new(&config.ytdlp_path);
    cmd.args([
        "--write-subs",
        "--write-auto-subs",
        "--sub-lang",
        lang,
        "--sub-format",
        "vtt",
        "--skip-download",
        "--no-warnings",
        "-o",
    ]);
    cmd.arg(&output_template);
    cmd.args(cookie_args(config));
    cmd.arg(url);

    let output = cmd.output().await.context("failed to run yt-dlp")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("yt-dlp subtitles failed: {stderr}");
    }

    // Find subtitle files
    let mut entries = tokio::fs::read_dir(tmp_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "vtt" || ext == "srt" {
                let text = tokio::fs::read_to_string(&path).await?;
                let filename = path.file_name().unwrap().to_string_lossy().to_string();
                let auto_captions = filename.contains(".auto.");
                return Ok(SubtitleResult {
                    lang: lang.to_string(),
                    text,
                    auto_captions,
                });
            }
        }
    }

    bail!("no subtitle file found for lang '{lang}'")
}

pub struct SubtitleResult {
    pub lang: String,
    pub text: String,
    pub auto_captions: bool,
}
