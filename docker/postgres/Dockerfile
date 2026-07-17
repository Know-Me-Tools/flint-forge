# Flint Forge CI/dev Postgres image — PG18 + pgvector + pg_graphql.
#
# The workspace migrations (migrations/0002+) require the `vector` (pgvector) and
# `pg_graphql` extensions; a bare `postgres:18` cannot run them (verified: migration
# 0002 fails with `extension "vector" is not available`). No single upstream image
# reliably bundles BOTH, so we pin our own.
#
# Build:   docker build -t flint-forge/postgres:18 docker/postgres
# Run:     docker run -e POSTGRES_PASSWORD=postgres -p 5432:5432 flint-forge/postgres:18
# Used by: scripts/ci-test.sh (DATABASE_URL) and the Dagger CheckDb service binding.
#
# Versions are pinned for reproducibility (12-factor: config/deps are explicit, not
# floating). Bump deliberately.

FROM postgres:18-bookworm

# Toolchain to build the extensions from source against this exact PG major.
# pgxs (in postgresql-server-dev) is the canonical build path for both extensions.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        git \
        ca-certificates \
        postgresql-server-dev-18 \
        libclang-dev \
        curl \
    && rm -rf /var/lib/apt/lists/*

# --- pgvector (the `vector` extension) ---
# v0.8.0 does not compile on PG18: pgvector calls vacuum_delay_point() with no
# arguments, but PG18 changed its signature to take a `bool is_analyze` (fixed
# upstream in 0.8.1+). Pinned to the latest 0.8.x.
ARG PGVECTOR_REF=v0.8.5
RUN git clone --branch "${PGVECTOR_REF}" --depth 1 https://github.com/pgvector/pgvector.git /tmp/pgvector \
    && cd /tmp/pgvector \
    && make \
    && make install \
    && rm -rf /tmp/pgvector

# --- pg_graphql (Rust/pgrx extension) ---
# pg_graphql ships prebuilt .deb assets per PG major; prefer those over a full pgrx
# build to keep the image small and the build fast. Pin the release.
# v1.5.11 has no pg18 asset (supabase published pg18 .deb starting at v1.5.12) —
# pinned to the latest release with pg18 amd64/arm64 assets.
ARG PG_GRAPHQL_REF=v1.6.1
RUN curl -fsSL -o /tmp/pg_graphql.deb \
      "https://github.com/supabase/pg_graphql/releases/download/${PG_GRAPHQL_REF}/pg_graphql-${PG_GRAPHQL_REF}-pg18-$(dpkg --print-architecture)-linux-gnu.deb" \
    && dpkg -i /tmp/pg_graphql.deb \
    && rm -f /tmp/pg_graphql.deb

# Preload pg_graphql (it registers on CREATE EXTENSION; migrations issue that).
# pgvector needs no preload. Health: the default postgres image already exposes 5432.
#
# NOTE: the `flint_meta` schema is provided by the `ext-flint-meta` pgrx extension
# (workspace-excluded, built via cargo-pgrx). CI that runs the full migration set must
# either install that extension into this image or apply a flint_meta bootstrap SQL
# first — tracked as a follow-up in the c003 change notes (the migrations themselves
# assume flint_meta already exists).
