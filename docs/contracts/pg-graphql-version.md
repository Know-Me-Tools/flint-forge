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

### Pinned version (supersedes "Pinned SHA")

**Pinned: `v1.6.2`** — a release version, not a git SHA.

supabase/pg_graphql **v1.6.2** was published 2026-08-26 with prebuilt PG18 assets for both
architectures this image is built for:

```
pg_graphql-v1.6.2-pg18-amd64-linux-gnu.deb
pg_graphql-v1.6.2-pg18-arm64-linux-gnu.deb
```

That removes the reason this document prescribed compiling from a git SHA with
`cargo pgrx package --pg18`: there was no released PG18 build when it was written, and there is now.
A release is a reproducible, checksummed artifact; a master SHA is a moving target that has to be
re-validated on every rebuild. **Pin the release.**

To move the pin, change `ARG PG_GRAPHQL_VERSION` in `images/postgres18/Dockerfile` and this line, and
confirm the new tag publishes a `-pg18-` asset for amd64 **and** arm64 before merging.

### Dockerfile impact — as built

The `.deb` lays its files out under `/var/lib/postgresql` and `/usr/lib/postgresql`, which are not the
paths `dpkg -i` installs into on this image, so the runtime stage extracts the package and copies the
real files into the PG18 directories:

```dockerfile
ARG PG_GRAPHQL_VERSION=v1.6.2
RUN curl -fsSL -o /tmp/pg_graphql.deb \
      "https://github.com/supabase/pg_graphql/releases/download/${PG_GRAPHQL_VERSION}/pg_graphql-${PG_GRAPHQL_VERSION}-pg18-$(dpkg --print-architecture)-linux-gnu.deb" \
 && dpkg -x /tmp/pg_graphql.deb /tmp/pg_graphql \
 && cp /tmp/pg_graphql/var/lib/postgresql/extension/pg_graphql.control \
       /tmp/pg_graphql/var/lib/postgresql/extension/pg_graphql--*.sql \
       /usr/share/postgresql/18/extension/ \
 && cp /tmp/pg_graphql/usr/lib/postgresql/lib/pg_graphql.so /usr/lib/postgresql/18/lib/ \
 && rm -rf /tmp/pg_graphql.deb /tmp/pg_graphql
```

**No `shared_preload_libraries` entry.** An earlier draft of this document listed
`shared_preload_libraries=pg_net,pg_cron,pg_graphql`; pg_graphql registers on `CREATE EXTENSION`, which
the init SQL issues, so preloading it is unnecessary. `pg_net`, `pg_cron` and `flint_llm` register
background workers and do need it.

### Absence is now fatal, in two places

`images/postgres18/init/01-extensions.sql` previously wrapped the create in a `DO $$ … EXCEPTION WHEN
others THEN RAISE WARNING` block, so an image without the extension booted green and GraphQL was silently
absent. GraphQL is a sync path, not an optional passthrough. The handler is removed, and
`init-baseline/99-assert.sql` now also asserts `pg_graphql` is present alongside every other extension it
already checked.

### Accept criteria — met

- [x] A released PG18 build exists and is pinned by version (`v1.6.2`, 2026-08-26)
- [x] Version documented in this file
- [x] `images/postgres18/Dockerfile` installs it from the release `.deb`
- [x] `01-extensions.sql` creates pg_graphql, and no longer tolerates its absence
- [x] Boot assertion includes pg_graphql

### Note — `docker/postgres/Dockerfile` is unused

That file already installed pg_graphql correctly (at v1.6.1) while the image everything actually builds
did not. It has **no build reference** in any compose file or workflow; `images/postgres18/Dockerfile` is
referenced by `docker-compose.yml`, `docker-compose.prod.yml` and four GitHub workflows. Raised separately
— having two Postgres image definitions where the unused one is the more correct is its own defect.
