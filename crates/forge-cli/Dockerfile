# syntax=docker/dockerfile:1
# Multi-stage build for the Flint Forge operator CLI.
# Context: repository root.

FROM rust:1.96-slim AS builder
WORKDIR /usr/src/flint-forge

# Copy workspace manifests and source.
COPY . .

# Build the release binary.
RUN cargo build -p forge-cli --release

FROM debian:bookworm-slim

# TLS root certificates are required for HTTPS calls to Kiln / webhooks.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/flint-forge/target/release/forge /usr/local/bin/forge

ENTRYPOINT ["forge"]
