# p9-c004 Tasks — Observability

## Tasks

- [ ] **Audit OTel crate versions first:** `cargo search opentelemetry | head -3` — pin compatible versions before writing any code
- [ ] Add `opentelemetry`, `opentelemetry_otlp`, `opentelemetry_sdk`, `tracing-opentelemetry`, `metrics`, `metrics-exporter-prometheus` to `[workspace.dependencies]`
- [ ] Init OTLP tracer in `fdb-gateway/src/main.rs` when `OTEL_EXPORTER_OTLP_ENDPOINT` env var is set
- [ ] Install `tracing-opentelemetry` subscriber layer alongside existing `tracing_subscriber`
- [ ] Add `#[tracing::instrument(skip(...))]` to: `healthz`, `handle_graphql_query`, `list_components`, `assemble_surface`, `invoke_function`
- [ ] Add Prometheus metrics recorder in `main.rs`
- [ ] Add `GET /metrics` route (no auth) returning Prometheus text format
- [ ] Emit `http_requests_total` counter and `http_request_duration_seconds` histogram via Tower middleware or per-handler
- [ ] Create `observability/grafana-dashboard.json` — JSON dashboard with panels for: request rate, P99 latency by route, error rate, active DB connections
- [ ] Add `OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317` to `.env.example` (commented out)
- [ ] `cargo clippy -p fdb-gateway -p fke-server -- -D warnings` clean
- [ ] `cargo test --workspace` passes
