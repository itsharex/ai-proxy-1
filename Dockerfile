# Build stage
FROM rust:1.80 AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y pkg-config libssl-dev

COPY . .

RUN cd src-tauri && cargo build --release --features server --no-default-features

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/src-tauri/target/release/ai-proxy-server /usr/local/bin/ai-proxy-server

RUN mkdir -p /data && chmod 755 /data

ENV AI_PROXY_DATA_DIR=/data
ENV RUST_LOG=info

EXPOSE 7860

VOLUME ["/data"]

ENTRYPOINT ["ai-proxy-server", "--host", "0.0.0.0", "--port", "7860"]
