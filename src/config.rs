use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub port: u16,
    pub ytdlp_path: String,
    pub cookies_path: Option<String>,
    pub cookies_browser: Option<String>,
    pub max_duration: u64,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            port: env::var("PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3002),
            ytdlp_path: env::var("YTDLP_PATH").unwrap_or_else(|_| "yt-dlp".into()),
            cookies_path: env::var("YTDLP_COOKIES_PATH").ok(),
            cookies_browser: env::var("YTDLP_COOKIES_BROWSER").ok(),
            max_duration: env::var("MAX_DURATION")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(7200),
        }
    }
}
