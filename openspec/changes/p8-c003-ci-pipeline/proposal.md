# p8-c003 — CI Pipeline (GitHub Actions + Docker)

**Phase:** 8 — SDK Completeness
**Priority:** P0
**Depends on:** none

## What this change delivers

- `.github/workflows/ci.yml` — runs `cargo fmt`, `cargo clippy`, `cargo test`, `cargo component build` on every PR
- `docker/fdb-gateway/Dockerfile` — multi-stage image for the Quarry gateway
- `docker/fke-server/Dockerfile` — multi-stage image for the Kiln server
- `.github/workflows/docker.yml` — builds + pushes to `ghcr.io` on merge to `main`

## Design

### `.github/workflows/ci.yml`

```yaml
name: CI
on: [push, pull_request]
jobs:
  rust:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo test --workspace
      - run: cargo install cargo-component --locked
      - run: cargo component build -p hello-component
```

### Dockerfiles — multi-stage with cargo-chef layer caching

```dockerfile
# docker/fdb-gateway/Dockerfile
FROM rust:1.85-slim AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release -p fdb-gateway

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/fdb-gateway /usr/local/bin/
EXPOSE 8080
CMD ["fdb-gateway"]
```

Same pattern for `fke-server` (port 8090).

### `.github/workflows/docker.yml`

```yaml
name: Docker
on:
  push:
    branches: [main]
jobs:
  build:
    runs-on: ubuntu-latest
    permissions:
      packages: write
    steps:
      - uses: actions/checkout@v4
      - uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}
      - uses: docker/build-push-action@v5
        with:
          context: .
          file: docker/fdb-gateway/Dockerfile
          push: true
          tags: ghcr.io/${{ github.repository_owner }}/flint-gateway:latest
```
