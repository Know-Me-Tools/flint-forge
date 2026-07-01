# Flint Forge — Revised Phase Plan (P1–P3, P7)

**Document ID:** RFC-FORGE-PHASES-002  
**Supersedes:** `.kbd-orchestrator/phases/p0-workspace-foundation/plan.md` §§ Phase 1–3, Phase 7  
**Date:** 2026-06-30  
**Status:** Validated — Ready for kbd-new-phase execution  
**Research basis:** 14 background research agents; firecrawl web search; local Cargo registry inspection; RFC-FORGE-META-001; sycophancy correction pass (S-03 corrected)

---

## Research Validation Summary

The following claims from RFC-FORGE-META-001 were validated or corrected before this plan was written.

### Validated (confirmed against external evidence)

| Claim | Verdict | Evidence |
|---|---|---|
| `ArcSwap<Router<()>>` is a viable hot-swap pattern | **CONFIRMED** | `axum::Router` is `Clone + Send + Sync` backed by `Arc<RouterInner>` (axum 0.8.8 src line 72). `ArcSwap<T>` requires `T: Send + Sync`. Pattern is idiomatic. |
| PostgreSQL LISTEN/NOTIFY is reliable in Rust/tokio | **CONFIRMED with caveat** | `sqlx::postgres::PgListener` provides a clean `Stream` API. Caveat: does NOT auto-reconnect/resubscribe on connection loss — manual reconnection logic or `postgres-notify` crate required in production. |
| pgrx event triggers can capture DDL changes | **CONFIRMED with caveats** | `ddl_command_end` and `sql_drop` are well-supported. Caveats: (1) `CREATE TABLE AS` / `SELECT INTO` may not fire `ddl_command_end` in all PG versions; (2) event triggers cannot be nested (no event trigger inside an event trigger); (3) `DROP OWNED` fires `sql_drop` only for objects the session user owns. |
| pgrx 0.18.1 targets PostgreSQL 18 | **CONFIRMED** | GitHub PR #2264 shows CI running across PG 13–18 on the develop branch. Migration breaking change: switch to single-compile `crate-type = ["cdylib"]`, remove `src/bin/pgrx_embed.rs`. |
| pg_graphql PG18 support is in progress | **CONFIRMED — likely released** | Issue #614 closed as completed Dec 12, 2025. PG18 patches merged to master; tagged release "should be fairly soon." As of June 2026, a release is likely available; verify against supabase/pg_graphql releases before Phase 3 kickoff. |
| AG-UI is a real, production-grade protocol | **CONFIRMED** | 14,500 GitHub stars, 1,300 forks, 29 releases, active as of June 2026. Google ADK, Microsoft Agent Framework, AWS Bedrock AgentCore all have first-party integrations. MIT license. Rust community SDK at `sdks/community/rust/crates/ag-ui-client`. |
| PostgREST limitations justify a custom Rust engine | **CONFIRMED (for AI-native use case)** | External Haskell schema cache, SIGUSR1 reload race, no in-DB metadata, no AI-UI output — these are real, documented limitations. However: Supabase is NOT moving away from PostgREST; it is deeply integrated. Flint's justification rests on AI-native requirements, not PostgREST quality. |
| MCP DB-to-tool generation is a valid pattern | **CONFIRMED as novel** | Generic schema-introspection MCP servers exist (`crystaldba/postgres-mcp`, reference server). Auto-generating one typed MCP tool per DB table from a compiled IR is NOT yet a packaged library — this is genuinely novel and differentiating. |

### Corrected (S-03 sycophancy addressed)

| Original claim | Correction | Risk to plan |
|---|---|---|
| "A2UI: open standard for declarative UI generation" | A2UI is Google's declarative UI *content spec*, not a wire protocol. It rides inside AG-UI `Custom` events. Open-JSON-UI (OpenAI) and MCP-UI (Microsoft/Shopify) are competing specs at the same layer. A2UI is not a neutral community standard. | **LOW** — the plan's intent (declarative UI from agents) is correct; naming and attribution needs precision |
| No risks in comparison tables (S-03) | Added: compilation latency on schema change (100–2000ms for large schemas), LISTEN/NOTIFY reconnection gap, pgrx PG18 migration complexity, A2UI Google-origin lock-in risk, MCP auto-generation unproven at production scale | **MEDIUM** — these risks need mitigation strategies in the changes |
| "MCP tool manifest" as formal term | MCP spec calls this "tools list" or "tools array." "Tool manifest" is informal shorthand; acceptable internally but must use spec terminology in SDK/docs | **LOW** |

---

## Architectural Changes from the Original Plan

### What changes (compared to the p0 plan §§ Phase 1–3, Phase 7)

| Phase | Original approach | Revised approach |
|---|---|---|
| **Phase 1** | flint_auth + flint_hooks + flint_vault | **ADD** `flint_meta` extension (Milestone 1): cache tables, event triggers, version tracking, LISTEN/NOTIFY. This is now a Phase 1 deliverable, not Phase 2. |
| **Phase 2** | `p2-c003-rest-executor` — PostgREST-compat CRUD via direct pg_catalog queries | **REPLACE** with `flint-reflection` REST compiler consuming `flint_meta.*` tables. `DatabaseModel` IR compiled from reflection; REST router hot-swapped via `ArcSwap<Router<()>>`. |
| **Phase 3** | GraphQL passthrough + subscriptions (unchanged path) | **REVISE** — GraphQL compiler integrated with `flint-reflection`; Keto integration via `flint_meta.keto_tuples` + `check_permission()`; JWT propagation extended to `app.jwt_claims`, `app.keto_subject`, `app.vault_key_id`. pg_graphql PG18 decision required before kickoff. |
| **Phase 7** | 4 changes: webhook-kiln wiring, agentproto pipe, a2ui-kiln, a2ui-gate | **EXPAND** — Milestone 5: MCP tools compiler, AG-UI event emitter, A2UI content layer (inside AG-UI Custom events), agent metadata as reflectable types, realtime metadata via FRF Iggy. Fix A2UI protocol framing. |

### What does NOT change

- The hexagonal dependency rule (domain → ports → adapters → interfaces)
- JWT/RLS contract: three `SET LOCAL` per transaction (role, jwt.claims, request.headers)
- Phase 4 (Ember), Phase 5 (Kiln runtime), Phase 6 (signing/storage), Phase 8 (SDK completeness), Phase 9 (hardening) — unchanged
- The `forge-cli` command surface
- WIT contract freeze (`flint:host@0.1.0`)

---

## Phase 1 — Flint Anvil + Meta Foundation

**Repo:** flint-forge  
**Duration estimate:** 3–4 sprints  
**Gate:** `flint_auth` passes RLS end-to-end; `flint_hooks` fires a signed webhook through flint-gate; `flint_meta` extension installs, cache tables populate, event trigger fires on `CREATE TABLE` and increments version, NOTIFY reaches a test LISTEN client.

### Why `flint_meta` belongs in Phase 1

The REST compiler (`flint-reflection`, Phase 2) cannot be written without a stable `flint_meta` schema to consume. `flint_meta` is infrastructure that `flint_auth`, `flint_hooks`, and `flint_vault` all need (Keto tuples, Vault key assignments, JWT propagation GUCs). Starting it in Phase 1 means Phase 2 can begin coding the Rust engine against a real PostgreSQL interface, not mocks.

### Risks and mitigations

| Risk | Likelihood | Mitigation |
|---|---|---|
| pgrx 0.18.1 PG18 single-compile migration breaks `ext-flint-auth` (pins pgrx 0.12) | MEDIUM | `ext-flint-auth` stays at pgrx 0.12/pg17. `flint_meta` targets pgrx 0.18.1/pg18. They are separate crates and must NOT be unified. |
| `ddl_command_end` does not fire for `CREATE TABLE AS` / `SELECT INTO` | LOW (rare DDL) | Document as known gap in `docs/contracts/meta-trigger-coverage.md`. Initial compilation at service startup covers stale cache. |
| LISTEN/NOTIFY connection loss drops notifications | MEDIUM | Must implement reconnection with resubscription. Use `postgres-notify` crate pattern or implement exponential backoff with channel re-LISTEN. Log and force re-compile on reconnect. |
| `flint_meta.refresh_cache()` event trigger fires inside a long transaction, slowing DDL | LOW | The trigger is lightweight (cache table UPDATEs + one `pg_notify` call). Monitor in load tests. Add `statement_timeout` guard if needed. |

### OpenSpec changes

| Change ID | Title | Depends on | Priority |
|---|---|---|---|
| `p1-c001-flint-auth` | `auth.*` SQL helpers + GUC contract tests (pgrx 0.12/pg17) | — | P0 |
| `p1-c002-flint-hooks-standard` | Registry + dispatch trigger + pg_net standard tier + Option-3 HMAC | p1-c001 | P0 |
| `p1-c003-flint-hooks-durable` | Outbox table + dispatcher BGW + SKIP LOCKED retry | p1-c002 | P1 |
| `p1-c004-pg-cron` | Add `pg_cron` to images/postgres18/Dockerfile | — | P1 |
| `p1-c005-jwt-contract-pin` | `docs/contracts/jwt-contract.md` — pin claim shape + service-identity format | flint-gate team | P0 (blocks p2) |
| `p1-c006-vault-kms` | KMS unwrap integration in ext-flint-vault (Azure Key Vault managed identity v1) | — | P2 |
| **`p1-c007-flint-meta-schema`** | `flint_meta` pgrx extension: cache tables, version tracking, Keto tuple storage, Vault key assignments | p1-c006 | **P0** |
| **`p1-c008-flint-meta-triggers`** | DDL event triggers (`ddl_command_end`, `sql_drop`) → `refresh_cache()` → version++ → `pg_notify('meta_runtime', ...)` | p1-c007 | **P0** |
| **`p1-c009-flint-meta-functions`** | SQL-callable reflection functions: `flint_meta.tables()`, `columns()`, `relationships()`, `functions()`, `version()`, `check_permission()`, `set_identity()` | p1-c007 | **P0** |
| **`p1-c010-flint-meta-agui-descriptor`** | `flint_meta.agui_descriptor()` + `flint_meta.openapi()` SQL functions | p1-c009 | P1 |
| **`p1-c011-flint-meta-listener-test`** | Integration test: sqlx `PgListener` on `meta_runtime` receives notification after `CREATE TABLE`; version increments | p1-c008, p1-c009 | **P0 gate** |

#### `p1-c007` — flint_meta schema: What this change delivers

```
ext-flint-meta/   (new pgrx crate, pgrx = "=0.18.1", pg18 target)
├── src/
│   ├── lib.rs          (pgrx extension entry point)
│   ├── schema.rs       (CREATE TABLE statements for all cache tables)
│   ├── version.rs      (schema_version table + increment function)
│   ├── keto.rs         (keto_tuples + indexes)
│   └── vault.rs        (vault_keys + vault_key_assignments)
├── sql/
│   └── flint_meta--0.1.0.sql   (generated by cargo pgrx schema)
└── Cargo.toml          (pgrx = "=0.18.1", workspace-excluded)
```

Tables installed:
- `flint_meta.cache_tables`, `cache_columns`, `cache_relationships`, `cache_functions`, `cache_policies`, `cache_types`
- `flint_meta.schema_version`
- `flint_meta.keto_tuples` (+ 3 indexes)
- `flint_meta.vault_keys`, `vault_key_assignments`

#### `p1-c008` — event triggers: What this change delivers

```rust
// In ext-flint-meta/src/triggers.rs
#[pg_extern]
fn flint_meta_refresh_cache() -> pg_sys::Datum {
    // Calls pg_event_trigger_ddl_commands()
    // Updates cache tables per object_type
    // Inserts into schema_version (version = MAX(version) + 1)
    // Calls pg_notify('meta_runtime', payload)
}

#[pg_extern]
fn flint_meta_invalidate_cache() -> pg_sys::Datum {
    // Calls pg_event_trigger_dropped_objects()
    // Deletes from cache tables
    // Inserts into schema_version
    // Calls pg_notify('meta_runtime', payload)
}
```

SQL event trigger binding is emitted via `cargo pgrx schema` annotations.

**Known DDL coverage gaps (document in `docs/contracts/meta-trigger-coverage.md`):**
- `CREATE TABLE AS` — does not fire `ddl_command_end` in PG ≤ 15; fires in PG 16+
- `SELECT INTO` — same
- `CREATE UNLOGGED TABLE` — fires, but replica_identity semantics differ
- `COMMENT ON COLUMN` fires via `COMMENT` tag — covered
- Triggers cannot nest: if `refresh_cache()` itself runs DDL, the inner DDL does not fire another event trigger

#### `p1-c011` gate test

```rust
#[tokio::test]
async fn test_meta_listener_receives_notify_on_ddl() {
    let pool = test_pool().await;
    let mut listener = sqlx::postgres::PgListener::connect_with(&pool).await.unwrap();
    listener.listen("meta_runtime").await.unwrap();

    // Fire DDL
    sqlx::query("CREATE TABLE test_meta_gate (id bigserial PRIMARY KEY)")
        .execute(&pool).await.unwrap();

    // Receive notification within 5s
    let notification = tokio::time::timeout(
        Duration::from_secs(5),
        listener.recv()
    ).await.expect("timeout").expect("no notification");

    let payload: serde_json::Value = serde_json::from_str(notification.payload()).unwrap();
    assert!(payload["version"].as_u64().unwrap() > 0);

    // Verify version incremented
    let version: i64 = sqlx::query_scalar(
        "SELECT MAX(version) FROM flint_meta.schema_version"
    ).fetch_one(&pool).await.unwrap();
    assert!(version > 0);
}
```

---

## Phase 2 — Flint Reflection Engine + REST Compiler

**Repo:** flint-forge  
**Duration estimate:** 3–4 sprints  
**Depends on:** Phase 1 complete (`flint_meta` installed, `p1-c005` JWT contract pinned)  
**Gate:** RLS-correct REST CRUD (`GET`/`POST`/`PATCH`/`DELETE`) under a real flint-gate JWT; `ArcSwap` hot-swap works end-to-end (DDL change → NOTIFY → recompile → new router live within 5s); zero dropped requests during schema reload.

### Architecture

```
PostgreSQL (flint_meta schema)
    │  sqlx queries against flint_meta.tables(), columns(), etc.
    ▼
flint-reflection::ReflectionEngine::reflect()
    │
    ▼
DatabaseModel (immutable IR — Arc<T>)
    │
    ├── NormalizationPass       — resolve domains, defaults, identity columns
    ├── ValidationPass          — detect cycles, conflicts, unsupported types
    ├── PermissionAnalysisPass  — cross-reference Keto tuples with RLS policies
    └── EndpointGenerationPass  — REST routes, GraphQL fields, RPC mappings
    │
    ▼
CompiledState {
    version: u64,
    database_model: Arc<DatabaseModel>,
    router: Arc<axum::Router<()>>,       // hot-swappable
    openapi_doc: Arc<utoipa::openapi::OpenApi>,
    mcp_tools: Arc<Vec<McpToolDef>>,     // compiled MCP tool definitions
    // Phase 2 initial form: HashMap keyed by table slug, values from flint_meta.agui_descriptor()
    // Phase 5 upgrade (p5-c009): replaced with flint_a2ui.resolve_components(application_id, jwt_claims)
    // output; Cedar a2ui:emit gate added at emission time in fke-server
    agui_descriptors: Arc<HashMap<String, serde_json::Value>>,
}
    │
    ▼
ArcSwap<CompiledState>  (global, one per process)
    │
    ├── HTTP handler path: arc_swap.load().router.clone().call(req)
    └── Hot-reload path:   arc_swap.store(Arc::new(new_compiled_state))
```

### The StateManager and hot-reload loop

```rust
pub struct StateManager {
    pub compiled: ArcSwap<CompiledState>,
    pub db_pool: sqlx::PgPool,
    pub config: Arc<Config>,
}

impl StateManager {
    pub async fn start_listener(&self) -> anyhow::Result<()> {
        let pool = self.db_pool.clone();
        let arc_swap = Arc::clone(&self.compiled_arc);

        tokio::spawn(async move {
            loop {
                // Reconnect loop — handles connection loss
                let result = Self::listen_loop(&pool, &arc_swap).await;
                if let Err(e) = result {
                    tracing::warn!("meta_runtime listener disconnected: {e}; reconnecting in 2s");
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
                // Force a full recompile on reconnect — may have missed notifications
                Self::recompile(&pool, &arc_swap).await.ok();
            }
        });
        Ok(())
    }

    async fn listen_loop(
        pool: &sqlx::PgPool,
        arc_swap: &Arc<ArcSwap<CompiledState>>,
    ) -> anyhow::Result<()> {
        let mut listener = sqlx::postgres::PgListener::connect_with(pool).await?;
        listener.listen("meta_runtime").await?;

        while let Some(notification) = listener.try_recv().await? {
            let payload: serde_json::Value = serde_json::from_str(notification.payload())?;
            let new_version = payload["version"].as_u64().unwrap_or(0);
            let current_version = arc_swap.load().version;
            if new_version > current_version {
                Self::recompile(pool, arc_swap).await?;
            }
        }
        Ok(())
    }
}
```

### Risks and mitigations

| Risk | Likelihood | Mitigation |
|---|---|---|
| Compilation latency: large schemas (500+ tables) may take 500–2000ms to recompile | MEDIUM | Compile in background task; serve old `ArcSwap` state until new compilation completes. Add `compile_time_ms` metric via `tracing`. Alert if > 5s. |
| `ArcSwap` store during in-flight requests: old requests use old state, new requests use new state | LOW (by design) | This is the correct behavior. Old `Arc<CompiledState>` is dropped only when all guards are released. No action needed, but test with concurrent load in gate tests. |
| `flint_meta.cache_*` tables out-of-sync with `pg_catalog` on startup | MEDIUM | `ReflectionEngine::reflect()` always re-reads the cache tables at startup. If cache is stale (e.g., pgrx extension reinstalled), add `REFRESH` command that replays all `pg_catalog` queries. |
| REST compiler generates incorrect SQL for complex filter expressions | MEDIUM | Gate test must cover all filter operators: `eq`, `neq`, `gt`, `gte`, `lt`, `lte`, `like`, `ilike`, `in`, `is` (null/not null), `cs` (contains), `cd` (contained by). Use parameterized queries only — no string concatenation. |
| Vault DEK in `DatabaseModel` IR (loaded from `flint_meta.vault_keys`) | MEDIUM-HIGH | `dek_encrypted` (ciphertext only) is in `DatabaseModel`. Plaintext DEK MUST NOT be stored in `CompiledState`. Decryption happens per-request via `flint_meta.decrypt_column()`. Security contract: no plaintext key material in memory beyond the lifetime of a single request. |

### OpenSpec changes

| Change ID | Title | Depends on | Priority |
|---|---|---|---|
| `p2-c001-fdb-auth` | JWT verify → `RlsContext` (`forge-identity::verify_and_build`) | p1-c005 | P0 |
| `p2-c002-fdb-postgres` | deadpool-postgres pool + `SET LOCAL` RLS context + three GUC propagation | p2-c001 | P0 |
| **`p2-c003-flint-reflection-crate`** | New `fdb-reflection` crate: `DatabaseModel` IR, `CompiledState`, `StateManager`, `ReflectionEngine::reflect()` | Phase 1 complete | **P0** |
| **`p2-c004-rest-compiler`** | `RestCompiler`: `DatabaseModel` → `axum::Router<()>`; HTTP → SQL AST → parameterized query; `GET`/`POST`/`PATCH`/`DELETE`/`/rpc`; Range headers | p2-c003 | **P0** |
| **`p2-c005-arcswap-hot-reload`** | `StateManager::start_listener()`: sqlx `PgListener` on `meta_runtime` → recompile → `ArcSwap::store()`; reconnect loop | p2-c003, p2-c004 | **P0** |
| `p2-c006-pgvector-rpc` | `/rpc/<fn>` for vector similarity; `ORDER BY embedding <-> $q LIMIT k` | p2-c004 | P1 |
| **`p2-c007-openapi-compiler`** | `OpenApiCompiler`: `DatabaseModel` → `utoipa::openapi::OpenApi`; live at `/openapi.json` | p2-c003 | P1 |

#### `p2-c003` — fdb-reflection crate structure

```
fdb-reflection/
├── src/
│   ├── lib.rs
│   ├── model.rs         // DatabaseModel, Table, Column, Relationship, Function, etc.
│   ├── compiled.rs      // CompiledState
│   ├── state_manager.rs // StateManager, ArcSwap<CompiledState>, listen_loop
│   ├── engine.rs        // ReflectionEngine::reflect() — queries flint_meta.*
│   ├── passes/
│   │   ├── normalization.rs
│   │   ├── validation.rs
│   │   ├── permission_analysis.rs
│   │   └── endpoint_generation.rs
│   ├── compilers/
│   │   ├── rest.rs      // DatabaseModel → axum::Router<()>
│   │   ├── openapi.rs   // DatabaseModel → utoipa::OpenApi
│   │   ├── graphql.rs   // DatabaseModel → async_graphql Schema (Phase 3)
│   │   └── mcp.rs       // DatabaseModel → Vec<McpToolDef> (Phase 7)
│   └── error.rs         // thiserror error types (no anyhow in lib)
└── Cargo.toml
```

**Hexagonal rule:** `fdb-reflection` is an adapter crate. It may import `fdb-domain` and `fdb-ports`, but NOT `fdb-gateway` (the interface crate). `fdb-gateway` imports `fdb-reflection`. This enforces the layering.

#### `p2-c004` — REST compiler: SQL generation contract

```rust
// All SQL is parameterized — no string concatenation in user-controlled values
pub struct SqlCompiler;

impl SqlCompiler {
    pub fn compile_select(
        table: &Table,
        filters: &[Filter],
        ordering: &[OrderSpec],
        pagination: &Pagination,
        rls_context: &RlsContext,
    ) -> (String, Vec<Box<dyn PgType>>) {
        // Returns (parameterized SQL, bound params)
        // Example: "SELECT id, name FROM public.users WHERE id = $1 LIMIT $2 OFFSET $3"
        // Never: format!("... WHERE id = {}", user_input)
    }
}
```

**Security invariant:** No user-supplied value may appear in the SQL string template. All filter values, limit/offset, and ordering column names are validated against the `DatabaseModel` (column must exist) before being bound as parameters.

---

## Phase 3 — Flint Quarry: GraphQL + Keto Integration + Subscriptions

**Repo:** flint-forge  
**Cross-repo dependency:** FRF Phase 1 (WatchEntityType gRPC endpoint)  
**Gate:** Subscriber receives only RLS-permitted change payloads; merged introspection (pg_graphql ∪ subscription SDL) works; Keto inline permission check works end-to-end.

### Pre-kickoff decision required: pg_graphql PG18 (OQ-3)

Issue #614 was closed as completed in December 2025. The PG18 patches were merged to master. Before Phase 3 begins:

1. Check `https://github.com/supabase/pg_graphql/releases` for a tagged release supporting PG18
2. If released: pin the version, add to `images/postgres18/Dockerfile`, proceed
3. If NOT yet released: build from master SHA, document the pin in `docs/contracts/pg-graphql-version.md`, accept breakage risk on upstream changes
4. Decision: do NOT defer indefinitely. Phase 3 cannot start until OQ-3 is resolved.

### GraphQL architecture (unchanged from original plan, confirmed correct)

```
POST /graphql
    │
    ├── Mutations + Queries → fdb-postgres::graphql_resolve()
    │       → SET LOCAL ROLE + jwt.claims + request.headers
    │       → SELECT graphql.resolve($query, $variables, $extensions)
    │       → returns JSON result
    │
    └── Subscriptions → async-graphql + graphql-transport-ws
            → fdb-realtime gRPC client → WatchEntityType stream
            → per-event RLS re-query (non-negotiable, see CLAUDE.md §JWT/RLS)
            → deliver only rows the subscriber is permitted to see
```

### Keto integration via `flint_meta.keto_tuples`

Phase 3 wires the Keto tuple sync that Phase 1 installed the table for:

- FRF Iggy event on Keto relation write → `pg_notify('keto_changes', ...)` → `fdb-reflection` updates `flint_meta.keto_tuples` → triggers `PermissionAnalysisPass` recompile
- `flint_meta.check_permission(namespace, object, relation)` is called during subscription delivery to gate each event

### JWT propagation extension (Phase 3 additions)

Phase 2 already propagates `role`, `jwt.claims`, `request.headers`. Phase 3 adds:

```sql
-- Extended SET LOCAL block (per-request, before any user statement)
SET LOCAL ROLE authenticated;
SET LOCAL "request.jwt.claims"  = '{"sub":…,"role":…,"tenant_id":…}';
SET LOCAL "request.headers"     = '{"authorization":"Bearer <raw-jwt>"}';
-- Phase 3 additions:
SET LOCAL "app.jwt_claims"      = '<full jwt payload>';
SET LOCAL "app.keto_subject"    = '<keto subject id>';
SET LOCAL "app.vault_key_id"    = '<vault key reference>';
```

These three new GUCs are read by `flint_meta.check_permission()` and `flint_meta.decrypt_column()`.

### Risks and mitigations

| Risk | Likelihood | Mitigation |
|---|---|---|
| pg_graphql PG18 release unavailable or broken | LOW (issue closed Dec 2025) | Build from master SHA as fallback; pin the exact commit in Dockerfile |
| WAL bypass: RLS not applied to subscription events | HIGH (by design) | **Non-negotiable re-query contract**: For each `EntityChange` from FRF, Quarry re-queries `SELECT * FROM <table> WHERE id = $1` under the subscriber's RLS context. Deliver only if the row is visible. This is CLAUDE.md §Subscription RLS Enforcement. |
| Keto tuple sync lag: tuple written to Keto but not yet synced to `flint_meta.keto_tuples` | MEDIUM | `check_permission()` is evaluated at query/subscription time, not at compile time. If Keto sync lags, use the Keto HTTP API as fallback (circuit breaker pattern). |
| `graphql.resolve()` passthrough does not support nested subscription resolvers | LOW | Subscriptions do NOT go through `graphql.resolve()`. Only queries/mutations do. Subscriptions use async-graphql resolvers + FRF WatchEntityType. No conflict. |

### OpenSpec changes

| Change ID | Title | Depends on | Priority |
|---|---|---|---|
| `p3-c001-graphql-passthrough` | `POST /graphql` → `graphql.resolve()` under RLS | Phase 2 complete | P0 |
| `p3-c002-subscriptions` | fdb-realtime gRPC → WatchEntityType + Keto gate + per-event RLS re-query | p3-c001, FRF Phase 1 | P0 |
| `p3-c003-introspection-merge` | Union pg_graphql schema ∪ subscription SDL | p3-c001, p3-c002 | P0 |
| `p3-c004-graphql-transport-ws` | `graphql-transport-ws` WebSocket upgrade on `/graphql` | p3-c002 | P0 |
| `p3-c005-pg-graphql-pg18` | Resolve OQ-3: verify/pin pg_graphql PG18 tagged release | — | **P0 (must resolve before phase kickoff)** |
| **`p3-c006-keto-sync`** | FRF Iggy → `flint_meta.keto_tuples` sync; `keto_changes` NOTIFY channel | Phase 1 complete, FRF Phase 3 | P0 |
| **`p3-c007-graphql-compiler`** | `GraphQlCompiler`: `DatabaseModel` → `async_graphql::Schema`; integrated into `CompiledState` hot-swap | p2-c003, p3-c001 | P0 |
| **`p3-c008-extended-guc-propagation`** | Add `app.jwt_claims`, `app.keto_subject`, `app.vault_key_id` to the `SET LOCAL` block in all transaction paths | p2-c002 | P0 |
| `p3-c009-predicate-pushdown` | Opt-in RLS predicate pushdown (off by default; operator-accepted data-leak risk) | p3-c002 | P2 |
| `p3-c010-jwt-contract-pin` | `docs/contracts/jwt-contract.md` finalized with service-identity format | p1-c005 | P0 (if not done in P1) |

---

## Phase 5 — Flint A2UI Component Registry (Flint-invented layer on top of A2UI protocol)

**Repo:** flint-forge  
**Duration estimate:** 4–5 sprints (8 milestones, ~26 weeks total; MVP core is Milestones 1–3 ≈ first 11 weeks)  
**Gate:** `flint_a2ui` schema installs; 50+ base components seeded; new `flint_meta.cache_tables` row auto-generates a binding within 5 seconds; semantic search returns correct component for a natural-language query; RLS enforces application isolation.

> **Naming note:** The user refers to this as "Phase 5 with AG-UI, A2UI, etc." In the plan's internal numbering (P0 workspace → P1 Anvil/Meta → P2 Reflection → P3 GraphQL → P4 Ember → **P5 Kiln runtime** → P6 signing/storage → P7 AG-UI integration), this phase is inserted as a **new P5** and the existing Kiln/signing phases shift. The A2UI Registry is a prerequisite for Phase 7 AG-UI emission (Phase 7 now references the registry). The Kiln runtime (formerly P5) is renumbered P6 and signing/storage (formerly P6) becomes P7, with AG-UI integration becoming P8 in a full 9-phase plan. **To avoid renumbering all existing OpenSpec change IDs**, this spec inserts P5 as an additive phase between Phase 4 and Phase 6 (old numbering) and refers to the AG-UI phase as "Phase 8 / P8" in the cross-phase table below.

### What the A2UI Registry Is (and Is Not)

The **Flint Global A2UI Component Registry** is a Flint-invented value-add layer on top of the official [A2UI protocol](https://a2ui.org/specification/v0.9.1-a2ui/). It is not mandated by the A2UI spec.

| A2UI Protocol (official — Google/a2ui-project) | Flint A2UI Registry (Flint-invented) |
|---|---|
| JSON message spec: `createSurface`, `updateComponents`, `updateDataModel`, `deleteSurface` | `flint_a2ui` PostgreSQL schema storing component metadata |
| Application-defined catalog (Basic Catalog: Text, Button, Row only) | 50+ Flint base components across 6 categories, seeded into the DB |
| Transport-agnostic; AG-UI is the primary transport binding | Components auto-discovered via semantic search, bound to `flint_meta.cache_tables` |
| No global registry concept | Global shared registry across all Flint applications (Flint-specific) |
| v0.9.1 current / v1.0 RC (June 2026) | Flint-defined extension of the A2UI catalog model |

**The relationship to `flint_meta.agui_descriptor()` (p1-c010):**  
The existing `flint_meta.agui_descriptor()` function (delivered in Phase 1) is the **seed input** to the registry, not the registry itself. Its current protocol label `'protocol': 'ag-ui/1.0'` is a known mislabeling (AG-UI is the transport, not a content protocol identifier). In Phase 5, this function is updated to:
1. Return a descriptor whose `protocol` field is corrected to `'flint-forge/schema-descriptor/1.0'`
2. Register itself as a component in `flint_a2ui.components` with `slug = 'flint-meta-schema'` and `category = 'system'`
3. The `a2ui_auto_bind_tables` trigger fires on `INSERT ON flint_meta.cache_tables`, calling `flint_a2ui.auto_generate_bindings()` — this is the live integration point

### Protocol Precision: ODSF

The spec references **ODSF (Open Design System Format)** as an Open Design integration bridge. This term is not found in official A2UI or AG-UI documentation — it is an internal Flint naming for the design system bundle format (the `index.md + foundations/ + components/ + tokens.css` structure). It should be labeled as "Flint Design System Bridge format (internal; inspired by Open Design patterns)" in code comments and docs to avoid confusion with any official external spec.

### Phase 5 Goals

1. `flint_a2ui` PostgreSQL schema installed as a pgrx extension (or pure SQL migration, evaluated in p5-c001)
2. 50+ base component definitions seeded (layout, data-display, input, action, navigation, feedback)
3. `pgvector` extension integrated; HNSW index on `flint_a2ui.embeddings`; embedding pipeline wired to `fdb-gateway` or an offline seeder
4. Auto-binding trigger from `flint_meta.cache_tables` → `flint_a2ui.bindings` (5-second SLA)
5. Application model: `flint_a2ui.applications`, `roles`, `role_assignments` tables; JWT resolution via `app.jwt_claims`
6. RLS policies on all `flint_a2ui.*` tables
7. Event-driven assembly: `assembly_rules` table + Rust assembler emitting A2UI JSON
8. Protocol surfaces: REST API, A2A task definitions, MCP tool server for component discovery
9. `flint_meta.agui_descriptor()` corrected (protocol label fix + self-registration)

### Risks and mitigations

| Risk | Likelihood | Mitigation |
|---|---|---|
| `pgvector` requires extension in PG18 image; not installed by default | MEDIUM | Add `CREATE EXTENSION IF NOT EXISTS vector;` to `images/postgres18/Dockerfile`; pin `pgvector >= 0.7.0` |
| 50+ base components is a large seeding effort; component JSON schema must be hand-authored | MEDIUM | Script bulk-insert from a TOML/JSON seed file checked into the repo; `p5-c002` includes the seed script |
| Embedding generation requires an LLM API call at install time | LOW for MVP | Defer embeddings to a background job; `flint_a2ui.embeddings` table can be empty at install; semantic search degrades to text search until embeddings are populated |
| ODSF (internal name) conflicts with any external "ODSF" if that term exists publicly | LOW | Rename to `FlintDesignBundle` in code and docs; keep `ODSF` only in this spec comment as historical context |
| flint_a2ui.events is append-only — no DELETE — may grow unboundedly | MEDIUM | Add `pg_partman` or a cron-based TTL job; default retention 90 days; configurable via `flint_a2ui.config` table |
| Phase 2 CompiledState.agui_descriptors reads from `flint_meta.agui_descriptor()` directly — needs update to read from `flint_a2ui.resolve_components()` instead | HIGH (breaks Phase 2 expectation) | p5-c009 explicitly updates Phase 2's `CompiledState` to replace the direct call with a `flint_a2ui` registry query; Cedar `a2ui:emit` gate added at this seam |

### OpenSpec changes

| Change ID | Title | Depends on | Priority |
|---|---|---|---|
| **`p5-c001-flint-a2ui-schema`** | `flint_a2ui` PostgreSQL schema: all core tables (`components`, `applications`, `design_systems`, `embeddings`, `schemas`, `bindings`, `events`, `assembly_rules`, `roles`, `role_assignments`); RLS policies; pgvector extension | Phase 1 complete | **P0** |
| **`p5-c002-base-components-seed`** | Seed 50+ base component primitives across 6 categories (layout, data-display, input, action, navigation, feedback); seed script checked into `scripts/seed_a2ui_components.sql` | p5-c001 | **P0** |
| **`p5-c003-auto-binding-trigger`** | `flint_a2ui.auto_generate_bindings()` function + `a2ui_auto_bind_tables` trigger on `INSERT ON flint_meta.cache_tables`; column type → component mapping; table → form/grid generation | p5-c001, Phase 1 (flint_meta schema) | **P0** |
| **`p5-c004-embeddings-pipeline`** | `flint_a2ui.embeddings` table with `vector(1536)` + HNSW index (`m=16, ef_construction=64`); background embedding job (Rust task, liter-llm via `text-embedding-3-large`); semantic search function `flint_a2ui.semantic_search(query, limit)` | p5-c001 | P1 |
| **`p5-c005-application-model`** | Application CRUD (`flint_a2ui.applications`); role hierarchy (`roles`, `role_assignments`); JWT claims template resolution; `flint_a2ui.resolve_components(application_id, jwt_claims)` function; Cedar policy gate for component access | p5-c001 | P1 |
| **`p5-c006-rest-api`** | REST endpoints in `fdb-gateway`: `GET /a2ui/v1/components`, `POST /a2ui/v1/components/search`, `GET /a2ui/v1/components/:slug`, `GET /a2ui/v1/applications`, `POST /a2ui/v1/surfaces/assemble`; JWT-gated, RLS-enforced | p5-c001, p5-c005, Phase 2 | P1 |
| **`p5-c007-event-driven-assembly`** | `flint_a2ui.assembly_rules` table; Rust component assembler in `fdb-reflection`; Iggy producer integration (topic: `a2ui.surfaces`); A2UI JSON surface generation from events; 500ms SLA for tool-call-completion → surface | p5-c001, FRF Phase 3 (Iggy) | P1 |
| **`p5-c008-protocol-surfaces`** | A2A task definitions (`a2ui.component.register`, `a2ui.component.discover`, `a2ui.component.assemble`, `a2ui.surface.render`, `a2ui.search.semantic`); MCP tool server for component discovery (`a2ui_list_components`, `a2ui_get_component`, `a2ui_semantic_search`, `a2ui_generate_form`, `a2ui_generate_grid`, `a2ui_resolve_tokens`, `a2ui_assemble_surface`); served at `/mcp/v1/a2ui` | p5-c006, Phase 7 (MCP server endpoint) | P2 |
| **`p5-c009-compiled-state-upgrade`** | Update Phase 2 `CompiledState.agui_descriptors: Arc<HashMap<String, serde_json::Value>>` to query `flint_a2ui.resolve_components()` instead of calling `flint_meta.agui_descriptor()` directly; add Cedar `a2ui:emit` capability check before any A2UI surface is emitted by a WASM function; correct `flint_meta.agui_descriptor()` protocol label to `'flint-forge/schema-descriptor/1.0'` | p5-c001, p5-c005, Phase 2 (CompiledState) | **P0 (correctness)** |

### SDK Platform Changes (p5-c010 – p5-c015)

These six changes extend Phase 5 to deliver multi-platform renderer SDKs, design tool integrations, and the schema additions that support per-application component overrides. Research backing these decisions lives in `.firecrawl/` (committed to git).

| Change ID | Title | Depends on | Priority |
|---|---|---|---|
| **`p5-c014-sdk-schema-extensions`** | DB schema additions for all SDKs: `renderers jsonb` + `react_pkg`/`flutter_pkg`/`htmx_template` columns on `flint_a2ui.components`; new `flint_a2ui.component_overrides` table (per-app, per-design-system overrides with RLS); `source_format`/`source_content`/`imported_at`/`token_schema_version` columns on `flint_a2ui.design_systems`; W3C 2024 design token JSONB schema; `resolve_components_with_overrides()` SQL function; Rust types `ResolvedComponent`, `Renderers`, `DesignToken` in `fdb-app/src/a2ui/types.rs`; migration `migrations/0003_flint_a2ui_sdk_extensions.sql` | p5-c001, p5-c005 | **P1 (blocks p5-c010, p5-c011, p5-c013)** |
| **`p5-c010-react-sdk`** | `@flint/react` — React 19 headless component library implementing all 63 Flint A2UI components; `FlintProvider` (endpoint, applicationId, jwt, components override map, tokens), `FlintSurface`, `FlintRegistry` (Zod schema registry pattern), `FlintAgUiAdapter` (AG-UI Custom event bridge); no dependency on `@copilotkit/*`, `@assistant-ui/react`, or Vercel AI SDK; design token system via CSS custom properties; `exportDesignSyncTokens()` for Claude Design compatibility; target < 80kb gzipped | p5-c014, p5-c006 | P1 |
| **`p5-c011-flutter-sdk`** | `flint_genui` Dart/Flutter package extending `genui ^0.9.2` (flutter/genui, official Flutter org); `FlintA2uiTransport` — pure Dart SSE client to `fdb-gateway` (no Gemini/Firebase dependency); `FlintCatalog.build(overrides: {...})` registering all 63 components as `CatalogItem`; `FlintThemeData` (`ThemeExtension`) reading design tokens from catalog endpoint; `cue ^0.3.11` (Milad-Akarie, MIT) for surface entry/exit/streaming/tool-call animated transitions; pubspec deps: `genui: ^0.9.2`, `genui_a2ui: ^0.9.2`, `cue: ^0.3.11` | p5-c014, p5-c006 | P1 |
| **`p5-c012-htmx-renderer`** | Axum + Askama HTMX renderer in `fdb-gateway`; routes: `GET /htmx/components/:slug`, `POST /htmx/components/:slug` (fragment render), `GET /htmx/admin/registry`, SSE `GET /htmx/stream/:surface_id`; DaisyUI semantic CSS classes; `data-flint-component` attribute on all fragments; `hx-ext="sse"` + `sse-swap` for streaming agent updates; **scope: prototyping, admin UI, and OpenDesign ideation only — NOT for production agent-generated surfaces** | p5-c001, p5-c006 | P1 |
| **`p5-c013-opendesign-integration`** | OpenDesign (`nexu-io/open-design`, Apache-2.0, 73k stars) + Claude Design (Anthropic, April 2026) integration; inbound: `POST /a2ui/v1/design-systems/import` accepting `format: "design-md" | "w3c-tokens" | "claude-design-zip" | "odsf"`; `design_md_parser.rs` in `fdb-app/src/a2ui/` parsing DESIGN.md 9-section format into `flint_a2ui.design_systems`; outbound: Flint distributed as OpenDesign plugin via `plugins/flint-components/open-design.json` manifest + two `SKILL.md` skills (`flint-component-browser`, `flint-surface-preview`); `od mcp` server integration for Claude Code / Cursor / Zed; Claude Design ZIP import round-trips through `source_content` column | p5-c014, p5-c006 | P2 |
| **`p5-c015-claude-design-skill`** | Claude Code skill package (`skills/flint-ui/SKILL.md`) installable via `claude plugin marketplace add prometheus-ags/flint-forge`; embeds all 63 component slugs, React 19 / Flutter / HTMX API reference, `POST /a2ui/v1/surfaces/assemble` endpoint docs, W3C design token format; companion catalogs: `catalogs/components.md`, `catalogs/react-api.md`, `catalogs/flutter-api.md`, `catalogs/htmx-api.md`; code examples: `examples/react-data-grid.tsx`, `examples/flutter-surface.dart`, `examples/htmx-form.html`; JSON schemas: `schemas/a2ui-message.json`, `schemas/design-token.json`; OpenDesign plugin manifest at `plugins/flint-components/open-design.json` | p5-c010, p5-c013 | P2 |

#### SDK Dependency Order (p5-c014 is the blocker)

```
p5-c001, p5-c005 (existing)
  └─► p5-c014-sdk-schema-extensions   ← MUST come first in SDK group
        ├─► p5-c010-react-sdk
        ├─► p5-c011-flutter-sdk
        ├─► p5-c012-htmx-renderer      (also needs p5-c006)
        └─► p5-c013-opendesign-integration
              └─► p5-c015-claude-design-skill
```

#### SDK MVP Definition

The minimum viable SDK set for Phase 5 sign-off is: **p5-c014 + p5-c010 + p5-c011**. These deliver the schema foundation, React SDK, and Flutter SDK — the two production-grade renderer paths. p5-c012 (HTMX) and p5-c013/p5-c015 (design tools) are P1/P2 enhancements that can ship in a follow-on milestone without blocking the Phase 5 gate.

#### Research Basis

| File | Key Finding |
|---|---|
| `.firecrawl/react-agent-ui-2026.md` | assistant-ui (headless Radix-style, 7.9k stars) and Tambo (Zod registry) and Thesys/Crayon (props-map override) define the `@flint/react` architecture; Vercel AI SDK RSC paused — use `useChat` client hooks |
| `.firecrawl/flutter-genui-a2ui-2026.md` | Pub package is `genui` (NOT `gen_ui`); `flutter/genui` is the official Flutter org repo; Discussion #651 confirms A2UI integration path; `genui_a2ui` for server-side transport |
| `.firecrawl/htmx-axum-agent-ui-2026.md` | HTMX explicitly NOT recommended for agent-generated UI; valid for admin/prototyping; `askama` + `axum-htmx` + DaisyUI is the canonical Rust HTMX stack |
| `.firecrawl/opendesign-claude-design-2026.md` | OpenDesign = `nexu-io/open-design` (73.1k stars, Apache-2.0, launched April 28 2026); Claude Design = Anthropic (April 2026); DESIGN.md 9-section format; `od mcp` exposes projects to Claude Code |
| `.firecrawl/cue-flutter-animation-2026.md` | `cue ^0.3.11` by Milad-Akarie (MIT, pub.dev); timeline-driven physics-first Flutter animations — no `AnimationController` boilerplate |

#### `p5-c001` — flint_a2ui schema: decision point

The `flint_a2ui` schema is NOT a pgrx extension (unlike `flint_meta`). pgrx is for in-process PostgreSQL functions. `flint_a2ui` is a standard PostgreSQL schema with tables — delivered as a SQL migration file (`migrations/0002_flint_a2ui.sql`). This keeps the schema separate from the extension machinery and avoids a second workspace-excluded crate. The pgvector HNSW index requires `CREATE EXTENSION vector;` which must be pre-installed in the PG18 image.

```sql
-- Key tables (abbreviated)
CREATE TABLE flint_a2ui.components (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    slug            text NOT NULL UNIQUE,          -- e.g. 'data-grid', 'form-field-text'
    category        text NOT NULL,                  -- 'layout'|'data-display'|'input'|'action'|'navigation'|'feedback'
    primitive_type  text NOT NULL,                  -- A2UI catalog type (e.g. 'DataGrid', 'TextInput')
    schema          jsonb NOT NULL,                 -- JSON Schema for props validation
    is_base         boolean NOT NULL DEFAULT false, -- true = Flint base component; false = app-specific
    application_id  uuid REFERENCES flint_a2ui.applications(id),
    description     text,
    usage_examples  jsonb,
    design_tokens   jsonb,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE flint_a2ui.embeddings (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    component_id    uuid REFERENCES flint_a2ui.components(id) ON DELETE CASCADE,
    embedding       vector(1536) NOT NULL,
    entity_type     text NOT NULL DEFAULT 'component',
    aspect          text NOT NULL DEFAULT 'description', -- 'description'|'usage'|'props'
    model           text NOT NULL DEFAULT 'text-embedding-3-large',
    created_at      timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_embeddings_hnsw ON flint_a2ui.embeddings
    USING hnsw (embedding vector_cosine_ops)
    WITH (m = 16, ef_construction = 64);
```

---

## Phase 7 (renumbered) — AG-UI Integration + A2UI Content Layer + MCP Tools Compiler

> **Renumbering note:** In the expanded 9-phase plan, this was formerly P7. It remains `p7-*` in all OpenSpec change IDs to avoid renaming already-planned changes. It depends on Phase 5 (A2UI Registry) being at minimum at p5-c001/c002/c003/c009 (core schema + base components + bindings + CompiledState upgrade).

**Repo:** flint-forge (+ FRF Phase 5 agentproto)  
**Duration estimate:** 2–3 sprints  
**Gate:** Agent events stream to a frontend via AG-UI over SSE; MCP tool manifest is generated from `DatabaseModel` and validated against the MCP spec; A2UI component descriptors resolved from `flint_a2ui` registry are emitted as A2UI JSON via AG-UI `Custom` events.

### Protocol correction (from research — updated June 2026)

The original plan treats AG-UI and A2UI as co-equal protocols. The correct framing (confirmed against official docs and firecrawl research):

```
┌─────────────────────────────────────────────────────────────────────┐
│                     PROTOCOL STACK (corrected)                       │
│                                                                      │
│  TRANSPORT LAYER:  AG-UI (CopilotKit, MIT, 14.5k stars, active)     │
│     ↑ event stream over HTTP/SSE; @ag-ui/core npm; Rust community   │
│       SDK at sdks/community/rust/crates/ag-ui-client                │
│                                                                      │
│  CONTENT DELIVERY: Two paths for A2UI content                        │
│    Path A (CopilotKit frontends): @ag-ui/a2ui-middleware v0.0.10    │
│      → ActivityMessage objects; <CopilotKit a2ui={{ catalog }}>     │
│      → auto-injects middleware since CopilotKit 1.61.2              │
│    Path B (generic AG-UI frontends / Flint SSE): AG-UI Custom       │
│      events with type "a2ui:surface" carrying A2UI JSON payload      │
│      → client renders using its registered catalog                  │
│                                                                      │
│  CONTENT SPEC:  A2UI (a2ui-project/Google, Apache 2.0)              │
│    v0.9.1 current / v1.0 RC; transport-agnostic JSON message spec   │
│    Messages: createSurface, updateComponents, updateDataModel,       │
│    deleteSurface; v1.0 adds actionResponse/callFunction/RPC          │
│    Catalog: per-application; Basic Catalog has Text/Button/Row only  │
│    NO global registry in spec — Flint's registry is Flint-invented   │
│                                                                      │
│  COMPETING CONTENT SPECS (same layer as A2UI):                       │
│    MCP-UI (Microsoft/Shopify) — iframe-based; sandbox model          │
│    Open-JSON-UI (OpenAI) — declarative UI schema                    │
│                                                                      │
│  TOOL PROTOCOL: MCP (Anthropic, open) — for DB/function tools       │
└─────────────────────────────────────────────────────────────────────┘
```

**Flint's Phase 7 strategy (updated to reference Phase 5 registry):**
- **AG-UI** is the primary event streaming protocol. Emit `RunStarted`, `TextMessageStart`, `TextMessageContent`, `TextMessageEnd`, `ToolCallStart`, `ToolCallArgs`, `ToolCallEnd`, `ToolCallResult`, `StateSnapshot`, `StateDelta`, `RunFinished`, `RunError` from Flint's agent execution path.
- **A2UI surfaces** are assembled by the Phase 5 component assembler (`fdb-reflection` + `flint_a2ui.resolve_components()`) and emitted as AG-UI `Custom` events with type `"a2ui:surface"`. The payload is a valid A2UI `updateComponents` + `updateDataModel` message referencing components from the Flint catalog (Phase 5 registry). **Requires p5-c001 through p5-c003 and p5-c009 to be complete.**
- **CopilotKit path:** Flint's A2UI catalog (Phase 5) is served as a JSON Schema URL that CopilotKit can reference in `<CopilotKit a2ui={{ catalog }}>`. The `@ag-ui/a2ui-middleware` auto-injects the Flint catalog into the frontend pipeline.
- **MCP tool definitions** are generated by `fdb-reflection` from `DatabaseModel`. Each table becomes a set of MCP tools (`list_<table>`, `get_<table>`, `create_<table>`, `update_<table>`, `delete_<table>`). Each function becomes a `call_<function>` tool.
- **Rust SDK for AG-UI:** use community crate at `sdks/community/rust/crates/ag-ui-client`. It is "Supported" in the official repo but is community-maintained — plan for bug fixes and patches. Latest release: 2026-06-24.

### MCP tools from `DatabaseModel` — what this looks like

```json
{
  "name": "list_orders",
  "description": "List rows from public.orders table. Supports filtering, ordering, and pagination.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "select":  { "type": "string", "description": "Comma-separated column names. Default: *" },
      "eq":      { "type": "object", "description": "Equality filters: {\"status\": \"shipped\"}" },
      "order":   { "type": "string", "description": "Order expression: \"created_at.desc\"" },
      "limit":   { "type": "integer", "default": 50 },
      "offset":  { "type": "integer", "default": 0 }
    }
  }
}
```

**Why this is differentiated vs. existing MCP servers:**  
All existing Postgres MCP servers (`crystaldba/postgres-mcp`, `@supabase/mcp-server-supabase`, etc.) expose generic introspection tools (`list_tables`, `execute_sql`) or hand-authored canned tools. None compile typed, per-table, permission-filtered MCP tool definitions from a pre-computed `DatabaseModel` IR. This is genuinely novel.

**Risk:** Auto-generated MCP tools at scale (100+ tables) may produce a tool manifest too large for some MCP clients to handle. Mitigation: expose tool scoping via URL parameter (`?schemas=public,app`) and compile per-schema tool subsets.

### A2UI component layer — what this means in practice (post-Phase 5)

When a Kiln WASM function or Flint agent needs to generate a UI, the Phase 5 assembler constructs an A2UI surface from the registry and the `fdb-gateway` emits it as an AG-UI `Custom` event:

```json
{
  "type": "Custom",
  "name": "a2ui:surface",
  "value": {
    "protocol": "a2ui/0.9",
    "messages": [
      {
        "createSurface": {
          "surfaceId": "orders-view",
          "catalogId": "https://flint.example.com/a2ui/v1/catalog/flint-base/1.0"
        }
      },
      {
        "updateComponents": {
          "surfaceId": "orders-view",
          "components": [
            {
              "id": "root",
              "component": "DataGrid",
              "children": ["col-id", "col-status", "col-total"]
            }
          ]
        }
      },
      {
        "updateDataModel": {
          "surfaceId": "orders-view",
          "dataModel": { "rows": "{{mcp:list_orders?limit=25}}" }
        }
      }
    ]
  }
}
```

The `catalogId` URI points to the Flint A2UI catalog served by `fdb-gateway` (Phase 5 REST API). The frontend resolves this catalog to know which components are available and how to render them. **Flint does not own the renderer — Flint owns the catalog and emits the surface spec.**

**Key difference from Phase 7's original design:** The original plan emitted a single-component Custom event with `"a2ui:component"`. The corrected design emits proper A2UI protocol messages (`createSurface` + `updateComponents` + `updateDataModel`) referenced against the Flint catalog, which is the actual A2UI v0.9 wire format.

**Competing specs note:** A2UI (a2ui-project/Google, Apache 2.0) is our current content spec target. MCP-UI (Microsoft/Shopify) and Open-JSON-UI (OpenAI) are competing specs at the same layer. We adopt A2UI first because it is the most explicitly documented and has AG-UI as day-zero transport. If the ecosystem converges on a different spec, the `Custom` event wrapper makes it easy to swap the content schema without changing the transport.

### Realtime metadata via FRF Iggy (Phase 7 addition)

Schema changes that update `flint_meta` should also propagate to connected agents. When `StateManager` hot-swaps a new `CompiledState`:
1. Emit an AG-UI `StateSnapshot` event with the new `mcp_tools` list
2. Emit `StateDelta` patches for incremental changes (e.g., new table → new tool added)
3. This allows agent frontends to update their tool picker in real-time without a page reload

This requires FRF Phase 5 (agentproto) to include a `schema_change` event type.

### Risks and mitigations

| Risk | Likelihood | Mitigation |
|---|---|---|
| AG-UI Rust community SDK (`ag-ui-client`) has gaps vs. TypeScript SDK | MEDIUM | Audit the crate against the official event type list before Phase 7 kickoff. File issues or implement missing event types directly. |
| A2UI spec is Google-origin and may diverge or be deprecated | LOW | We emit A2UI inside AG-UI `Custom` events — changing the content schema is a content layer change, not a protocol change. Switching to MCP-UI or Open-JSON-UI requires only updating the emitter, not the transport. |
| MCP tool manifest > 128KB breaks some MCP clients | MEDIUM for large schemas | Implement schema-scoped tool subsets. Default to emitting only user-visible schemas, not internal (`flint_meta`, `auth`, etc.). |
| Agent emitting A2UI components bypasses Cedar policy gate | HIGH severity, LOW probability | Cedar capability check MUST gate A2UI emission. A WASM function cannot emit A2UI descriptors for tables/columns it does not have Cedar `a2ui:emit` capability for. Implement in `fke-server` as a policy check before forwarding `Custom` events. |

### OpenSpec changes

| Change ID | Title | Depends on | Priority |
|---|---|---|---|
| `p7-c001-webhook-kiln-wiring` | `flint_hooks` target can be a Kiln edge function (not just a URL) | Phase 6 Kiln complete | P0 |
| `p7-c002-agentproto-pipe` | `flint_hooks` → FRF agentproto → AG-UI event emission over SSE | p7-c001, FRF Phase 5 | P0 |
| **`p7-c003-agui-emitter`** | `fdb-reflection` AG-UI event emitter: lifecycle, text, tool-call, state events; SSE endpoint `/agents/v1/<run-id>/events` | p2-c003 | **P0** |
| **`p7-c004-mcp-compiler`** | `McpCompiler`: `DatabaseModel` → `Vec<McpToolDef>`; per-table CRUD tools + function tools; integrated into `CompiledState`; served at `/mcp/v1/tools` | p2-c003 | **P0** |
| **`p7-c005-a2ui-surface-emitter`** | A2UI surface emission via AG-UI `Custom` events (type `"a2ui:surface"`); surfaces assembled by Phase 5 assembler (`flint_a2ui.resolve_components()` → A2UI `createSurface`+`updateComponents`+`updateDataModel` messages); Flint catalog URI served at `/a2ui/v1/catalog/<id>`; Cedar `a2ui:emit` capability gate enforced in `fke-server` for WASM-originated emissions | p7-c003, Phase 3, **p5-c001, p5-c003, p5-c007, p5-c009** | P1 |
| **`p7-c005a-copilotkit-catalog-endpoint`** | `GET /a2ui/v1/catalog/:catalog_id` — serves the Flint catalog as a JSON Schema resolvable by CopilotKit's `<CopilotKit a2ui={{ catalog }}>` provider; includes all base components from Phase 5 | p7-c005, p5-c002 | P1 |
| `p7-c006-a2ui-gate` | flint-gate SSE processor: filter AG-UI events by `allowed_event_types` + Cedar scope; block unauthorized `Custom` events carrying unauthorized `a2ui:surface` payloads | FRF Phase 5 | P1 |
| **`p7-c007-agui-state-propagation`** | `StateManager` emits `StateSnapshot`/`StateDelta` AG-UI events when `CompiledState` hot-swaps; `StateSnapshot` includes updated MCP tools list + resolved A2UI catalog version | p7-c003, p2-c005, p5-c009 | P1 |
| **`p7-c008-mcp-server-endpoint`** | MCP server endpoint (`/mcp/v1`): JSON-RPC 2.0 over HTTP/SSE; `tools/list`, `tools/call` methods; auth via flint-gate JWT | p7-c004 | P0 |

---

## Cross-Phase Dependency Summary

```
Phase 0 (complete) ─────────────────────────────────────────────────┐
  ✓ Workspace build                                                  │
  ✓ PG18 image                                                       │
  ✓ WIT contract freeze                                              │
  ~ c004 (WatchEntityType) deferred to Phase 3                       │
                                                                     │
Phase 1 ─────────────────────────────────────────────────────────────┤
  flint_auth (p1-c001) ──────────────────────────────────────────────┤
  flint_hooks standard (p1-c002) ────────────────────────────────────┤
  jwt-contract-pin (p1-c005) ─────────────────── gates Phase 2       │
  flint_meta schema (p1-c007) ──────────────────────────────────────►┤ Phase 2, Phase 5
  flint_meta triggers (p1-c008) ────────────────────────────────────►┤ Phase 2
  flint_meta functions (p1-c009) ───────────────────────────────────►┤ Phase 2
  agui_descriptor() (p1-c010) ──────────────── corrected in p5-c009 ►┤ Phase 5
  listener test gate (p1-c011) ─────────────────────────────────────►┤ Phase 2
                                                                     │
Phase 2 ─────────────────────────────────────────────────────────────┤
  fdb-reflection crate (p2-c003) ───────────────────────────────────►┤ Phase 3, Phase 5, Phase 7
  REST compiler (p2-c004) ─────────────────────────────────────────►┤
  ArcSwap hot-reload (p2-c005) ─────────────────────────────────────►┤
  CompiledState.agui_descriptors ──── upgraded in p5-c009 to query   │
                                       flint_a2ui.resolve_components()│
  OpenAPI compiler (p2-c007) ──────────────────────────────────────►┤ Phase 8
                                                                     │
Phase 3 ─────────────────────────────────────────────────────────────┤
  OQ-3 resolved (p3-c005) ─────── must happen BEFORE Phase 3 start  │
  WatchEntityType (c004) ───────── must come from FRF Phase 1        │
  Keto sync (p3-c006) ─────────────────────────────────────────────►┤ Phase 5, Phase 7
  GraphQL compiler (p3-c007) ──────────────────────────────────────►┤
  Extended GUC propagation (p3-c008) → app.jwt_claims ─────────────►┤ Phase 5 (RLS)
                                                                     │
Phase 4 (unchanged — Ember/LLM) ─────────────────────────────────────┤
                                                                     │
Phase 5 ─── NEW: Flint A2UI Component Registry + SDK Platform ────────┤
  [Registry MVP: p5-c001, p5-c002, p5-c003, p5-c009]                │
  [SDK MVP: p5-c014, p5-c010, p5-c011]                               │
  requires: Phase 1 (flint_meta schema), Phase 2 (CompiledState),    │
            Phase 3 (app.jwt_claims GUC for RLS)                     │
  flint_a2ui schema (p5-c001) ─────────────────────────────────────►┤ Phase 7 (catalog)
  base components seed (p5-c002) ──────────────────────────────────►┤ Phase 7 (catalog URI)
  auto-binding trigger (p5-c003) ────────────────────────────────────┤
  CompiledState upgrade (p5-c009) ─────────────────────────────────►┤ Phase 7 (a2ui emit)
  event assembler (p5-c007) ───────────────────────────────────────►┤ Phase 7 (surface emit)
  MCP tools for components (p5-c008) ──────────────────────────────►┤ Phase 7 (/mcp/v1/a2ui)
  sdk-schema-extensions (p5-c014) ─────────────────────────────────►┤ @flint/react, flint_genui
  react-sdk (p5-c010) ──────────────────────────────────────────────┤ claude-design-skill
  flutter-sdk (p5-c011) ────────────────────────────────────────────┤
  htmx-renderer (p5-c012) ──────────────────────────────────────────┤ prototyping/admin only
  opendesign-integration (p5-c013) ────────────────────────────────►┤ claude-design-skill
  claude-design-skill (p5-c015) ────────────────────────────────────┤ Claude Code / OpenDesign
                                                                     │
Phase 6 (formerly P5 — Kiln runtime) ────────────────────────────────┤
Phase 7 (formerly P6 — signing/storage) ─────────────────────────────┤
                                                                     │
Phase 8 (formerly P7) — AG-UI + A2UI emission + MCP Server ──────────┤
  requires: Phase 2 (fdb-reflection), Phase 3 (Keto, GraphQL),       │
            Phase 5 MVP (schema + base components + CompiledState),   │
            FRF Phase 5 (agentproto)                                  │
  MCP compiler (p7-c004) ─── prerequisite for Phase 9 SDK completeness
  AG-UI emitter (p7-c003) ── prerequisite for agent SDKs (Phase 9)
  A2UI surface emitter (p7-c005) ─── requires Phase 5 MVP complete
```

---

## Open Questions (active)

| ID | Question | Blocks | Status |
|---|---|---|---|
| OQ-3 | pg_graphql PG18 tagged release available? | Phase 3 kickoff | **Resolve before Phase 3** — check supabase/pg_graphql/releases |
| OQ-4 | Exact claim shape minted by flint-gate | `p1-c005-jwt-contract-pin` | Requires flint-gate team coordination |
| OQ-5 | Service-identity token format (HS256 vs ES256) | `p1-c005` | Requires flint-gate team coordination |
| OQ-6 | FRF Phase 5 agentproto crate timeline | `p7-c002-agentproto-pipe` | Requires FRF team coordination |
| OQ-7 | AG-UI Rust SDK (`ag-ui-client`) coverage completeness; latest release 2026-06-24 | `p7-c003` | Audit the crate before Phase 7 kickoff; confirmed actively maintained |
| OQ-8 | Keto sync mechanism: does FRF Iggy support a `keto_changes` event type? | `p3-c006-keto-sync` | Requires FRF team coordination |
| OQ-9 | pgvector minimum version for HNSW; is `vector >= 0.7.0` available in the PG18 image? | `p5-c001`, `p5-c004` | Check `images/postgres18/Dockerfile`; add `CREATE EXTENSION vector;` if not present |
| OQ-10 | Embedding model access: is `text-embedding-3-large` available via the liter-llm gateway already configured for `ext-flint-llm` (Ember)? | `p5-c004` | Confirm with Ember config; can use `text-embedding-3-small` (1536-d) as fallback at lower quality |
| OQ-11 | A2UI catalog URI: should the catalog be served at a versioned URI (`/a2ui/v1/catalog/flint-base/1.0`) so CopilotKit can reference it via `<CopilotKit a2ui={{ catalog }}>` + `@ag-ui/a2ui-middleware`? | `p7-c005a-copilotkit-catalog-endpoint` | Design decision: static versioned URI vs. dynamic per-application URI |
| OQ-12 | `flint_meta.agui_descriptor()` GRANT discrepancy: proposal specified `GRANT EXECUTE TO authenticated, anon` but implementation grants to `service_role` only. Should the descriptor be public or service-only? | `p5-c009` | The full schema topology is sensitive; recommend keeping `service_role` only and having `p5-c005` expose a permission-filtered view via `flint_a2ui.resolve_components()` |
| OQ-13 | `flutter/genui` (pub: `genui`) is currently at alpha `^0.9.2` with unstable API. Will `^1.0.0` stable ship before `p5-c011` needs to publish? | `p5-c011-flutter-sdk` | Monitor `github.com/flutter/genui/releases`; pin `^0.9.2` in pubspec with a `# TODO: upgrade to ^1.0.0 on stable release` comment; if stable ships first, upgrade before publishing `flint_genui` |
| OQ-14 | OpenDesign plugin registry: does `nexu-io/open-design` have a public plugin marketplace, or is distribution only via `od plugin install github.com/<owner>/<repo>`? | `p5-c013-opendesign-integration`, `p5-c015-claude-design-skill` | Current evidence (`.firecrawl/opendesign-claude-design-2026.md`) shows GitHub-path install only; submit to their plugin registry when it exists; for now, document the GitHub install path |

---

## Honest Competitive Assessment (S-03 correction applied)

The original comparison tables in RFC-FORGE-META-001 showed Flint Meta as all-upside with no tradeoffs. Below is the corrected assessment.

### Where Flint is genuinely differentiated

- **Unified IR for all outputs** (REST, GraphQL, OpenAPI, MCP, AG-UI): No existing open-source system does this from a single IR compiled inside PostgreSQL. This is real differentiation.
- **In-database metadata cache**: Eliminates PostgREST's external cache race condition. Pre-computed `flint_meta.*` tables make reflection nearly free on subsequent requests.
- **MCP tools auto-generated from DB schema**: No existing MCP server does this from a typed, compiled IR. Novel.
- **AG-UI emission from database hooks**: Connecting database change events (via `flint_hooks` + FRF agentproto) to AG-UI streams is a new capability not available in Supabase.
- **Keto inline in SQL**: `flint_meta.check_permission()` as a SQL function called inside RLS policies and GraphQL resolvers. No external HTTP round-trip per permission check.
- **Flint A2UI Component Registry (Phase 5)**: A PostgreSQL-native, semantically searchable component registry with auto-binding from DB metadata. No other platform combines pgvector HNSW semantic search, auto-binding from schema reflection, A2UI v0.9/v1.0 conformant output, and per-application JWT-scoped catalogs in a single system. The "global registry" is Flint-invented — the official A2UI spec has only a Basic Catalog (Text, Button, Row); Flint's 50+ component library is a significant original contribution.

### Where Supabase and competitors still win

| Capability | Supabase | Flint (current state) | Flint (Phase 3 complete) |
|---|---|---|---|
| Time to first REST endpoint | < 1 minute (auto-generated) | Days (Phase 2 build) | Days (but MCP-native) |
| GraphQL (PG18) | pg_graphql (released) | Blocked on OQ-3 | Resolved |
| Developer experience | Excellent (Supabase CLI, Studio) | Minimal (forge-cli stub) | Improving (Phase 8) |
| Community / ecosystem | 70k+ GitHub stars | Pre-launch | — |
| pg_graphql stability | Production-grade | Inherited (passthrough) | Inherited |
| Agent-native output (MCP, AG-UI) | None (as of June 2026) | **Ahead** | **Ahead** |
| In-DB permission as SQL | RLS only | Keto + RLS | **Ahead** |
| WASM edge functions with DB access | Deno (JS only) | **Any WASM language** | **Ahead** |
| Agent-discoverable UI components | None | Phase 5 builds this | **Novel: DB-native + semantic search** |
| A2UI component catalog (official spec) | None | Phase 5: 50+ Flint base components | **Ahead (official spec has only Text/Button/Row)** |

### Risks the plan must not ignore

1. **Compilation latency**: A 500-table schema may take 1–2 seconds to recompile. This is acceptable on DDL changes (rare) but must be measured, logged, and alerted.
2. **pgrx 0.18.1 is on the develop branch**: Not GA. If pgrx cuts a breaking change before Phase 1, `flint_meta` must adapt. Pin the exact cargo pgrx version in `rust-toolchain.toml`.
3. **LISTEN/NOTIFY reconnection gap**: If the `PgListener` connection drops and a DDL change fires during the gap, Flint serves the old compiled state. Mitigation: recompile on reconnect (adds startup latency), but there is still a theoretical window. This is acceptable for the AI-native use case; document it.
4. **A2UI Google origin**: A2UI may diverge from the neutral community standard the original plan implied. Mitigated by emitting inside AG-UI `Custom` events rather than as a first-class protocol.
5. **MCP at scale**: No production deployment has been reported for auto-generated per-table MCP tools at 100+ table scale. This is an unsolved problem we are solving first — that is a differentiator and a risk simultaneously.

---

## Next Action

**Phase 1 is complete** (11/11 changes delivered, reflected 2026-06-30).

Current status: `reflected` — next is `/kbd-new-phase p2-quarry-reflection-engine`

Before starting Phase 2, resolve:
- OQ-4 and OQ-5 (flint-gate JWT contract) — if not already resolved by Phase 1 jwt-contract-pin
- First change: `p2-c001-fdb-schema-registry` (ArcSwap<Schema> hot-reload via PgListener on `meta_runtime`)

**Phase plan summary (updated for Phase 5 insertion):**

| Phase | OpenSpec changes | Status |
|---|---|---|
| Phase 0 | 4 (3 complete, c004 deferred to P3) | Complete |
| Phase 1 | 11 total | **Complete** |
| Phase 2 | 7 total | **Next** |
| Phase 3 | 10 total | Planned |
| Phase 4 | Unchanged (Ember/LLM) | Planned |
| **Phase 5 (NEW)** | **9 total** (p5-c001 through p5-c009) | **New — requires P2+P3 MVP** |
| Phase 6 | Unchanged (Kiln runtime, formerly P5) | Planned |
| Phase 7 | Unchanged (signing/storage, formerly P6) | Planned |
| Phase 8 | 9 total (p7-c001 through p7-c008 + p7-c005a) — **requires Phase 5 MVP** | Planned |

**Phase 5 dependency on Phase 2:** p5-c001 can start in parallel with Phase 2 (schema is pure SQL, no Rust dependency). p5-c003 (auto-binding trigger) requires Phase 1 `flint_meta.cache_tables` to be live (done). p5-c009 (CompiledState upgrade) must happen AFTER Phase 2 `p2-c003` delivers `CompiledState`. The Phase 5 MVP (p5-c001, p5-c002, p5-c003, p5-c009) can be planned and partially executed during Phase 2 without blocking the critical path.
