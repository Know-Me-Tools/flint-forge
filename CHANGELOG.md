# Changelog

All notable changes to Flint Forge are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.10.0] — 2026-07-06

This is the first production-ready milestone of Flint Forge — the sovereign data
and edge-compute plane of the Flint platform. It delivers a fully-integrated stack
across nine development phases:

### Highlights

**Quarry (fdb-gateway)** — REST / GraphQL data gateway
- Full PostgREST-compatible REST layer with 12 filter operators and row-level security
- pg_graphql-backed dynamic GraphQL (queries, mutations, subscriptions)
- MCP JSON-RPC 2.0 server (`/mcp/v1`) with 7 A2UI tool definitions
- A2A protocol support (Agent Card, `tasks/send` dispatcher)
- AG-UI SSE event stream + A2UI surface emitter
- A2UI component registry with 55-slug catalog (OpenDesign import, HTMX renderers)
- Per-IP rate limiting (token bucket, 100/20 req/s, configurable via env)
- OTLP tracing + Prometheus `/metrics` endpoint
- Security response headers (OWASP Top 10 assessed)
- TLS termination via Caddy (automatic Let's Encrypt / ACME)
- Docker Compose secrets pattern (`postgres_password`, `jwt_secret`)

**Kiln (fke-server)** — WASM edge-function runtime
- Wasmtime 46 Component Model engine with epoch interruption + fuel limits
- Cedar policy enforcement point (per-function authz, `DbKilnPolicySource`)
- Multi-store support: filesystem, S3/MinIO, OCI/IPFS
- Ed25519/DID + Sigstore/Cosign artifact verification
- BGW (background worker) draining `webhook_outbox`
- WIT bindings + `wasi:http` dispatch via `ProxyPre`

**SDKs**
- `@flint/react` — 55-slug component registry + `useFlintRegistry()` hook
- `flint_genui` (Flutter/Dart) — SSE client with reconnect + `refresh()`

**Infrastructure**
- GitHub Actions CI (fmt, clippy, test, `cargo audit` security gate)
- Docker multi-stage builds for both services
- `docker-compose.yml` (dev), `docker-compose.prod.yml` (prod with TLS + secrets)
- `docker-compose.staging.yml` (resource limits + registry image pull)
- Prometheus + Alertmanager + 4 alerting rules
- Grafana dashboard (4 panels: request rate, P99, error rate, DB connections)
- k6 load-test scripts + regression gate
- Operational runbook (`docs/runbook.md`, 940+ lines)

### Breaking Changes

- `fix(p35-c004)!`: `PgBackend::acquire` RLS setup repaired — callers that were
  bypassing the RLS GUC injection due to the pre-fix bug will now correctly receive
  row-level security enforcement.

### Bug Fixes
- **p35-c004**: Repair PgBackend::acquire RLS setup + add DB-integration tests (G2) (**BREAKING**) ([`35fdf01`](https://github.com/Know-Me-Tools/flint-forge/commit/35fdf01a27dd2593b266f5b334e0243d41a6d40a))
- **p35-c002**: De-flake keto_sync interval test via pure resolve_interval (G3) ([`007ce1f`](https://github.com/Know-Me-Tools/flint-forge/commit/007ce1f2dc19beda30d5f1645e4a45040919a165))
- **p35-c001**: Clear workspace clippy-pedantic blockers (G4) ([`f2946f3`](https://github.com/Know-Me-Tools/flint-forge/commit/f2946f38758181cd112670a3483b9b5d7cd536ae))

### Features (committed individually, p3.x)
- **p35-c003**: CI Postgres image + DB test runner + Dagger service binding ([`4ef8150`](https://github.com/Know-Me-Tools/flint-forge/commit/4ef81509181872c75dd5860b2af455816e089d88))
- **p3-c020**: In-process Postgres LISTEN/NOTIFY ChangeStreamSource ([`094f74e`](https://github.com/Know-Me-Tools/flint-forge/commit/094f74e77a7a59ab6f87955de10ae2800b698b20))
- **p3-c019**: PostgREST parity — resource embedding, FTS, edge cases ([`b786335`](https://github.com/Know-Me-Tools/flint-forge/commit/b78633594f82ee8e3fb64a6dffcf153b07f63c52))
- **p3-c019**: fdb-query crate — PostgREST operator + safety layer + translator ([`8251704`](https://github.com/Know-Me-Tools/flint-forge/commit/8251704204af597501351ed3040fe54777c04935))
- **p3-g4**: GraphQL subscription seam to RLS-filtered change stream ([`c03aae2`](https://github.com/Know-Me-Tools/flint-forge/commit/c03aae2c02e008e9fc3b23e7f521b5f15de9596f))
- **p3-c014**: REST mutation handlers with Keto+Cedar gates ([`f43dccf`](https://github.com/Know-Me-Tools/flint-forge/commit/f43dccfdf85d0476b5fcc4be5b32adcab939f660))
- **p3-c013**: REST handle_list with 12 filter operators ([`9bfd8eb`](https://github.com/Know-Me-Tools/flint-forge/commit/9bfd8eb640f28c31bde7d0355ff24b3abada1bd4))
- **p3**: Reflection router, KetoCheck port, Cedar policy engine ([`11486ce`](https://github.com/Know-Me-Tools/flint-forge/commit/11486ce7a3ad2f11aecf57bd0d7203620ecf61e8))

### Testing
- **p3-c015**: REST filter-safety + vault DEK serde security gates ([`a0b180d`](https://github.com/Know-Me-Tools/flint-forge/commit/a0b180de1a87976135a57dee0dbb78e2b2da815c))

### Maintenance
- Initial commit — Flint Forge scaffold + KBD orchestrator state ([`2927d55`](https://github.com/Know-Me-Tools/flint-forge/commit/2927d5550ce4cc5e16ca1361332bfd0b11459f35))

---

*Phases p5–p10 (A2UI registry, AG-UI/MCP tools, Kiln production hardening, SDK
completeness, and production launch) were developed in a KBD long-running session
and landed as a single release commit. Starting from v0.11.0, all changes will
carry individual conventional commits and appear in this log automatically via
`git cliff --unreleased`.*
