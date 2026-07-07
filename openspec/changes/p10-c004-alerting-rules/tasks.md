# p10-c004 Tasks — Prometheus + Alertmanager + Alerting Rules

## Tasks

- [ ] Create `observability/prometheus.yml` with scrape configs for fdb-gateway and fke-server and rule_files reference
- [ ] Create `observability/alerts.rules.yml` with 4 rules: HighErrorRate, HighP99Latency, ServiceDown, HighDbConnections
- [ ] Create `observability/alertmanager.yml` skeleton with webhook receiver stub and `${ALERTMANAGER_WEBHOOK_URL}` placeholder
- [ ] Add `prometheus` service to `docker-compose.prod.yml` (prom/prometheus:v3.0.0, volumes, retention flag)
- [ ] Add `alertmanager` service to `docker-compose.prod.yml` (prom/alertmanager:v0.27.0, local-only port 9093)
- [ ] Add `prometheus_data` named volume to `docker-compose.prod.yml`
- [ ] Add `ALERTMANAGER_WEBHOOK_URL` to `.env.example` with comment
- [ ] Validate compose: `docker compose -f docker-compose.yml -f docker-compose.prod.yml config --quiet`
- [ ] `cargo test --workspace` passes (infrastructure-only change, no Rust code)
