# p17-c007 — OPTIONAL: route catch-all delegate (flips restartRequired)

**Phase:** p17-schema-provisioning
**Priority:** Round 6, OPTIONAL (plan D-P7) — depends on c004, c006 green
**Scope:** `crates/fdb-gateway/src/bootstrap.rs`, `rls_layer.rs` path, benchmark
**Source:** FFS-001 §8 Phase 5, D8; bootstrap.rs:298-300 note;
`crates/fdb-gateway/tests/mounts_reflection_router.rs` pins current behavior

## Problem

Reflection REST routes are mounted once at startup; a table provisioned via
/apply 404s until restart. v1 discloses restartRequired:true. This change
implements the catch-all delegate and flips the flag — ONLY if the benchmark
holds.

## What to build

- Replace one-time `.merge(reflection_router)` with a fallback handler that
  loads `state_manager.current().router` per unmatched request and delegates
  via tower::Service.
- Preserve the require_rls layer on the delegated path — RLS enforcement must
  be byte-identical to mounted-path behavior.
- Update mounts_reflection_router.rs DELIBERATELY (it pins the old behavior).
- Benchmark (measurable ship criterion, plan.md): p50/p99 on (a) an
  already-mounted route and (b) a delegated reflection route, before vs after,
  ≥1000 requests each, same build/host. SHIP only if mounted p99 regression
  <5% AND delegated added overhead <1ms p99. Record numbers in
  verification.md.
- Flip restartRequired to false ONLY when shipping; otherwise close this
  change as SKIPPED-with-evidence (recorded numbers) and keep the honest flag.

## Constraints
- "An honest flag beats a slow gateway" — FFS-001 §8 Phase 5 gate.
