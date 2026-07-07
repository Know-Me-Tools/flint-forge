# p8-c003 Tasks — CI Pipeline

## Tasks

- [ ] Create `.github/workflows/ci.yml` — cargo fmt + clippy + test + component build on push/PR
- [ ] Create `docker/fdb-gateway/Dockerfile` — multi-stage with cargo-chef layer caching, exposes port 8080
- [ ] Create `docker/fke-server/Dockerfile` — multi-stage with cargo-chef layer caching, exposes port 8090
- [ ] Create `.github/workflows/docker.yml` — build + push to `ghcr.io` on merge to `main`
- [ ] Verify CI workflow runs locally with `act` or via a test PR
- [ ] Add `.dockerignore` to exclude `target/`, `.git/`, test artifacts from Docker context
- [ ] Gate test: `docker build -f docker/fdb-gateway/Dockerfile .` succeeds (run locally)
- [ ] Gate test: `docker build -f docker/fke-server/Dockerfile .` succeeds
