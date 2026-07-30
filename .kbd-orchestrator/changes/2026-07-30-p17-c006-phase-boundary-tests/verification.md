# Verification — p17-c006

## Gate
Tenant B cannot see tenant A's row on a provisioned table; old key dies on
rotation; suite + clippy green; coverage gate reconciled with a recorded
decision.

## Evidence to record on completion
- [ ] Coverage decision recorded (wired | concession documented)
- [ ] Isolation test name + green run date
- [ ] Rotation test transcript or CI run
- [ ] OpenAPI hot-swap test green
- [ ] Suite run count + clippy green date (Base Rule #18 record)

## t1 coverage-gate decision (2026-07-30) — WIRED, not conceded
The p16-c009 coverage gate ran in the no-database `rust` CI job, so
DATABASE_URL-gated tests read as uncovered — it would fail any change whose
tests are inherently DB-bound. RESOLUTION: gate moved to the `integration`
job (ci.yml), measured with the live flint-forge-pg:18 container and
DATABASE_URL set, llvm-tools + fetch-depth added there. Gated tests now
count toward the >=90% changed-crate threshold.

## t3 rotation-test note
The test must not rotate the operator's real keys itself, so it gates on
FLINT_OLD_SERVICE_ROLE_KEY (exists only post-rotation) and documents the
4-step manual rotation procedure in its doc comment. New-key-passes +
old-key-401 both asserted when armed.

## Phase-boundary run record (Base Rule #18) — 2026-07-30
Environment: local flint-forge-pg:18 container (port 54317), migrations
0002–0015 + a2ui seed applied, Sansaba JWKS served at 127.0.0.1:8917,
real FLINT_SERVICE_ROLE_KEY/FLINT_ANON_KEY, iss/aud=flint-forge.

| Run | Scope | Result | Outcome |
|---|---|---|---|
| 1 | cargo test --workspace | 2 FAILED (new e2e) | Found a REAL bug: generated `CREATE SCHEMA IF NOT EXISTS` fails 42501 for flint_provisioner even on existing schemas (no database CREATE). Fix: generator never emits CREATE SCHEMA (operator owns schema lifecycle, FFS-001 §4.2 example loses to its own §10); routes gate on new port method schema_exists() with 409. |
| 2 | fix cycle (3 crates) | e2e green; 1 FAILED | Exposed rest_router_extraction — documented in main-health-2026-07-29.md as NEVER observed passing live. Pre-existing: fails identically on unmodified main against the same DB. Fixture repaired (grants for authenticated; test is about path extraction, not permissions). |
| 3 | workspace --no-fail-fast | 610 passed / 1 FAILED | rest_typed_columns_live_pg — also never run live; stale exact-SQL expectations predating filters.rs's documented deliberate `$n::text::<type>` cast. Test corrected to implementation's documented intent. |
| 4 | workspace (final) | **611 passed / 0 failed / 6 ignored** | fmt clean; clippy pedantic workspace clean. |
| 5 | split-test confirmation | green | two >100-line test fns split for the lint cap; behavior unchanged. |

Gate items:
- [x] Coverage decision: WIRED (gate moved to integration CI job with live DB)
- [x] Two-tenant isolation on a provisioned table: provisioned_table_isolates_two_tenants green (tenant B sees nothing; forged-tenant INSERT violates WITH CHECK; two-sided)
- [x] OpenAPI hot-swap: provisioned_table_appears_in_openapi_without_restart green (listener + watch channel, <30s)
- [~] Rotation: test written + armed on FLINT_OLD_SERVICE_ROLE_KEY (must not rotate the operator's real keys itself); manual 4-step procedure documented in the test doc comment — evidence lands on first real rotation
- [x] Suite + clippy green; run count = 5, recorded here and in progress.json
