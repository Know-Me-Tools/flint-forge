# Contract: pg_graphql Version Strategy

## OQ-3 Resolution (2026-06-30)

**Status: RESOLVED — build from pinned master SHA**

### Current Situation

As of 2026-06-30, `supabase/pg_graphql` has no tagged release for Postgres 18.
See upstream tracking issue: supabase/pg_graphql#614.

The latest stable release (v1.5.12 or later) targets PG17. A PG18 build requires
compiling from the `master` branch (which receives PG18 CI from Supabase internally
but has no versioned release artifact).

### Decision

**Strategy: compile pg_graphql from a pinned `master` SHA in the Dockerfile.**

Rationale:
- The Supabase Cloud PG18 preview already runs pg_graphql on PG18 internally.
- `master` HEAD compiles against PG18 server headers (verified in CI).
- Pinning to an explicit SHA gives reproducible builds; a SHA bump is a deliberate review.
- Alternative (PG17 sidecar) adds network topology complexity and loses RLS integration.

### Pinned SHA

At Phase 3 kickoff, the executor must:
1. Check `https://github.com/supabase/pg_graphql/commits/master` for the latest commit.
2. Test compile against PG18 locally: `cargo pgrx package --pg18`.
3. Pin the passing SHA in the Dockerfile comment and in this document.

**Placeholder (update before first build):**
```
PINNED_PG_GRAPHQL_SHA=<to-be-set-at-p3-c001-coding-time>
```

### Dockerfile Impact

The `images/postgres18/Dockerfile` must add a `pg_graphql` builder stage:

```dockerfile
FROM rust:1.96-bookworm AS pg_graphql_build
ARG PG_GRAPHQL_SHA=<pinned-sha>
RUN apt-get update && apt-get install -y --no-install-recommends \
      postgresql-common gnupg ca-certificates git clang pkg-config libssl-dev \
 && /usr/share/postgresql-common/pgdg/apt.postgresql.org.sh -y \
 && apt-get install -y --no-install-recommends postgresql-server-dev-18 \
 && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-pgrx --version 0.18.1 --locked
RUN cargo pgrx init --pg18 "$(which pg_config)"
RUN git clone https://github.com/supabase/pg_graphql /src/pg_graphql \
 && cd /src/pg_graphql \
 && git checkout ${PG_GRAPHQL_SHA}
WORKDIR /src/pg_graphql
RUN cargo pgrx package --pg-config "$(which pg_config)" --out-dir /out
```

And in the runtime stage:
```dockerfile
COPY --from=pg_graphql_build /out/ /
```

And in the init SQL:
```sql
CREATE EXTENSION IF NOT EXISTS pg_graphql CASCADE;
```

And in the CMD:
```
shared_preload_libraries=pg_net,pg_cron,pg_graphql
```

### Accept Criteria for Phase 3 Kickoff
- [ ] A passing PG18 build SHA is found (compile + `SELECT graphql.resolve(...)` works)
- [ ] SHA documented in this file
- [ ] Dockerfile updated with builder stage
- [ ] `01-extensions.sql` creates pg_graphql
- [ ] Boot assertion passes with pg_graphql in extension list

### Fallback

If no SHA compiles cleanly against PG18:
- Report blocker immediately before any Phase 3 coding begins
- Evaluate: PG17 sidecar (GraphQL on PG17, REST on PG18 via connection switch at gateway layer)
- This fallback is significant scope change — escalate before proceeding
