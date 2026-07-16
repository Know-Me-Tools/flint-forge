# Flint Forge

**The sovereign data & edge-compute plane of the Flint platform** — a Rust workspace
that sits behind [`flint-gate`](#external-systems) (ingress/auth) and consumes
[`flint-realtime-fabric`](#external-systems) (the realtime spine) to serve structured
data, run in-database compute, and execute signed WASM edge functions.

The full functional specification, architecture, and development plan is
**RFC-FORGE-001** — see [`docs/FLINT-FORGE-SPEC.md`](docs/FLINT-FORGE-SPEC.md)
(branded HTML: [`docs/FLINT-FORGE-SPEC.html`](docs/FLINT-FORGE-SPEC.html)).

> **Status: v1.0-ready.** The core server plane (Quarry + Kiln), Anvil pgrx extension
> suite, operator CLI, client SDKs, and production packaging are implemented, tested, and
> internally consistent. Active development is tracked as phased OpenSpec change sets
> (see [Phased build](#phased-build)).

---

## The metaphor

The metaphor is the forge. Raw material is **quarried** from the source, shaped on the
**anvil** in place, fired in the **kiln** into hardened tools — all inside the **forge**.

## Subsystems

| Subsystem | Flint name | Crate prefix | Role |
|---|---|---|---|
| REST/GraphQL DB API gateway | **Flint Quarry** | `fdb-*` | Extract structured data from Postgres via REST + GraphQL |
| pgrx in-database extension suite | **Flint Anvil** | `flint_*` (pgrx) | Shape data in place: auth context, webhooks, in-DB LLM, secrets, metadata |
| WASM edge-function gateway | **Flint Kiln** | `fke-*` | Compile (fire) and run signed WASM components |
| Shared core | Forge core | `forge-*` | Cross-cutting domain types, identity, policy |

### Flint Quarry (`fdb-*`) — data extraction

A PostgREST-compatible REST surface plus a hybrid GraphQL surface over Postgres 18.

- **REST** — PostgREST-compatible CRUD compiled from in-DB reflection metadata
- **GraphQL Query/Mutation** — delegated directly to `graphql.resolve()` inside Postgres
  under RLS (`pg_graphql` passthrough)
- **GraphQL Subscription** — `async-graphql` over `graphql-transport-ws`, resolvers pull
  from a `ChangeStreamSource`. Default source is PostgreSQL LISTEN/NOTIFY
  (`FLINT_CHANGE_SOURCE=listen`, complete and working). The Flint Realtime Fabric gRPC
  source (`FLINT_CHANGE_SOURCE=fabric`) is an opt-in alternative that currently fails
  closed — FRF does not yet expose the `WatchEntityType` RPC it depends on
- **Router hot-swap** — REST/GraphQL routers rebuilt on schema change and swapped via
  `ArcSwap<Router<()>>`

Crates: `fdb-domain`, `fdb-ports`, `fdb-app`, `fdb-postgres`, `fdb-realtime`, `fdb-auth`,
`fdb-reflection`, `fdb-gateway` (Axum composition root).

### Flint Anvil (`ext-flint-*`) — in-place shaping (pgrx extensions)

In-database Postgres extensions. **Excluded from the default workspace** because they
require a Postgres toolchain (`cargo pgrx`).

| Extension | Flint name | Role |
|---|---|---|
| `flint_auth` | — | JWT/RLS GUC injection contract; `auth.uid()`, `auth.role()`, `auth.bearer()` helpers |
| `flint_hooks` | — | Trigger → webhook dispatch, JWT-forwarding, two delivery tiers |
| `flint_llm` | **Flint Ember** | liter-llm bound into Postgres (sync + async LLM/embedding surfaces) |
| `flint_vault` | **Flint Vault** | Encrypted secret store (XChaCha20-Poly1305), KMS-wrapped DEK |
| `flint_meta` | — | Reflection cache tables, DDL event triggers, version tracking, LISTEN/NOTIFY |

### Flint Kiln (`fke-*`) — firing WASM edge functions

A polyglot WASM Component Model edge-function gateway on a Wasmtime host (shared substrate
with UAR Tier-2 WASM skills).

- **Admin REST** (control plane) — component compile/register, Cranelift-backed
- **`/functions/v1/<name>`** (data plane) — invoke compiled components, compiler off
- **Pluggable component stores** — OCI, IPFS, S3, filesystem
- **Signers** — DID, cosign

Crates: `fke-domain`, `fke-ports`, `fke-runtime`, `fke-store-{oci,ipfs,s3,fs}`,
`fke-sign-{did,cosign}`, `fke-registry`, `fke-server` (Axum composition root).

### Forge core (`forge-*`)

`forge-domain` (zero-infra shared types), `forge-identity` (identity primitives),
`forge-policy` (Cedar policy evaluation), `forge-cli` (`flint-forge` binary).

---

## Architecture

### Hexagonal dependency rule

Layering is enforced at the Cargo dependency level:

```
forge-domain          Layer 0: pure types, serde only, zero infra deps
  ▲
forge-ports / *-app   Layer 1: trait seams (ports) + use-cases
  ▲
adapters              fdb-postgres, fdb-realtime, fke-store-*, fke-sign-*, …
  ▲
interface crates      fdb-gateway, fke-server  (the only crates that import adapters)
```

**Domain and app crates never import adapter crates.** Composition happens only in the
interface crates.

### Four authorization layers

1. **Kratos** — authentication (at `flint-gate`, per session)
2. **Keto** — coarse relationship check (subscribe-time, cached)
3. **Postgres RLS** — authoritative row filter (every query / subscription event)
4. **Cedar** — action/capability policy (mutations, Kiln linker, Ember model-use)

Postgres **never verifies** JWT signatures — `flint-gate` does that upstream. Every pooled
connection sets `SET LOCAL ROLE`, `request.jwt.claims`, and `request.headers` per request
transaction before any user statement.

### Subscription RLS enforcement

WAL bypasses RLS. For each change delivered by the `ChangeStreamSource` (LISTEN/NOTIFY by
default, or the fabric adapter when opted in), Quarry **re-queries the changed row as the
subscriber** with full RLS context before delivering — non-negotiable protection against
WAL-bypass data leaks.

### External systems

Depended on, **not built here**:

- **`flint-gate`** — ingress, Kratos session → RLS JWT minting, WS/SSE/NDJSON stream proxy
- **`flint-realtime-fabric`** — CDC, Iggy spine, Keto per-event gate, the `WatchEntityType`
  RPC that Quarry subscriptions consume
- **UAR** — sovereign inference/governance plane that Ember and Kiln route LLM calls into;
  shares the Wasmtime component-host substrate with Kiln

---

## Build

```bash
cargo check                      # check all non-pgrx workspace crates
cargo run -p fdb-gateway         # Quarry gateway
cargo run -p fke-server          # Kiln server
cargo test                       # run all tests
cargo clippy --workspace -- -D warnings   # lint (pedantic; the CI gate)
cargo fmt --all                  # format
```

### pgrx extensions (Flint Anvil)

Built separately — they require a Postgres toolchain and are excluded from the default
workspace. All five extensions target **Postgres 18** with **pgrx 0.18.1**:

```bash
cargo install cargo-pgrx --version 0.18.1 --locked && cargo pgrx init
cargo pgrx run -p ext-flint-auth
cargo pgrx run -p ext-flint-hooks
cargo pgrx run -p ext-flint-llm
cargo pgrx run -p ext-flint-meta
cargo pgrx run -p ext-flint-vault
```

A pinned Postgres 18 image with all extensions pre-installed is built from
[`images/postgres18/Dockerfile`](images/postgres18/Dockerfile) and used by
`docker-compose.yml` and the CI integration-test job.

### CI

```bash
./scripts/ci-check.sh
```

### Toolchain

- **Edition:** 2021 · **MSRV:** `1.96` (channel `stable`, pinned in `rust-toolchain.toml`)
- **Key deps:** Axum 0.8.8, Tokio, `sqlx`/`deadpool-postgres`, `async-graphql` 7,
  `tonic` 0.12, `pgvector` 0.4, `wasmtime` 46, `arc-swap`

---

## Quality gates (CI-enforced)

- No `unwrap()`/`expect()` in library crates — `thiserror` in libs, `anyhow` only at binary
  entry points (`fdb-gateway`, `fke-server`, `forge-cli`)
- `clippy::pedantic` + `-D warnings` — the CI gate
- `#[non_exhaustive]` on all public enums; `#[repr(transparent)]` newtype IDs
- `tracing` spans across every port boundary
- No file over 500 lines — split into directory modules
- **Never log** JWT payloads, claims, relation tuples, or tenant identifiers

---

## Repository layout

```
crates/            All workspace + pgrx crates (fdb-*, fke-*, forge-*, ext-flint-*)
examples/          hello-component (sample WASM component)
wit/               WASM Component Model interface definitions (WIT)
migrations/        SQL migrations (Flint A2UI schema, triggers, SDK extensions)
docs/              RFC-FORGE-001 spec, phase plans, A2UI specs, competitive analysis
deploy/            Production deployment artifacts (Helm chart)
openspec/          Phased OpenSpec change sets (proposal.md + tasks.md per change)
scripts/           ci-check.sh, ci-test.sh, seed SQL
.kbd-orchestrator/ KBD process state (phases, plans, progress) — tracked, source of truth
```

---

## Phased build

Development proceeds as phased [OpenSpec](openspec/) change sets under
`openspec/changes/`. Each change maps to one `proposal.md` + `tasks.md`. Phases halt for
approval. Broad phase map:

- **P0** — workspace foundation, PG18 image, WIT contract, fabric `WatchEntityType`
- **P1** — Flint Anvil: `flint_auth`, `flint_hooks`, `flint_vault`, `flint_meta` (reflection
  cache, triggers, functions), JWT contract pin
- **P2** — Flint Quarry reflection engine: `fdb-auth`, `fdb-postgres`, `flint-reflection`
  REST compiler, `ArcSwap` hot-reload, pgvector RPC, OpenAPI compiler
- **P3** — GraphQL hybrid (passthrough + subscriptions + introspection merge), Keto sync,
  Cedar policy, full RLS CRUD handlers, gate tests
- **P5** — Flint A2UI registry: schema, component seeds, embeddings pipeline, REST API,
  protocol surfaces, React/Flutter/HTMX SDKs
- **P6/P7** — Kiln runtime hardening, signing/capability stores, AG-UI/MCP/A2A protocol surfaces
- **P8–P13** — SDK completeness, production launch, API stability, v1 release, continuous operations
- **P15** — v1.0 production-readiness gap closure: Anvil extension stabilization, migration
  integrity, operator CLI, E2E/performance validation, docs + Helm chart *(active phase)*

Start reading at `openspec/changes/p0-c001-workspace`.

The KBD orchestrator state (`.kbd-orchestrator/`, `.prometheus/`) is the durable,
file-based source of truth for process state and is committed to the repo.

---

## License

MIT © Prometheus AGS. See [LICENSE](LICENSE).
