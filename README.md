# yt-dlp-rs

A fast, lightweight HTTP API server wrapping [yt-dlp](https://github.com/yt-dlp/yt-dlp) — built with Rust and [Axum](https://github.com/tokio-rs/axum).

Extract video metadata, download audio, fetch subtitles, and split audio files via simple REST endpoints.

## Features

- **Video metadata** — extract full JSON metadata from any yt-dlp-supported URL
- **Audio download** — stream audio in m4a, mp3, or other formats with configurable quality
- **Subtitles** — fetch manual or auto-generated captions in any language
- **Audio splitting** — upload an audio file and split it into timed chunks (base64 JSON)
- **Cookie auth** — access age-restricted or private content via cookies file or browser extraction
- **Duration limits** — reject videos exceeding a configurable max duration
- **Docker ready** — multi-stage Dockerfile for lean production images

## Prerequisites

| Tool | Install |
|------|---------|
| **Rust** | [rustup.rs](https://rustup.rs/) |
| **yt-dlp** | `pip install yt-dlp` · [more options](https://github.com/yt-dlp/yt-dlp#installation) |
| **ffmpeg** | `sudo apt install ffmpeg` · `brew install ffmpeg` |

## Quick Start

```bash
git clone https://github.com/ethereumdegen/yt-dlp-server.git
cd yt-dlp-server
cp .env.example .env    # edit if needed
cargo run
```

Server starts at **http://localhost:3002**.

## Configuration

All settings are loaded from environment variables (or a `.env` file):

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `3002` | HTTP listen port |
| `YTDLP_PATH` | `yt-dlp` | Path to yt-dlp binary |
| `YTDLP_COOKIES_PATH` | — | Path to a Netscape-format cookies file |
| `YTDLP_COOKIES_BROWSER` | — | Browser to extract cookies from (`firefox`, `chrome`, etc.) |
| `MAX_DURATION` | `7200` | Max video duration in seconds (`0` = unlimited) |
| `RUST_LOG` | `info` | Log level (`trace`, `debug`, `info`, `warn`, `error`) |

## API Reference

### `GET /health`

Health check — returns server status and yt-dlp version.

```bash
curl http://localhost:3002/health
```

```json
{ "status": "ok", "yt_dlp_version": "2025.01.15" }
```

---

### `POST /api/v1/info`

Extract video metadata as JSON.

```bash
curl -X POST http://localhost:3002/api/v1/info \
  -H 'Content-Type: application/json' \
  -d '{"url": "https://youtube.com/watch?v=dQw4w9WgXcQ"}'
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `url` | string | yes | Video URL |

---

### `POST /api/v1/audio`

Download audio as a streaming file.

```bash
curl -X POST http://localhost:3002/api/v1/audio \
  -H 'Content-Type: application/json' \
  -d '{"url": "https://youtube.com/watch?v=dQw4w9WgXcQ"}' \
  -o audio.m4a
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `url` | string | yes | — | Video URL |
| `format` | string | no | `m4a` | Audio format (`m4a`, `mp3`, `opus`, etc.) |
| `quality` | string | no | `5` | Audio quality (`0` = best, `10` = worst) |

---

### `POST /api/v1/subtitles`

Fetch subtitles or auto-generated captions.

```bash
curl -X POST http://localhost:3002/api/v1/subtitles \
  -H 'Content-Type: application/json' \
  -d '{"url": "https://youtube.com/watch?v=dQw4w9WgXcQ", "lang": "en"}'
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `url` | string | yes | — | Video URL |
| `lang` | string | no | `en` | Language code |

**Response:**

```json
{
  "subtitles": [{ "lang": "en", "text": "..." }],
  "auto_captions": true
}
```

---

### `POST /api/v1/audio/split`

Upload an audio file and split it into fixed-length chunks. Returns base64-encoded segments.

```bash
curl -X POST http://localhost:3002/api/v1/audio/split \
  -F "file=@audio.m4a" \
  -F "segment_seconds=600"
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `file` | file | yes | — | Audio file (multipart) |
| `segment_seconds` | integer | no | `600` | Chunk length in seconds |

**Response:**

```json
{
  "chunks": [
    { "index": 0, "filename": "chunk_000.m4a", "size": 1048576, "data": "base64..." },
    { "index": 1, "filename": "chunk_001.m4a", "size": 524288, "data": "base64..." }
  ]
}
```

## Docker

```bash
docker build -t yt-dlp-rs .
docker run -p 3002:3002 yt-dlp-rs
```

With environment overrides:

```bash
docker run -p 3002:3002 \
  -e MAX_DURATION=3600 \
  -e RUST_LOG=debug \
  yt-dlp-rs
```

## Cookie Authentication

For age-restricted or private videos, provide cookies to yt-dlp:

**Option A — Cookies file** (export from browser with an extension like [Get cookies.txt](https://chromewebstore.google.com/detail/get-cookiestxt-locally/cclelndahbckbenkjhflpdbgdldlbecc)):

```bash
YTDLP_COOKIES_PATH=./cookies.txt cargo run
```

**Option B — Extract from browser directly:**

```bash
YTDLP_COOKIES_BROWSER=firefox cargo run
```

## Project Structure

```
src/
├── main.rs       # Router setup and server startup
├── config.rs     # Environment-based configuration
├── handlers.rs   # HTTP endpoint handlers
└── ytdlp.rs      # yt-dlp and ffmpeg command wrappers
```

## License

MIT
