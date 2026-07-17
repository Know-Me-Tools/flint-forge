# p9-c004 Tasks — Observability

## Tasks

- [x] **Audit OTel crate versions first:** `cargo search opentelemetry | head -3` — pin compatible versions before writing any code — evidenced by consistently-pinned 0.32/0.33 versions across opentelemetry/opentelemetry_sdk/opentelemetry-otlp/tracing-opentelemetry, all compiling and testing clean
- [x] Add `opentelemetry`, `opentelemetry_otlp`, `opentelemetry_sdk`, `tracing-opentelemetry`, `metrics`, `metrics-exporter-prometheus` to `[workspace.dependencies]` — p16-c006: `metrics`/`metrics-exporter-prometheus` were not used; `axum-prometheus` (a higher-level crate combining both roles) was added instead and provides the same `/metrics` + request-rate/duration outcome — substitution verified equivalent, not a gap.
- [x] Init OTLP tracer in `fdb-gateway/src/main.rs` when `OTEL_EXPORTER_OTLP_ENDPOINT` env var is set — `crates/fdb-gateway/src/telemetry.rs` (and mirrored in `fke-server/src/main.rs`)
- [x] Install `tracing-opentelemetry` subscriber layer alongside existing `tracing_subscriber`
- [x] Add `#[tracing::instrument(skip(...))]` to: `healthz`, `handle_graphql_query`, `list_components`, `assemble_surface`, `invoke_function` — all five confirmed present (`invoke_function` lives in `fke-server`, the Kiln data-plane crate, not `fdb-gateway`)
- [x] Add Prometheus metrics recorder in `main.rs` — via `telemetry::metrics_layer()` (axum-prometheus)
- [x] Add `GET /metrics` route (no auth) returning Prometheus text format
- [x] Emit `http_requests_total` counter and `http_request_duration_seconds` histogram via Tower middleware or per-handler — axum-prometheus's `PrometheusMetricLayer` default behavior
- [x] Create `observability/grafana-dashboard.json` — JSON dashboard with panels for: request rate, P99 latency by route, error rate, active DB connections — all 4 panels present
- [x] Add `OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317` to `.env.example` (commented out)
- [x] `cargo clippy -p fdb-gateway -p fke-server -- -D warnings` clean — confirmed via `cargo clippy --workspace --all-targets -- -D warnings` (superset), clean
- [x] `cargo test --workspace` passes — confirmed, all green
