-- Migration: 0009_flint_kiln_cedar_policies.sql
-- Adds Cedar policy table for the Kiln function gateway (p7b-c002).
-- Mirrors the schema of flint_meta.cedar_policies used by the Quarry,
-- namespaced under flint_kiln for operational separation.

CREATE TABLE IF NOT EXISTS flint_kiln.cedar_policies (
    id          uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    policy_text text        NOT NULL,
    enabled     boolean     NOT NULL DEFAULT true,
    description text,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now()
);

-- Only the service role may write policies.
ALTER TABLE flint_kiln.cedar_policies ENABLE ROW LEVEL SECURITY;

CREATE POLICY kiln_cedar_policies_read ON flint_kiln.cedar_policies
    FOR SELECT USING (true);

CREATE POLICY kiln_cedar_policies_write ON flint_kiln.cedar_policies
    FOR ALL USING (current_setting('role') = 'service_role');

-- Bootstrap: permit every kiln:invoke so the system works before an operator
-- authors scoped policies. Operators should replace or supplement this row with
-- publisher-specific grants, e.g.:
--   permit(
--     principal == KilnPublisher::"did:prometheus:<key>",
--     action == Action::"kiln:invoke",
--     resource == Resource::"kiln:functions"
--   );
INSERT INTO flint_kiln.cedar_policies (policy_text, description, enabled)
VALUES (
    'permit(principal, action, resource);',
    'bootstrap allow-all — replace with publisher-scoped policies',
    true
)
ON CONFLICT DO NOTHING;

-- Index for the common "load all enabled policies" query.
CREATE INDEX IF NOT EXISTS kiln_cedar_policies_enabled_idx
    ON flint_kiln.cedar_policies (enabled)
    WHERE enabled = true;
