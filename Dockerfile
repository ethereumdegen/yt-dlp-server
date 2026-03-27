# ---- Build Stage ----
FROM rust:1.85-slim AS builder

WORKDIR /app

# Cache dependency build
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main(){}' > src/main.rs && cargo build --release && rm -rf src

# Build real app
COPY src ./src
RUN touch src/main.rs && cargo build --release

# ---- Runtime Stage ----
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    ffmpeg \
    python3 \
    python3-pip \
    python3-venv \
    && python3 -m venv /opt/venv \
    && /opt/venv/bin/pip install --no-cache-dir yt-dlp \
    && apt-get purge -y python3-pip python3-venv \
    && apt-get autoremove -y \
    && rm -rf /var/lib/apt/lists/*

ENV PATH="/opt/venv/bin:$PATH"

COPY --from=builder /app/target/release/yt-dlp-rs /usr/local/bin/yt-dlp-rs

ENV PORT=3002
EXPOSE 3002

CMD ["yt-dlp-rs"]
