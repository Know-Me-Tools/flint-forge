# p10-c004 Tasks — Prometheus + Alertmanager + Alerting Rules

## Tasks

- [x] Create `observability/prometheus.yml` with scrape configs for fdb-gateway and fke-server and rule_files reference
- [x] Create `observability/alerts.rules.yml` with 4 rules: HighErrorRate, HighP99Latency, ServiceDown, HighDbConnections
- [x] Create `observability/alertmanager.yml` skeleton with webhook receiver stub and `${ALERTMANAGER_WEBHOOK_URL}` placeholder
- [x] Add `prometheus` service to `docker-compose.prod.yml` (prom/prometheus:v3.0.0, volumes, retention flag) — `docker-compose.prod.yml:146-164`
- [x] Add `alertmanager` service to `docker-compose.prod.yml` (prom/alertmanager:v0.27.0, local-only port 9093) — `docker-compose.prod.yml:169-182`
- [x] Add `prometheus_data` named volume to `docker-compose.prod.yml` — `:190`
- [x] Add `ALERTMANAGER_WEBHOOK_URL` to `.env.example` with comment — `.env.example:122`
- [x] Validate compose: `docker compose -f docker-compose.yml -f docker-compose.prod.yml config --quiet`
- [x] `cargo test --workspace` passes (infrastructure-only change, no Rust code)
