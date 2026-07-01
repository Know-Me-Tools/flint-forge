-- complain if script is sourced in psql, not CREATE EXTENSION
\echo Use "CREATE EXTENSION flint_auth" to load this file. \quit

-- flint_auth: GUC-backed identity helpers. Verification stays upstream in flint-gate.
CREATE SCHEMA IF NOT EXISTS auth;

CREATE OR REPLACE FUNCTION auth.jwt() RETURNS jsonb LANGUAGE sql STABLE AS
$$ SELECT coalesce(current_setting('request.jwt.claims', true), '{}')::jsonb $$;

CREATE OR REPLACE FUNCTION auth.uid() RETURNS text LANGUAGE sql STABLE AS
$$ SELECT auth.jwt()->>'sub' $$;

CREATE OR REPLACE FUNCTION auth.role() RETURNS text LANGUAGE sql STABLE AS
$$ SELECT coalesce(auth.jwt()->>'role', 'anon') $$;

CREATE OR REPLACE FUNCTION auth.bearer() RETURNS text LANGUAGE sql STABLE AS
$$ SELECT current_setting('request.headers', true)::json->>'authorization' $$;
