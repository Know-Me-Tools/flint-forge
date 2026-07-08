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
END
$$;

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

-- Schema security
REVOKE ALL ON SCHEMA auth FROM PUBLIC;
GRANT USAGE ON SCHEMA auth TO authenticated, anon, service_role;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA auth TO authenticated, anon, service_role;
