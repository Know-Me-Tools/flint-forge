-- Cedar policy bundles — loaded by forge-policy's CedarPolicyEngine.
-- Loaded via the PRIVILEGED pool (service_role) — never exposed to RLS.
CREATE TABLE IF NOT EXISTS flint_meta.cedar_policies (
    id          text        NOT NULL,
    name        text        NOT NULL,
    policy_text text        NOT NULL,
    enabled     boolean     NOT NULL DEFAULT true,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (id)
);

GRANT ALL ON flint_meta.cedar_policies TO service_role;
