# Flint Forge — Monitoring Reference

This document describes the Prometheus alerting configuration, the current
alert thresholds, and the review schedule for tuning them against real
production traffic.

---

## Alert rules

Alert rules are defined in `observability/alerts.rules.yml` and evaluated by
Prometheus every 15 seconds. Alerts route to Alertmanager (`observability/alertmanager.yml`).

### Current rules

| Alert | Expression | `for:` | Severity | Initial basis |
|---|---|---|---|---|
| `HighErrorRate` | 5xx rate > 1% of total requests | 5m | warning | OWASP industry baseline |
| `HighP99Latency` | P99 > 500 ms (all endpoints combined) | 5m | warning | Aspirational target × 2 |
| `ServiceDown` | `up == 0` | 1m | critical | Zero-tolerance |
| `HighDbConnections` | `sqlx_pool_connections_open > 8` | 3m | warning | 80% of default pool (10) |

**Inhibit rule:** `HighErrorRate`, `HighP99Latency`, and `HighDbConnections` are
suppressed when `ServiceDown` is already firing for the same job (avoids
alert storms when a service is unreachable).

---

## Grafana dashboard

The Grafana dashboard is at `observability/grafana-dashboard.json`. Import it
into a running Grafana instance:

```
Grafana → Dashboards → Import → Upload JSON file → grafana-dashboard.json
```

The dashboard has four panels:

| Panel | Metric | Alert threshold |
|---|---|---|
| HTTP Request Rate | `rate(axum_http_requests_total[…])` | — |
| P99 Latency by Route | `histogram_quantile(0.99, …)` | 200 ms / 500 ms |
| HTTP Error Rate | 5xx fraction | 1% |
| Active DB Connections | `sqlx_pool_connections_open` | 8 / 10 |

**Note:** Flint Forge emits `sqlx_pool_connections_open` and
`sqlx_pool_connections_idle` from the gateway telemetry task. If the DB
connections panel shows "no data", confirm the gateway `/metrics` endpoint is
scraped and that database pooling is enabled for the running service.

---

## Threshold review schedule

### First review (30 days post-deploy)

After 30 days of production traffic, review the following using the Grafana
dashboard and Prometheus query console:

```promql
# Observed P99 over 30 days
histogram_quantile(0.99,
  sum(rate(axum_http_requests_duration_seconds_bucket[30d])) by (le, endpoint)
)

# Error rate baseline
sum(rate(axum_http_requests_total{status_code=~"5.."}[30d]))
/ sum(rate(axum_http_requests_total[30d]))
```

**Decision criteria:**

| Observed value | Action |
|---|---|
| P99 consistently < 100 ms | Tighten `HighP99Latency` to 200 ms |
| P99 occasionally spikes to 400–600 ms | Keep 500 ms; investigate spike causes |
| Error rate baseline 0.01–0.1% | Keep 1% threshold |
| Error rate baseline > 0.1% | Investigate before tuning threshold |
| DB connections always ≤ 3 | Tighten `HighDbConnections` to 5 |

After tuning, update `observability/alerts.rules.yml` and commit:
```bash
git commit -m "ops: tune alerting thresholds from 30d production baseline"
```

### Quarterly review

Every 90 days:
1. Check `.cargo/audit.toml` — rotate any allowlist entries approaching their expiry date
2. Re-run `cargo audit` — ensure 0 unfixed CVSS ≥ 7.0
3. Review Alertmanager firing history — identify noisy or missing alerts
4. Run `cargo update` — apply safe patch bumps (see p13-c002 notes on `generic-array` exclusion)

---

## Known limitations (as of v1.0.0)

| Limitation | Impact | Resolution |
|---|---|---|
| Local-only k6 baselines | Thresholds were measured on Colima, not a production-like staging host | Fix: re-run k6 against staging and update `docs/performance.md` |
| Alertmanager webhook URL not configured | No notifications sent | Fix: set `ALERTMANAGER_WEBHOOK_URL` in `.env` and configure `observability/alertmanager.yml` |

---

## Alertmanager webhook setup

To activate alert delivery, edit `observability/alertmanager.yml` and
uncomment the webhook receiver with your actual URL:

```yaml
receivers:
  - name: default
    webhook_configs:
      - url: "https://hooks.slack.com/services/T.../B.../..."
        send_resolved: true
```

Then restart Alertmanager:

```bash
docker compose -f docker-compose.yml -f docker-compose.prod.yml \
  restart alertmanager
```
