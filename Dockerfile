## Build stage
#FROM rust:1.93-slim-bookworm as builder
#RUN apt-get update && apt-get install -y \
#    pkg-config \
#    libssl-dev \
#    && rm -rf /var/lib/apt/lists/*
#WORKDIR /app
#COPY . .
#RUN cargo build --release
#
## Runtime stage
#FROM debian:bookworm-slim
#RUN apt-get update && apt-get install -y ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*
#COPY --from=builder /app/target/release/fedisea-crawler /usr/local/bin/crawler
#CMD ["crawler"]

FROM lukemathwalker/cargo-chef:latest-rust-1.93-slim-bookworm AS chef

WORKDIR app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin fedisea-crawler

FROM debian:bookworm-slim AS runtime
WORKDIR app
COPY --from=builder /app/target/release/fedisea-crawler /usr/local/bin
ENTRYPOINT ["/usr/local/bin/fedisea-crawler"]