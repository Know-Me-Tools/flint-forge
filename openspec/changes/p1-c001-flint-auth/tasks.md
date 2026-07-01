# p1-c001 — Tasks

- [ ] Read `crates/ext-flint-auth/sql/flint_auth.sql` and verify all four functions match JWT contract §3.2
- [ ] Add `auth.tenant_id()` function: `SELECT auth.jwt()->>'tenant_id'`
- [ ] Add schema lockdown SQL: `REVOKE ALL ON SCHEMA auth FROM PUBLIC; GRANT USAGE ON SCHEMA auth TO authenticated, anon, service_role`
- [ ] Add `GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA auth TO authenticated, anon, service_role`
- [ ] Write pgrx `#[pg_test]` for `auth.uid()`: set `request.jwt.claims = '{"sub":"user-1","role":"authenticated"}'`, assert `auth.uid() = 'user-1'`
- [ ] Write pgrx `#[pg_test]` for `auth.role()` with role claim present: assert returns `'authenticated'`
- [ ] Write pgrx `#[pg_test]` for `auth.role()` without role claim: assert fallback to `'anon'`
- [ ] Write pgrx `#[pg_test]` for `auth.bearer()`: set `request.headers = '{"authorization":"Bearer test-token"}'`, assert returns `'Bearer test-token'`
- [ ] Write pgrx `#[pg_test]` for `auth.tenant_id()`: assert returns correct value from claims
- [ ] Update `docs/contracts/jwt-contract.md` with §usage-notes: `role` claim must be in `additional_claims` per route hook
- [ ] Run `cargo pgrx test -p ext-flint-auth --features pg17` — all tests pass
- [ ] GATE: all pgrx tests pass; schema lockdown SQL validates

## Notes

- Use `Spi::run("SELECT set_config(..., ..., true)")` to set GUCs within pgrx tests
- The `set_config(name, value, is_local)` with `is_local = true` is equivalent to `SET LOCAL`
- pgrx test executor runs inside a transaction by default — GUC values are transaction-scoped
