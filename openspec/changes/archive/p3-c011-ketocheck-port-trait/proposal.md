# p3-c011 — KetoCheck Port Trait + fdb-app Use-Case Wiring

## Change ID
`p3-c011-ketocheck-port-trait`

## Phase
`p3-auth-rls-keto`

## Goal Mapping
**G2** — Keto coarse relationship check at subscribe-time and mutation-time,
cached with TTL, invalidated on Keto webhook.

## Problem
`keto_sync.rs` (in `fdb-gateway`) holds a working `KetoCacheClient` and
`cache_check()`. `fdb-app` use-cases do not call it, and importing
`fdb-gateway` from `fdb-app` is a hexagonal **BLOCK** (constraints.md).

## Scope
- Define `#[async_trait] trait KetoCheck` in `fdb-ports`:
  `async fn check(&self, subject: &str, relation: &str, object: &str) -> bool;`
  Fail-closed semantics documented on the trait.
- Implement `KetoCacheAdapter` in `fdb-gateway::keto_sync` wrapping
  `cache_check()`; inject `Arc<dyn KetoCheck>` into `Quarry` (and any
  mutation use-case struct) at composition time.
- Call `KetoCheck::check()` in mutation use-cases before delegating to the
  REST executor. Return `403 Forbidden` (typed app error) when `false`.
- Add `KetoCheck` mock for unit tests in `fdb-app`.
- Constraint: never log the relation tuple or `subject` value at any level
  (constraint: BLOCK on logging relation tuples).

## Out of Scope
- Cedar integration (c012).
- CRUD handler bodies (c013, c014).
- TTL/invalidation tuning beyond what `keto_sync.rs` already does.

## Acceptance Criteria
- [ ] `KetoCheck` trait lives in `fdb-ports`, no adapter imports there
- [ ] `fdb-app` mutation use-case has `Arc<dyn KetoCheck>` injected
- [ ] Mutation use-case returns 403 on `check() == false`
- [ ] No relation tuple / subject string appears in any `tracing` span
- [ ] `cargo check --workspace` + clippy + `cargo test -p fdb-ports -p fdb-app` green
