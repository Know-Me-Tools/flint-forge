-- Migration: 0015_flint_schema_provisioning.sql
-- FFS-001 — ledger and dedicated role for the schema-provisioning API.
--
-- Two things land here and nothing else: an append-mostly audit ledger, and a
-- non-owner role that may CREATE only inside operator-allowlisted namespaces.
-- The role deliberately does NOT get BYPASSRLS and is NOT a table owner: the
-- caller's authority (a service_role JWT) and the database's authority
-- (flint_provisioner) are separate blast radii by design.
--
-- Depends on: 0013_force_rls (FORCE RLS convention), 0014_service_role_bypassrls.
-- Idempotent: IF NOT EXISTS guards throughout; the DO block is a no-op when the
-- role already exists.
--
-- NOT self-enabling: creating the role grants nothing. An operator must also set
-- FLINT_PROVISION_NAMESPACES and GRANT CREATE per namespace (see
-- docs/runbook.md §14 — Schema Provisioning).

CREATE SCHEMA IF NOT EXISTS flint_schema;

CREATE TABLE IF NOT EXISTS flint_schema.provision_ledger (
    plan_id        text        PRIMARY KEY,
    plan_hash      text        NOT NULL,
    namespace      text        NOT NULL,
    spec           jsonb       NOT NULL,
    generated_ddl  text        NOT NULL,
    status         text        NOT NULL CHECK (status IN ('planned','applied','failed')),
    applied_by     text,                      -- JWT `sub`. Never the raw bearer.
    applied_at     timestamptz,
    version_before bigint,
    version_after  bigint,
    error_code     text,                      -- SQLSTATE only, never the statement
    created_at     timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS provision_ledger_hash_applied_idx
    ON flint_schema.provision_ledger (plan_hash)
    WHERE status = 'applied';                 -- makes apply idempotent at the DB

-- Role first: the ledger policy below names it, so it must exist before the
-- policy is created on a fresh database.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'flint_provisioner') THEN
        CREATE ROLE flint_provisioner NOLOGIN;
    END IF;
END
$$;

ALTER TABLE flint_schema.provision_ledger ENABLE ROW LEVEL SECURITY;
ALTER TABLE flint_schema.provision_ledger FORCE ROW LEVEL SECURITY;

-- Deviation from FFS-001 §5 (recorded in the p17 plan): FORCE RLS with zero
-- policies is default-deny for every non-BYPASSRLS role — including
-- flint_provisioner, whose SELECT/INSERT/UPDATE grants below would otherwise
-- be dead letters and every ledger write would fail at runtime. This policy
-- opens the ledger to flint_provisioner alone; authenticated/anon/agent still
-- have no grant and no policy, and service_role passes via BYPASSRLS (0014).
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies
        WHERE schemaname = 'flint_schema'
          AND tablename  = 'provision_ledger'
          AND policyname = 'provision_ledger_provisioner_all'
    ) THEN
        CREATE POLICY provision_ledger_provisioner_all
            ON flint_schema.provision_ledger
            TO flint_provisioner
            USING (true)
            WITH CHECK (true);
    END IF;
END
$$;

GRANT USAGE ON SCHEMA flint_schema TO flint_provisioner;
GRANT SELECT, INSERT, UPDATE ON flint_schema.provision_ledger TO flint_provisioner;
