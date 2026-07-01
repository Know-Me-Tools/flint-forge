# Flint Platform — Analysis: Supabase-Equivalent Feature Map & Phase Finalization

**Phase:** p0-workspace-foundation  
**Stage:** Analyze  
**Date:** 2026-06-29  
**Scope:** Cross-repo (flint-forge, flint-gate, flint-realtime-fabric)

---

## 1. Supabase Feature Surface vs Flint Platform Coverage

Supabase's production feature surface (verified 2025) maps to seven capability areas. The table below maps each Supabase feature to the Flint component that covers it, with gap status.

| Supabase Feature | Flint Component | Repo | Status |
|---|---|---|---|
| **Postgres (managed)** | Postgres 18 + Anvil extensions | flint-forge | Infra only; pgrx extensions in progress |
| **Auth (email/social/magic link/MFA)** | Ory Kratos | flint-gate | Kratos integration present in flint-gate; authn working |
| **REST API (auto-generated)** | Flint Quarry REST | flint-forge | Designed (RFC §3.4); not implemented (all todo!()) |
| **GraphQL API** | Flint Quarry GraphQL | flint-forge | Designed (§3.2 hybrid); not implemented |
| **Realtime (change streaming)** | frf-postgres-cdc + fdb-realtime | frf + forge | Designed both sides; both at Phase 0 skeleton |
| **Storage (files/CDN)** | Not yet designed | — | GAP: no Flint Storage component exists |
| **Edge Functions (Deno/TS)** | Flint Kiln (WASM/Rust) | flint-forge | Designed (RFC §5); not implemented; superior model |
| **Row Level Security** | Postgres RLS + flint_auth | flint-forge | flint_auth SQL helpers designed; pgrx crate at skeleton |
| **Database webhooks** | flint_hooks | flint-forge | Designed (§4.2); not implemented |
| **Vector / AI (pgvector + embeddings)** | Flint Ember (flint_llm) + pgvector | flint-forge | flint_llm at skeleton (pgrx); pgvector in Docker image |
| **Secrets management** | Flint Vault (flint_vault) | flint-forge | **Production-complete** (XChaCha20+HKDF, envelope enc) |
| **Connection pooler (PgBouncer equiv)** | deadpool-postgres in Quarry | flint-forge | Designed; not implemented |
| **Dashboard / Studio** | Not designed | — | GAP: no admin UI component |
| **OAuth 2.1 / OIDC provider** | Ory Hydra (via Kratos) | external | Exists via Ory stack; not surfaced in Flint UI |
| **Auth JWTs for RLS** | flint-gate JWT minting | flint-gate | flint-gate mints JWTs; `forge-identity` verify at todo!() |
| **Coarse authZ (relationships)** | Ory Keto | frf + forge | Keto integration designed; not implemented |
| **Action/capability policy** | Cedar (PAUX-1)  | shared | Cedar in UAR; integration points designed but not wired |
| **Realtime presence** | frf-app presence | frf | Phase 1 design; not implemented |
| **Realtime broadcast** | frf-app broadcast | frf | Phase 1 design; not implemented |
| **Scheduled jobs (pg_cron equiv)** | Not designed | — | GAP: no scheduler in current specs |
| **Database functions (RPC)** | Quarry `/rpc/<fn>` | flint-forge | Designed (§3.4); not implemented |
| **CLI (`supabase` equiv)** | frf-cli + Dagger | frf + CI | frf-cli in Phase 7; Dagger CI exists |

### Flint Platform Advantages Over Supabase

Beyond feature parity, the platform has three structural advantages:

1. **AI-native database.** Flint Ember puts LLM embedding/completion directly in Postgres via pgrx — no external call to a sidecar. Supabase has no equivalent. The async worker + Surface 2 model avoids the lock-contention problems Supabase's `aisql` extension has in write paths.

2. **Sovereign edge compute.** Flint Kiln runs WASM Component Model with WIT contracts and Cedar-gated capability linker. Supabase Edge Functions are Deno (V8) with no fine-grained capability model. Kiln components are Turing-complete, WASM-native, and compiled/signed at deploy time — not JS bundles.

3. **True realtime spine.** flint-realtime-fabric uses Apache Iggy (durable, partitioned) as the substrate, with CDC-to-Keto-RLS-to-subscriber as a first-class pipeline. Supabase Realtime is Phoenix/Elixir PubSub with a row-re-query RLS gate — same technique, weaker durability. The Flint version adds offline CRDT sync, P2P sync, WebRTC media, AG-UI/A2A/A2UI — not possible in Supabase.

---

## 2. Three-Repo Integration Architecture

```
                ┌──────────── flint-gate ─────────────────────┐
                │ :4456 proxy — authn (Kratos), JWT mint,      │
                │ SSE/AG-UI/A2UI streaming, token meter         │
                │ :4457 admin — route CRUD (Postgres LISTEN)    │
                └──┬──────────────────────────┬────────────────┘
                   │ RLS JWT (Option-3)         │ stream passthrough
                   ▼                            ▼
       ┌── flint-forge ──────────────┐   ┌── flint-realtime-fabric ──────────────┐
       │ Quarry :REST/GraphQL        │   │ Gateway :gRPC (WatchEntityType)        │
       │ Kiln :WASM fn gateway       │   │ Spine: Apache Iggy                     │
       │ Postgres 18 + Anvil:        │   │ CDC: frf-postgres-cdc → Iggy           │
       │  flint_auth  flint_hooks    │   │ AuthZ: Keto per-event gate             │
       │  flint_llm   flint_vault    │   │ CRDT: Loro/automerge + redb            │
       │ WIT: flint:db flint:llm     │   │ Media: str0m / LiveKit                 │
       │      flint:secrets          │   │ Federation: Matrix (Tuwunel), ATProto  │
       └─────────────────────────────┘   └────────────────────────────────────────┘
```

### Integration Contracts (load-bearing, must be pinned before Phase 1 code)

1. **flint-gate → Quarry/Kiln:** RLS JWT format (claims shape, issuer, JWKS endpoint, service-identity token). Quarry's `fdb-auth` verifies this JWT. **Status: UNPINNED** (§8 #4 of RFC-FORGE-001).

2. **Quarry → flint-realtime-fabric:** `WatchEntityType(tenant, entity_type, filter) → stream EntityChange` gRPC call. This is the one fabric-side change required by Forge (RFC §7). **Status: Not designed in FRF proto yet** (FRF Phase 0 proto freeze must define this RPC).

3. **flint_hooks → flint-gate outbound:** Service-identity bearer token + `X-Forge-Origin-JWT` + `X-Forge-Signature`. Webhook calls route through flint-gate, which validates the service identity against its `api_keys` table. **Status: Schema defined in flint-gate; Forge outbound call not implemented**.

4. **Flint Kiln → flint-gate capability check:** Kiln's capability linker calls Cedar via flint-gate admin API for the `flint:db`, `flint:llm`, `flint:secrets` WIT imports. **Status: Designed; not implemented either side**.

5. **flint-gate → flint-realtime-fabric:** Not a direct integration. Gate sits in front of Forge; FRF is the substrate behind Forge. Gate streams SSE/WS from Quarry subscriptions; Quarry consumes FRF via gRPC. No gate→FRF direct call.

---

## 3. Phase Finalization

The seven-phase structure in RFC-FORGE-001 is sound. The analysis adds four coordination points that constrain phase ordering across repos.

### Cross-Repo Phase Dependency Map

```
FRF Phase 0 (workspace + proto freeze)
  ↓  WatchEntityType RPC definition
Forge Phase 0 (workspace + WIT freeze) ← current phase (p0-workspace-foundation)
  ↓  flint-gate JWT contract pin
Forge Phase 1 (Quarry REST + Anvil skeletons + fdb-auth)
  ↓  fdb-realtime gRPC client
FRF Phase 1 (Iggy + CDC + RLS pipeline + Rust SDK)
  ↓  entity sync working
Forge Phase 2 (GraphQL hybrid + subscriptions + schema hot-reload)
  ↓  Quarry subscriptions live
FRF Phase 2 (Generated SDKs + entity-management adapter)
  ...
Forge Phase 3 (Flint Kiln WASM gateway)
  ...parallel with...
FRF Phase 3 (CRDT + offline + FFI)
```

### Finalized Forge Phase Plan

The existing 7-phase plan (RFC §7) is adopted verbatim with these additions:

| Phase ID | Name | Key Deliverables | Cross-Repo Dependency |
|---|---|---|---|
| **p0** | Workspace Foundation | WIT freeze, PG18 image, workspace layout | FRF p0 for WatchEntityType proto definition |
| **p1** | Quarry REST + Anvil + Auth | `fdb-gateway` REST, `fdb-auth` JWT verify, `flint_auth` + `flint_vault` pgrx, RLS wiring | flint-gate JWT contract PIN |
| **p2** | GraphQL + Subscriptions + Hooks | GraphQL hybrid (Q/M→pg_graphql, Sub→async-graphql), `flint_hooks` webhook dispatch, `fdb-realtime` gRPC client | FRF p1 complete (WatchEntityType serving) |
| **p3** | Flint Kiln WASM Gateway | `fke-server`, `fke-store-*`, `fke-sign-*`, WIT linker, Cedar capability gate, Cranelift AOT | Cedar/UAR integration |
| **p4** | Flint Ember (in-DB AI) | `flint_llm` pgrx, async worker BGW, `llm.enable_embedding`, pgvector HNSW integration | Flint Vault (p1) for API key resolution |
| **p5** | AG-UI / A2A / A2UI + Agent Hooks | flint_hooks → FRF agentproto pipe, Kiln WASM→AG-UI emission, A2UI intent gating in flint-gate | FRF p5 (agentproto crate) |
| **p6** | Storage (Forge-native) | File storage API (S3-compatible), CDN integration, Vault-encrypted key management | — |
| **p7** | Hardening + Observability | Load targets, Cedar fan-out audit, Dagger release pipelines, tracing/metrics, opentelemetry | All previous phases |

**New addition — Phase 6 (Storage):** Supabase Storage is a gap. The Flint equivalent should be an S3-compatible object storage gateway backed by a Postgres metadata table, with Vault-encrypted presigned URL credentials and Keto/RLS access control. The `fke-store-*` crates (S3, OCI, IPFS, fs) are Kiln component stores — they can be reused as the object backend with a thin REST/presigned-URL gateway layer added.

**Missing capability — Scheduler:** Supabase ships `pg_cron`. Flint's equivalent is `flint_hooks` extended with a `scheduled_webhooks` table plus `pg_cron` as a postgres extension in the PG18 image. This is a Phase 1 addition (one table + cron extension in the Docker image), not a new phase.

---

## 4. Key Build-vs-Adopt Decisions

| Capability | Decision | Rationale |
|---|---|---|
| Realtime spine | **Adopt Apache Iggy** (GQAdonis fork) | Already chosen; LogBroker trait ensures swap path; FRF commitment |
| CRDT engine | **OPEN — decide before FRF Phase 3** | Loro vs automerge-rs; Loro wins on perf; automerge has larger ecosystem. Not a Forge concern directly. |
| GraphQL server | **Adopt pg_graphql** (Q/M) + **async-graphql** (Sub) | pg_graphql is the existing, proven Supabase model; async-graphql for subscriptions is idiomatic Rust |
| REST API | **Build** (PostgREST-compatible) | PostgREST is Haskell; porting to Rust with deadpool-postgres is correct. `sqlx` is alternate for simpler pools. |
| Auth | **Adopt Ory Kratos + Keto** | Already in stack; not rebuilding authn/Zanzibar |
| Action policy | **Adopt Cedar** (via PAUX-1 / UAR) | Already integrated; don't duplicate |
| Secret store | **Flint Vault** (already complete) | Only production-complete subsystem. KMS envelope beats pgsodium/Supabase-Vault |
| In-DB AI | **Build** (pgrx + liter-llm) | No Supabase equivalent; strategic differentiator |
| Edge functions | **Build** (WASM + WIT + Wasmtime) | Supabase uses Deno; WASM is the superior architecture for Rust-native platform |
| Storage | **Adopt S3 API** (MinIO/Cloudflare R2 backend) + **build** metadata layer | Don't build object storage primitives; build the Postgres-metadata + Vault + Keto access layer |
| Scheduler | **Adopt pg_cron** (add to PG18 image) | One-line addition; no reason to build a Postgres cron from scratch |
| Connection pooler | **Build** (deadpool-postgres in Quarry) | Already in the Quarry crate plan; correct choice for RLS-per-connection model |
| Dashboard / CLI | **Defer to post-p7** | Not blocking user-visible DB/auth/realtime capabilities |

---

## 5. Open Questions (require resolution before targeted phases)

| # | Question | Blocks | Target |
|---|---|---|---|
| OQ-1 | WIT `db.query` param type: `list<string>` or `list<json>`? | c003 close, p3 (Kiln) | This phase |
| OQ-2 | WIT `secrets.get` resource type: plain `string` or Cedar-gated `resource secret`? | c003 close, p3 | This phase |
| OQ-3 | pg_graphql PG18 strategy: wait for upstream / build from source SHA / PG17 sidecar? | p2 (GraphQL Q/M) | Before p2 spec |
| OQ-4 | flint-gate JWT claim shape: `sub`, `role`, `tenant_id` — exact field names and type? | p1 (fdb-auth verify) | Before p1 spec |
| OQ-5 | flint-gate service-identity token format: HS256 shared secret or ES256 keypair? | p1 (outbound Option-3) | Before p1 spec |
| OQ-6 | Keto check caching strategy: TTL-based or tuple-delete-invalidation via Iggy? | p2 (subscription RLS) | Before p2 spec |
| OQ-7 | CRDT engine choice (Loro vs automerge-rs): commit before FRF Phase 3 | FRF p3 | FRF decision |
| OQ-8 | Storage backend: MinIO (self-hosted) or Cloudflare R2 or both? | p6 (storage) | Before p6 spec |

---

## 6. Risk Register

| Risk | Severity | Mitigation |
|---|---|---|
| FRF proto not frozen when Forge p2 needs WatchEntityType | HIGH | Coordinate FRF Phase 0 freeze before Forge p1 starts |
| pg_graphql has no PG18 release (supabase/pg_graphql#614) | HIGH | Build from source SHA pinned in Dockerfile; or use pg_graphql-rs fork; decision OQ-3 |
| flint-gate JWT contract drift (unpinned) | HIGH | Pin in a shared contract file consumed by both repos before p1 spec |
| pgrx 0.18.1 PG18 compatibility issues in flint_llm | MEDIUM | Use PG17 sidecar for flint_llm until PG18 pgrx is proven stable |
| wasm-tools not installed (blocks c003) | MEDIUM | Install via cargo: `cargo install wasm-tools`; gate c003 on this |
| Iggy pre-1.0 breaking changes in GQAdonis fork | MEDIUM | LogBroker trait provides swap path; pin fork SHA; watch upstream |
| Supabase Storage gap (no Flint equivalent until p6) | LOW | Acceptable; Storage is not blocking any auth/API/realtime capability |
