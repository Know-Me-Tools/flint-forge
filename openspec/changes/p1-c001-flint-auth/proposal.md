# p1-c001 — flint_auth: `auth.*` SQL helpers + RLS contract (pgrx 0.12/pg17)

## Why

`auth.jwt()`, `auth.uid()`, `auth.role()`, and `auth.bearer()` are the GUC-backed vocabulary every RLS policy in the platform is written in. They must be tested end-to-end against the pinned JWT contract before any Phase 2 reflection engine consumes them.

## What

- Verify and harden the existing `ext-flint-auth/sql/flint_auth.sql` implementation against the JWT contract (`docs/contracts/jwt-contract.md`)
- Add pgrx integration tests that SET the three `request.*` GUCs and verify each function returns the correct value
- Add test for `auth.role()` fallback: when `role` claim is absent, returns `'anon'`
- Add schema security: `REVOKE ALL ON SCHEMA auth FROM PUBLIC`; `GRANT USAGE ON SCHEMA auth TO authenticated, anon, service_role`
- Add `auth.tenant_id()` function: `auth.jwt()->>'tenant_id'` — needed by multi-tenant RLS policies in Phase 2
- Document the `role` claim injection requirement in `docs/contracts/jwt-contract.md` §usage-notes

## Contract

`cargo pgrx test -p ext-flint-auth` passes all tests including:
- `auth.uid()` returns the `sub` claim value from `request.jwt.claims`
- `auth.role()` returns `'authenticated'` when `role` claim is set, `'anon'` when absent
- `auth.bearer()` returns the `authorization` value from `request.headers`
- Schema is locked down (no PUBLIC execute on auth functions)

## Out of scope

RLS policies on application tables (those are per-table in Phase 2). The auth *helpers* only.

## Constraints

- pgrx 0.12, pg17 features only — DO NOT upgrade to pgrx 0.18.1
- No `unwrap()` / `expect()` in any Rust code added (SQL-only functions are exempt)
- File size ≤ 500 lines

## Reference

- `docs/contracts/jwt-contract.md` §3 (GUC shape), §7 (RLS usage)
- `crates/ext-flint-auth/sql/flint_auth.sql` (existing implementation)
- CLAUDE.md §Critical Design Contracts: "Every pooled connection sets three SET LOCAL statements"
