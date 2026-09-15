# Postgres 18 image — extension inventory

Tracking doc for the extensions shipped by each `images/postgres18/` image
variant, how they are built, and what runtime wiring they need. Referenced
from the Dockerfile headers; keep this file in sync with any image change.

## Image variants

| Variant | Dockerfile | Purpose |
|---|---|---|
| **baseline** | `Dockerfile.baseline` | Stock PG18 + pgvector + pg_graphql (the deployed data-volume baseline) |
| **ember** (production) | `Dockerfile.ember` | Baseline digest + flint_llm (pgrx) + pg_net + pg_cron; preserves the live PG18/vector/GraphQL volume layout |
| **full** | `Dockerfile` | From-scratch image: all Anvil pgrx extensions + pg_net + pg_cron (no pg_graphql yet — see below) |

## Extension inventory

| Extension | Kind | Source / pin | Baseline | Ember | Full | `shared_preload_libraries` |
|---|---|---|---|---|---|---|
| pgvector | apt | `postgresql-18-pgvector` (PGDG) | ✓ | ✓ | ✓ | no |
| pgcrypto | contrib | ships with PG18 | ✓ | ✓ | ✓ | no |
| pg_graphql | apt (baseline) | PGDG `1.6.1` | ✓ | ✓ | ✗ | **yes** |
| flint_llm | pgrx 0.18.1 | `crates/ext-flint-llm` | ✗ | ✓ | ✓ | **yes** |
| flint_vault | pgrx 0.18.1 | `crates/ext-flint-vault` | ✗ | ✗ | ✓ | no |
| ext-flint-meta | pgrx 0.18.1 | `crates/ext-flint-meta` | ✗ | ✗ | ✓ | no |
| ext-flint-auth | pgrx 0.18.1 | `crates/ext-flint-auth` | ✗ | ✗ | ✓ | no |
| ext-flint-hooks | pgrx 0.18.1 | `crates/ext-flint-hooks` | ✗ | ✗ | ✓ | no |
| pg_net | C | `supabase/pg_net` **v0.20.5** | ✗ | ✓ | ✓ (unpinned master) | **yes** |
| pg_cron | C | `citusdata/pg_cron` **v1.6.8** | ✗ | ✓ | ✓ (unpinned master) | **yes** |
| wal-g | binary | v3.0.8, sha256-pinned | ✗ | ✗ | ✓ | no |

Notes:

- **pg_graphql is absent from the full image** — no reliable PG18 source
  build at last check (Supabase builds from master only). The full image's
  init script tolerates its absence (GraphQL passthrough degrades, the data
  plane still boots). Ember keeps the baseline's apt-packaged 1.6.1.
- pg_cron's Makefile needs the bookworm `-lintl` strip (glibc provides intl
  internally); both Dockerfiles carry the `sed` fix.
- pg_net links `libcurl4` at runtime — both runtimes install it.

## Runtime wiring

### `shared_preload_libraries`

Ember production value (set as k8s StatefulSet args in flint-core-infra,
which override the image `CMD` entirely — keep them in sync):

```
shared_preload_libraries=pg_graphql,flint_llm,pg_net,pg_cron
cron.database_name=flint
wal_level=logical
```

Forgetting a preload: `CREATE EXTENSION pg_net/pg_cron/flint_llm` errors
out, and `CREATE EXTENSION pg_graphql` fails without its hooks preloaded.

### First boot (fresh volume)

`/docker-entrypoint-initdb.d/` in the **full** image (`init/01-extensions.sql`)
creates every extension and schedules the pg_cron jobs. The ember image ships
no initdb scripts; fresh ember volumes are bootstrapped by the k8s
`postgres-bootstrap` ConfigMap (flint-core-infra), which sources the same
versioned upgrade scripts via `\i /opt/flint/upgrade/*.sql`.

### Existing volumes (upgrade path)

`upgrade/` scripts are baked into the ember image at `/opt/flint/upgrade/`
and run explicitly after an image replacement:

| Script | What it does |
|---|---|
| `001-adopt-ember.sql` | Adopts the legacy SQL-only `llm` schema into the real `flint_llm` pgrx extension (empty `0.0.0` container → `ALTER EXTENSION ADD` → update to `0.1.0`) |
| `002-pg-net-cron.sql` | `CREATE EXTENSION pg_net, pg_cron`; schedules `webhook-outbox-processor` (every minute), `webhook-outbox-gc` (nightly 03:00), `meta-full-refresh` (nightly 02:00) |

Both are idempotent (advisory-locked; `cron.schedule` upserts by job name).

## Webhook delivery pipeline (why pg_net/pg_cron are required)

`flint.dispatch_webhook()` (trigger) enqueues into `flint.webhook_outbox`;
`flint.process_webhook_outbox()` delivers pending rows via
`net.http_post(...)` — **this function is a no-op failure without pg_net
installed**. pg_cron drives it every minute via the
`webhook-outbox-processor` job. Entries with `target_type = 'kiln'` wait for
the Phase 6 background worker and are intentionally skipped by the cron path.
