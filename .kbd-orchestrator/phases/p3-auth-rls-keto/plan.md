# Plan — p3-auth-rls-keto

**Date:** 2026-07-01
**Planner:** kbd-plan (opencode)
**Assessment:** `.kbd-orchestrator/phases/p3-auth-rls-keto/assessment.md` (Complete, 2026-07-01)
**Change backend:** OpenSpec (detected at project root)
**Phase gate:** all four auth layers live end-to-end (Keto cache + Postgres RLS + Cedar + JWT context), zero plaintext credentials in logs, parameterized CRUD SQL.

---

## Change ID Namespace

The existing `openspec/changes/p3-c001*` … `p3-c009*` folders belong to the prior
`p3-graphql-hybrid-engine` plan (mostly pre-certified DONE — see assessment).
To avoid ID collision and keep ordering unambiguous, this phase uses
**`p3-c010` … `p3-c018`**. Pre-certified work is not re-planned; it is listed
under *Pre-certified (skip)* below for traceability.

## Pre-certified (skip — already shipped, no change folder)

| Prior ID | Verdict |
|---|---|
| p3-c001-graphql-passthrough | DONE — `handle_graphql_query()` in gateway |
| p3-c004-graphql-transport-ws | DONE — `graphql_ws_handler()` |
| p3-c006-keto-sync | DONE — `keto_sync.rs` (254 lines + unit tests) |
| p3-c008-extended-guc-propagation | DONE — 6 SET LOCAL GUCs in `fdb-postgres::acquire()` |

## Open Questions Carried Forward

| OQ | Gates | Resolution path |
|---|---|---|
| OQ-FRF-1 | G7 live path, G5 live delivery | Defer real `WatchEntityType`; p3-c017 ships production stub |
| OQ-3 | G4 passthrough confidence | p3-c018 pre-flight: `SELECT extversion … pg_graphql` |
| OQ-cedar | G1 | Resolved below — pin `cedar-policy = "4"` (latest `4.11.2`) |
| OQ-cedar-table | G1 | Resolved below — `flint_meta.cedar_policies` does NOT exist; p3-c012 adds it |

---

## Ordered Change List

| # | Change | Goal | Size | Recommended Agent | Blocked? |
|---|---|---|---|---|---|
| 1 | `p3-c010-mount-reflection-router` | unblock REST testing; CRUD 404s today | SMALL | opencode | no |
| 2 | `p3-c011-ketocheck-port-trait` | G2 | MEDIUM | opencode | no |
| 3 | `p3-c012-forge-policy-cedar` | G1 | LARGE | opencode | no |
| 4 | `p3-c013-rest-handle-list` | G3 (list) | LARGE | opencode | no |
| 5 | `p3-c014-rest-handle-mutations` | G3 (insert/update/delete) | MEDIUM | opencode | after c013 |
| 6 | `p3-c015-gate-tests-rest-and-vault` | G6 (tests 1+2) | MEDIUM | opencode | after c013/c014 |
| 7 | `p3-c016-gate-tests-mocks` | G6 (tests 3+4) | MEDIUM | opencode | after c011 |
| 8 | `p3-c017-fdb-realtime-stub` | G7 (production stub) | MEDIUM | opencode | conditional on OQ-FRF-1 (ships stub regardless) |
| 9 | `p3-c018-introspection-merge-verify` | G4 confidence | SMALL | opencode | OQ-3 pre-flight gates scope |

**Total: 9 changes** (8 unconditional + 1 OQ-conditional scope).

## Ordering Rationale

1. **c010 first** because every REST integration test is blocked while the
   reflection router is unmounted. Pure wiring change, smallest blast radius,
   unblocks c013/c014/c015 verification.
2. **c011 before c012** because the `KetoCheck` port trait is the canonical
   hexagonal seam; landing it first establishes the injection pattern that the
   Cedar engine (c012) will mirror for `Pep`.
3. **c012 (Cedar) before c013/c014 (CRUD bodies)** so that mutation handlers
   can wire `Pep::check()` and `KetoCheck::check()` in the same pass rather
   than retrofitting.
4. **c013 → c014 → c015** is a strict chain: list handler defines the
   `is_safe_identifier()` and operator-dispatch utilities that insert/update/
   delete reuse; the gate tests then exercise the full surface.
5. **c016** only depends on c011 (KetoCheck trait) and mocks — parallelizable
   with c013/c014 in principle, but sequenced after c011 for clarity.
6. **c017** ships the production stub regardless of OQ-FRF-1; the reconnect
   loop and service-token auth are valuable independent of the upstream RPC.
7. **c018 last** because the OQ-3 pre-flight may re-scope it to a stub-only
   change; running it last avoids blocking the phase gate on an external
   image dependency.

## Library / Dependency Adds (constraint WARN → justified)

| Add | Version | Justification |
|---|---|---|
| `cedar-policy` (c012) | `"4"` (current 4.11.2) | Core to G1; no in-workspace alternative. Pin major; avoid `cedar-policy-core` internals. |
| `flint_meta.cedar_policies` table (c012) | new SQL migration | Spec-mandated policy store. Loaded via privileged pool, never RLS pool. |

All other deps are already in `[workspace.dependencies]` (tonic, async-graphql, sqlx, reqwest, deadpool-postgres, arc-swap).

## Hexagonal Seams Established This Phase

- `KetoCheck` async trait → `fdb-ports` (c011) — implemented by `fdb-gateway::keto_sync::KetoCacheClient`, injected into `Quarry` at composition time. **Never** imported from `fdb-app`.
- `Pep` already in `forge-policy`; `CedarPolicyEngine` (c012) is the concrete impl, injected at composition time. Policy **loading** uses the privileged pool; **evaluation** is in-process.
- `ChangeStreamSource` already in `fdb-ports`; c017 production stub lives in `fdb-realtime` and is the only adapter.

## Security Gates (non-negotiable, enforced by c015 + c016)

1. `is_safe_identifier()` called on EVERY table/column name before SQL interpolation (c013, c014).
2. `test_rest_select_with_eq_filter` covers all 12 operators AND injection attempts (c015).
3. `CompiledState` serde must not emit `vault_key` or plaintext DEK (c015).
4. Subscription `rls_requery` drops events for rows the subscriber cannot see (c016, mocked).
5. Mutation use-case returns 403 when `KetoCheck::check()` is `false` (c016).
6. Cedar policy load failure → `Decision::Deny` (fail closed) (c012).
7. Zero logging of JWT payloads, claims, relation tuples, tenant IDs (every change).

## Phase Gate Criteria

Phase 3 is **complete** when:

- ✅ All 9 changes are `qa_passed` (c017 may ship stub-only if OQ-FRF-1 unresolved; c018 may ship verify-only if OQ-3 unresolved — both require explicit handoff skip note).
- ✅ `cargo check --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace` all green.
- ✅ The four gate tests in c015 + c016 pass.
- ✅ `./scripts/ci-check.sh` green.
- ✅ A single end-to-end demo script exists (in c016 or handoff) showing: real JWT → RLS row filter → Keto gate → Cedar gate → parameterized CRUD SQL → no secret in logs.

## First Change to Apply

`/kbd-apply p3-c010-mount-reflection-router`
