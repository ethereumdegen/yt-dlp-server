# yt-dlp-rs — DevOps Guide

## Prerequisites

- **Rust** (stable) — install via [rustup](https://rustup.rs/)
- **yt-dlp** — must be on PATH or specify via `YTDLP_PATH`
- **ffmpeg** — required for audio splitting (`/api/v1/audio/split`)

```bash
# install yt-dlp if you don't have it
pip install yt-dlp
# or
brew install yt-dlp
# or
sudo apt install yt-dlp
```

## Setup

```bash
cd ~/mod_tech/yt-dlp-rs
cp .env.example .env
# edit .env if needed
```

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `PORT` | `3002` | HTTP listen port |
| `YTDLP_PATH` | `yt-dlp` | Path to yt-dlp binary |
| `YTDLP_COOKIES_PATH` | — | Path to cookies.txt for auth |
| `YTDLP_COOKIES_BROWSER` | — | Browser name to extract cookies from (e.g. `firefox`, `chrome`) |
| `MAX_DURATION` | `7200` | Max video duration in seconds (rejects longer videos) |
| `RUST_LOG` | `info` | Log level (uses tracing/env-filter) |

## Run (dev)

```bash
cargo run
```

Server starts on `http://0.0.0.0:3002`.

## Build (release)

```bash
cargo build --release
./target/release/yt-dlp-rs
```

## Test Endpoints

```bash
# health check
curl http://localhost:3002/health

# video info
curl -X POST http://localhost:3002/api/v1/info \
  -H 'Content-Type: application/json' \
  -d '{"url":"https://youtube.com/watch?v=dQw4w9WgXcQ"}'

# download audio (defaults: format=m4a, quality=5)
curl -X POST http://localhost:3002/api/v1/audio \
  -H 'Content-Type: application/json' \
  -d '{"url":"https://youtube.com/watch?v=dQw4w9WgXcQ"}' \
  -o test.m4a

# download audio with options
curl -X POST http://localhost:3002/api/v1/audio \
  -H 'Content-Type: application/json' \
  -d '{"url":"https://youtube.com/watch?v=dQw4w9WgXcQ","format":"mp3","quality":"0"}' \
  -o test.mp3

# subtitles (defaults: lang=en)
curl -X POST http://localhost:3002/api/v1/subtitles \
  -H 'Content-Type: application/json' \
  -d '{"url":"https://youtube.com/watch?v=dQw4w9WgXcQ","lang":"en"}'

# split audio into chunks (multipart upload, returns base64 JSON chunks)
curl -X POST http://localhost:3002/api/v1/audio/split \
  -F "file=@test.m4a" \
  -F "segment_seconds=600"
```

## Cookies (for age-restricted / private videos)

Option A — cookies file:
```bash
# export cookies from browser using a browser extension
YTDLP_COOKIES_PATH=./cookies.txt cargo run
```

Option B — extract from browser directly:
```bash
YTDLP_COOKIES_BROWSER=firefox cargo run
```

## Logs

Set `RUST_LOG` for verbosity:

```bash
RUST_LOG=debug cargo run        # verbose
RUST_LOG=yt_dlp_rs=debug cargo run  # verbose only for this crate
RUST_LOG=warn cargo run          # quiet
```
