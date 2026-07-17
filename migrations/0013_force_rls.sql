-- Migration: 0013_force_rls.sql
-- p16-c001: FORCE ROW LEVEL SECURITY on every RLS-governed table Flint Forge's
-- own migrations own. `ENABLE ROW LEVEL SECURITY` alone does not apply to the
-- table owner (or a superuser) — only `FORCE` closes that gap, so RLS still
-- holds even if a future code path acquires a connection without correctly
-- de-escalating via `SET LOCAL ROLE` (see `fdb-postgres::PgBackend::acquire`,
-- the primary enforcement mechanism this migration backs up in depth).
--
-- Scope: this repo's own internal tables only (flint_a2ui.*, flint_kiln.*).
-- Tenant/operator-created tables are outside flint-forge's migration
-- ownership; operators MUST apply `FORCE ROW LEVEL SECURITY` to their own
-- RLS-governed tables (documented in docs/runbook.md).

ALTER TABLE flint_a2ui.components FORCE ROW LEVEL SECURITY;
ALTER TABLE flint_a2ui.events FORCE ROW LEVEL SECURITY;
ALTER TABLE flint_a2ui.component_overrides FORCE ROW LEVEL SECURITY;
ALTER TABLE flint_kiln.cedar_policies FORCE ROW LEVEL SECURITY;
