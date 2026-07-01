# Flint Forge

Sovereign data & edge-compute plane of the Flint platform. See `docs/FLINT-FORGE-SPEC.md`
(and the branded `docs/FLINT-FORGE-SPEC.html`) for the full functional specification,
architecture, and development plan (RFC-FORGE-001).

## Subsystems
- **Flint Quarry** (`fdb-*`) — REST/GraphQL DB API gateway over Postgres.
- **Flint Anvil** (`ext-flint-*`) — pgrx extensions: auth context, webhooks, in-DB LLM (Ember).
- **Flint Kiln** (`fke-*`) — polyglot WASM component edge-function gateway.
- **Forge core** (`forge-*`) — shared domain, identity, policy.

## Build
```
cargo check                      # builds all non-pgrx crates
cargo run -p fdb-gateway         # Quarry gateway (stub)
cargo run -p fke-server          # Kiln server (stub)
```
The `ext-flint-*` extensions build separately:
```
cargo install cargo-pgrx && cargo pgrx init
cargo pgrx run -p ext-flint-auth
```

> Status: scaffold. Contracts and module structure are in place; bodies are stubbed with
> `todo!()`. First full build will need dependency-version reconciliation (see spec §8).

## Phased build
OpenSpec change sets live in `openspec/changes/`. Start with `p0-c001-workspace`.
