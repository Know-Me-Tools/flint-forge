# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

---

## What This Repo Is

**Flint Forge** is the sovereign data and edge-compute plane of the Flint platform (RFC-FORGE-001). It is a Rust workspace containing three deliverables:

| Subsystem | Crate prefix | Role |
|---|---|---|
| **Flint Quarry** | `fdb-*` | REST + GraphQL DB API gateway over Postgres 18 |
| **Flint Anvil** | `ext-flint-*` | pgrx extensions: `flint_auth`, `flint_hooks`, `flint_llm` (Ember), `flint_vault` |
| **Flint Kiln** | `fke-*` | Polyglot WASM component edge-function gateway |
| **Forge core** | `forge-*` | Shared domain types, identity, policy |

> **Status:** Scaffold. Ports and module structure are in place; bodies are stubbed with `todo!()`. First full build needs dependency-version reconciliation (see spec §8).

The spec lives at `docs/FLINT-FORGE-SPEC.md` (RFC-FORGE-001). Phased change sets live under `openspec/changes/` — each maps to one `proposal.md` + `tasks.md`. Start with `p0-c001-workspace`.

---

## Build Commands

```bash
# Check all workspace crates (non-pgrx)
cargo check

# Run Quarry gateway
cargo run -p fdb-gateway

# Run Kiln server
cargo run -p fke-server

# Run all tests
cargo test

# Run tests for a single crate
cargo test -p fdb-app

# Lint (pedantic, -D warnings is the CI gate)
cargo clippy --workspace -- -D warnings

# Format
cargo fmt --all

# Type-check without build artifacts
cargo check --workspace
```

### pgrx Extensions (Flint Anvil)

The `ext-flint-*` crates are **excluded from the default workspace** because they require a Postgres toolchain. Build them separately:

```bash
cargo install cargo-pgrx && cargo pgrx init
cargo pgrx run -p ext-flint-auth    # targets Postgres 17 (pgrx 0.12)
cargo pgrx run -p flint_vault       # targets Postgres 18 (pgrx 0.18.1)
```

Note: `ext-flint-auth` pins `pgrx = "0.12"` (pg17); `flint_vault` pins `pgrx = "=0.18.1"` (pg18). These differ intentionally — do not unify without reconciling pgrx version support.

### CI Script

```bash
./scripts/ci-check.sh
```

---

## Workspace Architecture

### Hexagonal Dependency Rule

The strict layering — enforced at Cargo dependency level — is:

```
forge-domain          ← Layer 0: pure types, serde only, zero infra deps
  ↑
forge-ports / *-app   ← Layer 1: trait seams (ports) + use-cases
  ↑
adapters              ← fdb-postgres, fdb-realtime, fke-store-*, fke-sign-*, …
  ↑
interface crates      ← fdb-gateway, fke-server  (the only crates that import concrete adapters)
```

**Domain and app crates must never import adapter crates.** Composition happens only in interface crates.

### Crate Map

**Quarry (`fdb-*`)**
- `fdb-domain` — domain types: `TableMeta`, `RestQuery`, `RestResult`, `ChangeEvent`, `RlsContext`
- `fdb-ports` — five async traits: `DatabaseBackend`, `SchemaProvider`, `RestExecutor`, `GraphQlExecutor`, `ChangeStreamSource`
- `fdb-app` — use-cases: REST execution, GraphQL execution, subscription orchestration, RLS assembly
- `fdb-postgres` — adapter implementing all four Quarry ports via deadpool-postgres + pg_graphql passthrough
- `fdb-realtime` — `ChangeStreamSource` adapter; gRPC client of `flint-realtime-fabric`'s `WatchEntityType`
- `fdb-auth` — JWT verify (flint-gate issuer/JWKS) → `RlsContext` builder
- `fdb-gateway` — Axum 0.8.8 composition root: `/graphql`, REST routes, `/rpc`, `/healthz`

**Kiln (`fke-*`)**
- `fke-domain` — WASM component domain types
- `fke-ports` — store, signer, registry port traits
- `fke-runtime` — Wasmtime component host (shared substrate with UAR Tier-2)
- `fke-store-{oci,ipfs,s3,fs}` — four ComponentStore adapters
- `fke-sign-{did,cosign}` — two signer adapters
- `fke-registry` — component registry
- `fke-server` — Axum composition root: admin REST (control plane), `/functions/v1/<name>` (data plane)

**Anvil (`ext-flint-*`)** — pgrx extensions (workspace-excluded)
- `ext-flint-auth` — `auth` schema helpers: `auth.jwt()`, `auth.uid()`, `auth.role()`, `auth.bearer()`
- `ext-flint-hooks` — webhook dispatch: `flint.webhooks`, `flint.webhook_outbox`, `flint.dispatch_webhook()`
- `ext-flint-llm` — Flint Ember: in-DB LLM/embeddings via liter-llm gateway
- `ext-flint-vault` — Flint Vault: XChaCha20-Poly1305 encrypted secret store, KMS-wrapped DEK

**Forge core**
- `forge-domain` — zero-infra shared types
- `forge-identity` — identity primitives
- `forge-policy` — Cedar policy evaluation
- `forge-cli` — `flint-forge` CLI binary

---

## Critical Design Contracts

### JWT / RLS Context (§2.2)

Every pooled connection sets three `SET LOCAL` statements per request transaction before any user statement:
```sql
SET LOCAL ROLE authenticated;
SET LOCAL "request.jwt.claims" = '{"sub":…,"role":…,"tenant_id":…}';
SET LOCAL "request.headers"    = '{"authorization":"Bearer <raw-jwt>"}';
```
- **Claims → RLS** via `flint_auth` helpers (`auth.uid()`, `auth.role()`, etc.)
- **Raw token → outbound forwarding** by `flint_hooks`/`flint_llm` via `auth.bearer()`
- Postgres **never verifies** JWT signatures; flint-gate does that upstream

### Four Auth Layers (§2.3)

1. **Kratos** — authentication (flint-gate, per session)
2. **Keto** — coarse relationship check (subscribe-time, cached)
3. **Postgres RLS** — authoritative row filter (every query / subscription event)
4. **Cedar** — action/capability policy (mutations, Kiln linker, Ember model-use)

### GraphQL Hybrid (§3.2)

- **Query/Mutation:** delegated directly to `graphql.resolve()` inside Postgres under RLS — async-graphql is **not** in this path
- **Subscription:** async-graphql `GraphQLSubscription` over `graphql-transport-ws`; resolvers pull from `ChangeStreamSource`
- **Introspection:** merged union of pg_graphql schema ∪ sibling subscription SDL

### Subscription RLS Enforcement (§3.3)

WAL bypasses RLS. For each `EntityChange` from the fabric, Quarry re-queries the changed row as the subscriber with full RLS context to confirm visibility before delivering. This is non-negotiable. Predicate-pushdown optimization exists but is off by default (operator-accepted data-leak risk).

### Two Convergence Invariants (§2.4)

1. One in-transaction capture (`record + origin JWT from request.headers`), two consumers: `flint_hooks` (webhooks) and `flint_llm` (LLM jobs). The durable outbox tier is shared.
2. One Wasmtime component host (`fke-runtime`), two surfaces: Flint Kiln (HTTP-triggered) and UAR Tier-2 WASM skills.

---

## Quality Gates (CI-Enforced)

These rules apply to all crates and are non-negotiable:

- **No `unwrap()`/`expect()` in library crates** — use `thiserror` in libs, `anyhow` only at binary entry points (`fdb-gateway`, `fke-server`, `forge-cli`)
- **`clippy::pedantic` + `-D warnings`** — the CI gate; workspace `[lints]` applies to all members
- **`#[non_exhaustive]`** on all public enums
- **Newtype IDs** as `#[repr(transparent)]` wrappers
- **`tracing` spans** across every port boundary
- **No file over 500 lines** — split into directory modules
- **Never log** JWT payloads, claims, relation tuples, or tenant identifiers
- **MSRV:** `1.85` (pinned in `rust-toolchain.toml` as `1.90` channel)

---

## Prometheus Base Rules

The following base rules govern all reasoning, coding, and file modification in this repository.

### 1. Think Before Coding
State assumptions explicitly. Surface tradeoffs before implementation. If uncertain, ask. If multiple interpretations exist, present them. Stop and ask when something is unclear.

### 2. Simplicity First
Write the minimum code that solves the problem. No speculative abstractions. No future-proofing not requested. If 50 lines solves the problem, do not write 200.

### 3. Surgical Changes
Touch only what is necessary. Do not refactor unrelated code. Do not reformat unrelated files. Match existing conventions. Mention unrelated issues; do not fix them unless asked.

### 4. Goal-Driven Execution
Define success criteria first. Convert vague requests into testable outcomes. Run tests where available. Do not stop at implementation — stop only when success criteria are satisfied.

### 5. Truth Over Fluency
Never prefer a confident answer over a correct answer. State uncertainty explicitly. Do not invent APIs, functions, files, packages, commands, or behavior.

### 6. Evidence Before Conclusions
Cite evidence where available. Show reasoning path. Explain tradeoffs. Explain why alternatives were rejected. Prefer source code, tests, and official docs over guesses.

### 7. Preserve User Intent
Do not substitute your own preferences. Do not silently expand or reduce scope. Clarify when requirements conflict. Preserve the architectural direction unless explicitly told otherwise.

### 8. Minimize Irreversible Actions
Confirm intent before destructive or hard-to-reverse actions. Prefer reversible approaches. Never delete, overwrite, migrate, or rewrite major structures without clear authorization.

### 9. Maintain Architectural Consistency
Follow the hexagonal dependency rule (§2.1). Follow existing naming conventions (`fdb-*`, `fke-*`, `forge-*`). Avoid introducing new frameworks without justification.

### 10. Architecture Before Code
Before implementation, identify affected subsystems, data flow, interface contracts, persistence impact, security impact, and testing strategy. Never start coding until the architecture is understood.

### 11. Open Standards First
This repo uses: MCP, OpenAI-compatible APIs (via liter-llm), WASM Component Model (WIT), OpenAPI, GraphQL, PostgreSQL, IPFS-compatible distribution. Avoid vendor lock-in unless explicitly required.

### 12. No Hidden State
Business state lives in Postgres, the outbox table, durable queues, or the WASM component registry. State must not be hidden in untracked globals, implicit caches, or agent-only memory.

### 13. Strong Typing Required
No implicit `Any`. No untyped business objects. No stringly-typed domain models when proper newtypes are possible. Use `#[repr(transparent)]` ID newtypes as established in forge-domain.

### 14. Tests Are Part of Completion
Implementation is not complete until verified. Run `cargo test`, `cargo clippy`, and `cargo check`. Add tests for new behavior. If tests cannot be run, state why.

### 15. Prefer Small, Reviewable Changes
Keep diffs small. Separate mechanical changes from behavioral changes. Respect the 500-line file limit — split into directory modules when approaching it.

### 16. Preserve Existing Behavior
Do not break existing behavior unless the task explicitly requires it. Call out breaking changes clearly.

### 17. Security Is Not Optional
Every port boundary sets the JWT context before any user statement. Never log JWT payloads, claims, or tenant IDs. Treat `flint_vault` DEK access as highest sensitivity. Never expose secrets via SQL or WASM sandbox.

### 18. Agent Actions Must Be Auditable
Record decisions, file changes, tool calls, and external effects. Agentic execution without auditability is not acceptable.

### 19. Human Override Always Exists
Every automated decision must support inspection, override, and manual correction. Agents assist but humans remain in control of critical outcomes.

### 20. Repo-Level Rules Override Base Rules Only When Explicit
Project-specific task instructions, architecture docs, or OpenSpec change sets may add stricter requirements. They override these base rules only when explicit and non-contradictory with safety and correctness.

---

## Relevant Rust Skills (skills.sh)

The following rust-skills are most applicable to this codebase and should be activated when working in relevant areas:

| Skill | Why It Applies |
|---|---|
| `rust-skills:m01-ownership` | Extensive `Arc<T>`, `ArcSwap`, pooled connections, WASM sandbox lifetimes |
| `rust-skills:m03-mutability` | `ArcSwap` hot-reload pattern in `SchemaRegistry`; shared state in async Axum handlers |
| `rust-skills:m05-type-driven` | `#[repr(transparent)]` newtype IDs, `#[non_exhaustive]` enums, port traits as type contracts |
| `rust-skills:m06-error-handling` | `thiserror` in libs, `anyhow` only at binary edges — strictly enforced |
| `rust-skills:m07-concurrency` | Tokio + async-trait ports, `BoxStream`, subscription fan-out, background workers |
| `rust-skills:domain-web` | Axum 0.8.8 handlers, `graphql-transport-ws`, REST route composition |
| `rust-skills:domain-cloud-native` | Dagger CI, OCI/IPFS component stores, gRPC fabric client (tonic), Postgres pooling |
| `rust-skills:axum-patterns` | Quarry and Kiln both use Axum as composition root |
| `rust-skills:async-patterns` | `async-trait` ports, `BoxStream` subscriptions, deadpool connection lifecycles |
| `rust-skills:unsafe-checker` | pgrx extensions use `unsafe` extensively; Wasmtime component host integration |
| `rust-skills:m04-zero-cost` | pgrx extension overhead is zero-cost by design; fuel/epoch limits in Wasmtime |
| `rust-skills:m09-domain` | Hexagonal architecture, port/adapter separation, domain-driven bounded contexts |
