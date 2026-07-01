# Plan — p0-workspace-foundation

**Generated:** 2026-06-29  
**Stage:** Plan (post-Analyze)  
**Inputs:** assessment.md, analysis.md, library-candidates.json, FLINT-FORGE-SPEC.md, RFC-FRF-002, flint-gate README

---

## Immediate p0 Remaining Work (before Phase 0 is closed)

### p0-c003 — WIT contract freeze

**Status:** WIT file corrected in this session. Remaining tasks:

| # | Task | Owner | Blocker? |
|---|---|---|---|
| 1 | Install `wasm-tools` — `cargo install wasm-tools` | Developer machine | Yes — needed for validation |
| 2 | Run `wasm-tools component wit wit/flint/host` — must resolve cleanly | CI + dev | Yes |
| 3 | Generate Rust bindings: `wit-bindgen generate --lang rust wit/flint/host` for a hello component | Dev | Yes |
| 4 | Build sample component targeting `wasm32-wasip2` | Dev | Yes |
| 5 | Tag `flint:host@0.1.0` as frozen in `docs/contracts/wit-freeze.md` | Dev | Yes — gates p3 (Kiln) |

**WIT divergences fixed this session:**
- `list<json>` → `list<string>` with JSON-encode/decode at host boundary  
- `error` → `record host-error { code: string, message: string }`  
- `secrets.get` now returns `result<secret, host-error>` with `resource secret { reveal }` (Cedar-gated)  
- `opts: json` in `llm.complete` → `opts: string`  
- `claims()` in `identity` → `string` (JSON-encoded)  
- Spec §5.4 updated to match with explanatory note  

### p0-c004 — fabric WatchEntityType (cross-repo)

Schedule with flint-realtime-fabric team. FRF Phase 0 (proto freeze) must add:
```protobuf
rpc WatchEntityType(WatchEntityTypeRequest) returns (stream EntityChange);
message WatchEntityTypeRequest { string tenant = 1; string entity_type = 2; string filter_json = 3; }
```
This gates Forge Phase 3 (GraphQL subscriptions), not Phase 1 or 2.

### p0 Pre-Phase-1 contracts to pin

Before Phase 1 spec is written, create `docs/contracts/jwt-contract.md`:
- Exact claim shape minted by flint-gate: `sub` (string), `role` (string), `tenant_id` (string), `traits` (object)
- Service-identity token format for Option-3 outbound (HS256 shared secret vs ES256 keypair)
- JWKS endpoint flint-gate exposes for forge-identity to verify against

---

## Full Platform Phase Plan

This plan covers all three repos. Forge phases are the primary KBD scope; FRF and Gate phases are coordination points.

---

## Phase 1 — Flint Anvil: Auth + Hooks + Vault Hardening

**Repo:** flint-forge  
**Gate:** INSERT fires a signed Option-3 webhook through flint-gate; RLS works end-to-end in a test DB.

### OpenSpec changes

| Change ID | Title | Library candidates | Priority |
|---|---|---|---|
| `p1-c001-flint-auth` | `auth.*` SQL helpers + GUC contract tests (pgrx) | pgrx 0.18.1 | P0 |
| `p1-c002-flint-hooks-standard` | Registry + dispatch trigger + pg_net standard tier + Option-3 HMAC | pgrx, pg_net | P0 |
| `p1-c003-flint-hooks-durable` | Outbox table + dispatcher BGW + SKIP LOCKED retry | pgrx | P1 |
| `p1-c004-pg-cron` | Add `pg_cron` to images/postgres18/Dockerfile | pg_cron apt package | P1 |
| `p1-c005-jwt-contract-pin` | `docs/contracts/jwt-contract.md` — pin claim shape + service-identity format | — | P0 (blocker for p2) |
| `p1-c006-vault-kms` | KMS unwrap integration in ext-flint-vault (Azure Key Vault managed identity v1) | azure-identity crate | P2 |

**Developer tooling additions (Phase 1):**
- `forge-cli hook add <table> <url>` — calls Quarry admin API to register a webhook
- `forge-cli vault set <name> <category>` — stdin → encrypted secret via `vault.create_secret`
- `forge-cli vault get <name>` — decrypt + print (admin only, local dev)

---

## Phase 2 — Flint Quarry: REST + RLS + Connection Pool

**Repo:** flint-forge  
**Gate:** RLS-correct REST CRUD + vector search under a real flint-gate JWT.

### OpenSpec changes

| Change ID | Title | Library candidates | Priority |
|---|---|---|---|
| `p2-c001-fdb-auth` | JWT verify → `RlsContext` (`forge-identity::verify_and_build`) | lc-010: jsonwebtoken + reqwest JWKS | P0 |
| `p2-c002-fdb-postgres` | deadpool-postgres pool + SET LOCAL RLS context + SchemaProvider | lc-001: deadpool-postgres | P0 |
| `p2-c003-rest-executor` | PostgREST-compat REST: GET/POST/PATCH/DELETE, filter ops, `/rpc`, Range headers | lc-001 | P0 |
| `p2-c004-pgvector-rpc` | `/rpc/<fn>` for vector similarity; `ORDER BY embedding <-> $q LIMIT k` | — | P1 |
| `p2-c005-schema-hot-reload` | ArcSwap SchemaRegistry + DDL change detection via flint_hooks / poll | arc-swap | P1 |

**SDK additions (Phase 2 — Rust client):**
- `crates/forge-sdk-rust` — Rust client SDK: `ForgeClient { rest, graphql, realtime, auth }`
  - REST: typed `Table<T>` query builder mirroring PostgREST filter operators
  - Auth: JWT/API key header injection
  - No async-graphql dependency — client sends raw GraphQL over HTTP
- Published to crates.io as `forge-sdk`

---

## Phase 3 — Flint Quarry: GraphQL + Subscriptions

**Repo:** flint-forge  
**Cross-repo dependency:** FRF Phase 1 must be complete (WatchEntityType serving)  
**Gate:** Subscriber receives only RLS-permitted change payloads; merged introspection works.

### OpenSpec changes

| Change ID | Title | Library candidates | Priority |
|---|---|---|---|
| `p3-c001-graphql-passthrough` | `POST /graphql` → `graphql.resolve()` under RLS | lc-002: pg_graphql passthrough | P0 |
| `p3-c002-subscriptions` | fdb-realtime gRPC → WatchEntityType + Keto gate + per-event RLS re-query | lc-003: async-graphql + tonic | P0 |
| `p3-c003-introspection-merge` | Union pg_graphql schema ∪ subscription SDL | async-graphql dynamic | P0 |
| `p3-c004-graphql-transport-ws` | `graphql-transport-ws` WebSocket upgrade on `/graphql` | graphql-ws crate | P0 |
| `p3-c005-pg-graphql-pg18` | Resolve OQ-3: build pg_graphql from source SHA for PG18 or PG17 sidecar decision | — | P0 (must decide before this phase starts) |
| `p3-c006-predicate-pushdown` | Opt-in RLS predicate pushdown (off by default; operator risk acknowledgement) | — | P2 |

**SDK additions (Phase 3):**
- TypeScript SDK (`sdks/typescript/`) — `forge-sdk` npm package
  - REST client: `from('table').select().eq('id', val).execute()`  
  - Realtime: WebSocket subscription client over `graphql-transport-ws`
  - Auth helpers: session management, JWT refresh
  - Generated from OpenAPI spec (`openspec/specs/quarry.openapi.yaml`)
- Go SDK (`sdks/go/`) — `forge-sdk-go` module
  - REST + realtime client
  - Generated + hand-wrapped (OpenAPI codegen → idiomatic Go shim)
- Python SDK (`sdks/python/`) — `forge-sdk-py` PyPI package
  - Pure Python REST + realtime client (no PyO3 needed at this layer — PyO3 is for the Rust core bindings, Phase 6+)
  - `asyncio`-native, `httpx` + `websockets`

---

## Phase 4 — Flint Ember: In-DB LLM + Embeddings

**Repo:** flint-forge  
**Gate:** Embeddings stay synced without blocking inserts; sync surface honors `statement_timeout`.

### OpenSpec changes

| Change ID | Title | Library candidates | Priority |
|---|---|---|---|
| `p4-c001-liter-llm-binding` | pgrx liter-llm wrap; routing through flint-gate/UAR; keys from Vault | lc-015: liter-llm + pgrx BGW | P0 |
| `p4-c002-async-embeddings` | Surface 2 BGW + `llm.jobs` queue + `llm.enable_embedding` declarative | pgrx BGW | P0 |
| `p4-c003-rate-governor` | Rate-limit governor in BGW (per-model, per-tenant) | — | P1 |
| `p4-c004-sync-surface` | Surface 1 `llm.embed`/`llm.complete` with CHECK_FOR_INTERRUPTS + WaitLatch | — | P2 |
| `p4-c005-summaries` | `llm.enable_summary` async via BGW | — | P2 |

---

## Phase 5 — Flint Kiln: WASM Runtime + Invocation

**Repo:** flint-forge  
**Gate:** A signed component handles an HTTP request and calls back into Quarry under origin identity with RLS enforced.

### OpenSpec changes

| Change ID | Title | Library candidates | Priority |
|---|---|---|---|
| `p5-c001-component-host` | Wasmtime engine + `wasi:http/proxy` world + ProxyPre cache + fuel/epoch | lc-005: wasmtime + wasmtime-wasi-http | P0 |
| `p5-c002-host-capabilities` | `flint:db/llm/kv/identity/secrets` host impls; Cedar-gated linker | lc-013: cedar-policy | P0 |
| `p5-c003-invocation` | `/functions/v1/<name>` data plane (compiler-off); origin-JWT passthrough | — | P0 |
| `p5-c004-wit-bindgen-sdk` | wit-bindgen Rust guest bindings published as `forge-wit-bindings` crate | wit-bindgen | P0 |

**Developer tooling additions (Phase 5):**
- `forge-cli fn build <lang>` — scaffolds + builds a WASM component in Rust/TS/Go/Python
  - Rust: `cargo component build --target wasm32-wasip2`
  - TypeScript: `jco componentize` (via `componentize-js`)
  - Go: `TinyGo` wasip2 target
  - Python: `componentize-py`
- `forge-cli fn deploy <component.wasm>` — sign + upload + AOT-compile via Kiln admin API
- `forge-cli fn invoke <name> [--data '{...}']` — local test invocation

---

## Phase 6 — Flint Kiln: Registration, Signing, AOT, Storage

**Repo:** flint-forge  
**Gate:** Register → verify → AOT per target → invoke pre-compiled; unsigned artifacts refused.

### OpenSpec changes

| Change ID | Title | Library candidates | Priority |
|---|---|---|---|
| `p6-c001-signing` | `fke-sign-did` (Ed25519/DID-VC Kaia) + `fke-sign-cosign` (OIDC-keyless) | ed25519-dalek | P0 |
| `p6-c002-store-oci` | OCI registry ComponentStore (primary; wkg-compatible; sign-by-digest) | oci-client crate | P0 |
| `p6-c003-store-s3-ipfs-fs` | S3 + IPFS + filesystem adapters | aws-sdk-s3, object_store | P1 |
| `p6-c004-aot-control-plane` | Admin server Cranelift AOT compile + cross-compile (`all-arch`) + `.cwasm` cache | wasmtime (all-arch feature) | P0 |
| `p6-c005-admin-rest` | `/admin/functions` CRUD + locked-down listener + Cedar publisher gate | — | P0 |
| `p6-c006-storage-gateway` | Flint Storage: S3-compat file gateway + Postgres metadata + Vault creds + Keto ACL | lc-012: aws-sdk-s3 + object_store | P1 |

---

## Phase 7 — AG-UI / A2A / A2UI + Agent Hooks

**Repo:** flint-forge (+ FRF Phase 5 agentproto)  
**Gate:** Agent events stream to UI via SDKs; hook targets an edge function.

### OpenSpec changes

| Change ID | Title | Priority |
|---|---|---|
| `p7-c001-webhook-kiln-wiring` | `flint_hooks` target can be a Kiln edge function (not just a URL) | P0 |
| `p7-c002-agentproto-pipe` | flint_hooks → FRF agentproto → AG-UI emission | P0 |
| `p7-c003-a2ui-kiln` | Kiln WASM → A2UI intent emission (structured UI-intent events) | P1 |
| `p7-c004-a2ui-gate` | flint-gate A2UI intent gating in SSE processor (filter by allowed_intents + scope) | P1 |

---

## Phase 8 — SDK Completeness + Developer Experience

**Repos:** flint-forge (SDK crates), flint-gate (client helpers), flint-realtime-fabric (SDK generation)

This phase completes the SDK surface and developer tooling to Supabase-competitive DX.

### SDK matrix

| SDK | Package | Mechanism | Key capabilities |
|---|---|---|---|
| **Rust** | `forge-sdk` (crates.io) | Hand-written, `crates/forge-sdk-rust` | REST query builder, GraphQL, realtime subscriptions, auth, fn deploy |
| **TypeScript** | `@forge/sdk` (npm) | Generated OpenAPI + hand-written realtime/auth layer | REST, realtime WS, auth helpers, edge fn scaffold |
| **Go** | `forge-sdk-go` (GitHub module) | OpenAPI codegen + idiomatic wrapper | REST, realtime, auth |
| **Python** | `forge-sdk` (PyPI) | Pure Python, asyncio-native | REST, realtime, auth; PyO3 Rust core binding for performance-critical paths (Phase 8+) |

### OpenSpec changes

| Change ID | Title | Priority |
|---|---|---|
| `p8-c001-openapi-spec` | Generate `openspec/specs/quarry.openapi.yaml` from Axum routes | P0 |
| `p8-c002-ts-sdk` | `@forge/sdk` npm package: REST + realtime + auth + fn helper | P0 |
| `p8-c003-go-sdk` | `forge-sdk-go`: REST + realtime + auth | P0 |
| `p8-c004-python-sdk` | `forge-sdk` PyPI: asyncio REST + realtime + auth | P0 |
| `p8-c005-rust-sdk` | `forge-sdk` crates.io: full REST query builder + realtime + fn deploy | P0 |
| `p8-c006-pyo3-core` | PyO3 bindings over forge-sdk-rust for performance-critical Python paths | P2 |
| `p8-c007-cli-completeness` | `forge-cli` complete: hook/fn/vault/migrate/db commands, shell completions, config init | P1 |
| `p8-c008-local-dev-stack` | `forge-cli dev up` — docker-compose stack (PG18 + flint-gate + forge) for local dev | P1 |
| `p8-c009-sdk-docs-site` | Generated docs site from OpenAPI + WIT | P2 |

---

## Phase 9 — Hardening + Observability + Release Automation

**Repos:** all three  
**Gate:** Load targets met; rights delegation live; one git tag → all registries publish.

### OpenSpec changes

| Change ID | Title | Priority |
|---|---|---|
| `p9-c001-authz-load-test` | Keto fan-out load test + check-cache tuning (subscribe-time scoping + topic partitioning) | P0 |
| `p9-c002-user-rights-api` | User-controlled Keto tuple write/delete API (owner grants access to object) | P0 |
| `p9-c003-otel` | OpenTelemetry traces + metrics across all port boundaries (tracing → OTLP) | P1 |
| `p9-c004-dagger-release` | Dagger pipelines: on git tag → build + test + publish all SDKs to crates.io/npm/PyPI/GitHub | P0 |
| `p9-c005-subscription-limits` | Subscription cardinality limits + backpressure contract with flint-gate | P1 |
| `p9-c006-keto-cache-invalidation` | Tuple-delete Keto invalidation via FRF Iggy event | P1 |

---

## Cross-Repo Coordination Schedule

| Forge Phase | Requires from FRF | Requires from Gate |
|---|---|---|
| Phase 1 start | — | JWT contract pin (OQ-4/OQ-5) |
| Phase 3 start | FRF Phase 1 complete (WatchEntityType serving) | — |
| Phase 5 start | — | Cedar capability check API in gate admin |
| Phase 7 start | FRF Phase 5 complete (agentproto crate) | A2UI intent filtering in SSE processor |

---

## Developer Tooling Summary

The `forge-cli` crate (`crates/forge-cli`) becomes the primary developer interface, equivalent to the Supabase CLI.

### Command surface (across phases)

```
forge version
forge dev up                         # Phase 8 — docker-compose local stack
forge dev down
forge dev logs [service]

forge db migrate                     # Phase 1 — apply SQL migrations
forge db reset

forge hook add <table> <url>         # Phase 1 — register webhook
forge hook list
forge hook remove <id>

forge vault set <name> <category>    # Phase 1 — encrypt + store secret
forge vault get <name>               # Phase 1 — decrypt + print (admin)
forge vault rotate <name>            # Phase 6 — re-encrypt with new DEK epoch

forge fn new <name> [--lang rust|ts|go|python]  # Phase 5 — scaffold component
forge fn build [<name>]              # Phase 5 — compile to wasm32-wasip2
forge fn deploy <name>               # Phase 5 — sign + upload + AOT
forge fn list
forge fn invoke <name> [--data '{}'] # Phase 5 — test invocation
forge fn logs <name>

forge functions serve                # Phase 8 — local Kiln dev server

forge gen types [--lang ts|go|python|rust]  # Phase 8 — generate typed client from schema
```

### IDE / editor integration

- VS Code extension stub — `forge-vscode` (Phase 8): schema autocomplete, fn deploy, log tail
- Language server for WIT files via `wit-language-server` (Phase 5+)

---

## Waypoint After This Plan

- **Active phase:** `p0-workspace-foundation`
- **Status:** `plan_complete`
- **Next action:** Fix c003 (install wasm-tools, validate WIT, build sample component) → declare p0 freeze → move to Phase 1

---

## Recommended Execution Order (First 3 Sprints)

**Sprint 1 (close Phase 0):**
1. Developer installs `wasm-tools`, validates WIT, builds sample component → c003 gate passes
2. FRF team adds WatchEntityType to proto (schedule; not blocking Sprint 1)
3. Write `docs/contracts/jwt-contract.md` with flint-gate team → pins OQ-4/OQ-5

**Sprint 2 (Phase 1 — Anvil + Auth):**
4. `p1-c001-flint-auth` — pgrx auth helpers + tests
5. `p1-c002-flint-hooks-standard` — webhook dispatch
6. `p1-c005-jwt-contract-pin` — contract doc (prerequisite for Sprint 3)
7. `p1-c004-pg-cron` — one-line Dockerfile addition

**Sprint 3 (Phase 2 — Quarry REST):**
8. `p2-c001-fdb-auth` — JWT verify → RlsContext
9. `p2-c002-fdb-postgres` — pool + SET LOCAL
10. `p2-c003-rest-executor` — PostgREST-compat CRUD
11. Begin Rust SDK (`forge-sdk-rust`) alongside REST implementation
