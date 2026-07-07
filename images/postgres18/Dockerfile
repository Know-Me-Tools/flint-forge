# Flint Forge — Postgres 18 image (data plane, option (c): no pg_graphql yet).
#
# Extensions: pgvector (apt), pgcrypto (base), flint_auth + flint_hooks (SQL-only),
# flint_llm (pgrx 0.18.1), pg_net (source). pg_graphql is deferred to Phase 3 — see
# openspec/changes/p0-c002-pg18-image/tasks.md (supabase/pg_graphql#614, no PG18 release).
#
# Build (context = repo root):
#   docker build -f images/postgres18/Dockerfile -t flint-forge-pg:18 .

# ─────────────────────────── Stage: Anvil (flint_llm via pgrx) ───────────────────────────
FROM rust:1.96-bookworm AS anvil
RUN apt-get update && apt-get install -y --no-install-recommends \
      postgresql-common gnupg ca-certificates clang pkg-config libssl-dev \
 && /usr/share/postgresql-common/pgdg/apt.postgresql.org.sh -y \
 && apt-get install -y --no-install-recommends postgresql-server-dev-18 \
 && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-pgrx --version 0.18.1 --locked
RUN cargo pgrx init --pg18 "$(which pg_config)"
WORKDIR /build/flint_llm
COPY crates/ext-flint-llm/ .
RUN cargo pgrx package --pg-config "$(which pg_config)" --out-dir /out

# ─────────────────────────── Stage: pg_net (C, from source) ──────────────────────────────
FROM rust:1.96-bookworm AS pgnet
RUN apt-get update && apt-get install -y --no-install-recommends \
      postgresql-common gnupg ca-certificates git make gcc libcurl4-openssl-dev \
 && /usr/share/postgresql-common/pgdg/apt.postgresql.org.sh -y \
 && apt-get install -y --no-install-recommends postgresql-server-dev-18 \
 && rm -rf /var/lib/apt/lists/*
RUN git clone --depth 1 https://github.com/supabase/pg_net /src/pg_net
WORKDIR /src/pg_net
RUN make && make install DESTDIR=/out

# ──────────────────────────────── Stage: pg_cron (C, from source) ────────────────────────
FROM postgres:18 AS pgcron
RUN apt-get update && apt-get install -y --no-install-recommends \
      postgresql-common gnupg ca-certificates git make gcc \
 && /usr/share/postgresql-common/pgdg/apt.postgresql.org.sh -y \
 && apt-get install -y --no-install-recommends postgresql-server-dev-18 \
 && rm -rf /var/lib/apt/lists/*
RUN git clone --depth 1 https://github.com/citusdata/pg_cron /src/pg_cron
WORKDIR /src/pg_cron
RUN make PG_CONFIG=$(which pg_config) && make install DESTDIR=/out PG_CONFIG=$(which pg_config)

# ─────────────────────────────────── Stage: runtime ─────────────────────────────────────
FROM postgres:18
RUN apt-get update \
 && apt-get install -y --no-install-recommends postgresql-18-pgvector libcurl4 \
 && rm -rf /var/lib/apt/lists/*

# Compiled extensions from the builder stages.
COPY --from=anvil /out/ /
COPY --from=pgnet /out/ /
COPY --from=pgcron /out/ /

# SQL-only Flint extensions (no Rust): flint_auth, flint_hooks.
COPY images/postgres18/extensions/ /usr/share/postgresql/18/extension/

# First-boot: create extensions, load llm schema, assert.
COPY images/postgres18/init/01-extensions.sql      /docker-entrypoint-initdb.d/01-extensions.sql
COPY crates/ext-flint-llm/sql/flint_llm.sql        /docker-entrypoint-initdb.d/30-flint-llm-schema.sql
COPY images/postgres18/init-baseline/99-assert.sql /docker-entrypoint-initdb.d/99-assert.sql

# pg_net and pg_cron register background workers → must be preloaded. (flint_llm's BGW lands in Phase 4.)
CMD ["postgres", "-c", "wal_level=logical", "-c", "shared_preload_libraries=pg_net,pg_cron,ext_flint_llm", "-c", "cron.database_name=postgres"]
