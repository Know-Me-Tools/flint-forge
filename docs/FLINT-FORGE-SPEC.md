# Flint Forge — Master Functional Specification & Development Plan

**Document ID:** RFC-FORGE-001
**Status:** Draft for build (Assess complete across all subsystems; awaiting Phase 0 approval)
**Supersedes:** `flint-db-api-gateway/docs/IMPLEMENTATION-PLAN.md` (RFC-FDB-001) and the "pgforge" working name
**Consumed by:** the Prometheus KBD skills package / PMPO loop. Each numbered change in §7 maps to one OpenSpec change set (`proposal.md` + `tasks.md`). Phases halt for approval.

---

## 0. How to read this document

This is the single source of truth for the **Flint Forge** platform — the data and edge-compute plane that sits behind `flint-gate` (ingress/auth) and consumes `flint-realtime-fabric` (realtime spine). It specifies three deliverables and the contracts binding them:

1. **Flint Quarry** — the REST + GraphQL DB API Gateway over Postgres.
2. **Flint Anvil** — the pgrx in-database extension suite (`flint_auth`, `flint_hooks`, **Flint Ember** = `flint_llm`, **Flint Vault** = `flint_vault`).
3. **Flint Kiln** — the WASM-component Edge Function Gateway.

Detail level is "buildable": ports as trait signatures, SQL as DDL, edge interfaces as WIT, surfaces as route tables. Where a value is environment-dependent or version-sensitive, it is flagged in §8, not guessed.

---

## 1. Naming & System Map

The metaphor is the forge. Raw material is **quarried** from the source, shaped on the **anvil** in place, fired in the **kiln** into hardened tools — all inside the **forge**.

| Component | Flint name | Crate prefix | Role |
|---|---|---|---|
| Umbrella platform / monorepo | **Flint Forge** | `flint-forge/` (workspace) | Data + edge compute plane of the Flint platform |
| REST/GraphQL DB API Gateway | **Flint Quarry** | `fdb-` | Extract structured data from Postgres via REST + GraphQL |
| pgrx in-database extension suite | **Flint Anvil** | `flint_*` (pgrx) | Shape data in place: auth context, webhooks, in-DB LLM, secrets |
| — JWT/RLS context + helpers | (within Anvil) | `flint_auth` | GUC injection contract, `auth.*` helpers |
| — Webhook dispatch | (within Anvil) | `flint_hooks` | Trigger → webhook, JWT-forwarding, two delivery tiers |
| — In-database LLM / embeddings | **Flint Ember** | `flint_llm` | liter-llm bound into Postgres (sync + async surfaces) |
| — Sovereign secret store | **Flint Vault** | `flint_vault` | Encrypted secrets of any kind (DB passwords, API keys, tokens, params); KMS-wrapped DEK |
| WASM Edge Function Gateway | **Flint Kiln** | `fke-` | Compile (fire) and run signed WASM components |
| Shared core | (Forge core) | `forge-` | Cross-cutting domain types, identity, policy |

**External systems this plane depends on (not built here):**

- **`flint-gate`** — ingress, Kratos session → RLS JWT minting, WS/SSE/NDJSON stream proxy, `request.headers` carrier origin. The Forge plane trusts flint-gate's issuer/JWKS and never authenticates end users itself.
- **`flint-realtime-fabric`** — CDC (`frf-postgres-cdc`), Iggy spine, Keto per-event gate, the `WatchEntityType` RPC that Flint Quarry subscriptions consume (one fabric-side change, §7).
- **`UAR`** — the sovereign inference/governance plane that Flint Ember and Flint Kiln route LLM calls into; shares the Wasmtime component-host substrate with Flint Kiln.

### 1.1 Topology

```
                         ┌──────────── flint-gate (ingress / auth boundary) ────────────┐
  browser / app ───────▶ │ Kratos session → RLS JWT (Option-3 outbound)                │
  webhook callbacks ───▶ │ StreamProcessor: WS / SSE / NDJSON proxy + backpressure      │
                         └───────┬───────────────────────────────┬─────────────────────┘
                                 ▼                                 ▼
                     ┌──────── Flint Quarry ────────┐    ┌──────── Flint Kiln ────────┐
                     │ REST (PostgREST-compatible)  │    │ Admin REST (control plane,  │
                     │ GraphQL Q/M → pg_graphql     │    │   Cranelift compiler)       │
                     │ GraphQL Sub → async-graphql  │    │ /functions/v1/<name>        │
                     │   (graphql-transport-ws)     │    │   (data plane, compiler-off)│
                     └───────┬──────────────┬───────┘    └────────┬───────────┬────────┘
                             ▼              ▼                      ▼           ▼
                ┌─ Postgres 18 ────────────────┐         ComponentStore   flint-gate
                │ pg_graphql · pgvector · pg_net│         (OCI/IPFS/S3)    (governed
                │ pgcrypto · Flint Anvil:       │                          callbacks:
                │   flint_auth · flint_hooks ·  │◀── flint:db / flint:llm ─ flint:db,
                │   flint_llm · flint_vault     │     flint:secrets calls   flint:llm)
                └───────┬───────────────────────┘
                        ▼  WatchEntityType (gRPC stream)
              flint-realtime-fabric  (CDC → Iggy → Keto gate)
```

---

## 2. Cross-Cutting Foundations

### 2.1 Hexagonal dependency rule (all subsystems)

```
forge-domain (Layer 0: pure types, serde only, zero infra deps)
  ↑ imported by
forge-ports / *-app (Layer 1: trait seams + use-cases against ports)
  ↑ imported by
adapters (fdb-postgres, fdb-realtime, fke-store-*, fke-sign-*, …)
  ↑ wired by
interfaces (fdb-gateway, fke-server)  ← only crates that import concrete adapters
```

Domain and app layers never import an adapter crate. Each adapter implements exactly one port. Composition happens only in interface crates, via Cargo features, so a deployment compiles only the planes it runs.

### 2.2 Identity & JWT model (the load-bearing mechanism)

One per-transaction injection feeds **both** RLS and trigger-side JWT forwarding. Flint Quarry (and any Forge component opening a pooled connection) sets, per request transaction:

```sql
SET LOCAL ROLE authenticated;                          -- single pooled role (pooling-safe)
SET LOCAL "request.jwt.claims" = '{"sub":…,"role":…,"tenant_id":…}';   -- decoded claims → RLS
SET LOCAL "request.headers"    = '{"authorization":"Bearer <raw-jwt>"}'; -- raw token carrier (PostgREST convention)
```

- **Claims → RLS.** Policies read `current_setting('request.jwt.claims', true)::json->>'…'`, wrapped by `flint_auth` helpers `auth.uid()`, `auth.jwt()`, `auth.role()`.
- **Raw token → triggers.** `flint_hooks` and `flint_llm` read `current_setting('request.headers', true)::json->>'authorization'` to forward/attribute the originating identity.
- **Verification stays upstream.** flint-gate verifies the JWT signature and mints it. Postgres trusts the GUC; signing keys never enter Postgres. This preserves capability separation: the DB cannot mint or forge tokens.

**Outbound forwarding (Option-3 hybrid, ratified).** When the DB calls outward (webhook or LLM-via-gateway) the call is authenticated with a **service identity** flint-gate trusts; the originating user JWT travels in `X-Forge-Origin-JWT` for identity/audit/Cedar, plus an HMAC body signature (`X-Forge-Signature`). The delivery never fails on user-JWT expiry; the origin identity is still attributable.

### 2.3 Authorization — four layers, distinct jobs

| Layer | Question | Enforced | When |
|---|---|---|---|
| **Kratos** (authn) | Who is the caller? | flint-gate | per session |
| **Keto** (coarse) | May this subject touch this entity-type / tenant / relationship at all? | fabric + Quarry/Kiln pre-check | subscribe-time (cached); request entry |
| **Postgres RLS** (authoritative row filter) | May this subject see/modify **this row**? | Postgres, in-session | every query; every subscription event |
| **Cedar** (action / capability policy) | May this subject perform this mutating action / be granted this capability? | Quarry mutations; Kiln linker; Ember model-use | per action / per instantiation |

### 2.4 Two convergence invariants (design toward these, don't duplicate)

1. **One in-transaction capture, two consumers.** A trigger captures `(change + origin JWT from request.headers)` exactly once. `flint_hooks` consumes it to deliver webhooks; `flint_llm` consumes the same pattern to enqueue LLM jobs. The durable tier (outbox) is shared.
2. **One Wasmtime component host, two surfaces.** Flint Kiln and UAR's Tier-2 WASM skills share the component-host primitives (engine config, ProxyPre cache, capability linker, fuel/epoch limits). Kiln is the HTTP-triggered serverless surface over that shared host.

### 2.5 Quality gates (CI-enforced, all crates)

No `unwrap`/`expect` in library crates (`thiserror` in libs, `anyhow` only at binary edges); `clippy::pedantic` + `deny(warnings)`; `#[non_exhaustive]` on public enums; newtype IDs `#[repr(transparent)]`; `tracing` spans across every port boundary; pinned MSRV; semver discipline on `forge-domain` and SDK crates; Dagger CI; **no file over 500 lines** (split into directory modules). Never log JWT payloads, claims, relation tuples, or tenant identifiers.

---

## 3. Flint Quarry — REST/GraphQL DB API Gateway

**Crates:** `fdb-domain`, `fdb-ports`, `fdb-app`, `fdb-postgres`, `fdb-realtime`, `fdb-auth`, `fdb-gateway` (interface).
**Stack:** Axum 0.8.8 · async-graphql 7 (dynamic, subscriptions only) · async-graphql-axum · deadpool-postgres · tokio-postgres · tonic (fabric client) · arc-swap.
**Backend:** prebuilt Postgres 18 + pg_graphql + pgvector + pg_net + pgcrypto + Flint Anvil.

### 3.1 Ports (`fdb-ports`)

```rust
#[async_trait] pub trait DatabaseBackend: Send + Sync {
    async fn acquire(&self, rls: &RlsContext) -> Result<Conn, BackendError>; // sets ROLE + request.jwt.claims + request.headers
}
#[async_trait] pub trait SchemaProvider: Send + Sync {
    async fn introspect(&self) -> Result<Vec<TableMeta>, BackendError>;       // columns, PKs, FKs, rls_enabled
    fn subscribe_ddl(&self) -> watch::Receiver<SchemaVersion>;                // hot-reload signal
}
#[async_trait] pub trait RestExecutor: Send + Sync {
    async fn execute(&self, q: RestQuery, rls: &RlsContext) -> Result<RestResult, BackendError>;
}
#[async_trait] pub trait GraphQlExecutor: Send + Sync {   // the reversibility seam
    async fn execute(&self, req: GraphQlRequest, rls: &RlsContext) -> Result<serde_json::Value, BackendError>;
}
#[async_trait] pub trait ChangeStreamSource: Send + Sync {
    async fn watch(&self, sub: SubscriptionSpec, who: &RlsContext)            // entity_type + filter + subject
        -> Result<BoxStream<'static, Result<ChangeEvent, StreamError>>, StreamError>;
}
```

`GraphQlExecutor`'s Postgres adapter delegates to pg_graphql. A future uniform/multi-backend dialect is a second adapter — callers unchanged.

### 3.2 GraphQL — hybrid by operation type

| Operation | Path | Mechanism |
|---|---|---|
| Query / Mutation | `POST /graphql` | Confirm not a subscription → `graphql.resolve($query,$vars)` inside Postgres under RLS context → return JSON verbatim. async-graphql is **not** in this path. |
| Subscription | `GET /graphql` upgraded to `graphql-transport-ws` | async-graphql `GraphQLSubscription`; resolvers pull from `ChangeStreamSource`; reshape `ChangeEvent` → change payload. |
| Introspection | `__schema` / `__type` | **Merge** of pg_graphql introspection ∪ sibling subscription/custom SDL. The only seam where the two schemas meet. |

**Subscription change payload** (sibling async-graphql dynamic schema; `record` columns generated from the same introspection pg_graphql uses):

```graphql
enum ChangeOp { INSERT UPDATE DELETE UPSERT }
type TChangePayload { op: ChangeOp!  record: T  old_record: T }
type Subscription { tChanges(filter: TFilter): TChangePayload! }
```

### 3.3 Subscription sourcing & RLS enforcement (D1=C, D2)

1. **Source:** `fdb-realtime` is a gRPC client of the fabric's `WatchEntityType(tenant, entity_type, filter) → stream EntityChange` (the fabric-side change in §7). It does **not** run its own CDC.
2. **Subscribe-time gate (coarse):** one Keto `check(subject,"view",entity_type|tenant)`, cached for the subscription lifetime.
3. **Per-event authoritative filter:** for each `EntityChange`, re-query the changed row **as the subscriber** (`SELECT … WHERE pk = $changed_pk` with the subscriber's claims set so RLS applies). Deliver only if the row survives RLS. (The WAL bypasses RLS; this is the only path faithful to query-time RLS semantics — same technique as Supabase Realtime.)
4. **Opt-in predicate-pushdown (per hot table):** translate the RLS predicate into a payload filter, skipping the re-query. **Operator-accepted risk, off by default:** predicate drift silently leaks/drops rows.

### 3.4 REST surface (PostgREST-compatible subset)

`GET/POST/PATCH/DELETE /<schema>.<table>` with `select`, filter operators (`eq,neq,gt,gte,lt,lte,like,ilike,in,is,cs,cd,fts`), `order`, `limit`, `offset`, `Range`/`Content-Range` headers, and `Prefer` (`return=representation|minimal`, `resolution=merge-duplicates`, `count=exact`). `POST /rpc/<fn>` invokes Postgres functions. **pgvector** similarity is exposed first as `/rpc/<fn>` over a SQL function (`ORDER BY embedding <-> $q LIMIT k`); a GraphQL vector field is a later sibling-schema enhancement.

### 3.5 RLS context assembly (`fdb-auth`)

Verify the inbound JWT against flint-gate's issuer/JWKS → build `RlsContext { role, claims_json, raw_bearer }` → `DatabaseBackend::acquire` issues the three `SET LOCAL` statements (§2.2) on the transaction before any user statement runs.

### 3.6 Schema hot-reload

`SchemaRegistry { current: ArcSwap<SubscriptionSchema>, … }`; on DDL change (detected via `SchemaProvider::subscribe_ddl`, itself fed by a `flint_hooks` registration on DDL or a poll), rebuild the sibling subscription schema and atomically swap. pg_graphql manages its own schema refresh inside Postgres; Quarry only rebuilds the subscription sibling.

### 3.7 Crate map

```
fdb-domain/   TableMeta, RestQuery, RestResult, ChangeEvent, ChangeOp, SubscriptionSpec, RlsContext
fdb-ports/    the five traits in §3.1
fdb-app/      REST use-case, GraphQL exec use-case, subscription orchestration, RLS assembly
fdb-postgres/ DatabaseBackend + SchemaProvider + RestExecutor + GraphQlExecutor (pg_graphql passthrough) + pgvector
fdb-realtime/ ChangeStreamSource over fabric WatchEntityType + Keto gate + per-event RLS re-query
fdb-auth/     JWT verify (flint-gate issuer/JWKS) → RlsContext
fdb-gateway/  Axum composition root: /graphql (Q/M/Sub), REST routes, /rpc, /healthz, admin
```

---

## 4. Flint Anvil — pgrx Extension Suite

**Framework:** pgrx (latest stable) targeting Postgres 18. **Image extensions required:** `pg_graphql`, `pgvector`, `pg_net`, `pgcrypto`, plus the three Flint extensions below. Each extension is a workspace crate built with `cargo pgrx`. Background workers require `shared_preload_libraries` access at the image level (§8).

### 4.1 `flint_auth` — JWT/RLS context + helpers

Defines the `auth` schema and the GUC-reading helpers so policy authors never touch `current_setting` directly.

```sql
CREATE SCHEMA IF NOT EXISTS auth;
CREATE FUNCTION auth.jwt()  RETURNS jsonb LANGUAGE sql STABLE AS
  $$ SELECT coalesce(current_setting('request.jwt.claims', true), '{}')::jsonb $$;
CREATE FUNCTION auth.uid()  RETURNS text  LANGUAGE sql STABLE AS
  $$ SELECT auth.jwt()->>'sub' $$;
CREATE FUNCTION auth.role() RETURNS text  LANGUAGE sql STABLE AS
  $$ SELECT coalesce(auth.jwt()->>'role', 'anon') $$;
CREATE FUNCTION auth.bearer() RETURNS text LANGUAGE sql STABLE AS  -- raw token for forwarding
  $$ SELECT current_setting('request.headers', true)::json->>'authorization' $$;
```

Role model: single pooled role (`authenticated`/`anon`) + claims-based RLS (pooling-safe); per-user role switching is supported but not the default. `flint_auth` ships no policies — it provides the vocabulary policies are written in.

### 4.2 `flint_hooks` — webhook dispatch

Gateway-managed registry + one generic `SECURITY DEFINER` trigger per hooked table.

```sql
CREATE SCHEMA flint;
CREATE TABLE flint.webhooks (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  schema_name text NOT NULL, table_name text NOT NULL,
  events text[] NOT NULL,                       -- {'INSERT','UPDATE','DELETE'}
  target_url text NOT NULL,                      -- routes through flint-gate
  forward_jwt boolean NOT NULL DEFAULT false,    -- Option-1 opt-in (else Option-3 default)
  custom_headers jsonb NOT NULL DEFAULT '{}',
  secret text NOT NULL,                          -- HMAC key
  tier text NOT NULL DEFAULT 'standard',         -- 'standard' (pg_net) | 'durable' (outbox)
  active boolean NOT NULL DEFAULT true,
  timeout_ms int NOT NULL DEFAULT 5000);

CREATE TABLE flint.webhook_outbox (             -- durable tier
  id bigserial PRIMARY KEY, webhook_id uuid NOT NULL,
  payload jsonb NOT NULL, headers jsonb NOT NULL,
  status text NOT NULL DEFAULT 'pending', visible_at timestamptz NOT NULL DEFAULT now(),
  retry_count int NOT NULL DEFAULT 0, created_at timestamptz NOT NULL DEFAULT now());
```

`flint.dispatch_webhook()` (SECURITY DEFINER, owner has `net.*` + registry read): builds payload `{type, table, schema, record, old_record}`; reads `auth.bearer()` (in-transaction, GUC visible); for each matching active registration builds headers:
- **Option-3 default:** `Authorization: Bearer <service-token>`, `X-Forge-Origin-JWT: <raw user jwt>`, `X-Forge-Signature: hmac_sha256(payload, secret)`.
- **Option-1 (forward_jwt=true):** `Authorization: <raw user jwt>` — for prompt-delivery cases only.

**Standard tier:** `PERFORM net.http_post(url, payload, headers, timeout_ms)` — async, best-effort, response in `net._http_response`. **Durable tier:** insert into `flint.webhook_outbox` (same transaction, captures JWT); a dispatcher consumer (`flint-realtime-fabric` Iggy or a `flint_hooks` BGW) delivers with ordering + retry via `FOR UPDATE SKIP LOCKED`. Never attach triggers to `net.*` tables. Provisioning of the generic trigger per table is driven by the gateway admin API from the registry.

### 4.3 Flint Ember — `flint_llm` (in-database LLM / embeddings via liter-llm)

Two surfaces with a hard boundary. **Routing is sovereign:** liter-llm in the extension calls **inward to flint-gate / UAR**, never directly to providers. The DB holds a flint-gate credential, never provider keys **in plaintext** — that credential (and, where a deployment lets liter-llm reach a provider directly, the provider key) lives in **Flint Vault** (§4.4) and is resolved in-process via `vault.resolve_api_key` / `vault.get_secret`, never as raw config. Every in-DB LLM call inherits Cedar policy, rate-limiting, cost attribution, model routing, and may be served by candle-vllm/RunPod.

**Surface 1 — synchronous (read/explicit path only).**
```sql
llm.embed(input text, model text DEFAULT 'default') RETURNS vector
llm.complete(prompt text, opts jsonb DEFAULT '{}') RETURNS text
```
Implementation: liter-llm async call on a dedicated runtime thread that touches no Postgres internals; the backend thread blocks on a channel under a **hard timeout + periodic `CHECK_FOR_INTERRUPTS()`/`WaitLatch`** so `statement_timeout` and `pg_cancel_backend` work. `EXECUTE` granted narrowly (never to `anon`). Legitimate only when the output **gates the write** (classification deciding commit) or is **required-at-commit** — rare, low-volume, latency-tolerant, operator-opt-in per table. **Never the default in a write-path trigger.**

**Surface 2 — asynchronous enrichment (default for triggers).** Triggers enqueue; a pgrx background worker (own process, hosts the tokio runtime + liter-llm + rate-limit governor) dequeues, batches, calls the model, writes results back via SPI.
```sql
CREATE TABLE llm.jobs (
  id bigserial PRIMARY KEY, kind text NOT NULL,          -- 'embed' | 'summarize' | 'classify'
  schema_name text, table_name text, pk jsonb,
  source jsonb, target_column text, model text,
  origin_jwt text,                                       -- captured at enqueue (attribution + Cedar)
  status text NOT NULL DEFAULT 'pending', visible_at timestamptz NOT NULL DEFAULT now(),
  retry_count int NOT NULL DEFAULT 0);
-- dequeue: SELECT … FOR UPDATE SKIP LOCKED (PGMQ pattern)
```
Declarative layer (provisions column + enqueue trigger + HNSW index + worker config, index-like):
```sql
llm.enable_embedding(table regclass, column text, model text, dim int)
llm.enable_summary(table regclass, source_col text, target_col text, prompt_template text)
```

**v1 scope:** embeddings via Surface 2 first (safe, pairs with pgvector); summaries/completions follow once worker + governor are proven; Surface 1 added when a real gating use case appears.

**Guardrails (named):** synchronous LLM in a write trigger holds row locks for model latency → backend/pool exhaustion under load → DB stall; a bulk insert through a row-level sync trigger is N serial calls in one transaction. The async worker exists to prevent this. pgrx async is officially "unexplored" — Surface 1's interrupt/cancellation safety is real engineering, budget for it. LLM spend is non-transactional; Surface 2 acts post-commit (reads committed rows) so rollbacks incur no spend.

### 4.4 Flint Vault — `flint_vault` (sovereign secret store)

A pgrx extension that stores **secrets of any kind** — database passwords, external-service API keys (LLM providers, Stripe, SendGrid, …), connection strings, OAuth/refresh tokens, certificates, and arbitrary secret parameters — encrypted at rest in an ordinary Postgres table, so backups/WAL/replicas carry **ciphertext only**. It is the single sovereign store every Forge subsystem reads from; LLM keys (§4.3) are one consumer among many. It descends from the Supabase-Vault/pgsodium lineage but drops the deprecated `pgsodium` dependency and the file-only root key.

**Crypto.** XChaCha20-Poly1305 AEAD (24-byte nonce per row). The row `id` is bound in as **associated data**, so a ciphertext copied to another row fails authentication. The per-row working key is `HKDF-SHA256(DEK, info = category ‖ key_id)`, so a category can be rotated independently.

**Envelope encryption (the key improvement over the reference).** The Data Encryption Key (DEK) is **never a raw file**. At postmaster start it is unwrapped from a Key Encryption Key (KEK) that lives in an external KMS — **Azure Key Vault via managed identity** (or AWS KMS / GCP KMS / Vault Transit) — through a KMS-agnostic unwrap command (the EDB-TDE `PGDATAKEYUNWRAPCMD` pattern: wrapped DEK on stdin → 32 raw bytes on stdout). The KEK never enters the DB or the process. *Stakes:* revoking the KEK renders every secret cryptographically dead (KMS kill-switch); rotating the KEK only rewraps the DEK — **zero data re-encryption**. A raw-DEK env var remains as the dev fallback. The DEK is held in `secrecy::Secret`, zeroized on drop, and is **never selectable from SQL**. `_PG_init` fails fast and loud if a configured key cannot be loaded.

**Schema (in schema `vault`).**
```sql
CREATE TYPE vault.secret_category AS ENUM
  ('api_key','password','connection_string','token','certificate','secret_param');

CREATE TABLE vault.secrets (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  category vault.secret_category NOT NULL,
  name text NOT NULL, description text NOT NULL DEFAULT '',
  provider text,                 -- api_key: 'anthropic','stripe',…; target system; or NULL
  scope text,                    -- optional tenant/environment scope
  secret text NOT NULL,          -- base64 ciphertext
  key_id uuid NOT NULL DEFAULT '…0001', nonce bytea NOT NULL,
  created_at timestamptz DEFAULT now(), updated_at timestamptz DEFAULT now(),
  UNIQUE (category, name, COALESCE(scope,'')));
CREATE TABLE vault.access_log (…);   -- append-only audit of every privileged read/write
CREATE VIEW  vault.decrypted_secrets AS … ;   -- decrypt-on-read (revoked from PUBLIC)
```

**Access model — "available but not exposed" (two tiers).** Grounded in the wasmCloud secrets model and host-boundary credential injection (IronClaw):
- **In-process (DB consumers).** `flint_llm`, `flint_hooks`, and flint-gate's service role resolve plaintext in the trusted Postgres process via `SECURITY DEFINER` functions `vault.get_secret(name[,scope])` and `vault.resolve_api_key(provider[,scope])`, granted only to a `flint_secret_reader` role (`flint_llm_worker` inherits it) — **never to `PUBLIC`**, every call audited.
- **Edge (WASM, brokered — default for high-value secrets).** A component **never receives the raw value**: it calls `flint:llm` or a host-mediated outbound, and Flint Kiln injects the secret at the host boundary (the key never enters WASM linear memory).
- **Edge (delegated — escape hatch, default-deny).** When a component must read a value, `flint:secrets.get(name)` returns an opaque `secret` resource and `reveal(secret)` is a separate, **Cedar-gated, audited** call (§5.4).

**Write path.** `vault.create_secret` / `update_secret` are `SECURITY DEFINER`, revoked from `PUBLIC`, granted to a `vault_admin` role, and meant to be driven by the trusted host (flint-gate/Tauri/Axum) with statement logging disabled on that path (plaintext is a function argument — the one residual hole inherited from the reference).

**Reconciliation (supersedes a prior decision).** The earlier "provider keys live in flint-gate, never in the DB" is refined to: *no secret is in the DB in plaintext.* Secrets are ciphertext in `flint_vault`, the DEK is KMS-wrapped, and decryption happens only in the trusted DB process or trusted host — never to SQL clients or WASM. v1 resolution uses the `SECURITY DEFINER` SQL path (residual risk: a role that can assume `flint_secret_reader` can read plaintext, and it transits an SPI result); v2 hardens to a true in-process Rust entrypoint with no SQL round-trip.

**v1 scope:** crate compiles + packages against pgrx 0.18.1 / PG18 (`flint_vault.so` + generated SQL + control, verified). Open items — KEK provider, brokered-vs-delegated default, crypto lib (RustCrypto vs `dryoc` for mlock'd pages), wrapped-DEK location — are in §8.

---

## 5. Flint Kiln — Edge Function Gateway

**Crates:** `fke-domain`, `fke-ports`, `fke-runtime`, `fke-store-{oci,ipfs,s3,fs}`, `fke-sign-{did,cosign}`, `fke-registry`, `fke-server` (interface).
**Stack:** Wasmtime (Component Model, async) · wasmtime-wasi · wasmtime-wasi-http · Axum 0.8.8 · Ed25519 (ed25519-dalek) · cosign/sigstore-rs (interop) · oci-distribution / `wkg` · IPFS client.
**Deployable unit:** a single signed, content-addressed WASM **component** (no source directories). Polyglot.

### 5.1 Runtime model

Bespoke Wasmtime host targeting the standard `wasi:http/proxy` world (a component **exports** `wasi:http/incoming-handler`). Shares component-host primitives with UAR's Tier-2 skills.

- **Per-request isolation:** pre-instantiate once with `ProxyPre`; per request create a fresh `Store` + `ProxyPre::instantiate_async`. Fresh linear memory per invocation → no cross-request state leakage. Stateless (serverless) semantics: functions externalize state to `flint:kv`/DB.
- **Resource governance:** epoch-based interruption + fuel for per-request timeouts; `StoreLimits` for memory ceilings; the pooling allocator for low-latency instances (tune hot-function count vs. reserved virtual memory).

### 5.2 Compilation strategy (Cranelift; AOT; control/data-plane split)

Wasmtime+Cranelift already emits native machine code (x86_64, aarch64, s390x, riscv64). The dial is *when* and *which backend*. `CompilationStrategy` is a first-class config dimension.

| Strategy | Use | Mechanism |
|---|---|---|
| **Cranelift-AOT** (default, prod) | registered functions | At registration (post-verify), `Engine::precompile_component` → `.cwasm`; cache keyed `(source_digest, target_arch, wasmtime_version)`; invocation `deserialize`s native code, zero compile latency. |
| **Winch** (dev) | fast iterate | baseline compile ~15–20× faster, code ~1.1–1.5× slower; x86_64 (aarch64 in dev). No Winch→Cranelift auto-tiering. |
| **Pulley** (fallback) | portability / no-codegen targets | portable interpreter; slowest; default where Cranelift has no backend. |

**Control-plane / data-plane split (the security win):**
- **Control plane = Flint Kiln admin server** — built *with* Cranelift; the only component that compiles. AOT-compiles at registration. Cross-compiles for all fleet arches via the `all-arch` feature → emits `.cwasm` per `(digest, target)`.
- **Data plane = invocation server** — built with `cranelift`/`winch` features **disabled**; can only `deserialize`+run pre-compiled `.cwasm`. The request path handling untrusted/webhook-triggered input has **no compiler in it**.

**`.cwasm` trust (RCE-sensitive):** `deserialize` trusts the artifact — a tampered `.cwasm` runs as native code. Resolution: sign the *source* `.wasm` (§5.5), AOT-compile only inside the trusted control plane, treat the `.cwasm` as trusted because it was produced from a verified source in a controlled pipeline; optionally seal the `.cwasm` with a runtime key the data plane checks before `deserialize`. A Wasmtime upgrade invalidates the `.cwasm` cache → pipeline recompiles on version bumps.

### 5.3 Supported languages

Any toolchain that produces a component targeting the `wasi:http/proxy` world runs unmodified:

| Language | Toolchain | Notes |
|---|---|---|
| Rust | `cargo-component` (`--proxy`) + `wasm32-wasip2` | first-class; reference for `flint:*` bindings |
| JavaScript / TypeScript | `jco` / `componentize-js` (StarlingMonkey) | larger component; bigger instantiate cost/memory |
| Python | `componentize-py` | maturity varies; pin versions |
| Go | TinyGo `wasip2` | maturity varies |
| C / C++ | `wasi-sdk` + `wit-bindgen` | |
| C# / .NET, others | `componentize-dotnet` etc. | as ecosystem matures |

Components may also be **composed** (auth in Rust, logic in Go, gateway in TS) into one component via WAC/WIT before signing.

### 5.4 Capability interfaces (WIT) — governed by Cedar

The host `Linker` adds only interfaces the function's signed manifest **requests** ∩ Cedar **allows for that publisher** (default-deny). v1 host interfaces (plus `wasi:http/outgoing-handler`, `wasi:clocks`, `wasi:random`):

> **WIT type note.** WIT has no native `json` type and no bare `error` type. In the canonical `wit/flint/host/world.wit`, `list<json>` is represented as `list<string>` (each element is a JSON-encoded string; the host encodes/decodes at the boundary), scalar `json` becomes `string`, and `error` becomes the `host-error` record `{ code: string, message: string }`. The `resource secret` type is valid WIT. The spec below shows the *intent*; the file is authoritative.

```wit
package flint:host@0.1.0;

record host-error { code: string, message: string }

interface db {       // governed DB access — routes through flint-gate under origin JWT
  // params: JSON-encoded values (list<string> in WIT; no native json type)
  // rows:   JSON-encoded row objects (list<string> in WIT)
  query: func(sql: string, params: list<string>) -> result<list<string>, host-error>;
}
interface llm {      // governed inference — routes through flint-gate / UAR
  embed:    func(input: string, model: option<string>) -> result<list<f32>, host-error>;
  complete: func(prompt: string, opts: string)         -> result<string, host-error>;
}
interface kv { get: func(k: string) -> option<list<u8>>; set: func(k: string, v: list<u8>); }
interface identity {
  origin-jwt: func() -> option<string>;
  claims:     func() -> string;   // JSON-encoded claim set
}
interface secrets {                                  // backed by flint_vault (§4.4)
  resource secret { reveal: func() -> result<string, host-error>; }  // Cedar-gated + audited
  get: func(name: string) -> result<secret, host-error>; // returns a handle, NOT the value
}
world edge-function {
  export wasi:http/incoming-handler@0.2.12;
  import wasi:http/outgoing-handler@0.2.12;
  import db; import llm; import kv; import identity; import secrets;
}
```

A function never holds a raw DB connection, provider key, or any secret value — only governed host capabilities. High-value secrets are **brokered** (the host injects them at the boundary; they never enter WASM memory); a value a function genuinely needs comes only through the Cedar-gated, audited `secret.reveal()` (default-deny), backed by `flint_vault` (§4.4). Its only I/O is through the linker, so it cannot escape the governance boundary.

### 5.5 Signing model (content-address → sign digest → bind to DID)

- **Content-address** every component: sha256 digest (IPFS CID where applicable). Registry maps `name@version → digest`; artifacts immutable, name pointers mutable.
- **Sovereign default — Ed25519 detached signature over a manifest**, keyed to the publisher's `did:prometheus:` identity, issued as a Kaia Verifiable Credential. Manifest:
```json
{ "publisher_did":"did:prometheus:…", "content_digest":"sha256:…",
  "capabilities":["flint:db","flint:llm","wasi:http/outgoing"],
  "version":"1.2.0", "not_before":"…", "not_after":"…" }
```
  Runtime verifies signature + publisher DID + validity **before instantiation**; refuses unsigned or capability-over-requesting components. Granted capabilities = `manifest.capabilities ∩ Cedar(publisher)`.
- **OCI interop — Cosign/Sigstore:** sign by digest, OIDC-keyless, signature + Rekor transparency-log entry stored alongside the artifact (SLSA/supply-chain alignment for components shared outside the sovereign boundary).

### 5.6 Storage abstraction (`ComponentStore` port)

```rust
#[async_trait] pub trait ComponentStore: Send + Sync {
    async fn put(&self, bytes: &[u8]) -> Result<ContentId, StoreError>;     // returns digest/CID
    async fn get(&self, id: &ContentId) -> Result<Vec<u8>, StoreError>;
    async fn exists(&self, id: &ContentId) -> Result<bool, StoreError>;
}
```
Adapters, all content-addressed: **OCI registry (primary** — `wkg`/cosign/admission tooling; sign by digest never tag), **IPFS** (sovereign, CID-native), **S3** (pragmatic), **filesystem** (dev).

### 5.7 Surfaces

- **Admin REST (control plane — locked down harder than invocation; separate auth/Cedar, ideally separate listener):** `POST /admin/functions` (register: upload/ref component, manifest, verify signature, AOT-compile per target, cache); `GET /admin/functions`; `POST /admin/functions/{name}/versions`; `PATCH …/activate`; `DELETE …`. The admin surface is the platform's highest-value attack target — signing-key custody (Kaia/Key Vault) + per-publisher Cedar gating are non-negotiable.
- **Invocation (data plane):** `ANY /functions/v1/<name>` — behind flint-gate carrying the Option-3 origin JWT; build `wasi:http` IncomingRequest, pass origin identity via `flint:identity`, dispatch to the pre-instantiated `.cwasm`. Callbacks to DB/LLM route back through flint-gate → RLS + Cedar enforced.

### 5.8 Crate map

```
fke-domain/    FunctionManifest, ContentId, Capability, CompilationStrategy, TargetArch
fke-ports/     ComponentStore, SignatureVerifier, ComponentRegistry, Compiler
fke-runtime/   Wasmtime engine, ProxyPre cache, Cedar-gated Linker, fuel/epoch/limits, .cwasm (de)serialize
fke-store-{oci,ipfs,s3,fs}/   ComponentStore adapters
fke-sign-{did,cosign}/        SignatureVerifier adapters
fke-registry/  name@version → (digest, manifest, cwasm cache); SurrealDB or Postgres-backed
fke-server/    Axum: admin REST (control plane) + /functions/v1 (data plane)
```

---

## 6. Workspace Layout (`flint-forge/` monorepo)

```
flint-forge/
├── Cargo.toml                      # [workspace]
├── crates/
│   ├── forge-domain/               # shared cross-cutting types
│   ├── forge-identity/             # JWT verify, RlsContext, Option-3 outbound, claims
│   ├── forge-policy/               # Cedar PEP shared by Quarry mutations + Kiln linker + Ember
│   │
│   ├── fdb-domain/  fdb-ports/  fdb-app/                      # Flint Quarry
│   ├── fdb-postgres/  fdb-realtime/  fdb-auth/  fdb-gateway/
│   │
│   ├── ext-flint-auth/             # Flint Anvil — pgrx (extension: flint_auth)
│   ├── ext-flint-hooks/            #              pgrx (extension: flint_hooks)
│   ├── ext-flint-llm/              # Flint Ember — pgrx (extension: flint_llm) + BGW
│   ├── ext-flint-vault/            # Flint Vault — pgrx (extension: flint_vault)
│   │
│   ├── fke-domain/  fke-ports/  fke-runtime/                  # Flint Kiln
│   ├── fke-store-oci/  fke-store-ipfs/  fke-store-s3/  fke-store-fs/
│   ├── fke-sign-did/  fke-sign-cosign/  fke-registry/  fke-server/
│   └── forge-cli/                  # ops + dev CLI (register functions, manage hooks, run migrations)
├── wit/flint/host/                 # WIT: db, llm, kv, identity, secrets, edge-function world
├── images/postgres18/              # Dockerfile for the prebuilt PG18 + extensions image
├── openspec/changes/               # the change sets (§7)
├── docs/                           # this spec + ADRs
└── dagger/                         # CI pipelines
```

`forge-domain/identity/policy` are shared; everything else obeys the hexagonal rule (§2.1). pgrx extension crates build via `cargo pgrx` and ship into the PG18 image, not the gateway binaries.

---

## 7. Development Plan (KBD/PMPO, phased — halt at every boundary)

Each change = one OpenSpec change set. Phase gate: complete tasks → verify exit criterion → **stop and report** → no auto-advance.

**Phase 0 — Foundation**
- `p0-c001-workspace` — `[workspace]`, `forge-domain/identity/policy`, CI green, a `/healthz` in both `fdb-gateway` and `fke-server` stubs.
- `p0-c002-pg18-image` — Dockerfile: PG18 + pg_graphql + pgvector + pg_net + pgcrypto; confirm `wal_level=logical`, `shared_preload_libraries`, pgrx build headers (§8).
- `p0-c003-wit-contract` — `wit/flint/host/*`; freeze `flint:host@0.1.0` before SDK/bindings.
- *(fabric repo)* `frf-p?-cNNN-watch-entity-type` — add `WatchEntityType(tenant, entity_type, filter) → stream EntityChange` to `entity.proto` + `EntityService` + `SubscribePipeline` filter path + Keto gate.
- **Exit:** workspace compiles; image builds and reports extension presence; WIT frozen; fabric change approved + proto-frozen.

**Phase 1 — Flint Anvil (auth + hooks)**
- `p1-c001-flint-auth` — `auth.*` helpers; GUC contract tests.
- `p1-c002-flint-hooks-standard` — registry, generic dispatch trigger, pg_net standard tier, Option-3 headers, HMAC.
- `p1-c003-flint-hooks-durable` — outbox + dispatcher (Iggy/BGW), `FOR UPDATE SKIP LOCKED`, retry/ordering.
- **Exit:** an INSERT fires a signed Option-3 webhook through flint-gate; durable tier survives a dispatcher restart.

**Phase 2 — Flint Quarry (REST + RLS)**
- `p2-c001-fdb-auth` — JWT verify → `RlsContext`.
- `p2-c002-fdb-postgres` — pool, `SET LOCAL` context (role + claims + headers), `SchemaProvider`.
- `p2-c003-rest-executor` — PostgREST-compatible REST + `/rpc` + pgvector RPC.
- **Exit:** RLS-correct REST CRUD under a flint-gate JWT; vector search via `/rpc`.

**Phase 3 — Flint Quarry (GraphQL)**
- `p3-c001-graphql-passthrough` — `POST /graphql` → `graphql.resolve` under RLS.
- `p3-c002-subscriptions` — `fdb-realtime` over fabric `WatchEntityType` + Keto gate + per-event RLS re-query; async-graphql sibling schema; `graphql-transport-ws`.
- `p3-c003-introspection-merge` — union pg_graphql ∪ subscription SDL.
- `p3-c004-predicate-pushdown` — opt-in per-table fast path with drift warning surfaced in config.
- **Exit:** a subscriber receives only RLS-permitted change payloads; one introspection returns the merged schema.

**Phase 4 — Flint Ember (`flint_llm`)**
- `p4-c001-liter-llm-binding` — pgrx wrap, routing **through flint-gate/UAR**, no provider keys in DB.
- `p4-c002-async-embeddings` — Surface 2 BGW + `llm.jobs` queue + `llm.enable_embedding` (declarative); rate-limit governor.
- `p4-c003-sync-surface` — `llm.embed`/`llm.complete` with interrupt/timeout safety (gated).
- `p4-c004-summaries` — `llm.enable_summary` async.
- **Exit:** embeddings stay synced via the worker without blocking inserts; sync calls honor `statement_timeout`/cancel.

**Phase 5 — Flint Kiln (runtime + invocation)**
- `p5-c001-component-host` — shared Wasmtime host (with UAR), `wasi:http/proxy`, ProxyPre, fuel/epoch/limits.
- `p5-c002-host-capabilities` — `flint:db/llm/kv/identity/secrets` host impls routing through flint-gate; Cedar-gated linker.
- `p5-c003-invocation` — `/functions/v1/<name>` data plane (compiler-off build), origin-JWT passthrough.
- **Exit:** a signed component handles an HTTP request and calls back into Quarry/Ember under origin identity, RLS-enforced.

**Phase 6 — Flint Kiln (registration, signing, storage, AOT)**
- `p6-c001-signing` — `fke-sign-did` (Ed25519/DID-VC) + `fke-sign-cosign`; verify-before-instantiate.
- `p6-c002-storage` — `ComponentStore` + OCI (primary), IPFS, S3, fs.
- `p6-c003-aot-control-plane` — admin server compiles, cross-compiles (`all-arch`), `.cwasm` cache keyed `(digest,target,version)`; data-plane deserialize-only.
- `p6-c004-admin-rest` — register/version/activate/delete; locked-down admin listener.
- **Exit:** register → verify → AOT per target → invoke pre-compiled with zero compile latency; unsigned/tampered artifacts refused.

**Phase 7 — Hardening & convergence**
- Webhook↔Kiln wiring (a hook targets an edge function); shared outbox/JWT capture proven across `flint_hooks` and `flint_llm`; shared component host proven across Kiln and UAR; backpressure contract with flint-gate; subscription cardinality limits; Keto cache invalidation; load tests; observability; Dagger end-to-end.

---

## 8. External Dependencies & Open Items (confirm; mostly non-blocking for Phase 0)

1. **Fabric `WatchEntityType`** — the one cross-repo change; gates Phase 3 subscriptions, not Phase 0.
2. **Prebuilt PG18 image** — confirm it ships/permits: `pg_net`, `pgcrypto`, `pg_graphql` (PG18-current), pgrx custom extensions + `shared_preload_libraries` for BGWs, and `wal_level=logical` (else fabric uses a `frf-postgres-notify` source — transparent to Quarry via `WatchEntityType`).
3. **Version currency (confirm at each phase kickoff):** Wasmtime (Component Model, `wasi:http/proxy`; WASI 0.3/`wasi:http/service` is forthcoming — target 0.2 now), async-graphql 7, pgrx (latest stable, PG18), pg_graphql, liter-llm, polyglot toolchains (`jco`/`componentize-js`, `componentize-py`, TinyGo wasip2).
4. **flint-gate contract** — pin the exact minted claim set (`sub`, `role`, traits) and the service-identity credential Forge uses for Option-3 outbound and capability callbacks.
5. **Naming nit (non-blocking):** Flint Quarry retains the ratified `fdb-` crate prefix; rename to `fq-` is trivial if desired.
6. **Flint Vault (`flint_vault`, §4.4) — open decisions:** (a) **KEK provider** — confirm Azure Key Vault via managed identity as the v1 unwrap target (AWS/GCP/Vault Transit satisfy the same stdin→stdout unwrap contract); (b) **edge default** — high-value secrets brokered, everything else delegated-but-default-deny (`reveal` needs an explicit Cedar grant); (c) **resolution path** — ship v1 `SECURITY DEFINER` SQL now, harden to in-process Rust later, or hold; (d) **crypto lib** — keep pure-Rust `chacha20poly1305` or switch to `dryoc` for libsodium-compatible AEAD + mlock'd/guarded key pages; (e) **wrapped-DEK location** — file mounted into the DB pod vs. a bootstrap table row.

---

## 9. Non-goals

- No end-user authentication (Kratos via flint-gate). No TLS termination (reverse proxy / flint-gate).
- No second database backend built (the `GraphQlExecutor`/`DatabaseBackend` seams preserve the option).
- No uniform cross-backend GraphQL dialect (deferred; reversible at the port).
- No re-exposure of the fabric's own realtime client surface (the fabric owns it).
- No synchronous LLM calls on the write hot path by default (Surface 2 async is the default; Surface 1 is gated/explicit).
- No runtime-less "fully native" WASM execution (would strip the sandbox/capability/governance model; AOT keeps the runtime).
- No secret of any kind stored in Postgres **in plaintext**, and no raw secret value exposed to edge components — secrets live encrypted in `flint_vault` (§4.4) under a KMS-wrapped DEK; inference still routes through flint-gate/UAR, and edge access is brokered or Cedar-gated `reveal`.
```
