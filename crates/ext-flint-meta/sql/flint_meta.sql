-- Flint Meta: pre-computed schema cache for the flint-reflection engine.
-- All tables are populated by DDL event triggers (p1-c008) and queried
-- by the Rust reflection engine (Phase 2). Never write to these from
-- application code — they are internal infrastructure.
--
-- The `flint_meta` schema is created and owned by the extension control file.

-- ── Cache: tables ──────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS flint_meta.cache_tables (
    schema_name   text        NOT NULL,
    table_name    text        NOT NULL,
    is_view       bool        NOT NULL DEFAULT false,
    description   text,
    -- pg_class.relrowsecurity — `ENABLE ROW LEVEL SECURITY`.
    rls_enabled   bool        NOT NULL DEFAULT false,
    -- pg_class.relforcerowsecurity — `FORCE ROW LEVEL SECURITY`. Without it the
    -- table OWNER bypasses every policy, so `rls_enabled` alone overstates
    -- protection for any table owned by the connecting role.
    rls_forced    bool        NOT NULL DEFAULT false,
    -- Whether `authenticated` or `anon` hold any table privilege. This is what
    -- makes a table reachable through the Data API at all: a table with RLS off
    -- but no grants is correctly hidden (the `REVOKE` pattern), and must not be
    -- reported as exposed.
    api_granted   bool        NOT NULL DEFAULT false,
    -- Count of RLS policies. `rls_enabled` with zero policies denies everything
    -- for non-owners — usually a mid-development state, and the shape that
    -- presents as "every write 403s while reads work".
    policy_count  int         NOT NULL DEFAULT 0,
    updated_at    timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (schema_name, table_name)
);

-- Upgrade path for an already-installed extension: the three columns above are
-- additions, so an existing cache_tables needs them backfilled rather than
-- recreated (the table is a cache, but dropping it would lose the rows until
-- the next full_refresh).
ALTER TABLE flint_meta.cache_tables
    ADD COLUMN IF NOT EXISTS rls_forced   bool NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS api_granted  bool NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS policy_count int  NOT NULL DEFAULT 0;

-- ── Cache: columns ─────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS flint_meta.cache_columns (
    schema_name    text    NOT NULL,
    table_name     text    NOT NULL,
    column_name    text    NOT NULL,
    data_type      text    NOT NULL,
    column_default text,
    is_nullable    bool    NOT NULL DEFAULT true,
    is_pk          bool    NOT NULL DEFAULT false,
    is_fk          bool    NOT NULL DEFAULT false,
    description    text,
    ordinal        int     NOT NULL,
    PRIMARY KEY (schema_name, table_name, column_name),
    FOREIGN KEY (schema_name, table_name) REFERENCES flint_meta.cache_tables (schema_name, table_name) ON DELETE CASCADE
);

-- ── Cache: relationships ───────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS flint_meta.cache_relationships (
    id              bigserial   PRIMARY KEY,
    from_schema     text        NOT NULL,
    from_table      text        NOT NULL,
    from_column     text        NOT NULL,
    to_schema       text        NOT NULL,
    to_table        text        NOT NULL,
    to_column       text        NOT NULL,
    constraint_name text        NOT NULL,
    UNIQUE (from_schema, from_table, from_column, to_schema, to_table, to_column)
);

-- ── Cache: functions ───────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS flint_meta.cache_functions (
    schema_name     text    NOT NULL,
    function_name   text    NOT NULL,
    return_type     text    NOT NULL,
    argument_types  text[]  NOT NULL DEFAULT '{}',
    is_stable       bool    NOT NULL DEFAULT false,
    description     text,
    PRIMARY KEY (schema_name, function_name, argument_types)
);

-- ── Cache: policies ────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS flint_meta.cache_policies (
    schema_name   text    NOT NULL,
    table_name    text    NOT NULL,
    policy_name   text    NOT NULL,
    command       text    NOT NULL,  -- SELECT, INSERT, UPDATE, DELETE, ALL
    roles         text[]  NOT NULL DEFAULT '{}',
    permissive    bool    NOT NULL DEFAULT true,
    PRIMARY KEY (schema_name, table_name, policy_name)
);

-- ── Cache: types ───────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS flint_meta.cache_types (
    schema_name   text    NOT NULL,
    type_name     text    NOT NULL,
    kind          text    NOT NULL,  -- 'enum', 'composite', 'domain', 'base'
    labels        text[]  NOT NULL DEFAULT '{}',  -- for enums
    PRIMARY KEY (schema_name, type_name)
);

-- ── Schema version ─────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS flint_meta.schema_version (
    version     bigserial   PRIMARY KEY,
    changed_at  timestamptz NOT NULL DEFAULT now(),
    ddl_tag     text,
    object_name text
);

-- Seed: version 1 is the initial schema install.
INSERT INTO flint_meta.schema_version (ddl_tag, object_name)
VALUES ('INSTALL', 'ext-flint-meta');

-- ── Keto tuples (permission cache) ─────────────────────────────────────────
--
-- Read ONLY when the gateway runs with FLINT_AUTHZ_MODE=rls+keto. In the
-- default `rls` mode this table is never queried.
--
-- Seeding contract — a tuple grants a subject one mutation verb on one table:
--
--   namespace   'entities'                 (the only namespace the gate uses)
--   object_id   '<schema>.<table>'         e.g. 'public.orders' — SCHEMA-QUALIFIED
--   relation    'insert' | 'update' | 'delete'
--   subject_id  the JWT subject (PII — never log it)
--
-- Both halves matter. A bare table name in object_id, or a catch-all relation
-- such as 'mutate', will never match: `fdb_reflection`'s `mutation_guard` looks
-- up exactly (entities, "<schema>.<table>", <verb>, subject). A second gate in
-- `fdb-app` once used the bare-table/'mutate' shape, but it was unreachable
-- from any production path and was removed in p17 — tuples seeded against it
-- silently matched nothing.
--
-- The gate is FAIL-CLOSED: with rls+keto set and this table empty, every
-- mutation is denied while reads keep working. The gateway therefore refuses to
-- start on an empty cache rather than serving traffic that uniformly 403s.
CREATE TABLE IF NOT EXISTS flint_meta.keto_tuples (
    namespace   text    NOT NULL,
    object_id   text    NOT NULL,
    relation    text    NOT NULL,
    subject_id  text    NOT NULL,
    PRIMARY KEY (namespace, object_id, relation, subject_id)
);

CREATE INDEX IF NOT EXISTS keto_tuples_subject_idx
    ON flint_meta.keto_tuples (subject_id, namespace, relation);

CREATE INDEX IF NOT EXISTS keto_tuples_object_idx
    ON flint_meta.keto_tuples (namespace, object_id);

-- ── Vault key metadata ─────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS flint_meta.vault_keys (
    key_id          uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    key_name        text        NOT NULL UNIQUE,
    kms_key_ref     text        NOT NULL,
    algorithm       text        NOT NULL DEFAULT 'RSA-OAEP-256',
    created_at      timestamptz NOT NULL DEFAULT now(),
    rotated_at      timestamptz
);

CREATE TABLE IF NOT EXISTS flint_meta.vault_key_assignments (
    assignment_id   uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    key_id          uuid        NOT NULL REFERENCES flint_meta.vault_keys (key_id) ON DELETE RESTRICT,
    category        text        NOT NULL,
    assigned_at     timestamptz NOT NULL DEFAULT now(),
    UNIQUE (category)
);

-- ── Cedar policy bundles ───────────────────────────────────────────────────
-- Cedar policies (text form) loaded by forge-policy's CedarPolicyEngine.
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

-- ── Security: schema lockdown ───────────────────────────────────────────────
REVOKE ALL ON ALL TABLES IN SCHEMA flint_meta FROM PUBLIC;
GRANT SELECT ON flint_meta.cache_tables TO authenticated, anon;
GRANT SELECT ON flint_meta.cache_columns TO authenticated, anon;
GRANT SELECT ON flint_meta.cache_relationships TO authenticated, anon;
GRANT SELECT ON flint_meta.cache_functions TO authenticated, anon;
GRANT SELECT ON flint_meta.cache_policies TO service_role;
GRANT SELECT ON flint_meta.cache_types TO authenticated, anon;
GRANT SELECT ON flint_meta.schema_version TO service_role;
GRANT ALL ON flint_meta.keto_tuples TO service_role;
GRANT ALL ON flint_meta.cedar_policies TO service_role;
GRANT ALL ON flint_meta.vault_keys, flint_meta.vault_key_assignments TO vault_admin;
