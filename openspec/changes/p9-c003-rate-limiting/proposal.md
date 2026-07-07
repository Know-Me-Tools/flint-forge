# p9-c003 — Rate Limiting Middleware

**Phase:** 9 — Production Hardening
**Priority:** P0
**Depends on:** none

## What this change delivers

Per-IP token-bucket rate limiting on all `fdb-gateway` REST routes.
Returns `429 Too Many Requests` with a `Retry-After` header when the limit is exceeded.

## Design

### Crate choice: `tower-governor = "0.8"`

`tower-governor` wraps the `governor` crate (token-bucket algorithm, IP-keyed)
and provides a Tower middleware compatible with Axum. It extracts the client IP
from the `X-Forwarded-For` header (configurable) or the peer socket address.

### Configuration (env vars)

| Variable | Default | Meaning |
|---|---|---|
| `FLINT_RATE_LIMIT_REST` | `100` | Requests per second per IP for REST routes |
| `FLINT_RATE_LIMIT_GRAPHQL` | `20` | Requests per second per IP for `/graphql` |
| `FLINT_RATE_LIMIT_BURST` | `10` | Burst allowance (tokens above steady-state) |

Setting any variable to `0` disables limiting for that route group.

### Integration in `main.rs`

```rust
use tower_governor::{GovernorConfigBuilder, GovernorLayer};

let governor_conf = GovernorConfigBuilder::default()
    .per_second(rest_limit)
    .burst_size(burst)
    .finish()
    .expect("governor config");

let app = Router::new()
    // ... existing routes ...
    .layer(GovernorLayer { config: Arc::new(governor_conf) });
```

### New workspace dep

```toml
tower-governor = "0.8"
```

### `429` response body

```json
{ "error": "rate limit exceeded", "retry_after_secs": 1 }
```
