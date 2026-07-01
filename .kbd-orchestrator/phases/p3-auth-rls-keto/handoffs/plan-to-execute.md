# Plan → Execute Handoff — p3-auth-rls-keto

**From:** kbd-plan (opencode)
**Date:** 2026-07-01
**Phase:** p3-auth-rls-keto

## Summary

9 ordered changes planned (`p3-c010` … `p3-c018`). 7 are MVP-unconditional;
2 are OQ-conditional (`p3-c017` on OQ-FRF-1, `p3-c018` on OQ-3) and ship
scoped-down versions regardless.

## Ordering rationale

c010 first (unblocks all REST testing — CRUD currently 404s). c011 before
c012 to establish the hexagonal injection seam (`KetoCheck` in `fdb-ports`)
that Cedar (`Pep`) mirrors. c013 → c014 → c015 is a strict chain: list
handler defines `is_safe_identifier()` and operator dispatch that mutations
reuse, then gate tests exercise the surface. c016 only needs c011 + mocks.
c017 and c018 last — both ship stubs/verify-scopes if their OQ is unresolved.

## First change to apply

`/kbd-apply p3-c010-mount-reflection-router`

## Key invariants for the executor

1. **Hexagonal seam:** `KetoCheck` trait → `fdb-ports`; `Pep` already in
   `forge-policy`. Both injected at the gateway composition root, never
   imported by `fdb-app` concretely.
2. **Identifier safety:** `is_safe_identifier()` is the single chokepoint —
   no SQL interpolation without it. Enforced by `test_rest_select_with_eq_filter` (c015).
3. **Fail closed:** Cedar load/eval failure → `Decision::Deny`. Keto unreachable → 403.
4. **No logging** of JWT payloads, claims, relation tuples, or tenant IDs (constraint BLOCK).
5. **Subscription RLS re-query** is non-negotiable; events for rows the
   subscriber cannot see are silently dropped, never errored (constraint WARN).

## Carry-forward from P2

- `test_rest_select_with_eq_filter` → c015 deliverable
- `test_vault_dek_not_in_compiled_state` → c015 deliverable
