# p3-c016 — Gate Tests: Subscription RLS Drop + Keto Mutation Gate (mocks)

## Change ID
`p3-c016-gate-tests-mocks`

## Phase
`p3-auth-rls-keto`

## Goal Mapping
**G6 (tests 3 + 4)** — both fully implementable with mocks, no OQ-FRF-1 dependency.

## Problem
Two more non-negotiable gates have no test files:
3. `test_subscription_rls_drops_unauthorized_events`
4. `test_keto_check_gates_mutation`

## Scope
- `test_subscription_rls_drops_unauthorized_events`:
  - Construct a mock `ChangeStreamSource` emitting `EntityChange` events for
    rows the subscriber's `RlsContext` should NOT see
  - Assert the subscriber stream yields zero events (silent drop, not error)
  - Use a mock RLS pool that returns zero rows for the re-query
  - No live PG, no OQ-FRF-1 dependency
- `test_keto_check_gates_mutation`:
  - `MockKetoCheck` (from c011) returns `false` for a specific (subject, relation, object)
  - Invoke the mutation use-case; assert 403 typed error
  - Positive path: `MockKetoCheck` returns `true`; mutation proceeds to mock executor
- Both tests live under `fdb-app/tests/` (or co-located with the use-case).

## Out of Scope
- Live subscription end-to-end (blocked on OQ-FRF-1).
- Cedar mutation gate test (covered by c012 unit tests; optional add here).

## Acceptance Criteria
- [ ] `test_subscription_rls_drops_unauthorized_events` exists, passes, uses mocks only
- [ ] `test_keto_check_gates_mutation` exists, passes, asserts 403 on deny
- [ ] Neither test requires live PG or FRF
- [ ] `cargo test --workspace` green
