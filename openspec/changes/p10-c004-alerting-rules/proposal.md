# p10-c004 — Prometheus + Alertmanager + Alerting Rules

**Phase:** 10 — Production Launch
**Priority:** P1
**Depends on:** p10-c001 (Prometheus needs to know fdb-gateway's internal hostname + port)

## Problem

The `/metrics` endpoint exists on `fdb-gateway` (p9-c004) but nothing scrapes
it. There are no alerting rules and no Alertmanager to route them. The Grafana
dashboard has panels but no alert annotations.

## Solution

Add Prometheus + Alertmanager as services in `docker-compose.prod.yml`; author
4 alerting rules; wire Alertmanager with a webhook stub for operators to fill in.

### New files

**`observability/prometheus.yml`** — scrape config:

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

rule_files:
  - /etc/prometheus/alerts.rules.yml

alerting:
  alertmanagers:
    - static_configs:
        - targets: ['alertmanager:9093']

scrape_configs:
  - job_name: flint-quarry
    static_configs:
      - targets: ['fdb-gateway:8080']

  - job_name: flint-kiln
    static_configs:
      - targets: ['fke-server:8090']
```

**`observability/alerts.rules.yml`** — 4 production alert rules:

```yaml
groups:
  - name: flint-forge
    rules:
      - alert: HighErrorRate
        expr: |
          sum(rate(axum_http_requests_total{status_code=~"5.."}[5m]))
          / sum(rate(axum_http_requests_total[5m])) > 0.01
        for: 5m
        labels: { severity: warning }
        annotations:
          summary: "Error rate > 1% sustained for 5 minutes"

      - alert: HighP99Latency
        expr: |
          histogram_quantile(0.99,
            sum(rate(axum_http_requests_duration_seconds_bucket[5m])) by (le)
          ) > 0.5
        for: 5m
        labels: { severity: warning }
        annotations:
          summary: "P99 latency > 500 ms sustained for 5 minutes"

      - alert: ServiceDown
        expr: up == 0
        for: 1m
        labels: { severity: critical }
        annotations:
          summary: "Service {{ $labels.job }} is down"

      - alert: HighDbConnections
        expr: sqlx_pool_connections_open > 8
        for: 3m
        labels: { severity: warning }
        annotations:
          summary: "DB pool connections > 8 sustained for 3 minutes"
```

**`observability/alertmanager.yml`** — skeleton config (operator fills webhook URL):

```yaml
global:
  resolve_timeout: 5m

route:
  receiver: default
  group_wait: 30s
  group_interval: 5m
  repeat_interval: 4h

receivers:
  - name: default
    webhook_configs:
      - url: ${ALERTMANAGER_WEBHOOK_URL}   # set in .env or secrets
        send_resolved: true
```

### `docker-compose.prod.yml` additions

```yaml
  prometheus:
    image: prom/prometheus:v3.0.0
    restart: unless-stopped
    volumes:
      - ./observability/prometheus.yml:/etc/prometheus/prometheus.yml:ro
      - ./observability/alerts.rules.yml:/etc/prometheus/alerts.rules.yml:ro
      - prometheus_data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.retention.time=15d'

  alertmanager:
    image: prom/alertmanager:v0.27.0
    restart: unless-stopped
    volumes:
      - ./observability/alertmanager.yml:/etc/alertmanager/alertmanager.yml:ro
    ports:
      - "127.0.0.1:9093:9093"   # local-only; not exposed publicly

volumes:
  prometheus_data:
```
