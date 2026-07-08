# Flint Forge — Security Audit

**Date:** 2026-07-04  
**Scope:** `fdb-gateway` (Quarry REST/GraphQL), `fke-server` (Kiln WASM runtime) REST APIs  
**Auditor:** Internal (p9-c005)  
**Standard:** OWASP Top 10 2021

---

## OWASP Top 10 Assessment

| # | Risk | Status | Notes |
|---|---|---|---|
| A01 | Broken Access Control | ✅ MITIGATED | Cedar policy gate (`CedarPolicyEngine`) + 6-GUC RLS enforcement on all data routes via `require_rls` middleware. Keto relation-check adapter wired into mutation gates. |
| A02 | Cryptographic Failures | ✅ MITIGATED | JWT verified via `fdb_auth::rls_from_bearer`; Ed25519 signatures on Kiln artifact manifests; TLS required in production (NoTls only in local dev pools). |
| A03 | Injection | ✅ MITIGATED | `sqlx` parameterized queries throughout; no raw SQL string concatenation. Cedar policy text loaded from DB, not user input. pg_graphql handles SQL generation internally. |
| A04 | Insecure Design | ✅ MITIGATED | `AllowAllPolicySource` renamed `TestAllowAllPolicySource` and gated `#[cfg(test)]`; production Kiln uses `DbKilnPolicySource` (deny-all until DB read). BGW publisher identity wired via `kiln_bgw`. |
| A05 | Security Misconfiguration | ✅ MITIGATED | Security headers (`X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `Referrer-Policy: strict-origin-when-cross-origin`) applied globally via `SetResponseHeaderLayer`; pgrx extensions excluded from default workspace build. |
| A06 | Vulnerable Components | ✅ MITIGATED | `cargo audit` runs in CI (`.github/workflows/ci.yml`). High-severity advisories are blocked; remaining allowlist entries are documented in `.cargo/audit.toml`. Workspace dependencies are pinned; `cargo update` is manual. |
| A07 | Identification and Authentication Failures | ✅ MITIGATED | Bearer token required on all data routes via `require_rls` middleware; WebSocket subscriptions authenticated at `connection_init`; fail-closed on missing or invalid tokens. |
| A08 | Software and Data Integrity Failures | ✅ MITIGATED | Ed25519 / Cosign Kiln artifact verification (`fke-sign-did`, `fke-sign-cosign`); Fulcio certificate chain validated (x509-cert, p7b-c004). No unsigned WASM components executed in production. |
| A09 | Security Logging and Monitoring Failures | ⚠️ PARTIAL | `tracing` in place with structured spans; rate-limiting events logged at `INFO`. No SIEM integration or alerting pipeline yet (p9-c004 pending). Auth failures logged at `WARN`. |
| A10 | Server-Side Request Forgery (SSRF) | ✅ MITIGATED | No server-side URL fetching from user-controlled input on critical paths. FRF endpoint and Keto base URL are operator-configured env vars, not user input. Reqwest calls limited to fixed, configured targets. |

---

## Findings and Recommendations

### ✅ A06 — Vulnerable Components

**Finding:** `cargo audit` is integrated into CI and fails the build on any unfixed
advisory with CVSS ≥ 7.0 that is not explicitly allowlisted. The allowlist in
`.cargo/audit.toml` documents operator-controlled exceptions (e.g., `quick-xml` via S3,
`rsa`/`sqlx-mysql` in a PostgreSQL-only codebase).

**Severity:** Medium (mitigated)  
**Affected components:** All crates  
**Controls:**

1. CI runs `cargo audit` on every push and pull request.
2. Workspace dependencies are pinned at the workspace root; upgrades are intentional.
3. `wasmtime` was upgraded to 46 to resolve RUSTSEC-2026-0095/0096 and related advisories.

**Recommendation:**

1. Subscribe to the RustSec advisory database RSS feed.
2. Evaluate `cargo deny` for combined license + advisory enforcement in a future cycle.

---

### ⚠️ A09 — Security Logging and Monitoring Failures

**Finding:** While `tracing` provides structured spans and auth-failure `WARN` events,
there is no SIEM integration, no log aggregation pipeline, and no alerting on anomalous
patterns (repeated auth failures, rate-limit spikes, schema-compile errors).

**Severity:** Low–Medium  
**Affected components:** `fdb-gateway`, `fke-server`  
**Recommendation:**

1. Wire `tracing-subscriber` to a JSON formatter for log aggregation:
   ```rust
   tracing_subscriber::fmt().json().init();
   ```
2. Forward structured logs to a SIEM (Loki, Splunk, Datadog) via log shipper (p9-c004).
3. Add rate-limit-exceeded event counters exported to Prometheus/OTEL (p9-c004).
4. Implement alert rules for:
   - >10 auth failures per minute from a single IP
   - Schema-compile failure rate > 0 (DDL regression indicator)
   - Kiln invocation error rate > 1%

---

## Controls Confirmed in This Change (p9-c005)

| Control | Implementation | Verified by |
|---|---|---|
| `X-Content-Type-Options: nosniff` | `SetResponseHeaderLayer::if_not_present` in `fdb-gateway/src/main.rs` | Unit test `security_headers_present_on_healthz` |
| `X-Frame-Options: DENY` | `SetResponseHeaderLayer::if_not_present` in `fdb-gateway/src/main.rs` | Unit test `security_headers_present_on_healthz` |
| `Referrer-Policy: strict-origin-when-cross-origin` | `SetResponseHeaderLayer::if_not_present` in `fdb-gateway/src/main.rs` | Unit test `security_headers_present_on_healthz` |
| `if_not_present` semantics | Header not overwritten when handler sets its own value | Unit test `if_not_present_does_not_overwrite_handler_header` |
| `TestAllowAllPolicySource` test-only gating | `#[cfg(test)]` on struct + impl in `fke-server/src/kiln_policy.rs` | Compile-time: struct not visible in release binary |

---

## Operator CLI and Extension Threat Model (p15)

| Control | Implementation | Verified by |
|---|---|---|
| CLI auth delegation | `forge token mint` only signs smoke-test JWTs; it does not mint production tokens. | Unit tests in `crates/forge-cli` |
| CLI container isolation | `FORGE_CONTAINER=1` runs the CLI inside the `flint-forge-cli` image; host FS not required. | `crates/forge-cli/Dockerfile` + CI build |
| pgrx extension build isolation | Extensions are excluded from the default workspace and built with `cargo pgrx` in a Linux CI container. | `.github/workflows/ci.yml` integration job |
| Migration ordering integrity | `scripts/verify-migrations.sh` enforces strict prefix ordering; CI runs `sqlx migrate run` against an empty Postgres 18 DB. | `scripts/ci-test.sh` integration stage |
| Kiln cache hit path | `EdgeRuntime::is_loaded()` prevents redundant WASM fetches from the DB store. | Unit test `is_loaded_reflects_cache_state` |

## Out of Scope / Future Work

- **Content Security Policy (CSP):** Not yet applied. Requires route-level policy
  customisation (HTMX CDN vs. API-only routes have different needs). Tracked as p10 item.
- **Strict-Transport-Security (HSTS):** Applied at the TLS termination layer (nginx/Caddy
  in front of the service); not added at the application layer to avoid false-positive
  enforcement in local dev.
- **Certificate Pinning:** Not applicable — internal mTLS is handled by the mesh, not the
  application layer.
- **Pen Test:** External penetration test recommended before production launch.
