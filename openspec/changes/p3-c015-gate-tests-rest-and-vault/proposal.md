# p3-c015 — Gate Tests: REST Filter Safety + Vault DEK Serde

## Change ID
`p3-c015-gate-tests-rest-and-vault`

## Phase
`p3-auth-rls-keto`

## Goal Mapping
**G6 (tests 1 + 2)** — the two P2-carried security gates.

## Problem
Two non-negotiable security gates have been deferred since P2:
1. `test_rest_select_with_eq_filter` (column-name SQL injection validation)
2. `test_vault_dek_not_in_compiled_state` (DEK serde leak gate)

## Scope
- `test_rest_select_with_eq_filter`:
  - Cover ALL 12 filter operators (eq/neq/gt/gte/lt/lte/like/ilike/in/is/cs/cd)
  - Add injection-attempt cases: `; DROP TABLE`, `--`, `UNION SELECT`, `' OR '1'='1`, `pg_sleep`, identifier with whitespace, over-length identifier
  - Assert `is_safe_identifier()` rejects every injection attempt
  - Assert generated SQL round-trips via `sqlx::query` dry-run or mock executor
- `test_vault_dek_not_in_compiled_state`:
  - Serialize a populated `CompiledState` to JSON via serde
  - Assert NO field name contains `vault_key`, `dek`, `master_key`, or any plaintext key material
  - Assert `Debug` render also redacts (best-effort: scan `format!("{:?}", state)`)
- These tests live alongside the code they exercise (`fdb-reflection/tests/`
  and `fke-runtime/tests/` or wherever `CompiledState` lives — verify).

## Out of Scope
- Subscription / Keto mock tests (c016).
- Live integration tests requiring PG18 container (mark `#[ignore]` with OQ-9 note).

## Acceptance Criteria
- [ ] `test_rest_select_with_eq_filter` exists, covers 12 operators + ≥6 injection vectors, passes
- [ ] `test_vault_dek_not_in_compiled_state` exists, passes, asserts no DEK field in serde JSON
- [ ] Both tests run in default `cargo test` (no `#[ignore]` unless gated by OQ-9 with explicit comment)
- [ ] `cargo test --workspace` green
