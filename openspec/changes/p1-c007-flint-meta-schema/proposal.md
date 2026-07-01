# p1-c007 — ext-flint-meta: schema (new pgrx 0.18.1/pg18 extension)

## Why

`flint_meta` is the pre-computed schema cache that enables `flint-reflection` (Phase 2) to compile a REST router and GraphQL SDL without hot-querying `pg_catalog`. It is also the central store for Keto relationship tuples and Vault key assignments. Without this schema, Phase 2 has no data to consume.

## What

Create `crates/ext-flint-meta/` from scratch as a pgrx 0.18.1/pg18 extension (using `ext-flint-vault/Cargo.toml` as the structural template — it is already correctly configured for pgrx 0.18.1 single-compile):

### Cache tables

```sql
-- Schema
CREATE SCHEMA IF NOT EXISTS flint_meta;

-- Table metadata cache
CREATE TABLE flint_meta.cache_tables (
    schema_name text NOT NULL,
    table_name  text NOT NULL,
    is_view     boolean NOT NULL DEFAULT false,
    description text,
    rls_enabled boolean NOT NULL DEFAULT false,
    cached_at   timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (schema_name, table_name)
);

-- Column metadata cache
CREATE TABLE flint_meta.cache_columns (
    schema_name  text NOT NULL,
    table_name   text NOT NULL,
    column_name  text NOT NULL,
    data_type    text NOT NULL,
    is_nullable  boolean NOT NULL,
    is_pk        boolean NOT NULL DEFAULT false,
    is_unique    boolean NOT NULL DEFAULT false,
    default_expr text,
    description  text,
    PRIMARY KEY (schema_name, table_name, column_name)
);

-- Foreign key relationship cache
CREATE TABLE flint_meta.cache_relationships (
    schema_name        text NOT NULL,
    table_name         text NOT NULL,
    column_name        text NOT NULL,
    foreign_schema     text NOT NULL,
    foreign_table      text NOT NULL,
    foreign_column     text NOT NULL,
    constraint_name    text NOT NULL,
    PRIMARY KEY (constraint_name)
);

-- Function/RPC cache
CREATE TABLE flint_meta.cache_functions (
    schema_name  text NOT NULL,
    function_name text NOT NULL,
    argument_types text NOT NULL,
    return_type  text NOT NULL,
    is_stable    boolean NOT NULL DEFAULT false,
    description  text,
    PRIMARY KEY (schema_name, function_name, argument_types)
);

-- RLS policy cache
CREATE TABLE flint_meta.cache_policies (
    schema_name   text NOT NULL,
    table_name    text NOT NULL,
    policy_name   text NOT NULL,
    command       text NOT NULL,  -- SELECT, INSERT, UPDATE, DELETE, ALL
    roles         text[] NOT NULL,
    using_expr    text,
    with_check    text,
    PRIMARY KEY (schema_name, table_name, policy_name)
);

-- Type cache (enums, domains, composites)
CREATE TABLE flint_meta.cache_types (
    schema_name text NOT NULL,
    type_name   text NOT NULL,
    type_kind   text NOT NULL,  -- 'enum', 'domain', 'composite'
    labels      text[],         -- for enums
    PRIMARY KEY (schema_name, type_name)
);

-- Schema version tracking
CREATE TABLE flint_meta.schema_version (
    version     bigint NOT NULL,
    updated_at  timestamptz NOT NULL DEFAULT now(),
    updated_by  text NOT NULL DEFAULT session_user,
    ddl_tag     text,
    object_identity text,
    PRIMARY KEY (version)
);
INSERT INTO flint_meta.schema_version (version, ddl_tag) VALUES (1, 'initial');
```

### Keto tuple storage

```sql
CREATE TABLE flint_meta.keto_tuples (
    namespace   text NOT NULL,
    object_id   text NOT NULL,
    relation    text NOT NULL,
    subject_id  text NOT NULL,
    subject_set_namespace text,
    subject_set_object    text,
    subject_set_relation  text,
    created_at  timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (namespace, object_id, relation, subject_id)
);
CREATE INDEX keto_tuples_subject_idx ON flint_meta.keto_tuples (subject_id, namespace);
CREATE INDEX keto_tuples_object_idx  ON flint_meta.keto_tuples (namespace, object_id);
```

### Vault key metadata

```sql
CREATE TABLE flint_meta.vault_keys (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    name        text NOT NULL UNIQUE,
    algorithm   text NOT NULL DEFAULT 'xchacha20-poly1305',
    status      text NOT NULL DEFAULT 'active',
    created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE flint_meta.vault_key_assignments (
    key_id      uuid NOT NULL REFERENCES flint_meta.vault_keys(id),
    category    text NOT NULL,
    scope       text,
    assigned_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (key_id, category, COALESCE(scope, ''))
);
```

## Contract

`cargo pgrx install -p ext-flint-meta` (or `cargo pgrx run`) creates all tables and the `flint_meta` schema. `SELECT flint_meta.version()` returns `1`. All tables exist and accept INSERT/SELECT.

## Constraints

- pgrx = "=0.18.1", pg18 feature ONLY — `crate-type = ["cdylib"]`, no `pgrx_embed.rs` bin
- Add to root `Cargo.toml` `exclude` list (already excluded by pattern if name starts with `ext-`)
- No `unwrap()` / `expect()` in Rust code — `error!()` macro is the pgrx abort primitive
- File size ≤ 500 lines per file — split: `src/schema.rs`, `src/keto.rs`, `src/vault_meta.rs`, `src/version.rs`

## Reference

- `crates/ext-flint-vault/Cargo.toml` — USE AS TEMPLATE (pgrx 0.18.1 single-compile, pg18)
- `docs/FLINT-PHASE-PLAN-REVISED.md` §Phase 1 p1-c007
- RFC-FORGE-META-001 §4 (schema definitions)
