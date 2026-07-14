# Flint Forge — Postgres 18 data-plane image.
#
# Extensions built in the Anvil stage (all pgrx 0.18.1 / pg18):
#   flint_llm      — LLM embedding/summary job queue + background worker
#   flint_vault    — XChaCha20-Poly1305 secret store
#   ext-flint-meta — flint_meta schema-cache + reflection triggers
#   ext-flint-auth — auth.* GUC-backed identity helpers
#   ext-flint-hooks — flint.webhooks / webhook_outbox / dispatch
#
# C extensions built from source:
#   pg_net (Supabase), pg_cron (Citus)
#
# Apt extensions:
#   pgvector, pgcrypto
#
# Third-party extensions without a reliable PG18 source (e.g., pg_graphql) are
# tracked in docs/postgres-extensions.md and added once packages are available.
#
# Build (context = repo root):
#   docker build -f images/postgres18/Dockerfile -t flint-forge-pg:18 .

# ─────────────────────────── Stage: Anvil (all Flint pgrx extensions) ────────────────────
FROM rust:1.96-bookworm AS anvil
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      postgresql-common gnupg ca-certificates clang pkg-config libssl-dev \
 && /usr/share/postgresql-common/pgdg/apt.postgresql.org.sh -y \
 && apt-get update \
 && apt-get install -y --no-install-recommends postgresql-server-dev-18 \
 && apt-get install -y -f --no-install-recommends \
 && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-pgrx --version 0.18.1 --locked
RUN cargo pgrx init --pg18 "$(which pg_config)"

# Helper: build a standalone pgrx crate and overlay any hand-written .control file.
ARG PGRX_PG_CONFIG=/usr/lib/postgresql/18/bin/pg_config
WORKDIR /build

# flint_llm
COPY crates/ext-flint-llm/ /build/ext-flint-llm/
WORKDIR /build/ext-flint-llm
RUN cargo pgrx package --pg-config "$PGRX_PG_CONFIG" --out-dir /out

# flint_vault
COPY crates/ext-flint-vault/ /build/ext-flint-vault/
WORKDIR /build/ext-flint-vault
RUN cargo pgrx package --pg-config "$PGRX_PG_CONFIG" --out-dir /out
# Preserve the hand-tuned control file (sets schema=vault, superuser=true).
RUN cp /build/ext-flint-vault/flint_vault.control /out/usr/share/postgresql/18/extension/

# ext-flint-meta
COPY crates/ext-flint-meta/ /build/ext-flint-meta/
WORKDIR /build/ext-flint-meta
RUN cargo pgrx package --pg-config "$PGRX_PG_CONFIG" --out-dir /out

# ext-flint-auth
COPY crates/ext-flint-auth/ /build/ext-flint-auth/
WORKDIR /build/ext-flint-auth
RUN cargo pgrx package --pg-config "$PGRX_PG_CONFIG" --out-dir /out

# ext-flint-hooks
COPY crates/ext-flint-hooks/ /build/ext-flint-hooks/
WORKDIR /build/ext-flint-hooks
RUN cargo pgrx package --pg-config "$PGRX_PG_CONFIG" --out-dir /out

# ─────────────────────────── Stage: pg_net (C, from source) ──────────────────────────────
FROM rust:1.96-bookworm AS pgnet
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      postgresql-common gnupg ca-certificates git make gcc libcurl4-openssl-dev \
 && /usr/share/postgresql-common/pgdg/apt.postgresql.org.sh -y \
 && apt-get update \
 && apt-get install -y --no-install-recommends postgresql-server-dev-18 \
 && apt-get install -y -f --no-install-recommends \
 && rm -rf /var/lib/apt/lists/*
RUN git clone --depth 1 https://github.com/supabase/pg_net /src/pg_net
WORKDIR /src/pg_net
RUN make && make install DESTDIR=/out

# ──────────────────────────────── Stage: pg_cron (C, from source) ────────────────────────
# Use the same Rust/Debian base as the other builder stages to avoid the broken
# python3-minimal postinst state present in the slim postgres runtime image.
FROM rust:1.96-bookworm AS pgcron
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      postgresql-common gnupg ca-certificates git make gcc \
 && /usr/share/postgresql-common/pgdg/apt.postgresql.org.sh -y \
 && apt-get update \
 && apt-get install -y --no-install-recommends postgresql-server-dev-18 \
 && apt-get install -y -f --no-install-recommends \
 && rm -rf /var/lib/apt/lists/*
RUN git clone --depth 1 https://github.com/citusdata/pg_cron /src/pg_cron
WORKDIR /src/pg_cron
# pg_cron's Makefile hardcodes `SHLIB_LINK = $(libpq) -lintl`, but glibc provides
# intl internally and ships no `libintl.so` link target on Debian bookworm (any
# arch), so `-lintl` fails at link time. Strip the spurious flag.
RUN sed -i 's/^SHLIB_LINK = $(libpq) -lintl/SHLIB_LINK = $(libpq)/' Makefile
RUN make PG_CONFIG=$(which pg_config) && make install DESTDIR=/out PG_CONFIG=$(which pg_config)

# ─────────────────────────────────── Stage: runtime ─────────────────────────────────────
FROM postgres:18
RUN apt-get update \
 && apt-get install -y --no-install-recommends postgresql-18-pgvector libcurl4 curl \
 && rm -rf /var/lib/apt/lists/*

# p16-c008: wal-g — continuous WAL archiving + base backups to S3-compatible storage
# for point-in-time recovery (see docs/runbook.md §13.4). Pinned release + checksum,
# matching the version-pinning convention used for pgvector/pg_graphql above.
# Built against Ubuntu 22.04 (glibc 2.35) rather than 24.04 (glibc 2.39) because
# Debian bookworm (this image's base) ships glibc 2.36 — the 22.04 build is the
# closest ABI match that is guaranteed to run.
ARG WALG_VERSION=v3.0.8
ARG WALG_SHA256=b0df1b484035eb5f131db7bbd303d1a460391848fdcce34ba1e0a564cca493e9
RUN curl -fsSL -o /tmp/wal-g.tar.gz \
      "https://github.com/wal-g/wal-g/releases/download/${WALG_VERSION}/wal-g-pg-22.04-amd64.tar.gz" \
 && echo "${WALG_SHA256}  /tmp/wal-g.tar.gz" | sha256sum -c - \
 && tar -xzf /tmp/wal-g.tar.gz -C /tmp \
 && install -m 0755 /tmp/wal-g-pg-22.04-amd64 /usr/local/bin/wal-g \
 && rm -f /tmp/wal-g.tar.gz /tmp/wal-g-pg-22.04-amd64

# Compiled extensions from the builder stages.
COPY --from=anvil /out/ /
COPY --from=pgnet /out/ /
COPY --from=pgcron /out/ /

# Third-party extensions not yet in PGDG for Postgres 18 (e.g., pg_graphql) are
# intentionally deferred until a reliable PG18 package/source is available.
# See docs/postgres-extensions.md for the tracking list.

# First-boot: create extensions, grant permissions, and assert the data plane is complete.
COPY images/postgres18/init/01-extensions.sql      /docker-entrypoint-initdb.d/01-extensions.sql
COPY images/postgres18/init/02-grants.sql          /docker-entrypoint-initdb.d/02-grants.sql
COPY images/postgres18/init-baseline/99-assert.sql /docker-entrypoint-initdb.d/99-assert.sql

# p16-c008: translates wal-g S3 Docker secrets into env vars, then execs the
# official postgres entrypoint unchanged (no-op if the secrets aren't mounted
# — see docs/runbook.md §13.4).
COPY images/postgres18/entrypoint-walg.sh /usr/local/bin/entrypoint-walg.sh
RUN chmod +x /usr/local/bin/entrypoint-walg.sh
ENTRYPOINT ["entrypoint-walg.sh"]

# p16-c008: periodic wal-g base-backup loop — used only by the `backup`
# service in docker-compose.prod.yml (that service overrides `entrypoint:` to
# run this script directly instead of Postgres itself; it shares this same
# image purely so wal-g is already present, not to run a second Postgres).
COPY images/postgres18/backup-loop.sh /usr/local/bin/backup-loop.sh
RUN chmod +x /usr/local/bin/backup-loop.sh

# pg_net, pg_cron and flint_llm register background workers → must be preloaded.
# Production (docker-compose.prod.yml) overrides `command:` to additionally set
# archive_mode/archive_command for wal-g continuous WAL archiving.
CMD ["postgres", "-c", "wal_level=logical", "-c", "shared_preload_libraries=pg_net,pg_cron,flint_llm", "-c", "cron.database_name=flint"]
