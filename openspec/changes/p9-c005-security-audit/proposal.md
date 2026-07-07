# p9-c005 — Security Audit + Hardening

**Phase:** 9 — Production Hardening
**Priority:** P1
**Depends on:** none

## What this change delivers

- HTTP security response headers on all `fdb-gateway` routes
- Deletion of `AllowAllPolicySource` test stub from production binary
- `docs/security-audit.md` — OWASP Top 10 review results
- Verification that no JWT/bearer payloads appear in any log output

## Design

### Security headers middleware

Apply via Tower `SetResponseHeaderLayer` to the `app` router:

```rust
use tower_http::set_header::SetResponseHeaderLayer;
use axum::http::HeaderValue;

let security_headers = tower::ServiceBuilder::new()
    .layer(SetResponseHeaderLayer::if_not_present(
        axum::http::header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    ))
    .layer(SetResponseHeaderLayer::if_not_present(
        axum::http::header::X_FRAME_OPTIONS,
        HeaderValue::from_static("DENY"),
    ))
    .layer(SetResponseHeaderLayer::if_not_present(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    ));
```

`tower-http` is already a transitive dep via Axum — check if it needs to be an explicit dep.

### `AllowAllPolicySource` cleanup

The struct is in `fke-server/src/kiln_policy.rs` with `#[allow(dead_code)]`.
Options:
1. Delete it — move any tests that use it to use `DbKilnPolicySource` with a mock pool
2. Rename to `TestAllowAllPolicySource` and gate `#[cfg(test)]`

Option 2 is cleaner — keeps it for tests, removes production risk.

### Audit checklist

- [ ] No JWT, bearer, or claims values in any `tracing` macros at any level
- [ ] All `expect()` and `panic!()` in binary crates are at startup only (OK) — not in request handlers
- [ ] User-facing errors never include stack traces, SQL errors, or internal paths
- [ ] MCP `tools/call` validates tool name is in the known set before dispatch
