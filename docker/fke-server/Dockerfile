# syntax=docker/dockerfile:1
# Multi-stage build for fke-server using cargo-chef for layer caching.
# pgrx extension crates (ext-flint-*) are excluded from the workspace Cargo.toml
# and are never compiled here.

FROM rust:1.96-slim AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*
# Cache dependencies first; only invalidated when Cargo.lock changes.
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release -p fke-server

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/fke-server /usr/local/bin/fke-server
# p11-c005: entrypoint reads /run/secrets/postgres_password before exec-ing the binary.
COPY docker/fke-server/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh
EXPOSE 8090
ENV RUST_LOG=info
ENTRYPOINT ["/entrypoint.sh"]
