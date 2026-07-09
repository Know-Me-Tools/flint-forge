-- flint_auth: GUC-backed identity helpers. Verification stays upstream in flint-gate.
-- The `auth` schema is created and owned by the extension control file.

-- Application-facing JWT roles are created idempotently by this extension so
-- downstream extensions (meta, hooks, vault) can grant to them at install time.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'authenticated') THEN
        CREATE ROLE authenticated NOLOGIN;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'anon') THEN
        CREATE ROLE anon NOLOGIN;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'service_role') THEN
        CREATE ROLE service_role NOLOGIN;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'agent') THEN
        CREATE ROLE agent NOLOGIN NOINHERIT;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'authenticator') THEN
        CREATE ROLE authenticator NOLOGIN NOINHERIT;
    END IF;
END
$$;

ALTER ROLE anon NOINHERIT;
ALTER ROLE authenticated NOINHERIT;
ALTER ROLE service_role NOINHERIT BYPASSRLS;
ALTER ROLE agent NOINHERIT;

GRANT anon TO authenticator;
GRANT authenticated TO authenticator;
GRANT agent TO authenticator;
GRANT service_role TO authenticator;

ALTER ROLE anon SET statement_timeout = '3s';
ALTER ROLE authenticated SET statement_timeout = '30s';
ALTER ROLE agent SET statement_timeout = '15s';
ALTER ROLE service_role SET statement_timeout = '60s';

CREATE OR REPLACE FUNCTION auth.jwt() RETURNS jsonb LANGUAGE sql STABLE AS
$$ SELECT coalesce(current_setting('request.jwt.claims', true), '{}')::jsonb $$;

CREATE OR REPLACE FUNCTION auth.uid() RETURNS text LANGUAGE sql STABLE AS
$$ SELECT auth.jwt()->>'sub' $$;

CREATE OR REPLACE FUNCTION auth.role() RETURNS text LANGUAGE sql STABLE AS
$$ SELECT coalesce(auth.jwt()->>'role', 'anon') $$;

CREATE OR REPLACE FUNCTION auth.bearer() RETURNS text LANGUAGE sql STABLE AS
$$ SELECT current_setting('request.headers', true)::json->>'authorization' $$;

CREATE OR REPLACE FUNCTION auth.tenant_id() RETURNS text LANGUAGE sql STABLE AS
$$ SELECT auth.jwt()->>'tenant_id' $$;

CREATE OR REPLACE FUNCTION auth.agent_id() RETURNS text LANGUAGE sql STABLE AS
$$ SELECT auth.jwt()->>'agent_id' $$;

CREATE OR REPLACE FUNCTION auth.workflow_id() RETURNS text LANGUAGE sql STABLE AS
$$ SELECT auth.jwt()->>'workflow_id' $$;

CREATE OR REPLACE FUNCTION auth.principal_type() RETURNS text LANGUAGE sql STABLE AS
$$ SELECT coalesce(auth.jwt()->>'principal_type', 'User') $$;

CREATE OR REPLACE FUNCTION auth.is_service_role() RETURNS boolean LANGUAGE sql STABLE AS
$$ SELECT auth.role() = 'service_role' $$;

CREATE TABLE IF NOT EXISTS auth.api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key_hash TEXT NOT NULL UNIQUE,
    key_prefix TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('anon', 'authenticated', 'agent', 'service_role')),
    principal_type TEXT NOT NULL DEFAULT 'User' CHECK (principal_type IN ('User', 'Agent', 'Service')),
    scopes TEXT[],
    allowed_ips INET[],
    tenant_id UUID,
    created_by UUID,
    created_at TIMESTAMPTZ DEFAULT now(),
    expires_at TIMESTAMPTZ,
    last_used_at TIMESTAMPTZ,
    is_active BOOLEAN DEFAULT true
);

CREATE INDEX IF NOT EXISTS idx_auth_api_keys_hash ON auth.api_keys(key_hash);
CREATE INDEX IF NOT EXISTS idx_auth_api_keys_active ON auth.api_keys(is_active) WHERE is_active = true;
CREATE INDEX IF NOT EXISTS idx_auth_api_keys_role ON auth.api_keys(role);

-- Schema security
REVOKE ALL ON SCHEMA auth FROM PUBLIC;
GRANT USAGE ON SCHEMA auth TO authenticated, anon, agent, service_role;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA auth TO authenticated, anon, agent, service_role;
