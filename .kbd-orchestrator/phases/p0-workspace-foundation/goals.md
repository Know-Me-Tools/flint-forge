# Goals — p0-workspace-foundation

Four parallel change sets that together establish the compiling, deployable scaffold for Flint Forge.
All must be green before Phase 1 work begins.

- **c001** — Establish the Cargo workspace with hexagonal crate graph; `cargo check` green for all non-pgrx crates; `forge-domain`, `forge-identity` (RlsContext, Option-3 outbound), `forge-policy` (Cedar PEP trait) types in place; `/healthz` stubs in `fdb-gateway` and `fke-server`; CI: fmt + clippy::pedantic.
- **c002** — Build and validate the Postgres 18 Docker image carrying pgvector, pg_net, pg_graphql, pgcrypto, and all three Flint Anvil extensions (`flint_auth`, `flint_hooks`, `flint_llm`); `wal_level=logical`; boot assertion fails fast on any missing extension.
- **c003** — Freeze the `flint:host@0.1.0` WIT contract (`wit/flint/host/world.wit`) with interfaces `db`, `llm`, `kv`, `identity`, `secrets` and WASI 0.2 `edge-function` world; a trivial Rust component targeting it must compile via `wasm-tools`/`wit-bindgen`.
- **c004** (cross-repo: flint-realtime-fabric) — Add `WatchEntityType` server-streaming RPC to `proto/flint/v1/entity.proto` with Keto coarse gate; gates Phase 3 Quarry subscriptions.

## Success Criteria

- `cargo check --workspace` exits 0 (non-pgrx crates).
- `cargo clippy --workspace -- -D warnings` exits 0.
- Postgres 18 image starts; all extensions present; `SHOW wal_level = logical`.
- `wasm-tools component wit wit/flint/host/world.wit` resolves without error.
- `WatchEntityType` RPC compiles in `flint-realtime-fabric` and Keto rejects unauthorized pairs.
