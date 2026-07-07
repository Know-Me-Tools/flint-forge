# Plan — p9-hardening

**Phase:** p9-hardening
**Planned:** 2026-07-04
**Change backend:** OpenSpec (`openspec/changes/`)
**Assessment:** `.kbd-orchestrator/phases/p9-hardening/assessment.md`

---

## Ordering rationale

G1 (Docker Compose), G2 (Runbook), G3 (Rate limiting) are all P0, independent,
and implementable without needing a live environment — start these in parallel.
G5 (Security audit) and G6 (cargo bench only) are also independent and can run
alongside the P0 batch. G4 (Observability) is heaviest in dep footprint — do
it after G3 proves the middleware pattern. G7 (Staging) depends on G1.

```
Session 1 (parallel):  c001 (compose)     c002 (runbook)     c003 (rate limit)
Session 2 (parallel):  c005 (security)    c006-bench (bench)
Session 3:             c004 (observability) — heaviest dep footprint
Session 4:             c007 (staging) — after c001 compose exists
```

---

## Change list

| # | Change ID | Title | Priority | Domain | Effort |
|---|---|---|---|---|---|
| 1 | **p9-c001-docker-compose** | `docker-compose.yml` + `.env.example` | P0 | DevOps | Low-Med |
| 2 | **p9-c002-runbook** | `docs/runbook.md` (8 sections) | P0 | Docs | Low |
| 3 | **p9-c003-rate-limiting** | Tower-governor middleware + env config | P0 | Rust | Medium |
| 4 | **p9-c005-security-audit** | Headers middleware + AllowAll cleanup + audit doc | P1 | Rust + Docs | Medium |
| 5 | **p9-c006-performance-audit** | criterion benchmarks + k6 scripts + perf doc | P1 | Rust + Shell | Medium |
| 6 | **p9-c004-observability** | OTLP traces + Prometheus `/metrics` + Grafana | P1 | Rust | High |
| 7 | **p9-c007-staging-deploy** | smoke_test.sh + deploy.yml + staging compose | P2 | DevOps | Medium |

---

## Constraint notes (from AGENTS.md)

- `#![forbid(unsafe_code)]` in all Rust crates
- New workspace deps go in `[workspace.dependencies]` first
- Files under 500 lines
- No `unwrap()`/`expect()` in handler code — only at startup

---

## New workspace dependencies required

| Dep | Version | Priority | OQ |
|---|---|---|---|
| `tower-governor` | `"0.8"` | c003 | OQ-P9-1: verify IP extraction works behind reverse proxy |
| `criterion` | `"0.5"` | c006 | dev-dep only |
| `opentelemetry` + `_otlp` + `_sdk` | `"0.27"` | c004 | OQ-P9-2: audit API compatibility with tracing-otel 0.28 |
| `tracing-opentelemetry` | `"0.28"` | c004 | — |
| `metrics` + `metrics-exporter-prometheus` | `"0.24"` / `"0.16"` | c004 | — |

---

## Phase gate

- [ ] `docker compose up` starts the full stack end-to-end
- [ ] `docs/runbook.md` covers startup, 5 errors, migration, rollback
- [ ] Rate limiting returns 429 on excess requests
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` clean

---

## Recommended first action

```
/kbd-build p9-c001-docker-compose and p9-c002-runbook and p9-c003-rate-limiting concurrently
```

Three P0 changes, fully independent, covering DevOps + docs + Rust — ideal for a single
parallel session that closes all blocking items at once.
