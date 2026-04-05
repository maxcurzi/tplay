# ---- Build Stage ----
FROM rust:bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    clang \
    libssl-dev \
    ffmpeg \
    libavfilter-dev \
    libavdevice-dev \
    libavformat-dev \
    libavcodec-dev \
    libswscale-dev \
    libasound2-dev \
    libmpv-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY . .

RUN cargo build --release

# ---- Runtime Stage ----
FROM debian:bookworm-slim

# Install runtime dependencies only
RUN apt-get update && apt-get install -y --no-install-recommends \
    ffmpeg \
    libmpv2 \
    libasound2 \
    libssl3 \
    yt-dlp \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the built binary
COPY --from=builder /build/target/release/tplay /usr/local/bin/tplay

# Default working directory for media files
WORKDIR /media

ENTRYPOINT ["tplay"]
CMD ["--help"]
