# Build stage
FROM rust:1.88.0 AS builder
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl3 \
    ca-certificates \
    libgcc-s1 \
    libstdc++6 \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/watcher

COPY Cargo.toml ./
COPY src/ ./src/

RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl3 \
    ca-certificates \
    libgcc-s1 \
    libstdc++6 \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /usr/src/watcher/target/release/watcher .
RUN chmod +x /app/watcher
RUN chmod 777 watcher

USER 1000

ENTRYPOINT ["./watcher"]
