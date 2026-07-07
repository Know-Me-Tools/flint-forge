# p9-c004 — Observability (Tracing + Metrics)

**Phase:** 9 — Production Hardening
**Priority:** P1
**Depends on:** none

## What this change delivers

- OTLP trace export from `fdb-gateway` and `fke-server`
- Prometheus `/metrics` endpoint on `fdb-gateway`
- `#[tracing::instrument]` on key request handlers
- `observability/grafana-dashboard.json` — importable Grafana dashboard

## Design

### New workspace deps

```toml
opentelemetry = { version = "0.27", features = ["trace"] }
opentelemetry_otlp = { version = "0.27", features = ["grpc-tonic"] }
opentelemetry_sdk = { version = "0.27", features = ["rt-tokio"] }
tracing-opentelemetry = "0.28"
metrics = "0.24"
metrics-exporter-prometheus = "0.16"
```

### OTLP setup in `main.rs`

```rust
let otlp_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
    .unwrap_or_else(|_| "http://localhost:4317".into());
// build OTLP pipeline → install as global tracer
```

Only initialised when `OTEL_EXPORTER_OTLP_ENDPOINT` is set — no-op when absent.

### Key metrics to expose

| Metric | Type | Description |
|---|---|---|
| `http_requests_total` | counter | Requests by method, route, status |
| `http_request_duration_seconds` | histogram | P50/P95/P99 by route |
| `db_pool_connections` | gauge | Active Postgres connections |
| `mcp_tools_compiled` | gauge | Count from `CompiledState.mcp_tools_doc` |
| `kiln_invocations_total` | counter | BGW invocations by status |

### Key handlers to instrument

`healthz`, `handle_graphql_query`, `list_components`, `assemble_surface`, `invoke_function`

### `GET /metrics` route

Added to `fdb-gateway/src/main.rs` — no auth required (internal scraping only).
Returns Prometheus text format.
