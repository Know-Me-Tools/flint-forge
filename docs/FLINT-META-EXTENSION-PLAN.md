# Flint Forge Meta Extension: Architecture and Implementation Plan

**Document ID:** RFC-FORGE-META-001  
**Date:** June 2026  
**Status:** Architecture Design — Not Yet Implemented  
**Scope:** `flint_meta` pgrx extension + `flint-reflection` Rust runtime replacing PostgREST, postgres-meta, and extending pg_graphql with Prometheus-native capabilities.

---

## 1. Executive Summary

PostgREST and postgres-meta are excellent tools, but they are constrained by history:

- PostgREST cannot modify PostgreSQL; it reverse-engineers metadata from `pg_catalog` and maintains an external schema cache that must be invalidated after DDL.
- postgres-meta provides a management API but is a separate service with its own connection pool and caching logic.
- Neither is designed for AI-native platforms where metadata must drive REST endpoints, GraphQL schemas, OpenAPI documents, MCP tool manifests, and AG-UI/A2UI interface generation simultaneously.

Flint Forge eliminates these constraints by building a **database-driven compiler**:

1. **The database owns the metadata** — a pgrx extension (`flint_meta`) installs a `flint_meta.*` schema directly inside PostgreSQL with pre-computed reflection tables, event triggers, and version tracking.
2. **Rust merely reflects it** — a `flint-reflection` Rust engine consumes the `flint_meta.*` schema via simple SQL queries, compiles an immutable intermediate representation (IR), and generates REST routers, GraphQL gateways, OpenAPI documents, MCP tool manifests, and AG-UI/A2UI descriptors.
3. **Zero external cache invalidation** — event triggers increment a version counter on every DDL change; the Rust engine polls `SELECT version FROM flint_meta.schema_version` and atomically rebuilds the compiled state via `ArcSwap` without dropping requests.
4. **Unified governance** — JWTs from Flint Gate (Kratos → JWT) flow through to the database, where Keto permissions, row-level encryption keys, and Cedar policies are evaluated inline via the same extension.

This document specifies the architecture, crate structure, and five-milestone implementation roadmap.

---

## 2. Philosophy: Why This Is Better Than PostgREST

### 2.1 PostgREST's Limitations

PostgREST is a standalone Haskell web server that turns PostgreSQL into a REST API. Its architecture is:

```
PostgreSQL
    ↓  (pg_catalog queries at startup/reload)
PostgREST (SchemaCache in Haskell heap)
    ↓  (HTTP-to-SQL mapping)
REST
```

The critical limitations are:

| Limitation | Impact |
|-----------|--------|
| **External schema cache** | Must be invalidated after DDL via SIGUSR1, `NOTIFY pgrst, 'reload schema'`, or polling. Race conditions possible. |
| **Expensive catalog queries** | Every reload joins `pg_class`, `pg_attribute`, `pg_type`, `pg_proc`, `pg_namespace`, `pg_constraint`, `pg_description`, etc. |
| **Complex view key tracing** | Views hide base-table keys; PostgREST must regex-parse `pg_node_tree` to trace PK/FK through view layers. |
| **No in-database metadata** | Other tools (GraphQL, OpenAPI, MCP) must re-implement the same catalog queries. |
| **Function overloading** | Must disambiguate overloaded functions from `pg_proc` argument arrays. |
| **No identity-aware metadata** | The metadata model is not filtered by the caller's JWT identity or Keto permissions. |
| **No AI-UI metadata** | No facility to generate AG-UI/A2UI hints, JSON Schema, or MCP tool definitions from the same metadata. |

### 2.2 The Flint Meta Philosophy

Instead of thinking about the stack as:

```
PostgreSQL
    ↓
PostgREST
    ↓
REST
```

Think of it as:

```
                  PostgreSQL 18
       pg_graphql
       pg_catalog
       information_schema
       custom extensions
       event triggers
       metadata tables
                  │
                  ▼
       Metadata Runtime (inside Postgres)
                  │
       LISTEN / NOTIFY
       logical notifications
       metadata versioning
                  │
                  ▼
       Rust Reflection Engine
        Axum
        async_graphql
        utoipa/OpenAPI
        REST compiler
        GraphQL passthrough
                  │
                  ▼
          generated routers
          generated OpenAPI
          generated SDL
          generated MCP tools
          generated AG-UI descriptors
```

**What changed:** The database owns the metadata. Rust merely reflects it.

### 2.3 Key Architectural Decisions

| Decision | Rationale |
|----------|-----------|
| **Metadata lives in PostgreSQL** | No external cache to invalidate. DDL changes update `flint_meta.*` tables atomically via event triggers. |
| **Reflection is a compiler** | DatabaseModel → Normalization → Validation → Permission Analysis → Endpoint Generation → Router → OpenAPI → SDL → MCP → AG-UI. Everything from one IR. |
| **Immutable compiled state** | `ArcSwap` holds the compiled router, GraphQL schema, and OpenAPI document. Old requests use the old state; new requests use the new state. Zero downtime. |
| **JWT flows through** | The JWT from Flint Gate (with Keto claims, Cedar capabilities, and Vault key references) is passed as `SET LOCAL` GUC variables so the extension evaluates permissions and encryption inline. |
| **Keto tuples in PostgreSQL** | Keto relation tuples are stored in PostgreSQL tables (`flint_keto.*`) and the extension provides `flint_meta.check_permission(namespace, object, relation)` as a SQL-callable function. |
| **AG-UI/A2UI as first-class output** | The compiler generates metadata hints (field types, display formats, validation rules, component hints) that AI agents consume to generate user interfaces on-the-fly. |

---

## 3. Architecture Overview

### 3.1 The Two-Layer Stack

```
┌─────────────────────────────────────────────────────────────────────┐
│                        LAYER 1: INSIDE POSTGRESQL                    │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │                    flint_meta (pgrx extension)              │    │
│  │                                                              │    │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  │    │
│  │  │meta.cache│  │meta.schema│  │meta.keto│  │meta.vault│  │    │
│  │  │_tables   │  │_version  │  │_tuples   │  │_keys     │  │    │
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘  │    │
│  │                                                              │    │
│  │  ┌──────────────────────────────────────────────────────┐  │    │
│  │  │  SQL-Callable Functions:                             │  │    │
│  │  │  meta.schemas()  meta.tables()  meta.columns()      │  │    │
│  │  │  meta.functions()  meta.relationships()              │  │    │
│  │  │  meta.check_permission(ns, obj, rel)                 │  │    │
│  │  │  meta.decrypt_column(ciphertext, column_id)            │  │    │
│  │  │  meta.openapi()  meta.graphql()  meta.agui()          │  │    │
│  │  └──────────────────────────────────────────────────────┘  │    │
│  │                                                              │    │
│  │  ┌──────────────────────────────────────────────────────┐  │    │
│  │  │  Event Triggers:                                       │  │    │
│  │  │  ddl_command_end → meta.refresh_cache() → version++  │  │    │
│  │  │  sql_drop → meta.invalidate_cache() → version++        │  │    │
│  │  └──────────────────────────────────────────────────────┘  │    │
│  │                                                              │    │
│  │  ┌──────────────────────────────────────────────────────┐  │    │
│  │  │  NOTIFY Channels:                                    │  │    │
│  │  │  'meta_runtime' → {version, tx_id, changes[]}          │  │    │
│  │  │  'keto_changes' → {namespace, object, relation}      │  │    │
│  │  │  'vault_rotation' → {key_id, old_version, new_version}│  │    │
│  │  └──────────────────────────────────────────────────────┘  │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  PostgreSQL Core (pg_catalog) + flint_* extensions           │    │
│  │  (flint_auth, flint_hooks, flint_llm, flint_vault)         │    │
│  └─────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              │ LISTEN / NOTIFY (lightweight payload)
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                       LAYER 2: RUST REFLECTION ENGINE                  │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │              flint-reflection (Rust crate)                   │    │
│  │                                                              │    │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  │    │
│  │  │ Axum     │  │ async-   │  │ utoipa   │  │ MCP      │  │    │
│  │  │ Router   │  │ graphql  │  │ OpenAPI  │  │ Manifest │  │    │
│  │  │ (hot)    │  │ Gateway  │  │ Generator│  │ Compiler │  │    │
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘  │    │
│  │                                                              │    │
│  │  ┌──────────────────────────────────────────────────────┐  │    │
│  │  │  Immutable IR (ArcSwap):                             │  │    │
│  │  │  DatabaseModel → Router → GraphQL → OpenAPI → MCP    │  │    │
│  │  │  → AG-UI → A2UI                                      │  │    │
│  │  └──────────────────────────────────────────────────────┘  │    │
│  │                                                              │    │
│  │  ┌──────────────────────────────────────────────────────┐  │    │
│  │  │  SQL Compiler:                                       │  │    │
│  │  │  HTTP Request → AST → Query AST → SQL → Prepared     │  │    │
│  │  │  Statement → JSON                                    │  │    │
│  │  └──────────────────────────────────────────────────────┘  │    │
│  │                                                              │    │
│  │  ┌──────────────────────────────────────────────────────┐  │    │
│  │  │  Token Propagation:                                  │  │    │
│  │  │  JWT Claims → SET LOCAL app.jwt_claims = '...'        │  │    │
│  │  │  Keto Subject → SET LOCAL app.keto_subject = '...'     │  │    │
│  │  │  Vault Key Ref → SET LOCAL app.vault_key = '...'     │  │    │
│  │  └──────────────────────────────────────────────────────┘  │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  flint-gate (Axum) → JWT mint → Kratos + Keto + Cedar      │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  Realtime Fabric (Iggy) → WebSocket mux → CRDT → SSE       │    │
│  └─────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────┘
```

### 3.2 Design Principles

| Principle | Implementation |
|-----------|---------------|
| **Database as source of truth** | `flint_meta` owns metadata tables, cache tables, version tracking, and event triggers. |
| **Rust as compiler** | `flint-reflection` compiles metadata into executable artifacts (routers, schemas, manifests). |
| **Zero external cache** | No schema cache in Rust. The database is the cache. Rust queries `flint_meta.*` tables, not `pg_catalog` directly. |
| **Atomic hot-swap** | `ArcSwap` replaces compiled state without locking. Old requests continue; new requests see new state. |
| **Identity-propagated** | JWT claims, Keto subject, and Vault key references are propagated via `SET LOCAL` GUC variables, evaluated by the extension. |
| **AI-native metadata** | Every output format (REST, GraphQL, OpenAPI, MCP, AG-UI) is generated from the same IR. |

---

## 4. The Meta Extension (`flint_meta`): Inside PostgreSQL

### 4.1 Schema Design

The `flint_meta` extension creates a dedicated schema with the following categories:

#### 4.1.1 Cache Tables (Pre-Computed Reflection)

These tables store the results of expensive `pg_catalog` queries so reflection is nearly free.

```sql
CREATE TABLE flint_meta.cache_tables (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    schema_name text NOT NULL,
    table_name  text NOT NULL,
    table_type  text NOT NULL CHECK (table_type IN ('table', 'view', 'materialized_view', 'foreign_table', 'partitioned')),
    comment     text,
    rls_enabled boolean NOT NULL DEFAULT false,
    replica_identity text NOT NULL DEFAULT 'DEFAULT',
    estimated_rows bigint,
    insertable  boolean NOT NULL DEFAULT false,
    updatable   boolean NOT NULL DEFAULT false,
    deletable   boolean NOT NULL DEFAULT false,
    primary_key_columns text[] NOT NULL DEFAULT '{}',
    unique_columns text[] NOT NULL DEFAULT '{}',
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    UNIQUE(schema_name, table_name)
);

CREATE TABLE flint_meta.cache_columns (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    table_id    uuid NOT NULL REFERENCES flint_meta.cache_tables(id) ON DELETE CASCADE,
    column_name text NOT NULL,
    ordinal_position int NOT NULL,
    data_type   text NOT NULL,
    is_nullable boolean NOT NULL DEFAULT true,
    column_default text,
    is_identity boolean NOT NULL DEFAULT false,
    is_generated boolean NOT NULL DEFAULT false,
    max_length  int,
    numeric_precision int,
    numeric_scale int,
    comment     text,
    -- AG-UI / A2UI hints
    ui_hint     jsonb,  -- e.g., {"component": "email", "validators": ["email"]}
    -- Encryption metadata
    encrypted   boolean NOT NULL DEFAULT false,
    key_id      uuid,     -- References flint_meta.vault_keys
    -- Keto permission metadata
    permission_namespace text,
    permission_relation  text,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    UNIQUE(table_id, column_name)
);

CREATE TABLE flint_meta.cache_relationships (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    schema_name text NOT NULL,
    table_name  text NOT NULL,
    column_name text NOT NULL,
    target_schema text NOT NULL,
    target_table  text NOT NULL,
    target_column text NOT NULL,
    relation_type text NOT NULL CHECK (relation_type IN ('many_to_one', 'one_to_one', 'one_to_many', 'many_to_many')),
    constraint_name text NOT NULL,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    UNIQUE(schema_name, table_name, column_name, constraint_name)
);

CREATE TABLE flint_meta.cache_functions (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    schema_name text NOT NULL,
    function_name text NOT NULL,
    argument_names text[] NOT NULL DEFAULT '{}',
    argument_types text[] NOT NULL DEFAULT '{}',
    argument_defaults text[] NOT NULL DEFAULT '{}',
    return_type text,
    return_setof boolean NOT NULL DEFAULT false,
    volatility text NOT NULL DEFAULT 'VOLATILE' CHECK (volatility IN ('VOLATILE', 'STABLE', 'IMMUTABLE')),
    is_strict boolean NOT NULL DEFAULT false,
    security_definer boolean NOT NULL DEFAULT false,
    language text NOT NULL,
    comment text,
    -- REST endpoint mapping
    rest_method text,
    rest_path   text,
    -- AG-UI hint
    ui_hint     jsonb,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    UNIQUE(schema_name, function_name)
);

CREATE TABLE flint_meta.cache_policies (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    schema_name text NOT NULL,
    table_name  text NOT NULL,
    policy_name text NOT NULL,
    command     text NOT NULL CHECK (command IN ('ALL', 'SELECT', 'INSERT', 'UPDATE', 'DELETE')),
    permissive  boolean NOT NULL DEFAULT true,
    roles       text[] NOT NULL DEFAULT '{}',
    using_expression text,
    with_check_expression text,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    UNIQUE(schema_name, table_name, policy_name)
);

CREATE TABLE flint_meta.cache_types (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    schema_name text NOT NULL,
    type_name   text NOT NULL,
    type_kind   text NOT NULL CHECK (type_kind IN ('base', 'composite', 'domain', 'enum', 'range', 'multirange')),
    base_type   text,
    enum_values text[],
    -- JSON Schema representation
    json_schema jsonb,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    UNIQUE(schema_name, type_name)
);
```

#### 4.1.2 Version Tracking

```sql
CREATE TABLE flint_meta.schema_version (
    id          bigserial PRIMARY KEY,
    version     bigint NOT NULL DEFAULT 0,
    transaction_id bigint NOT NULL DEFAULT txid_current(),
    lsn         pg_lsn NOT NULL DEFAULT pg_current_xlog_insert_location(),
    reason      text,
    changed_objects jsonb NOT NULL DEFAULT '[]',
    created_at  timestamptz NOT NULL DEFAULT now()
);

-- Initial version
INSERT INTO flint_meta.schema_version (version, reason, changed_objects)
VALUES (0, 'initial', '[]');
```

#### 4.1.3 Keto Tuple Storage

```sql
CREATE TABLE flint_meta.keto_tuples (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    namespace   text NOT NULL,
    object      text NOT NULL,
    relation    text NOT NULL,
    subject_id  text,
    subject_set_namespace text,
    subject_set_object    text,
    subject_set_relation  text,
    commit_time timestamptz NOT NULL DEFAULT now(),
    -- For time-based permission control
    effective_from timestamptz NOT NULL DEFAULT now(),
    effective_until timestamptz,
    -- For metadata association
    table_id    uuid REFERENCES flint_meta.cache_tables(id),
    column_id   uuid REFERENCES flint_meta.cache_columns(id),
    function_id uuid REFERENCES flint_meta.cache_functions(id),
    -- Unique constraint (same as Keto)
    UNIQUE(namespace, object, relation, subject_id, subject_set_namespace, subject_set_object, subject_set_relation)
);

CREATE INDEX idx_keto_tuples_subject ON flint_meta.keto_tuples(
    subject_id, subject_set_namespace, subject_set_object, subject_set_relation
);
CREATE INDEX idx_keto_tuples_object_relation ON flint_meta.keto_tuples(
    namespace, object, relation
);
CREATE INDEX idx_keto_tuples_effective ON flint_meta.keto_tuples(
    effective_from, effective_until
);
```

#### 4.1.4 Vault Key Storage

```sql
CREATE TABLE flint_meta.vault_keys (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    key_id      text NOT NULL UNIQUE,          -- External key identifier (e.g., AWS KMS key ID)
    key_type    text NOT NULL CHECK (key_type IN ('aes-256-gcm', 'chacha20-poly1305', 'rsa-4096')),
    owner_id    text NOT NULL,                 -- User or tenant who owns this key
    -- Encrypted data encryption key (DEK) envelope
    dek_encrypted bytea NOT NULL,
    dek_version   int NOT NULL DEFAULT 1,
    -- Key metadata
    created_at  timestamptz NOT NULL DEFAULT now(),
    rotated_at  timestamptz,
    expires_at  timestamptz,
    -- Audit trail
    created_by  text NOT NULL,
    rotation_reason text
);

CREATE TABLE flint_meta.vault_key_assignments (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    key_id      uuid NOT NULL REFERENCES flint_meta.vault_keys(id),
    table_id    uuid NOT NULL REFERENCES flint_meta.cache_tables(id),
    column_id   uuid NOT NULL REFERENCES flint_meta.cache_columns(id),
    -- Row-level assignment (NULL = all rows in this column)
    row_filter  text,
    -- When the assignment is effective
    effective_from timestamptz NOT NULL DEFAULT now(),
    effective_until timestamptz,
    created_at  timestamptz NOT NULL DEFAULT now(),
    UNIQUE(key_id, table_id, column_id, COALESCE(row_filter, 'ALL'))
);
```

### 4.2 SQL-Callable Functions

These functions are the **only** interface the Rust layer should use to query metadata. Rust should never directly query `pg_catalog` outside of the extension.

```sql
-- Reflection functions
CREATE OR REPLACE FUNCTION flint_meta.schemas()
RETURNS TABLE(name text, owner text, acl jsonb)
AS $$ SELECT schema_name, schema_owner, schema_acl FROM flint_meta.cache_schemas; $$ LANGUAGE sql STABLE;

CREATE OR REPLACE FUNCTION flint_meta.tables()
RETURNS SETOF flint_meta.cache_tables
AS $$ SELECT * FROM flint_meta.cache_tables; $$ LANGUAGE sql STABLE;

CREATE OR REPLACE FUNCTION flint_meta.columns(table_id uuid)
RETURNS SETOF flint_meta.cache_columns
AS $$ SELECT * FROM flint_meta.cache_columns WHERE cache_columns.table_id = $1; $$ LANGUAGE sql STABLE;

CREATE OR REPLACE FUNCTION flint_meta.relationships(table_schema text, table_name text)
RETURNS SETOF flint_meta.cache_relationships
AS $$ SELECT * FROM flint_meta.cache_relationships WHERE cache_relationships.schema_name = $1 AND cache_relationships.table_name = $2; $$ LANGUAGE sql STABLE;

CREATE OR REPLACE FUNCTION flint_meta.functions()
RETURNS SETOF flint_meta.cache_functions
AS $$ SELECT * FROM flint_meta.cache_functions; $$ LANGUAGE sql STABLE;

CREATE OR REPLACE FUNCTION flint_meta.policies(table_schema text, table_name text)
RETURNS SETOF flint_meta.cache_policies
AS $$ SELECT * FROM flint_meta.cache_policies WHERE cache_policies.schema_name = $1 AND cache_policies.table_name = $2; $$ LANGUAGE sql STABLE;

CREATE OR REPLACE FUNCTION flint_meta.version()
RETURNS TABLE(version bigint, transaction_id bigint, lsn pg_lsn, changed_objects jsonb)
AS $$ SELECT version, transaction_id, lsn, changed_objects FROM flint_meta.schema_version ORDER BY id DESC LIMIT 1; $$ LANGUAGE sql STABLE;

-- Permission checking (Keto integration)
CREATE OR REPLACE FUNCTION flint_meta.check_permission(
    p_namespace text,
    p_object text,
    p_relation text
) RETURNS boolean
AS $$
DECLARE
    v_subject text := current_setting('app.keto_subject', true);
BEGIN
    IF v_subject IS NULL THEN
        RETURN false;
    END IF;
    
    -- Direct check
    RETURN EXISTS (
        SELECT 1 FROM flint_meta.keto_tuples
        WHERE namespace = p_namespace
          AND object = p_object
          AND relation = p_relation
          AND (subject_id = v_subject OR subject_id = '*')
          AND effective_from <= now()
          AND (effective_until IS NULL OR effective_until > now())
    );
END;
$$ LANGUAGE plpgsql STABLE SECURITY DEFINER;

-- Column decryption (Vault integration)
CREATE OR REPLACE FUNCTION flint_meta.decrypt_column(
    p_ciphertext bytea,
    p_column_id uuid
) RETURNS text
AS $$
DECLARE
    v_key_id uuid;
    v_dek bytea;
    v_owner text := current_setting('app.jwt_sub', true);
BEGIN
    -- Find the key assignment for this column and user
    SELECT ka.key_id INTO v_key_id
    FROM flint_meta.vault_key_assignments ka
    JOIN flint_meta.vault_keys vk ON ka.key_id = vk.id
    WHERE ka.column_id = p_column_id
      AND vk.owner_id = v_owner
      AND ka.effective_from <= now()
      AND (ka.effective_until IS NULL OR ka.effective_until > now())
    LIMIT 1;
    
    IF v_key_id IS NULL THEN
        RAISE EXCEPTION 'No key found for column % and user %', p_column_id, v_owner;
    END IF;
    
    -- The actual decryption would be done via pgrx calling the KMS
    -- For now, this is a stub that the Rust extension will override
    RETURN 'ENCRYPTED';
END;
$$ LANGUAGE plpgsql STABLE SECURITY DEFINER;

-- AG-UI / A2UI metadata generation
CREATE OR REPLACE FUNCTION flint_meta.agui_descriptor(
    p_table_schema text,
    p_table_name text,
    p_identity text DEFAULT NULL
) RETURNS jsonb
AS $$
DECLARE
    v_result jsonb;
    v_table_id uuid;
BEGIN
    SELECT id INTO v_table_id
    FROM flint_meta.cache_tables
    WHERE schema_name = p_table_schema AND table_name = p_table_name;
    
    SELECT jsonb_build_object(
        'type', 'table',
        'schema', p_table_schema,
        'name', p_table_name,
        'fields', (
            SELECT jsonb_agg(jsonb_build_object(
                'name', column_name,
                'type', data_type,
                'nullable', is_nullable,
                'default', column_default,
                'ui', ui_hint,
                'encrypted', encrypted,
                'readable', flint_meta.check_permission(
                    COALESCE(permission_namespace, p_table_schema || ':' || p_table_name),
                    column_name,
                    'read'
                ),
                'writable', flint_meta.check_permission(
                    COALESCE(permission_namespace, p_table_schema || ':' || p_table_name),
                    column_name,
                    'write'
                )
            ) ORDER BY ordinal_position)
            FROM flint_meta.cache_columns
            WHERE table_id = v_table_id
        ),
        'relationships', (
            SELECT jsonb_agg(jsonb_build_object(
                'name', column_name,
                'target', target_schema || '.' || target_table,
                'type', relation_type
            ))
            FROM flint_meta.cache_relationships
            WHERE schema_name = p_table_schema AND table_name = p_table_name
        ),
        'actions', (
            SELECT jsonb_agg(jsonb_build_object(
                'name', function_name,
                'path', rest_path,
                'method', rest_method,
                'ui', ui_hint
            ))
            FROM flint_meta.cache_functions
            WHERE schema_name = p_table_schema
              AND rest_path LIKE '/' || p_table_name || '%'
        )
    ) INTO v_result;
    
    RETURN v_result;
END;
$$ LANGUAGE plpgsql STABLE SECURITY DEFINER;

-- OpenAPI generation
CREATE OR REPLACE FUNCTION flint_meta.openapi()
RETURNS jsonb
AS $$
DECLARE
    v_result jsonb;
BEGIN
    SELECT jsonb_build_object(
        'openapi', '3.1.0',
        'info', jsonb_build_object('title', 'Flint API', 'version', '1.0.0'),
        'paths', (
            SELECT jsonb_object_agg(
                '/' || ct.schema_name || '/' || ct.table_name,
                jsonb_build_object(
                    'get', jsonb_build_object(
                        'summary', 'List ' || ct.table_name,
                        'parameters', (
                            SELECT jsonb_agg(jsonb_build_object(
                                'name', cc.column_name,
                                'in', 'query',
                                'schema', jsonb_build_object('type', cc.data_type)
                            ))
                            FROM flint_meta.cache_columns cc
                            WHERE cc.table_id = ct.id
                        )
                    ),
                    'post', jsonb_build_object(
                        'summary', 'Create ' || ct.table_name,
                        'requestBody', jsonb_build_object(
                            'content', jsonb_build_object(
                                'application/json', jsonb_build_object(
                                    'schema', jsonb_build_object(
                                        'type', 'object',
                                        'properties', (
                                            SELECT jsonb_object_agg(
                                                cc.column_name,
                                                jsonb_build_object('type', cc.data_type)
                                            )
                                            FROM flint_meta.cache_columns cc
                                            WHERE cc.table_id = ct.id
                                        )
                                    )
                                )
                            )
                        )
                    )
                )
            )
            FROM flint_meta.cache_tables ct
        )
    ) INTO v_result;
    
    RETURN v_result;
END;
$$ LANGUAGE plpgsql STABLE SECURITY DEFINER;
```

### 4.3 Event Triggers

Event triggers fire on every DDL change, updating cache tables and incrementing the version.

```sql
-- Refresh function: called by event triggers
CREATE OR REPLACE FUNCTION flint_meta.refresh_cache()
RETURNS event_trigger
AS $$
DECLARE
    rec record;
    v_changes jsonb := '[]';
    v_version bigint;
BEGIN
    -- Collect all DDL commands from this event
    FOR rec IN SELECT * FROM pg_event_trigger_ddl_commands() LOOP
        v_changes := v_changes || jsonb_build_object(
            'command', rec.command_tag,
            'object_identity', rec.object_identity,
            'schema_name', rec.schema_name,
            'object_type', rec.object_type
        );
        
        -- Update specific cache tables based on object type
        CASE rec.object_type
            WHEN 'table' THEN
                PERFORM flint_meta.refresh_table_cache(rec.object_identity);
            WHEN 'view' THEN
                PERFORM flint_meta.refresh_view_cache(rec.object_identity);
            WHEN 'function' THEN
                PERFORM flint_meta.refresh_function_cache(rec.object_identity);
            WHEN 'type' THEN
                PERFORM flint_meta.refresh_type_cache(rec.object_identity);
        END CASE;
    END LOOP;
    
    -- Increment version and notify
    INSERT INTO flint_meta.schema_version (version, transaction_id, reason, changed_objects)
    SELECT MAX(version) + 1, txid_current(), 'ddl_change', v_changes
    FROM flint_meta.schema_version;
    
    PERFORM pg_notify('meta_runtime', jsonb_build_object(
        'version', (SELECT MAX(version) FROM flint_meta.schema_version),
        'transaction_id', txid_current(),
        'changes', v_changes
    )::text);
END;
$$ LANGUAGE plpgsql;

-- Drop event trigger
CREATE OR REPLACE FUNCTION flint_meta.invalidate_cache()
RETURNS event_trigger
AS $$
DECLARE
    rec record;
    v_changes jsonb := '[]';
BEGIN
    FOR rec IN SELECT * FROM pg_event_trigger_dropped_objects() LOOP
        v_changes := v_changes || jsonb_build_object(
            'command', 'DROP',
            'object_identity', rec.object_identity,
            'schema_name', rec.schema_name,
            'object_type', rec.object_type
        );
        
        -- Remove from cache tables
        DELETE FROM flint_meta.cache_tables WHERE schema_name = rec.schema_name AND table_name = rec.object_name;
        DELETE FROM flint_meta.cache_functions WHERE schema_name = rec.schema_name AND function_name = rec.object_name;
        DELETE FROM flint_meta.cache_types WHERE schema_name = rec.schema_name AND type_name = rec.object_name;
    END LOOP;
    
    INSERT INTO flint_meta.schema_version (version, transaction_id, reason, changed_objects)
    SELECT MAX(version) + 1, txid_current(), 'ddl_drop', v_changes
    FROM flint_meta.schema_version;
    
    PERFORM pg_notify('meta_runtime', jsonb_build_object(
        'version', (SELECT MAX(version) FROM flint_meta.schema_version),
        'transaction_id', txid_current(),
        'changes', v_changes
    )::text);
END;
$$ LANGUAGE plpgsql;

-- Bind triggers
CREATE EVENT TRIGGER flint_meta_ddl_trigger
    ON ddl_command_end
    WHEN TAG IN (
        'CREATE TABLE', 'ALTER TABLE', 'DROP TABLE',
        'CREATE VIEW', 'ALTER VIEW', 'DROP VIEW',
        'CREATE FUNCTION', 'ALTER FUNCTION', 'DROP FUNCTION',
        'CREATE TYPE', 'ALTER TYPE', 'DROP TYPE',
        'CREATE INDEX', 'DROP INDEX',
        'CREATE POLICY', 'ALTER POLICY', 'DROP POLICY',
        'COMMENT'
    )
    EXECUTE FUNCTION flint_meta.refresh_cache();

CREATE EVENT TRIGGER flint_meta_drop_trigger
    ON sql_drop
    EXECUTE FUNCTION flint_meta.invalidate_cache();
```

### 4.4 JWT Identity Propagation

The Rust layer propagates JWT claims, Keto subject, and Vault key references via `SET LOCAL` GUC variables. These are session-local and automatically cleared at transaction end.

```sql
-- Called by the Rust layer before every request
CREATE OR REPLACE FUNCTION flint_meta.set_identity(
    p_jwt_claims jsonb,
    p_keto_subject text,
    p_vault_key_id text
) RETURNS void
AS $$
BEGIN
    PERFORM set_config('app.jwt_claims', p_jwt_claims::text, true);
    PERFORM set_config('app.jwt_sub', p_jwt_claims->>'sub', true);
    PERFORM set_config('app.jwt_roles', p_jwt_claims->'realm_access'->'roles' ?| ARRAY['admin', 'user']::text[], true);
    PERFORM set_config('app.keto_subject', p_keto_subject, true);
    PERFORM set_config('app.vault_key_id', p_vault_key_id, true);
END;
$$ LANGUAGE plpgsql;

-- Helper to read current identity
CREATE OR REPLACE FUNCTION flint_meta.current_identity()
RETURNS jsonb
AS $$
BEGIN
    RETURN jsonb_build_object(
        'sub', current_setting('app.jwt_sub', true),
        'roles', current_setting('app.jwt_roles', true),
        'keto_subject', current_setting('app.keto_subject', true),
        'vault_key_id', current_setting('app.vault_key_id', true)
    );
END;
$$ LANGUAGE plpgsql STABLE;
```

---

## 5. The Rust Reflection Engine (`flint-reflection`)

### 5.1 Core Data Structures

```rust
use std::sync::Arc;
use arc_swap::ArcSwap;
use serde::{Serialize, Deserialize};
use tokio::sync::watch;

/// Immutable intermediate representation of the entire database model.
/// This is the "source of truth" from which all outputs are compiled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseModel {
    pub version: u64,
    pub schemas: Vec<Schema>,
    pub tables: Vec<Table>,
    pub relationships: Vec<Relationship>,
    pub functions: Vec<Function>,
    pub types: Vec<Type>,
    pub policies: Vec<Policy>,
    pub keto_namespaces: Vec<KetoNamespace>,
    pub vault_keys: Vec<VaultKey>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub id: uuid::Uuid,
    pub schema_name: String,
    pub table_name: String,
    pub table_type: TableType,
    pub comment: Option<String>,
    pub rls_enabled: bool,
    pub columns: Vec<Column>,
    pub primary_key: Vec<String>,
    pub insertable: bool,
    pub updatable: bool,
    pub deletable: bool,
    pub permissions: Vec<Permission>,  // Keto permissions attached to this table
    pub ui_hint: Option<AguiDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    pub ordinal_position: i32,
    pub data_type: String,
    pub is_nullable: bool,
    pub default_value: Option<String>,
    pub is_identity: bool,
    pub is_generated: bool,
    pub max_length: Option<i32>,
    pub encrypted: bool,
    pub key_id: Option<uuid::Uuid>,  // References Vault key for column-level encryption
    pub permission_namespace: Option<String>,
    pub permission_relation: Option<String>,
    pub ui_hint: Option<serde_json::Value>,  // AG-UI/A2UI component hints
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub schema_name: String,
    pub table_name: String,
    pub column_name: String,
    pub target_schema: String,
    pub target_table: String,
    pub target_column: String,
    pub relation_type: RelationType,
    pub constraint_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub schema_name: String,
    pub function_name: String,
    pub arguments: Vec<Argument>,
    pub return_type: Option<String>,
    pub return_setof: bool,
    pub volatility: Volatility,
    pub is_strict: bool,
    pub security_definer: bool,
    pub language: String,
    pub rest_method: Option<String>,
    pub rest_path: Option<String>,
    pub ui_hint: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KetoNamespace {
    pub id: String,
    pub name: String,
    pub relations: Vec<KetoRelation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KetoRelation {
    pub name: String,
    pub subject_sets: Vec<String>,  // e.g., ["owner", "editor"] for inheritance
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultKey {
    pub id: uuid::Uuid,
    pub key_id: String,
    pub key_type: String,
    pub owner_id: String,
    pub dek_encrypted: Vec<u8>,
    pub dek_version: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub namespace: String,
    pub object: String,
    pub relation: String,
    pub allowed_subjects: Vec<String>,
}

/// The compiled state held by ArcSwap. All fields are immutable.
/// Hot-swapping replaces the entire struct atomically.
#[derive(Debug, Clone)]
pub struct CompiledState {
    pub version: u64,
    pub database_model: Arc<DatabaseModel>,
    pub router: Arc<axum::Router>,
    pub graphql_schema: Arc<async_graphql::Schema>,
    pub openapi_doc: Arc<utoipa::openapi::OpenApi>,
    pub mcp_manifest: Arc<serde_json::Value>,
    pub agui_descriptors: Arc<std::collections::HashMap<String, serde_json::Value>>,
}

/// Global state manager. Only one instance exists per process.
pub struct StateManager {
    pub compiled: ArcSwap<CompiledState>,
    pub version_rx: watch::Receiver<u64>,
    pub db_pool: sqlx::PgPool,
    pub config: Arc<Config>,
}
```

### 5.2 The Compiler Pipeline

```
PostgreSQL (flint_meta schema)
    ↓  SELECT * FROM flint_meta.tables(), flint_meta.columns(), etc.
ReflectionEngine::reflect()
    ↓
DatabaseModel (immutable IR)
    ↓  ┌─────────────────┐
       │ Normalization   │  Resolve domains, defaults, identity columns
       └─────────────────┘
    ↓  ┌─────────────────┐
       │ Validation      │  Check for cycles, conflicts, unsupported types
       └─────────────────┘
    ↓  ┌─────────────────┐
       │ Permission Analysis │  Cross-reference Keto tuples with RLS policies
       └─────────────────┘
    ↓  ┌─────────────────┐
       │ Endpoint Generation │  Generate REST routes, GraphQL fields, RPC mappings
       └─────────────────┘
    ↓  ┌─────────────────┐
       │ OpenAPI Compiler │  Generate utoipa OpenApi from DatabaseModel
       └─────────────────┘
    ↓  ┌─────────────────┐
       │ GraphQL SDL      │  Generate schema.graphql from DatabaseModel
       └─────────────────┘
    ↓  ┌─────────────────┐
       │ MCP Compiler     │  Generate MCP tool manifest from DatabaseModel
       └─────────────────┘
    ↓  ┌─────────────────┐
       │ AG-UI Compiler   │  Generate AG-UI/A2UI descriptors from DatabaseModel
       └─────────────────┘
    ↓  ┌─────────────────┐
       │ ArcSwap Hot-Swap │  Atomic replacement of compiled state
       └─────────────────┘
```

### 5.3 Hot-Swap Mechanism

```rust
use arc_swap::ArcSwap;
use tokio::sync::watch;

impl StateManager {
    pub fn new(db_pool: sqlx::PgPool, config: Arc<Config>) -> Self {
        let initial_state = Arc::new(CompiledState {
            version: 0,
            database_model: Arc::new(DatabaseModel::default()),
            router: Arc::new(axum::Router::new()),
            graphql_schema: Arc::new(async_graphql::Schema::new()),
            openapi_doc: Arc::new(utoipa::openapi::OpenApi::new()),
            mcp_manifest: Arc::new(serde_json::Value::Null),
            agui_descriptors: Arc::new(HashMap::new()),
        });
        
        let (version_tx, version_rx) = watch::channel(0u64);
        
        let manager = Self {
            compiled: ArcSwap::new(initial_state),
            version_rx,
            db_pool,
            config,
        };
        
        // Spawn background listener
        manager.spawn_listener(version_tx);
        
        manager
    }
    
    fn spawn_listener(&self, version_tx: watch::Sender<u64>) {
        let pool = self.db_pool.clone();
        let compiled = Arc::clone(&self.compiled); // ArcSwap is Clone
        let config = Arc::clone(&self.config);
        
        tokio::spawn(async move {
            loop {
                match listen_for_changes(&pool).await {
                    Ok(new_version) => {
                        // Fetch new metadata from PostgreSQL
                        let new_model = match reflect_database(&pool).await {
                            Ok(model) => model,
                            Err(e) => {
                                tracing::error!("Failed to reflect database: {}", e);
                                continue;
                            }
                        };
                        
                        // Compile new state
                        let new_state = match compile_state(new_model, &config).await {
                            Ok(state) => state,
                            Err(e) => {
                                tracing::error!("Failed to compile state: {}", e);
                                continue;
                            }
                        };
                        
                        // Atomic swap
                        compiled.store(Arc::new(new_state));
                        
                        // Notify watchers
                        let _ = version_tx.send(new_version);
                        
                        tracing::info!("Hot-swapped to version {}", new_version);
                    }
                    Err(e) => {
                        tracing::error!("Listener error: {}", e);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        });
    }
}

/// Listens for PostgreSQL NOTIFY on 'meta_runtime' channel
async fn listen_for_changes(pool: &sqlx::PgPool) -> Result<u64, Error> {
    // Use sqlx's LISTEN support or a dedicated connection
    let mut conn = pool.acquire().await?;
    
    // Issue LISTEN
    sqlx::query("LISTEN meta_runtime").execute(&mut *conn).await?;
    
    // Wait for notification
    // sqlx doesn't directly support async LISTEN, so we use a polling loop
    // or a dedicated tokio-postgres connection for LISTEN/NOTIFY
    
    // Simplified: poll version every 100ms
    loop {
        let row = sqlx::query_as::<_, (i64,)>(
            "SELECT version FROM flint_meta.schema_version ORDER BY id DESC LIMIT 1"
        )
        .fetch_one(&mut *conn)
        .await?;
        
        return Ok(row.0 as u64);
    }
}
```

### 5.4 REST Router Compilation

Instead of a static router, the router is compiled from the DatabaseModel.

```rust
use axum::{
    routing::{get, post, patch, delete},
    Router,
};

pub fn compile_router(model: &DatabaseModel) -> Router {
    let mut router = Router::new();
    
    for table in &model.tables {
        let base_path = format!("/{}/{}", table.schema_name, table.table_name);
        
        // GET /schema/table — list with filtering, pagination, embedding
        if table.rls_enabled || has_select_policy(model, table) {
            router = router.route(
                &base_path,
                get(handle_list).layer(RequirePermission {
                    namespace: table_permission_ns(table),
                    object: table.table_name.clone(),
                    relation: "read".to_string(),
                })
            );
        }
        
        // GET /schema/table/:id — single item
        router = router.route(
            &format!("{}/:id", base_path),
            get(handle_get)
        );
        
        // POST /schema/table — create
        if table.insertable {
            router = router.route(
                &base_path,
                post(handle_create)
            );
        }
        
        // PATCH /schema/table/:id — update
        if table.updatable {
            router = router.route(
                &format!("{}/:id", base_path),
                patch(handle_update)
            );
        }
        
        // DELETE /schema/table/:id — delete
        if table.deletable {
            router = router.route(
                &format!("{}/:id", base_path),
                delete(handle_delete)
            );
        }
    }
    
    // RPC functions → POST /rpc/:schema/:function
    for func in &model.functions {
        if let Some(ref path) = func.rest_path {
            let method = match func.rest_method.as_deref() {
                Some("GET") => get,
                Some("POST") => post,
                Some("PATCH") => patch,
                Some("DELETE") => delete,
                _ => post,
            };
            router = router.route(path, method(handle_rpc));
        }
    }
    
    router
}

/// Every handler receives the compiled DatabaseModel via Arc
async fn handle_list(
    State(state): State<Arc<CompiledState>>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Compile HTTP query params to SQL AST
    let ast = QueryCompiler::compile(&params, &state.database_model)?;
    
    // Execute via SQLx
    let rows = sqlx::query(&ast.sql)
        .bind(&ast.params)
        .fetch_all(&state.db_pool)
        .await?;
    
    // Convert to JSON
    Ok(Json(rows_to_json(rows)))
}
```

### 5.5 SQL Compiler

The SQL compiler is the heart of the system. Every HTTP request becomes a compiled SQL query.

```rust
pub struct QueryCompiler;

impl QueryCompiler {
    pub fn compile(params: &QueryParams, model: &DatabaseModel) -> Result<CompiledQuery, Error> {
        let mut builder = QueryBuilder::new();
        
        // 1. Resolve table and columns
        let table = model.find_table(&params.table)?;
        
        // 2. Build projection
        let columns = if params.select.is_empty() {
            table.columns.iter().filter(|c| !c.encrypted).map(|c| c.name.clone()).collect()
        } else {
            params.select.clone()
        };
        
        builder.select(&columns);
        
        // 3. Build FROM
        builder.from(&table.schema_name, &table.table_name);
        
        // 4. Build WHERE from filter params
        for (col, op, val) in &params.filters {
            match op.as_str() {
                "eq" => builder.where_eq(col, val),
                "gt" => builder.where_gt(col, val),
                "lt" => builder.where_lt(col, val),
                "like" => builder.where_like(col, val),
                "in" => builder.where_in(col, val.split(',').collect()),
                _ => return Err(Error::UnsupportedOperator(op.clone())),
            }
        }
        
        // 5. Handle relationships (resource embedding)
        for rel in &params.embed {
            if let Some(relationship) = model.find_relationship(&table.schema_name, &table.table_name, rel) {
                builder.join(relationship)?;
            }
        }
        
        // 6. Build ORDER BY
        if let Some(order) = &params.order {
            builder.order_by(order);
        }
        
        // 7. Build LIMIT/OFFSET
        builder.limit(params.limit.unwrap_or(100));
        builder.offset(params.offset.unwrap_or(0));
        
        // 8. Build prepared statement with parameters
        builder.compile()
    }
}

pub struct CompiledQuery {
    pub sql: String,
    pub params: Vec<serde_json::Value>,
    pub table: String,
    pub columns: Vec<String>,
}
```

### 5.6 GraphQL Gateway

For GraphQL, we leverage `pg_graphql` directly rather than re-implementing it in Rust.

```rust
use async_graphql::{Schema, EmptySubscription, EmptyMutation};

pub fn compile_graphql_schema(model: &DatabaseModel) -> Schema {
    // The GraphQL schema is primarily handled by pg_graphql inside PostgreSQL.
    // We only need a thin gateway for:
    // - Authentication (inject JWT claims)
    // - Subscriptions (WebSocket/SSE via Realtime Fabric)
    // - Federation (merge multiple pg_graphql schemas)
    // - Custom resolvers (for Prometheus-specific types)
    
    Schema::build(
        QueryRoot::new(model),
        MutationRoot::new(model),
        SubscriptionRoot::new(model),
    )
    .data(model.clone())
    .finish()
}

/// Query root delegates to pg_graphql
struct QueryRoot {
    model: Arc<DatabaseModel>,
}

#[async_graphql::Object]
impl QueryRoot {
    async fn pg(&self, ctx: &Context<'_>) -> async_graphql::Result<serde_json::Value> {
        let query = ctx.look_ahead().field_name(); // Get the GraphQL query
        
        // Execute via pg_graphql's graphql.resolve()
        let result = sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT graphql.resolve($1)"
        )
        .bind(query)
        .fetch_one(&ctx.data::<sqlx::PgPool>()?)
        .await?;
        
        Ok(result)
    }
    
    // Custom Prometheus resolvers
    async fn agent(&self, ctx: &Context<'_>) -> async_graphql::Result<Agent> {
        // Custom resolver for Prometheus Agent metadata
        Ok(Agent::default())
    }
    
    async fn workflow(&self, ctx: &Context<'_>) -> async_graphql::Result<Workflow> {
        // Custom resolver for Prometheus Workflow metadata
        Ok(Workflow::default())
    }
}
```

---

## 6. JWT Integration with Keto for Database Permissions

### 6.1 The Flow

```
User Request
    ↓
Flint Gate (Axum)
    ↓  Kratos validates session
    ↓  Keto checks permissions (coarse)
    ↓  Cedar evaluates capabilities
    ↓  JWT minted with claims:
       {
         "sub": "user:alice",
         "roles": ["admin"],
         "keto_subject": "user:alice",
         "keto_namespace": "documents",
         "vault_key_id": "key-123"
       }
    ↓
Flint Forge (flint-reflection)
    ↓  SET LOCAL app.jwt_claims = '{...}'
    ↓  SET LOCAL app.keto_subject = 'user:alice'
    ↓  SET LOCAL app.vault_key_id = 'key-123'
    ↓
PostgreSQL (flint_meta extension)
    ↓  flint_meta.check_permission('documents', 'doc1', 'read')
       → queries flint_meta.keto_tuples
       → returns true/false
    ↓  flint_meta.decrypt_column(ciphertext, column_id)
       → queries flint_meta.vault_key_assignments
       → calls Vault/KMS to decrypt DEK
       → decrypts column value
    ↓
Row-Level Security (RLS)
    ↓  CREATE POLICY user_isolation ON customers
       USING (tenant_id = current_setting('app.jwt_claims')::jsonb->>'tenant_id');
```

### 6.2 Keto Integration Architecture

Keto stores relation tuples in PostgreSQL tables. We extend this with our own `flint_meta.keto_tuples` table that is synchronized with Keto's storage.

```sql
-- The Keto tuple format: namespace:object#relation@subject
-- Example: documents:doc1#owner@user:alice

-- Direct permission check function
CREATE OR REPLACE FUNCTION flint_meta.keto_check(
    p_namespace text,
    p_object text,
    p_relation text
) RETURNS boolean
AS $$
DECLARE
    v_subject text := current_setting('app.keto_subject', true);
BEGIN
    -- Direct check
    IF EXISTS (
        SELECT 1 FROM flint_meta.keto_tuples
        WHERE namespace = p_namespace
          AND object = p_object
          AND relation = p_relation
          AND (subject_id = v_subject OR subject_id = '*')
          AND effective_from <= now()
          AND (effective_until IS NULL OR effective_until > now())
    ) THEN
        RETURN true;
    END IF;
    
    -- Subject set check (transitive)
    RETURN EXISTS (
        SELECT 1 FROM flint_meta.keto_tuples kt
        WHERE kt.namespace = p_namespace
          AND kt.object = p_object
          AND kt.relation = p_relation
          AND kt.subject_id IS NULL
          AND kt.subject_set_namespace IS NOT NULL
          AND kt.subject_set_object IS NOT NULL
          AND kt.subject_set_relation IS NOT NULL
          AND flint_meta.keto_check(
              kt.subject_set_namespace,
              kt.subject_set_object,
              kt.subject_set_relation
          )
          AND kt.effective_from <= now()
          AND (kt.effective_until IS NULL OR kt.effective_until > now())
    );
END;
$$ LANGUAGE plpgsql STABLE SECURITY DEFINER;

-- Table-level permission check
CREATE OR REPLACE FUNCTION flint_meta.table_is_readable(
    p_schema text,
    p_table text
) RETURNS boolean
AS $$
BEGIN
    RETURN flint_meta.keto_check(
        p_schema || ':' || p_table,
        p_table,
        'read'
    );
END;
$$ LANGUAGE plpgsql STABLE SECURITY DEFINER;

-- Column-level permission check
CREATE OR REPLACE FUNCTION flint_meta.column_is_readable(
    p_table_id uuid,
    p_column_name text
) RETURNS boolean
AS $$
DECLARE
    v_namespace text;
BEGIN
    SELECT permission_namespace INTO v_namespace
    FROM flint_meta.cache_columns
    WHERE table_id = p_table_id AND column_name = p_column_name;
    
    IF v_namespace IS NULL THEN
        -- No explicit permission = readable by default
        RETURN true;
    END IF;
    
    RETURN flint_meta.keto_check(v_namespace, p_column_name, 'read');
END;
$$ LANGUAGE plpgsql STABLE SECURITY DEFINER;
```

### 6.3 Keto Tuple Storage and API

The Rust layer provides an API to manage Keto tuples stored in PostgreSQL:

```rust
pub struct KetoManager {
    db_pool: sqlx::PgPool,
}

impl KetoManager {
    pub async fn write_tuple(
        &self,
        namespace: &str,
        object: &str,
        relation: &str,
        subject: &str,
    ) -> Result<uuid::Uuid, Error> {
        sqlx::query_scalar::<_, uuid::Uuid>(
            r#"
            INSERT INTO flint_meta.keto_tuples (namespace, object, relation, subject_id)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (namespace, object, relation, subject_id, subject_set_namespace, subject_set_object, subject_set_relation)
            DO UPDATE SET commit_time = now()
            RETURNING id
            "#
        )
        .bind(namespace)
        .bind(object)
        .bind(relation)
        .bind(subject)
        .fetch_one(&self.db_pool)
        .await
    }
    
    pub async fn check_permission(
        &self,
        namespace: &str,
        object: &str,
        relation: &str,
        subject: &str,
    ) -> Result<bool, Error> {
        let result = sqlx::query_scalar::<_, bool>(
            "SELECT flint_meta.keto_check($1, $2, $3)"
        )
        .bind(namespace)
        .bind(object)
        .bind(relation)
        .fetch_one(&self.db_pool)
        .await?;
        
        Ok(result)
    }
    
    /// Bulk load tuples from Keto server (sync)
    pub async fn sync_from_keto(&self, keto_url: &str) -> Result<u64, Error> {
        // Fetch all tuples from Keto's REST API
        let tuples = reqwest::get(format!("{}/relation-tuples", keto_url))
            .await?
            .json::<Vec<KetoTuple>>()
            .await?;
        
        // Insert into PostgreSQL
        let mut tx = self.db_pool.begin().await?;
        for tuple in tuples {
            sqlx::query(
                r#"
                INSERT INTO flint_meta.keto_tuples (namespace, object, relation, subject_id, subject_set_namespace, subject_set_object, subject_set_relation)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                ON CONFLICT DO NOTHING
                "#
            )
            .bind(&tuple.namespace)
            .bind(&tuple.object)
            .bind(&tuple.relation)
            .bind(&tuple.subject_id)
            .bind(&tuple.subject_set_namespace)
            .bind(&tuple.subject_set_object)
            .bind(&tuple.subject_set_relation)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        
        Ok(tuples.len() as u64)
    }
}
```

---

## 7. Key Vault and Column-Level Encryption

### 7.1 Encryption Architecture

```
User Data
    ↓
INSERT INTO customers (ssn, email)
    ↓
Flint Meta Extension
    ↓  For each encrypted column:
       1. Look up key assignment for this column + user
       2. Fetch DEK from vault_keys (encrypted with KEK)
       3. Call external KMS (HashiCorp Vault, AWS KMS) to decrypt DEK
       4. Encrypt column value with DEK (AES-256-GCM or ChaCha20-Poly1305)
       5. Store ciphertext + nonce + key version
    ↓
PostgreSQL Storage
    (ciphertext in encrypted column)
```

### 7.2 Key Management

```sql
-- Key rotation
CREATE OR REPLACE FUNCTION flint_meta.rotate_key(
    p_key_id uuid,
    p_reason text
) RETURNS void
AS $$
DECLARE
    v_old_key flint_meta.vault_keys%ROWTYPE;
    v_new_dek_encrypted bytea;
BEGIN
    SELECT * INTO v_old_key FROM flint_meta.vault_keys WHERE id = p_key_id;
    
    -- Generate new DEK
    -- (In practice, this calls the KMS to generate and encrypt a new DEK)
    v_new_dek_encrypted := gen_random_bytes(32); -- Placeholder
    
    -- Update key record
    UPDATE flint_meta.vault_keys
    SET dek_encrypted = v_new_dek_encrypted,
        dek_version = dek_version + 1,
        rotated_at = now(),
        rotation_reason = p_reason
    WHERE id = p_key_id;
    
    -- Notify that re-encryption is needed
    PERFORM pg_notify('vault_rotation', jsonb_build_object(
        'key_id', p_key_id,
        'old_version', v_old_key.dek_version,
        'new_version', v_old_key.dek_version + 1
    )::text);
END;
$$ LANGUAGE plpgsql;

-- Background re-encryption (triggered by vault_rotation notification)
CREATE OR REPLACE FUNCTION flint_meta.reencrypt_column(
    p_table_id uuid,
    p_column_id uuid,
    p_old_key_version int,
    p_new_key_version int
) RETURNS bigint
AS $$
DECLARE
    v_count bigint := 0;
    v_row record;
    v_table_name text;
    v_schema_name text;
    v_column_name text;
BEGIN
    SELECT ct.schema_name, ct.table_name, cc.column_name
    INTO v_schema_name, v_table_name, v_column_name
    FROM flint_meta.cache_tables ct
    JOIN flint_meta.cache_columns cc ON cc.table_id = ct.id
    WHERE ct.id = p_table_id AND cc.id = p_column_id;
    
    FOR v_row IN EXECUTE format(
        'SELECT id, %I FROM %I.%I WHERE %I IS NOT NULL',
        v_column_name, v_schema_name, v_table_name, v_column_name
    ) LOOP
        -- Decrypt with old key
        -- Re-encrypt with new key
        -- Update row
        v_count := v_count + 1;
    END LOOP;
    
    RETURN v_count;
END;
$$ LANGUAGE plpgsql;
```

### 7.3 Rust Vault Integration

```rust
use vault_client::VaultClient;

pub struct VaultManager {
    db_pool: sqlx::PgPool,
    vault_client: VaultClient,
}

impl VaultManager {
    /// Encrypt a column value
    pub async fn encrypt_column(
        &self,
        plaintext: &str,
        column_id: uuid::Uuid,
        user_id: &str,
    ) -> Result<Vec<u8>, Error> {
        // 1. Find key assignment
        let key = sqlx::query_as::<_, VaultKey>(
            r#"
            SELECT vk.*
            FROM flint_meta.vault_keys vk
            JOIN flint_meta.vault_key_assignments vka ON vk.id = vka.key_id
            WHERE vka.column_id = $1
              AND vk.owner_id = $2
              AND vka.effective_from <= now()
              AND (vka.effective_until IS NULL OR vka.effective_until > now())
            ORDER BY vk.dek_version DESC
            LIMIT 1
            "#
        )
        .bind(column_id)
        .bind(user_id)
        .fetch_one(&self.db_pool)
        .await?;
        
        // 2. Decrypt DEK via KMS
        let dek = self.vault_client.decrypt(&key.dek_encrypted).await?;
        
        // 3. Encrypt plaintext with DEK
        let ciphertext = encrypt_aes_gcm(plaintext.as_bytes(), &dek)?;
        
        Ok(ciphertext)
    }
    
    /// Decrypt a column value
    pub async fn decrypt_column(
        &self,
        ciphertext: &[u8],
        column_id: uuid::Uuid,
        user_id: &str,
    ) -> Result<String, Error> {
        let key = sqlx::query_as::<_, VaultKey>(
            r#"
            SELECT vk.*
            FROM flint_meta.vault_keys vk
            JOIN flint_meta.vault_key_assignments vka ON vk.id = vka.key_id
            WHERE vka.column_id = $1
              AND vk.owner_id = $2
              AND vka.effective_from <= now()
              AND (vka.effective_until IS NULL OR vka.effective_until > now())
            ORDER BY vk.dek_version DESC
            LIMIT 1
            "#
        )
        .bind(column_id)
        .bind(user_id)
        .fetch_one(&self.db_pool)
        .await?;
        
        let dek = self.vault_client.decrypt(&key.dek_encrypted).await?;
        let plaintext = decrypt_aes_gcm(ciphertext, &dek)?;
        
        Ok(String::from_utf8(plaintext)?)
    }
}
```

---

## 8. Realtime Metadata Propagation

### 8.1 The Notification Architecture

Three notification channels are used for different types of changes:

| Channel | Purpose | Payload |
|---------|---------|---------|
| `meta_runtime` | DDL changes (schema version) | `{version, tx_id, changes[]}` |
| `keto_changes` | Permission changes | `{namespace, object, relation, subject}` |
| `vault_rotation` | Key rotation | `{key_id, old_version, new_version}` |
| `agui_update` | AG-UI descriptor changes | `{table_schema, table_name, descriptor}` |

### 8.2 Rust Listener Implementation

```rust
use tokio_postgres::connect;

pub async fn listen_meta_runtime(
    db_url: &str,
    state_manager: Arc<StateManager>,
) -> Result<(), Error> {
    let (client, mut connection) = connect(db_url, tokio_postgres::NoTls).await?;
    
    // Listen on multiple channels
    client.execute("LISTEN meta_runtime", &[]).await?;
    client.execute("LISTEN keto_changes", &[]).await?;
    client.execute("LISTEN vault_rotation", &[]).await?;
    client.execute("LISTEN agui_update", &[]).await?;
    
    // Spawn connection handler
    tokio::spawn(async move {
        while let Some(msg) = connection.recv().await {
            match msg {
                Ok(tokio_postgres::AsyncMessage::Notification(notif)) => {
                    handle_notification(&notif, &state_manager).await;
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("Connection error: {}", e);
                    break;
                }
            }
        }
    });
    
    Ok(())
}

async fn handle_notification(
    notif: &tokio_postgres::Notification,
    state_manager: &StateManager,
) {
    match notif.channel() {
        "meta_runtime" => {
            let payload: serde_json::Value = serde_json::from_str(notif.payload()).unwrap();
            let version = payload["version"].as_u64().unwrap();
            
            // Trigger recompilation
            state_manager.trigger_recompile(version).await;
        }
        "keto_changes" => {
            // Invalidate permission caches
            state_manager.invalidate_permission_cache().await;
        }
        "vault_rotation" => {
            // Trigger background re-encryption
            state_manager.trigger_reencryption(notif.payload()).await;
        }
        "agui_update" => {
            // Push AG-UI update to connected clients via SSE/WebSocket
            state_manager.push_agui_update(notif.payload()).await;
        }
        _ => {}
    }
}
```

### 8.3 Realtime Fabric Integration

The Realtime Fabric (Iggy spine) receives metadata change notifications and fans them out to WebSocket/SSE clients:

```
PostgreSQL NOTIFY
    ↓
flint-reflection listener
    ↓
Iggy Producer (topic: "meta.changes")
    ↓
Iggy Spine
    ↓  ┌──────────────────┐
       │ WebSocket mux    │  → Frontend clients (AG-UI updates)
       │ SSE stream       │  → Browser clients (schema updates)
       │ gRPC fanout      │  → Microservices (MCP tool updates)
       │ CRDT sync        │  → Peer nodes (federation)
       └──────────────────┘
```

---

## 9. AG-UI / A2UI Metadata Generation

### 9.1 The Metadata Hint Schema

AG-UI and A2UI require structured metadata to generate interfaces dynamically. The `flint_meta` extension stores these hints in `cache_columns.ui_hint` and generates descriptors on demand.

```json
{
  "type": "table",
  "schema": "public",
  "name": "customers",
  "fields": [
    {
      "name": "email",
      "type": "text",
      "nullable": false,
      "default": null,
      "ui": {
        "component": "email",
        "label": "Email Address",
        "placeholder": "user@example.com",
        "validators": ["email", "required"],
        "inputType": "email",
        "autocomplete": "email",
        "icon": "Mail"
      },
      "encrypted": false,
      "readable": true,
      "writable": true
    },
    {
      "name": "ssn",
      "type": "text",
      "nullable": true,
      "default": null,
      "ui": {
        "component": "password",
        "label": "Social Security Number",
        "validators": ["ssn_format"],
        "mask": "***-**-****",
        "sensitive": true
      },
      "encrypted": true,
      "readable": true,
      "writable": false
    }
  ],
  "relationships": [
    {
      "name": "orders",
      "target": "public.orders",
      "type": "one_to_many",
      "ui": {
        "component": "relation_table",
        "label": "Orders",
        "displayFields": ["id", "total", "status"]
      }
    }
  ],
  "actions": [
    {
      "name": "create_invoice",
      "path": "/rpc/public/create_invoice",
      "method": "POST",
      "ui": {
        "component": "button",
        "label": "Generate Invoice",
        "icon": "FileText",
        "color": "primary",
        "confirmation": true
      }
    }
  ],
  "permissions": {
    "read": ["admin", "user"],
    "write": ["admin"],
    "delete": ["admin"]
  }
}
```

### 9.2 AG-UI Descriptor Generation

The `flint_meta.agui_descriptor()` function (shown in §4.2) generates this JSON dynamically, filtered by the caller's identity. An AI agent or frontend can call:

```sql
SELECT flint_meta.agui_descriptor('public', 'customers', 'user:alice');
```

This returns the descriptor with:
- `readable`/`writable` fields computed from Keto permissions
- Encrypted columns marked as `sensitive`
- UI hints from `cache_columns.ui_hint`
- Actions filtered by the user's capabilities

### 9.3 A2UI Protocol Integration

A2UI (AI-to-UI) is an open standard for declarative UI generation. The Rust layer emits A2UI messages when metadata changes:

```rust
/// A2UI message types
pub enum A2uiMessage {
    CreateSurface {
        surface_id: String,
        descriptor: serde_json::Value,
    },
    UpdateComponents {
        surface_id: String,
        components: Vec<ComponentUpdate>,
    },
    UpdateDataModel {
        surface_id: String,
        data: serde_json::Value,
    },
    DeleteSurface {
        surface_id: String,
    },
}

/// When metadata changes, push A2UI update to connected clients
pub async fn push_agui_update(
    &self,
    payload: &str,
) -> Result<(), Error> {
    let update: AguiUpdate = serde_json::from_str(payload)?;
    
    let message = A2uiMessage::UpdateComponents {
        surface_id: update.table_id.to_string(),
        components: vec![ComponentUpdate {
            id: "metadata".to_string(),
            properties: json!({
                "schema": update.schema,
                "table": update.table,
                "fields": update.fields,
                "actions": update.actions,
            }),
        }],
    };
    
    // Send via SSE
    self.sse_broadcaster.send(message).await?;
    
    // Send via WebSocket
    self.ws_broadcaster.send(message).await?;
    
    Ok(())
}
```

---

## 10. Crate Structure

The project is split into six crates plus the pgrx extension:

```
flint-meta-extension/         (pgrx)
├── Cargo.toml
├── src/
│   ├── lib.rs                 (pg_module_magic!, extension entry)
│   ├── meta_schema.rs         (Schema/table creation, cache tables)
│   ├── reflection.rs          (System catalog queries → cache tables)
│   ├── event_triggers.rs      (DDL capture, refresh, invalidate)
│   ├── version.rs             (Version tracking, NOTIFY)
│   ├── keto_integration.rs    (Keto tuple storage, permission checks)
│   ├── vault_integration.rs   (Key storage, encryption/decryption hooks)
│   ├── agui_generation.rs     (AG-UI/A2UI descriptor generation)
│   ├── jwt_propagation.rs     (SET LOCAL GUC variable handling)
│   └── sql_functions.rs       (#[pg_extern] SQL-callable functions)
└── sql/
    ├── bootstrap.sql          (Initial schema creation)
    ├── event_triggers.sql     (Event trigger definitions)
    └── functions.sql            (SQL wrapper functions)

flint-reflection/               (Rust workspace crate)
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── model/
│   │   ├── mod.rs             (DatabaseModel, Table, Column, etc.)
│   │   ├── keto.rs            (KetoNamespace, KetoRelation)
│   │   ├── vault.rs           (VaultKey, VaultKeyAssignment)
│   │   └── permissions.rs     (Permission, Capability)
│   ├── compiler/
│   │   ├── mod.rs             (Compiler pipeline orchestration)
│   │   ├── normalizer.rs      (Domain resolution, default handling)
│   │   ├── validator.rs       (Cycle detection, conflict checking)
│   │   ├── permission_analyzer.rs (Keto/RLS/Cedar cross-reference)
│   │   ├── endpoint_generator.rs (REST route generation)
│   │   ├── sql_compiler.rs    (HTTP → SQL AST → Prepared Statement)
│   │   ├── openapi_compiler.rs (OpenAPI document generation)
│   │   ├── graphql_compiler.rs (GraphQL SDL generation)
│   │   ├── mcp_compiler.rs    (MCP tool manifest generation)
│   │   └── agui_compiler.rs   (AG-UI/A2UI descriptor generation)
│   ├── runtime/
│   │   ├── mod.rs             (StateManager, ArcSwap handling)
│   │   ├── listener.rs        (PostgreSQL LISTEN/NOTIFY loop)
│   │   ├── router.rs          (Axum router compilation)
│   │   ├── graphql_gateway.rs (async-graphql gateway)
│   │   └── sse_broadcaster.rs (SSE stream for metadata updates)
│   ├── handlers/
│   │   ├── mod.rs
│   │   ├── list.rs            (GET /schema/table)
│   │   ├── get.rs             (GET /schema/table/:id)
│   │   ├── create.rs          (POST /schema/table)
│   │   ├── update.rs          (PATCH /schema/table/:id)
│   │   ├── delete.rs          (DELETE /schema/table/:id)
│   │   ├── rpc.rs             (POST /rpc/:schema/:function)
│   │   └── graphql.rs         (POST /graphql)
│   ├── permissions/
│   │   ├── mod.rs             (Permission middleware)
│   │   ├── keto.rs            (Keto check integration)
│   │   ├── cedar.rs           (Cedar policy evaluation)
│   │   └── rls.rs             (Row-level security enforcement)
│   └── vault/
│       ├── mod.rs             (Vault manager)
│       ├── kms.rs             (AWS KMS / HashiCorp Vault client)
│       └── encryption.rs      (AES-256-GCM / ChaCha20-Poly1305)

flint-meta-rest/                (REST-specific crate)
├── Cargo.toml
└── src/
    └── lib.rs                 (Axum routes, REST handler logic)

flint-meta-graphql/             (GraphQL gateway crate)
├── Cargo.toml
└── src/
    └── lib.rs                 (async-graphql gateway, pg_graphql delegation)

flint-meta-openapi/             (OpenAPI generation crate)
├── Cargo.toml
└── src/
    └── lib.rs                 (utoipa OpenAPI generation)

flint-meta-server/              (Executable crate)
├── Cargo.toml
└── src/
    └── main.rs                (Server startup, config loading, signal handling)
```

---

## 11. Implementation Roadmap

### 11.1 Milestone 1: Meta Extension (pgrx)

**Duration:** 3-4 weeks  
**Goal:** Implement the `flint_meta` pgrx extension with cache tables, event triggers, and version tracking.

| Deliverable | Description |
|-------------|-------------|
| Cache tables | `cache_tables`, `cache_columns`, `cache_relationships`, `cache_functions`, `cache_policies`, `cache_types` |
| Event triggers | `ddl_command_end` and `sql_drop` triggers that refresh cache tables |
| Version tracking | `schema_version` table with auto-increment on DDL |
| SQL functions | `meta.tables()`, `meta.columns()`, `meta.functions()`, `meta.version()` |
| Reflection | pgrx functions that query `pg_catalog` and populate cache tables |
| LISTEN/NOTIFY | `pg_notify('meta_runtime', ...)` on every DDL change |
| Tests | `cargo pgrx test` across PostgreSQL 16, 17, 18 |

**Exit criteria:** `SELECT meta.tables()` returns accurate metadata; `CREATE TABLE` increments version and sends NOTIFY.

### 11.2 Milestone 2: Reflection Engine

**Duration:** 4-5 weeks  
**Goal:** Build the Rust compiler that consumes `meta.*` and produces the immutable IR.

| Deliverable | Description |
|-------------|-------------|
| DatabaseModel | Immutable IR with all metadata types |
| Reflection | SQLx queries to `flint_meta.*` tables (no `pg_catalog` direct access) |
| Version watcher | Poll or LISTEN for `meta_runtime` changes |
| ArcSwap state | `CompiledState` with `DatabaseModel`, `Router`, `OpenAPI` |
| Hot-swap | Atomic replacement without dropping requests |
| Tests | 100% coverage of model, reflection, and state management |

**Exit criteria:** Schema change triggers recompilation; old requests finish; new requests use new router.

### 11.3 Milestone 3: REST Runtime

**Duration:** 4-5 weeks  
**Goal:** Implement the Axum-based REST layer with SQL compilation.

| Deliverable | Description |
|-------------|-------------|
| Route compiler | Generate Axum routes from `DatabaseModel` |
| SQL compiler | HTTP params → SQL AST → Prepared Statement |
| Query params | `?select=`, `?order=`, `?limit=`, `?offset=`, `?embed=` |
| Filter operators | `eq`, `gt`, `lt`, `like`, `in`, `is`, `fts` |
| Resource embedding | Automatic joins via `cache_relationships` |
| Pagination | Keyset pagination with `Range` headers |
| OpenAPI | Auto-generated `/openapi.json` from `DatabaseModel` |
| Tests | Integration tests against real PostgreSQL |

**Exit criteria:** Full CRUD on any table; `GET /customers?select=*,orders(*)` works; OpenAPI valid.

### 11.4 Milestone 4: GraphQL Gateway + Permissions

**Duration:** 3-4 weeks  
**Goal:** Add GraphQL gateway, Keto integration, and JWT propagation.

| Deliverable | Description |
|-------------|-------------|
| GraphQL gateway | `async-graphql` gateway delegating CRUD to `pg_graphql` |
| Subscriptions | SSE/WebSocket for real-time GraphQL updates |
| JWT propagation | `SET LOCAL app.jwt_claims` before every query |
| Keto integration | `flint_meta.keto_check()` function; tuple storage |
| RLS enforcement | `SET LOCAL ROLE` with RLS policies |
| Cedar gating | Policy evaluation at request boundary |
| Vault integration | Column encryption/decryption via `flint_meta.decrypt_column()` |
| Tests | Permission tests, encryption tests, GraphQL integration |

**Exit criteria:** JWT flows through; Keto permissions enforced; encrypted columns transparent.

### 11.5 Milestone 5: Prometheus Reflection Platform

**Duration:** 4-6 weeks  
**Goal:** Extend the reflection system beyond SQL to generate all Prometheus-native outputs.

| Deliverable | Description |
|-------------|-------------|
| MCP compiler | Generate MCP tool manifest from `DatabaseModel` |
| AG-UI compiler | Generate AG-UI descriptors with permission filtering |
| A2UI compiler | Generate A2UI messages for real-time interface updates |
| Agent metadata | `Agent`, `Workflow`, `Prompt`, `Artifact` as reflectable types |
| Realtime integration | Push metadata changes via Iggy → WebSocket/SSE |
| Federation | Multi-node metadata synchronization |
| Documentation | Auto-generated docs from `DatabaseModel` |
| Tests | End-to-end tests with AI agents and frontend clients |

**Exit criteria:** AI agent can query metadata and generate a UI; MCP tools auto-discovered; federation works.

---

## 12. Integration with the Flint Ecosystem

### 12.1 How This Fits Together

```
┌─────────────────────────────────────────────────────────────────────┐
│                        FLINT ECOSYSTEM                               │
│                                                                      │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────────┐  │
│  │  Flint Gate  │───→│  Flint Forge │───→│  Realtime Fabric   │  │
│  │  (Axum/Rust) │JWT │  (Meta +    │    │  (Iggy/WebSocket)  │  │
│  │  Kratos/Keto │    │  Reflection)│    │  CRDT/SSE/gRPC     │  │
│  │  Cedar/Vault │    │             │    │                    │  │
│  └──────────────┘    └──────┬──────┘    └──────────────────────┘  │
│                             │                                       │
│                    ┌────────┴────────┐                              │
│                    │  PostgreSQL 18   │                              │
│                    │                  │                              │
│                    │  flint_meta      │ ←── This document            │
│                    │  flint_auth      │                              │
│                    │  flint_hooks     │                              │
│                    │  flint_llm       │                              │
│                    │  flint_vault     │                              │
│                    │  pg_graphql      │                              │
│                    │  pgvector        │                              │
│                    └──────────────────┘                              │
│                                                                      │
│  External: AWS KMS / HashiCorp Vault / Ory Keto / Ory Kratos       │
└─────────────────────────────────────────────────────────────────────┘
```

### 12.2 Data Flow Summary

| Step | Component | Action |
|------|-----------|--------|
| 1 | User | Makes HTTP request with JWT |
| 2 | Flint Gate | Validates JWT (Kratos), checks Keto (coarse), evaluates Cedar |
| 3 | Flint Gate | Mints enriched JWT with `keto_subject`, `vault_key_id` |
| 4 | Flint Forge | Receives request, propagates JWT via `SET LOCAL` |
| 5 | flint_meta | `check_permission()` queries `keto_tuples` inline |
| 6 | flint_meta | `decrypt_column()` queries `vault_keys` + calls KMS |
| 7 | RLS | PostgreSQL RLS policies filter rows based on JWT claims |
| 8 | SQLx | Executes compiled SQL, returns JSON |
| 9 | Realtime | Iggy pushes change notifications to WebSocket/SSE clients |
| 10 | AG-UI | AI agent receives `agui_descriptor()` and generates UI |

### 12.3 Key Differentiators

| Capability | PostgREST | postgres-meta | Flint Meta |
|-----------|-----------|--------------|------------|
| Schema cache location | External (Haskell heap) | External (Node.js) | **Inside PostgreSQL** |
| Cache invalidation | SIGUSR1 / NOTIFY | Polling / API call | **Event triggers + version++** |
| Metadata queries | `pg_catalog` joins | `pg_catalog` joins | **Pre-computed cache tables** |
| Keto permissions | ❌ | ❌ | **Inline SQL function** |
| Vault encryption | ❌ | ❌ | **Inline column decryption** |
| AG-UI generation | ❌ | ❌ | **Dynamic descriptor function** |
| MCP tools | ❌ | ❌ | **Auto-generated manifest** |
| Hot-swap router | ❌ | ❌ | **ArcSwap zero-downtime** |
| GraphQL | Separate resolver | Separate resolver | **pg_graphql passthrough** |
| OpenAPI | ❌ | ❌ | **Auto-generated from IR** |
| Identity propagation | ❌ | ❌ | **JWT via `SET LOCAL`** |
| Realtime metadata | ❌ | ❌ | **LISTEN/NOTIFY + Iggy** |

---

## 13. Security Considerations

### 13.1 Threat Model

| Threat | Mitigation |
|--------|------------|
| **Schema cache poisoning** | Cache tables are updated only by event triggers (inside the transaction that executed DDL). Atomic guarantee: if DDL rolls back, cache update rolls back. |
| **JWT claim forgery** | JWT is signed by Flint Gate (Kratos). The extension verifies signature via `SET LOCAL` (not client-provided). |
| **Keto tuple tampering** | `keto_tuples` table is writable only by `SECURITY DEFINER` functions. Direct DML is blocked by RLS. |
| **Key exfiltration** | DEKs are encrypted with KEK (KMS). The extension never stores plaintext DEKs. Key rotation is auditable. |
| **SQL injection in REST** | SQL compiler uses parameterized prepared statements. No string concatenation. |
| **Privilege escalation** | `flint_meta` functions run as `SECURITY DEFINER` but check `current_setting('app.jwt_claims')` before every action. |
| **Side-channel on encrypted columns** | Deterministic encryption for searchable columns; randomized encryption for sensitive data. Column-level granularity. |

### 13.2 Audit Trail

All security-relevant actions are logged:

```sql
CREATE TABLE flint_meta.audit_log (
    id          bigserial PRIMARY KEY,
    event_type  text NOT NULL,     -- 'jwt_set', 'keto_check', 'key_decrypt', 'schema_change'
    actor       text NOT NULL,     -- JWT subject
    object      text,              -- Target object
    action      text,              -- Action performed
    result      boolean,           -- Success/failure
    details     jsonb,             -- Additional context
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_audit_actor ON flint_meta.audit_log(actor, created_at);
CREATE INDEX idx_audit_event ON flint_meta.audit_log(event_type, created_at);
```

---

## 14. Performance Targets

| Metric | Target | PostgREST Equivalent |
|--------|--------|---------------------|
| Schema reload latency | < 5ms | ~50-100ms (full catalog scan) |
| Version check latency | < 1ms | N/A (external cache) |
| Hot-swap downtime | 0ms | ~100-200ms (SIGUSR1 restart) |
| Metadata query time | < 2ms | ~50-100ms (complex joins) |
| Keto permission check | < 5ms | N/A (external API call) |
| Column decryption | < 10ms | N/A (not supported) |
| AG-UI descriptor generation | < 20ms | N/A (not supported) |
| REST request → SQL | < 1ms | < 1ms (similar) |
| GraphQL resolve (pg_graphql) | < 5ms | N/A (separate resolver) |
| Cache table refresh (DDL) | < 50ms | N/A (external) |

---

## 15. Conclusion

The Flint Meta Extension represents a fundamental architectural shift from the PostgREST model:

- **PostgREST**: External cache, expensive catalog queries, manual invalidation, no identity-aware metadata, no AI-native outputs.
- **Flint Meta**: Database-owned cache, pre-computed reflection tables, automatic invalidation via event triggers, identity-propagated permissions, multi-format compilation (REST, GraphQL, OpenAPI, MCP, AG-UI).

By building the metadata layer inside PostgreSQL as a pgrx extension and the reflection layer in Rust as a compiler, we achieve:

1. **Zero external cache invalidation** — the database is the cache.
2. **Identity-aware metadata** — every query respects the caller's JWT, Keto permissions, and Cedar capabilities.
3. **Transparent encryption** — column-level encryption with per-owner keys, managed by the extension.
4. **AI-native interfaces** — AG-UI and A2UI descriptors generated from the same metadata model that drives REST and GraphQL.
5. **Real-time synchronization** — metadata changes propagate instantly via LISTEN/NOTIFY and the Realtime Fabric.
6. **Zero-downtime updates** — ArcSwap hot-swaps the compiled state without dropping requests.

This is not a PostgREST replacement. It is a **Universal Reflection Runtime** for the entire Prometheus ecosystem.

---

**Document ID:** RFC-FORGE-META-001  
**Version:** 1.0  
**Date:** June 2026  
**Status:** Architecture Design — Ready for Implementation
