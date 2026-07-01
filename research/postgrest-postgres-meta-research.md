# PostgREST, postgres-meta, and pg_graphql: Deep Technical Research

**Research Date:** 2026-06-30
**Sources:** Official documentation, GitHub source code (PostgREST v14+, postgres-meta master, pg_graphql master)

---

## 1. PostgREST Architecture

### 1.1 Overview

PostgREST is a standalone Haskell web server that turns a PostgreSQL database directly into a RESTful API. Its core philosophy is **"database as single source of truth"** — the database schema, constraints, and permissions define the API surface entirely. No ORM is involved; every HTTP request resolves to a single SQL statement executed inside PostgreSQL.

### 1.2 Schema Cache Mechanism

PostgREST maintains an in-memory **SchemaCache** data structure that stores metadata about the database schema. This cache is necessary because:

- Detecting foreign key relationships (including how they pass through views) is expensive
- Stored procedure metadata (parameters, return type, volatility) must be known to resolve RPC calls
- Overloaded functions need catalog information to disambiguate
- Resource embedding (automatic joins) requires relationship graph knowledge

#### SchemaCache Data Structure (from `SchemaCache.hs`)

```haskell
data SchemaCache = SchemaCache
  { dbTables          :: TablesMap        -- ^ Tables, views, mat views, foreign tables
  , dbRelationships   :: RelationshipsMap   -- ^ M2O, O2O, O2M, M2M relationships
  , dbRoutines        :: RoutineMap       -- ^ Functions/procedures for RPC
  , dbRepresentations :: RepresentationsMap -- ^ Domain type casting
  , dbMediaHandlers   :: MediaHandlerMap  -- ^ Custom media type handlers
  , dbTimezones       :: TimezoneNames    -- ^ Valid timezone names
  , dbTablesFuzzyIndex :: TablesFuzzyIndex -- ^ Approximate name matching
  }
```

The cache is loaded **once at startup** and must be reloaded when DDL changes occur.

### 1.3 pg_catalog Queries Used by Schema Cache

PostgREST executes several expensive system catalog queries at startup/reload. These directly query `pg_catalog` tables rather than using `information_schema` views because the latter include privilege filters that can be overly restrictive.

#### 1.3.1 Tables Query (`allTables`)

This query discovers tables, views, materialized views, foreign tables, and partitioned tables, along with their columns, primary keys, and mutability flags.

Key catalog tables joined:
- `pg_class` (relation metadata: name, kind, namespace, RLS settings)
- `pg_namespace` (schema names)
- `pg_attribute` (column definitions)
- `pg_type` / `pg_type bt` (data types, including domain base types)
- `pg_attrdef` (column defaults)
- `pg_constraint` (primary key constraints)
- `pg_description` (comments/descriptions)
- `pg_depend` (sequence dependencies for identity columns)

Key SQL patterns used:

```sql
-- Column default expression extraction (handles domains, identity, generated)
CASE
  WHEN (t.typbasetype != 0) AND (ad.adbin IS NULL) THEN pg_get_expr(t.typdefaultbin, 0)
  WHEN a.attidentity = 'd' THEN format('nextval(%L)', seq.objid::regclass)
  WHEN a.attgenerated = 's' THEN null
  ELSE pg_get_expr(ad.adbin, ad.adrelid)::text
END AS column_default

-- Type formatting with domain resolution
CASE
  WHEN t.typtype = 'd' THEN
    CASE WHEN bt.base_namespace = 'pg_catalog'::regnamespace
         THEN format_type(bt.base_type, NULL::integer)
         ELSE format_type(a.atttypid, a.atttypmod)
    END
  ELSE
    CASE WHEN t.typnamespace = 'pg_catalog'::regnamespace
         THEN format_type(a.atttypid, NULL::integer)
         ELSE format_type(a.atttypid, a.atttypmod)
    END
END::text AS data_type

-- Updatability check using pg_relation_is_updatable bitmask
-- CMD_INSERT = 1 << 3 = 8
-- CMD_UPDATE = 1 << 2 = 4
-- CMD_DELETE = 1 << 4 = 16
(
  c.relkind IN ('r','p')
  OR (
    c.relkind in ('v','f')
    AND (pg_relation_is_updatable(c.oid::regclass, TRUE) & 8) = 8
  )
) AS insertable
```

#### 1.3.2 Relationships Query (`allM2OandO2ORels`)

Discovers many-to-one and one-to-one relationships by analyzing foreign key constraints.

Key catalog tables:
- `pg_constraint` (foreign key constraints, `contype = 'f'`)
- `pg_class` (local and referenced tables)
- `pg_namespace` (schemas)
- `pg_attribute` (key columns on both sides)

```sql
-- Unnest conkey/confkey arrays to resolve column names
FROM unnest(traint.conkey, traint.confkey) WITH ORDINALITY AS _(col, ref, ord)
JOIN pg_attribute cols ON cols.attrelid = traint.conrelid AND cols.attnum = col
JOIN pg_attribute refs ON refs.attrelid = traint.confrelid AND refs.attnum = ref
```

The query also detects one-to-one relationships by checking if the local side of the FK is also a primary key or unique constraint.

#### 1.3.3 View Key Dependencies (`allViewsKeyDependencies`)

This is one of the most complex queries. It traces primary key and foreign key columns through view definitions by parsing `pg_rewrite.ev_action` (the internal query tree stored as `pg_node_tree` text).

The query transforms the `pg_node_tree` format into JSON via a chain of `replace()` and `regexp_replace()` calls, then recursively traverses the view dependency graph using a `WITH RECURSIVE` CTE to find which base table columns each view column references.

```sql
-- Heavy regex-based transformation of pg_node_tree to JSON
replace(
  replace(
    regexp_replace(
      replace(view_definition::text, '<>', '()'),
      ' :[^}{,]+', ',"":', 'g'
    ),
    '(', '['
  ),
  ')', ']'
)::json as view_definition
```

#### 1.3.4 Functions Query (`allFunctions`)

Discovers stored procedures callable via `/rpc` endpoints.

Key catalog tables:
- `pg_proc` (function definitions, args, return types, volatility)
- `pg_namespace` (schema)
- `pg_type` (return and argument types)
- `pg_description` (comments)

```sql
-- Argument parsing using unnest with ordinality on proargnames, proargtypes, proargmodes
unnest(proargnames, proargtypes, proargmodes) WITH ORDINALITY AS _ (name, type, mode, idx)

-- Base type recursion for domains
WITH RECURSIVE recurse AS (
  SELECT oid, typbasetype, typnamespace AS base_namespace,
         COALESCE(NULLIF(typbasetype, 0), oid) AS base_type
  FROM pg_type
  UNION
  SELECT t.oid, b.typbasetype, b.typnamespace AS base_namespace,
         COALESCE(NULLIF(b.typbasetype, 0), b.oid) AS base_type
  FROM recurse t JOIN pg_type b ON t.typbasetype = b.oid
)
```

#### 1.3.5 Data Representations Query (`dataRepresentations`)

Discovers implicit casts between domains and JSON/text types for automatic serialization.

```sql
SELECT c.castsource::regtype::text, c.casttarget::regtype::text, c.castfunc::regproc::text
FROM pg_catalog.pg_cast c
JOIN pg_catalog.pg_type src_t ON c.castsource::oid = src_t.oid
JOIN pg_catalog.pg_type dst_t ON c.casttarget::oid = dst_t.oid
WHERE c.castcontext = 'i'  -- implicit
  AND c.castmethod = 'f'  -- function-based
  AND has_function_privilege(c.castfunc, 'execute')
  AND ((src_t.typtype = 'd' AND c.casttarget IN ('json'::regtype::oid, 'text'::regtype::oid))
    OR (dst_t.typtype = 'd' AND c.castsource IN ('json'::regtype::oid, 'text'::regtype::oid)))
```

### 1.4 HTTP-to-SQL Mapping

PostgREST maps HTTP verbs and URL patterns to SQL operations:

| HTTP Verb | URL Pattern | SQL Operation |
|-----------|-------------|---------------|
| `GET` | `/table` | `SELECT ... FROM table` |
| `GET` | `/table?id=eq.1` | `SELECT ... FROM table WHERE id = 1` |
| `POST` | `/table` | `INSERT INTO table ... RETURNING ...` |
| `PATCH` | `/table?id=eq.1` | `UPDATE table SET ... WHERE id = 1 RETURNING ...` |
| `DELETE` | `/table?id=eq.1` | `DELETE FROM table WHERE id = 1 RETURNING ...` |
| `POST` | `/rpc/function` | `SELECT ... FROM function(...)` |

- **Resource Embedding**: `GET /projects?select=*,clients(*)` generates a SQL join using the detected foreign key relationship
- **Query parameters**: Filter operators (`eq`, `gt`, `lt`, `like`, `in`, etc.) are translated to `WHERE` clauses
- **Order**: `?order=col.asc` → `ORDER BY col ASC`
- **Pagination**: `?limit=10&offset=20` → `LIMIT 10 OFFSET 20`

### 1.5 DDL Change Handling: LISTEN/NOTIFY and Event Triggers

PostgREST has three mechanisms to reload the schema cache when DDL changes occur:

#### 1.5.1 UNIX Signal (SIGUSR1 / SIGUSR2)

```bash
killall -SIGUSR1 postgrest
# or in Docker:
docker kill -s SIGUSR1 <container>
```

#### 1.5.2 PostgreSQL NOTIFY

From inside the database, any client can trigger a reload:

```sql
NOTIFY pgrst, 'reload schema';
```

PostgREST `LISTEN`s on the `pgrst` channel and reloads the cache asynchronously.

#### 1.5.3 Automatic Reloading with Event Triggers

The recommended approach uses PostgreSQL event triggers to automatically notify on DDL changes:

**Coarse-grained trigger** (catches everything):

```sql
CREATE OR REPLACE FUNCTION pgrst_watch() RETURNS event_trigger
  LANGUAGE plpgsql
  AS $$
BEGIN
  NOTIFY pgrst, 'reload schema';
END;
$$;

CREATE EVENT TRIGGER pgrst_watch
  ON ddl_command_end
  EXECUTE PROCEDURE pgrst_watch();
```

**Fine-grained trigger** (only relevant commands, excludes `pg_temp`):

```sql
-- Watch CREATE and ALTER
CREATE OR REPLACE FUNCTION pgrst_ddl_watch() RETURNS event_trigger AS $$
DECLARE
  cmd record;
BEGIN
  FOR cmd IN SELECT * FROM pg_event_trigger_ddl_commands()
  LOOP
    IF cmd.command_tag IN (
      'CREATE SCHEMA', 'ALTER SCHEMA'
    , 'CREATE TABLE', 'CREATE TABLE AS', 'SELECT INTO', 'ALTER TABLE'
    , 'CREATE FOREIGN TABLE', 'ALTER FOREIGN TABLE'
    , 'CREATE VIEW', 'ALTER VIEW'
    , 'CREATE MATERIALIZED VIEW', 'ALTER MATERIALIZED VIEW'
    , 'CREATE FUNCTION', 'ALTER FUNCTION'
    , 'CREATE TRIGGER'
    , 'CREATE TYPE', 'ALTER TYPE'
    , 'CREATE RULE'
    , 'COMMENT'
    )
    AND cmd.schema_name is distinct from 'pg_temp'
    THEN
      NOTIFY pgrst, 'reload schema';
    END IF;
  END LOOP;
END; $$ LANGUAGE plpgsql;

-- Watch DROP
CREATE OR REPLACE FUNCTION pgrst_drop_watch() RETURNS event_trigger AS $$
DECLARE
  obj record;
BEGIN
  FOR obj IN SELECT * FROM pg_event_trigger_dropped_objects()
  LOOP
    IF obj.object_type IN (
      'schema', 'table', 'foreign table', 'view'
    , 'materialized view', 'function', 'trigger', 'type', 'rule'
    )
    AND obj.is_temporary IS false
    THEN
      NOTIFY pgrst, 'reload schema';
    END IF;
  END LOOP;
END; $$ LANGUAGE plpgsql;

CREATE EVENT TRIGGER pgrst_ddl_watch
  ON ddl_command_end
  EXECUTE PROCEDURE pgrst_ddl_watch();

CREATE EVENT TRIGGER pgrst_drop_watch
  ON sql_drop
  EXECUTE PROCEDURE pgrst_drop_watch();
```

**Important design notes:**
- Requests wait until the schema cache reload is complete to prevent stale cache errors
- The event trigger functions use `pg_event_trigger_ddl_commands()` and `pg_event_trigger_dropped_objects()` to inspect what changed
- The `is_temporary` / `pg_temp` checks prevent reloading when temporary objects are created inside functions

---

## 2. postgres-meta

### 2.1 Overview

postgres-meta is a RESTful API server (written in TypeScript/Node.js) for managing PostgreSQL metadata and schema objects. It serves as the backend for Supabase Studio's table editor and other management UI. Unlike PostgREST which exposes data APIs, postgres-meta exposes **management APIs** for schema inspection and mutation.

### 2.2 API Surface

#### Core Schema Endpoints

| Endpoint | Methods | SQL Operation |
|----------|---------|---------------|
| `/tables` | GET, POST, PATCH, DELETE | List, create, alter, drop tables |
| `/columns` | GET, POST, PATCH, DELETE | List, add, alter/rename, drop columns |
| `/functions` | GET, POST, PATCH, DELETE | List, create, alter, drop functions |
| `/triggers` | GET, POST, PATCH, DELETE | List, create, alter, drop triggers |
| `/extensions` | GET, POST, PATCH, DELETE | List, create, alter, drop extensions |
| `/schemas` | GET, POST, PATCH, DELETE | List, create, alter, drop schemas |
| `/roles` | GET, POST, PATCH, DELETE | List, create, alter, drop roles |
| `/publications` | GET, POST, PATCH, DELETE | List, create, alter, drop publications |
| `/types` | GET, POST, PATCH, DELETE | List, create, alter, drop types |
| `/relationships` | GET | List foreign key relationships |

#### Helper Endpoints

| Endpoint | Description |
|----------|-------------|
| `POST /query` | Execute arbitrary SQL query |
| `POST /format` | Format SQL query with prettier |
| `POST /parse` | Parse SQL into AST |
| `POST /explain` | EXPLAIN a SQL query |
| `GET /config/version` | PostgreSQL version info |
| `GET /generators/openapi` | Generate OpenAPI spec |
| `GET /generators/typescript` | Generate TypeScript types |
| `GET /generators/swift` | Generate Swift types |
| `GET /generators/python` | Generate Python types |

### 2.3 SQL Catalog Queries

postgres-meta normalizes `pg_catalog` output into a JSON-friendly structure. It directly queries `pg_catalog` tables rather than using `information_schema` views.

#### 2.3.1 Tables Query (`table.sql.ts`)

```sql
SELECT
  c.oid :: int8 AS id,
  nc.nspname AS schema,
  c.relname AS name,
  c.relrowsecurity AS rls_enabled,
  c.relforcerowsecurity AS rls_forced,
  CASE
    WHEN c.relreplident = 'd' THEN 'DEFAULT'
    WHEN c.relreplident = 'i' THEN 'INDEX'
    WHEN c.relreplident = 'f' THEN 'FULL'
    ELSE 'NOTHING'
  END AS replica_identity,
  pg_total_relation_size(format('%I.%I', nc.nspname, c.relname)) :: int8 AS bytes,
  pg_size_pretty(pg_total_relation_size(format('%I.%I', nc.nspname, c.relname))) AS size,
  pg_stat_get_live_tuples(c.oid) AS live_rows_estimate,
  pg_stat_get_dead_tuples(c.oid) AS dead_rows_estimate,
  obj_description(c.oid) AS comment,
  coalesce(pk.primary_keys, '[]') as primary_keys,
  coalesce(
    jsonb_agg(relationships) filter (where relationships is not null),
    '[]'
  ) as relationships
FROM pg_namespace nc
JOIN pg_class c ON nc.oid = c.relnamespace
-- ... primary_keys subquery using pg_index, pg_attribute ...
-- ... relationships subquery using pg_constraint, pg_attribute ...
WHERE c.relkind IN ('r', 'p')
  AND NOT pg_is_other_temp_schema(nc.oid)
  AND (
    pg_has_role(c.relowner, 'USAGE')
    OR has_table_privilege(c.oid, 'SELECT, INSERT, UPDATE, DELETE, TRUNCATE, REFERENCES, TRIGGER')
    OR has_any_column_privilege(c.oid, 'SELECT, INSERT, UPDATE, REFERENCES')
  )
GROUP BY c.oid, c.relname, c.relrowsecurity, c.relforcerowsecurity,
         c.relreplident, nc.nspname, pk.primary_keys
```

#### 2.3.2 Columns Query (`columns.sql.ts`)

Adapted from `information_schema.columns` but directly querying `pg_catalog`:

```sql
SELECT
  c.oid :: int8 AS table_id,
  nc.nspname AS schema,
  c.relname AS table,
  (c.oid || '.' || a.attnum) AS id,
  a.attnum AS ordinal_position,
  a.attname AS name,
  CASE WHEN a.atthasdef THEN pg_get_expr(ad.adbin, ad.adrelid) ELSE NULL END AS default_value,
  -- Type resolution with domain/array handling
  CASE
    WHEN t.typtype = 'd' THEN CASE
      WHEN bt.typelem <> 0 :: oid AND bt.typlen = -1 THEN 'ARRAY'
      WHEN nbt.nspname = 'pg_catalog' THEN format_type(t.typbasetype, NULL)
      ELSE 'USER-DEFINED'
    END
    ELSE CASE
      WHEN t.typelem <> 0 :: oid AND t.typlen = -1 THEN 'ARRAY'
      WHEN nt.nspname = 'pg_catalog' THEN format_type(a.atttypid, NULL)
      ELSE 'USER-DEFINED'
    END
  END AS data_type,
  COALESCE(bt.typname, t.typname) AS format,
  a.attidentity IN ('a', 'd') AS is_identity,
  CASE a.attidentity WHEN 'a' THEN 'ALWAYS' WHEN 'd' THEN 'BY DEFAULT' ELSE NULL END AS identity_generation,
  a.attgenerated IN ('s') AS is_generated,
  NOT (a.attnotnull OR t.typtype = 'd' AND t.typnotnull) AS is_nullable,
  (
    c.relkind IN ('r', 'p')
    OR c.relkind IN ('v', 'f') AND pg_column_is_updatable(c.oid, a.attnum, FALSE)
  ) AS is_updatable,
  uniques.table_id IS NOT NULL AS is_unique,
  check_constraints.definition AS "check",
  array_to_json(array(
    SELECT enumlabel FROM pg_catalog.pg_enum enums
    WHERE enums.enumtypid = coalesce(bt.oid, t.oid)
       OR enums.enumtypid = coalesce(bt.typelem, t.typelem)
    ORDER BY enums.enumsortorder
  )) AS enums,
  col_description(c.oid, a.attnum) AS comment
FROM pg_attribute a
LEFT JOIN pg_attrdef ad ON a.attrelid = ad.adrelid AND a.attnum = ad.adnum
JOIN (pg_class c JOIN pg_namespace nc ON c.relnamespace = nc.oid)
  ON a.attrelid = c.oid
JOIN (pg_type t JOIN pg_namespace nt ON t.typnamespace = nt.oid)
  ON a.atttypid = t.oid
LEFT JOIN (pg_type bt JOIN pg_namespace nbt ON bt.typnamespace = nbt.oid)
  ON t.typtype = 'd' AND t.typbasetype = bt.oid
-- ... unique/check constraint subqueries ...
WHERE a.attnum > 0 AND NOT a.attisdropped
  AND (c.relkind IN ('r', 'v', 'm', 'f', 'p'))
  AND (
    pg_has_role(c.relowner, 'USAGE')
    OR has_column_privilege(c.oid, a.attnum, 'SELECT, INSERT, UPDATE, REFERENCES')
  )
```

#### 2.3.3 Functions Query (`functions.sql.ts`)

```sql
WITH functions AS (
  SELECT p.*,
    -- Normalize argument arrays to always be same length
    coalesce(p.proargmodes,
      array_fill('i'::text, array[cardinality(coalesce(p.proallargtypes, p.proargtypes))])
    ) as arg_modes,
    coalesce(p.proargnames,
      array_fill(''::text, array[cardinality(coalesce(p.proallargtypes, p.proargtypes))])
    ) as arg_names,
    coalesce(p.proallargtypes, p.proargtypes) as arg_types,
    array_cat(
      array_fill(false, array[pronargs - pronargdefaults]),
      array_fill(true, array[pronargdefaults])
    ) as arg_has_defaults
  FROM pg_proc AS p
  WHERE p.prokind = 'f'
)
SELECT
  f.oid::int8 as id,
  n.nspname as schema,
  f.proname as name,
  l.lanname as language,
  case when l.lanname = 'internal' then '' else f.prosrc end as definition,
  case when l.lanname = 'internal' then f.prosrc else pg_get_functiondef(f.oid) end as complete_statement,
  coalesce(f_args.args, '[]') as args,
  pg_get_function_arguments(f.oid) as argument_types,
  pg_get_function_identity_arguments(f.oid) as identity_argument_types,
  f.prorettype::int8 as return_type_id,
  pg_get_function_result(f.oid) as return_type,
  f.proretset as is_set_returning_function,
  case f.provolatile
    when 'i' then 'IMMUTABLE'
    when 's' then 'STABLE'
    when 'v' then 'VOLATILE'
  end as behavior,
  f.prosecdef as security_definer,
  f_config.config_params as config_params
FROM functions f
LEFT JOIN pg_namespace n ON f.pronamespace = n.oid
LEFT JOIN pg_language l ON f.prolang = l.oid
LEFT JOIN pg_type rt ON rt.oid = f.prorettype
-- ... args aggregation using unnest ...
```

#### 2.3.4 Relationships Query (`table_relationships.sql.ts`)

This query is adapted from PostgREST's own relationship query:

```sql
WITH pks_uniques_cols AS (
  SELECT connamespace, conrelid, jsonb_agg(column_info.cols) as cols
  FROM pg_constraint
  JOIN LATERAL (
    SELECT array_agg(cols.attname order by cols.attnum) as cols
    FROM (SELECT unnest(conkey) AS col) _
    JOIN pg_attribute cols ON cols.attrelid = conrelid AND cols.attnum = col
  ) column_info ON TRUE
  WHERE contype IN ('p', 'u')
  GROUP BY connamespace, conrelid
)
SELECT
  traint.conname AS foreign_key_name,
  ns1.nspname AS schema,
  tab.relname AS relation,
  column_info.cols AS columns,
  ns2.nspname AS referenced_schema,
  other.relname AS referenced_relation,
  column_info.refs AS referenced_columns,
  (column_info.cols IN (SELECT * FROM jsonb_array_elements(pks_uqs.cols))) AS is_one_to_one
FROM pg_constraint traint
JOIN LATERAL (
  SELECT
    jsonb_agg(cols.attname order by ord) AS cols,
    jsonb_agg(refs.attname order by ord) AS refs
  FROM unnest(traint.conkey, traint.confkey) WITH ORDINALITY AS _(col, ref, ord)
  JOIN pg_attribute cols ON cols.attrelid = traint.conrelid AND cols.attnum = col
  JOIN pg_attribute refs ON refs.attrelid = traint.confrelid AND refs.attnum = ref
) AS column_info ON TRUE
JOIN pg_namespace ns1 ON ns1.oid = traint.connamespace
JOIN pg_class tab ON tab.oid = traint.conrelid
JOIN pg_class other ON other.oid = traint.confrelid
JOIN pg_namespace ns2 ON ns2.oid = other.relnamespace
LEFT JOIN pks_uniques_cols pks_uqs ON pks_uqs.connamespace = traint.connamespace AND pks_uqs.conrelid = traint.conrelid
WHERE traint.contype = 'f' AND traint.conparentid = 0
```

### 2.4 Mutations

postgres-meta translates REST mutations into DDL statements:

**Create table:**
```typescript
async create({ name, schema = 'public', comment }: PostgresTableCreate) {
  const tableSql = `CREATE TABLE ${ident(schema)}.${ident(name)} ();`;
  const commentSql = comment === undefined ? ''
    : `COMMENT ON TABLE ${ident(schema)}.${ident(name)} IS ${literal(comment)};`;
  const sql = `BEGIN; ${tableSql} ${commentSql} COMMIT;`;
  // ...
}
```

**Alter table** (complex multi-step transaction):
```typescript
async update(id: number, { name, schema, rls_enabled, rls_forced, replica_identity, primary_keys, comment }: PostgresTableUpdate) {
  // 1. Retrieve old table info
  // 2. Build ALTER statements for each change:
  //    - SET SCHEMA
  //    - RENAME TO
  //    - ENABLE/DISABLE ROW LEVEL SECURITY
  //    - FORCE/NO FORCE ROW LEVEL SECURITY
  //    - REPLICA IDENTITY
  //    - DROP CONSTRAINT + ADD PRIMARY KEY
  //    - COMMENT ON TABLE
  // 3. Execute in a single BEGIN/COMMIT block
  // 4. Re-fetch to return updated record
}
```

### 2.5 JSON Normalization Strategy

postgres-meta normalizes pg_catalog output through several techniques:

1. **Type casting**: `::int8`, `::text`, `::bigint` to ensure consistent JSON serializable types
2. **Array aggregation**: `array_agg(...)`, `jsonb_agg(...)`, `coalesce(..., '[]')` to group related objects
3. **Privilege filtering**: `has_table_privilege`, `has_column_privilege`, `pg_has_role` to only show accessible objects
4. **Enum extraction**: `array_to_json(array(SELECT enumlabel FROM pg_enum ...))` for enum type columns
5. **Default expression extraction**: `pg_get_expr(ad.adbin, ad.adrelid)` to show column defaults as text
6. **Size formatting**: `pg_size_pretty(pg_total_relation_size(...))` for human-readable sizes alongside raw bytes

---

## 3. pg_graphql

### 3.1 Overview

pg_graphql is a PostgreSQL extension written in Rust (using pgrx) that adds GraphQL support directly inside the database. It reflects an existing SQL schema into a GraphQL schema and exposes a single SQL function `graphql.resolve(...)` for query execution. All parsing, schema generation, and resolution happen inside the database process — no external servers required.

### 3.2 Architecture

The extension consists of several Rust modules:

| Module | Purpose |
|--------|---------|
| `sql_types.rs` | Rust data structures representing the reflected SQL schema |
| `graphql.rs` | GraphQL schema construction (types, fields, connections) |
| `builder.rs` | Query builders for connections, nodes, mutations, function calls |
| `transpile.rs` | SQL generation from GraphQL AST |
| `resolve.rs` | Top-level entry point: parse, validate, execute |
| `parser_util.rs` | GraphQL AST manipulation helpers |

### 3.3 Schema Reflection

#### 3.3.1 SQL Context Loading

When `graphql.resolve()` is called, it first loads the SQL schema context via a single massive SQL query (`load_sql_context.sql`). This query reads the entire relevant `pg_catalog` state and returns it as a single JSON object with these top-level keys:

- `config`: search_path, role, schema_version
- `enums`: all enum types with their values
- `types`: all type mappings (array, composite, table, other)
- `composites`: composite type definitions
- `foreign_keys`: all foreign key constraints with column mappings
- `schemas`: schema-level directives (from `COMMENT ON SCHEMA`)
- `tables`: all tables/views/mat-views/foreign tables with columns, indexes, permissions, directives
- `functions`: all functions with args, return types, volatility, permissions, directives

Key catalog tables queried:
- `pg_namespace` (schemas)
- `pg_class` (tables, views, indexes)
- `pg_attribute` (columns)
- `pg_type` (types, including enums)
- `pg_enum` (enum values)
- `pg_constraint` (foreign keys)
- `pg_index` (primary/unique indexes)
- `pg_proc` (functions)
- `pg_description` / `col_description` (comment directives)

#### 3.3.2 Directives via SQL Comments

pg_graphql uses PostgreSQL `COMMENT` statements to configure the GraphQL schema:

```sql
-- Schema-level: enable name inflection (snake_case → camelCase/PascalCase)
COMMENT ON SCHEMA public IS e'@graphql({"inflect_names": true})';

-- Table-level: custom name, enable totalCount, enable aggregates, set max_rows
COMMENT ON TABLE my_table IS e'@graphql({
  "name": "MyCustomName",
  "totalCount": {"enabled": true},
  "aggregate": {"enabled": true},
  "max_rows": 100
})';

-- Column-level: custom name/description
COMMENT ON COLUMN my_table.my_col IS e'@graphql({"name": "myCol", "description": "..."})';

-- Function-level: custom name/description
COMMENT ON FUNCTION my_func() IS e'@graphql({"name": "myFunc"})';
```

The `graphql.comment_directive()` SQL function parses these JSON fragments from comments.

#### 3.3.3 GraphQL Schema Construction

From the loaded SQL context, pg_graphql constructs a GraphQL schema in memory:

- **Query type**: Each table gets a `tableNameCollection` (paginated connection) and `tableNameByPk` (single row lookup) field
- **Mutation type**: Each table gets `insertIntoTableName`, `updateTableName`, `deleteFromTableName` fields
- **Connection types**: Follow Relay connection spec (`edges { node { ... } }`) with cursor-based pagination
- **Relationship fields**: Foreign keys become nested object fields (one-to-many, many-to-one)
- **Function fields**: Single-argument functions returning table types become computed fields
- **Type mapping**: PostgreSQL types map to GraphQL scalars (e.g., `int4` → `Int`, `text` → `String`, `timestamptz` → `Datetime`)

### 3.4 `graphql.resolve()` Function

The entry point is defined in `lib.rs`:

```rust
#[pg_extern(name = "_internal_resolve")]
fn resolve(
    query: &str,
    variables: default!(Option<JsonB>, "'{}'"),
    operationName: default!(Option<&str>, "null"),
    extensions: default!(Option<JsonB>, "null"),
) -> pgrx::JsonB {
    // 1. Parse GraphQL query text into AST
    let query_ast = parse_query::<&str>(query);

    match query_ast {
        Err(err) => { /* return parser error */ }
        Ok(document) => {
            // 2. Load SQL config (search_path, role, schema_version)
            let sql_config = sql_types::load_sql_config();
            // 3. Load SQL context (tables, columns, fks, functions, types)
            let context = sql_types::load_sql_context(&sql_config);

            match context {
                Ok(context) => {
                    let graphql_schema = __Schema { context };
                    // 4. Resolve the query against the GraphQL schema
                    resolve_inner(document, &variables, &operationName, &graphql_schema)
                }
                Err(err) => { /* return schema loading error */ }
            }
        }
    }
}
```

The resolution pipeline (`resolve.rs`):

1. **Parse** the GraphQL document into AST
2. **Extract** operation definitions and fragment definitions
3. **Validate** operation names and detect fragment cycles
4. **Select** the operation to execute (by name or if only one exists)
5. **Resolve** the selection set against the schema type (Query, Mutation, or Subscription)
6. **Build** query builders (Connection, Node, NodeByPk, FunctionCall, Insert, Update, Delete)
7. **Execute** builders which generate and run SQL via SPI (Server Programming Interface)
8. **Return** JSON response with `data` and `errors`

### 3.5 Query Resolution Inside PostgreSQL

#### 3.5.1 Query Execution Path

For a `Query` operation, the resolver iterates over top-level fields:

```rust
for selection in selections.iter() {
    let maybe_field_def = map.get(selection.name.as_ref());
    match maybe_field_def {
        Some(field_def) => match field_def.type_.unmodified_type() {
            __Type::Connection(_) => {
                // Paginated collection query
                let builder = to_connection_builder(...);
                let data = builder.execute()?;
                res_data[alias_or_name(selection)] = data;
            }
            __Type::NodeInterface(_) => {
                // Relay node lookup by global ID
                let builder = to_node_builder(...);
                let data = builder.execute()?;
            }
            __Type::Node(_) => {
                // Single row lookup by PK
                let builder = to_node_by_pk_builder(...);
                let data = builder.execute()?;
            }
            __Type::__Type(_) => {
                // Introspection: __type query
                let builder = schema_type.to_type_builder(...);
                res_data[alias_or_name(selection)] = serde_json::json!(builder);
            }
            __Type::__Schema(_) => {
                // Introspection: __schema query
                let builder = schema_type.to_schema_builder(...);
                res_data[alias_or_name(selection)] = serde_json::json!(builder);
            }
            _ => {
                // Function call (RPC-like)
                let builder = to_function_call_builder(...);
                let data = builder.execute()?;
            }
        }
    }
}
```

#### 3.5.2 Mutation Execution Path

Mutations use `Spi::connect_mut` for a mutable SPI session that can execute multiple SQL statements:

```rust
let spi_result: GraphQLResult<JsonB> = Spi::connect_mut(|mut conn| {
    for selection in selections.iter() {
        match field_def.type_.unmodified_type() {
            __Type::InsertResponse(_) => {
                let builder = to_insert_builder(...)?;
                let (data, conn) = builder.execute(conn)?;
                res_data[alias_or_name(selection)] = data;
            }
            __Type::UpdateResponse(_) => {
                let builder = to_update_builder(...)?;
                let (data, conn) = builder.execute(conn)?;
            }
            __Type::DeleteResponse(_) => {
                let builder = to_delete_builder(...)?;
                let (data, conn) = builder.execute(conn)?;
            }
            _ => {
                let builder = to_function_call_builder(...)?;
                let (data, conn) = builder.execute(conn)?;
            }
        }
    }
    Ok(res_data)
});
```

### 3.6 SQL Generation (Transpilation)

The `transpile.rs` module converts GraphQL query ASTs into SQL. Key patterns:

- **Connection queries**: Generate `SELECT ... FROM table WHERE ... ORDER BY ... LIMIT ... OFFSET ...` with cursor encoding/decoding
- **Node queries**: `SELECT ... FROM table WHERE id = $1`
- **NodeByPk queries**: `SELECT ... FROM table WHERE pk_col1 = $1 AND pk_col2 = $2`
- **Insert mutations**: `INSERT INTO table (...) VALUES (...) RETURNING ...`
- **Update mutations**: `UPDATE table SET ... WHERE ... RETURNING ...`
- **Delete mutations**: `DELETE FROM table WHERE ... RETURNING ...`
- **Function calls**: `SELECT * FROM function($1, $2, ...)`

All SQL generation respects:
- Row Level Security (RLS) policies (executes as the current user)
- Column-level privileges (only selects columns the user can see)
- `max_rows` limits (from schema or table directives)
- Search path (only schemas in `current_schemas(false)` are visible)

### 3.7 Caching

pg_graphql uses a **SizedCache** (from the `cached` Rust crate) for the SQL context:

```rust
#[cached(
    type = "SizedCache<String, Arc<Context>>",
    create = "{ SizedCache::with_size(250) }",
    convert = r#"{ calculate_hash(_config) }"#
)]
pub fn load_sql_context(_config: &Config) -> GraphQLResult<Arc<Context>> {
    let query = include_str!("../sql/load_sql_context.sql");
    let sql_result: serde_json::Value = get_one_readonly::<(JsonB,)>(query)
        .expect("failed to read sql context")
        .expect("sql context is missing")
        .0;
    // ... parse and cross-reference types, columns, functions ...
}
```

The cache key is a hash of the `Config` (search_path + role + schema_version). This means:
- Same user + same search path + same schema version = cached context
- Schema changes require calling `graphql.rebuild_schema()` to bump the schema version

### 3.8 Rebuilding Schema

Unlike PostgREST which uses LISTEN/NOTIFY, pg_graphql requires **explicit** schema rebuilding after DDL changes:

```sql
-- After creating/altering/dropping tables, run:
SELECT graphql.rebuild_schema();
```

This increments the internal schema version, invalidating the cache so the next `graphql.resolve()` call loads fresh metadata.

---

## 4. Comparative Analysis

### 4.1 Schema Discovery Approaches

| Aspect | PostgREST | postgres-meta | pg_graphql |
|--------|-----------|---------------|------------|
| **Trigger** | HTTP request needs schema cache | Management UI needs metadata | GraphQL query needs schema |
| **Catalog queries** | Multiple targeted queries (tables, rels, funcs, views) | Multiple targeted queries per endpoint | One massive JSON query (`load_sql_context.sql`) |
| **Caching** | In-memory Haskell cache with explicit reload | No cache (queries DB each time) | Rust `SizedCache` keyed by config hash |
| **Reload mechanism** | SIGUSR1, NOTIFY, event triggers | N/A (always fresh) | `graphql.rebuild_schema()` explicit call |
| **Privilege filtering** | Minimal (uses `pg_has_role`, `has_*_privilege`) | Yes (`has_table_privilege`, `has_column_privilege`) | Yes (`has_table_privilege`, `has_column_privilege`, `has_schema_privilege`) |
| **Direct pg_catalog use** | Heavy | Heavy | Heavy |

### 4.2 SQL Patterns Summary

All three tools share common SQL patterns for catalog introspection:

1. **Type resolution chain**: `pg_attribute → pg_type → pg_type (base type)` for domains
2. **Default extraction**: `pg_get_expr(ad.adbin, ad.adrelid)` from `pg_attrdef`
3. **Enum extraction**: `pg_enum` joined by `enumtypid`
4. **Constraint analysis**: `pg_constraint` with `unnest(conkey)` / `unnest(confkey)`
5. **Comment extraction**: `obj_description(oid, 'pg_class')` and `col_description(oid, attnum)`
6. **Privilege checking**: `has_table_privilege`, `has_column_privilege`, `pg_has_role`
7. **View dependency tracing**: `pg_rewrite.ev_action` parsed via regex transformations

### 4.3 Architectural Design Decisions

| Decision | PostgREST | postgres-meta | pg_graphql |
|----------|-----------|---------------|------------|
| **Language** | Haskell | TypeScript/Node.js | Rust (pgrx) |
| **Process model** | Standalone server | Standalone server | In-database extension |
| **HTTP exposure** | Direct (port 3000) | Behind proxy/API gateway | Via PostgREST RPC (`/rpc/graphql.resolve`) |
| **Transaction model** | One transaction per HTTP request | One transaction per management operation | One transaction per GraphQL query (mutations use mutable SPI) |
| **Schema staleness handling** | Event triggers + NOTIFY | No caching | Manual `rebuild_schema()` + versioned cache |

---

## 5. Key Code Examples

### 5.1 PostgREST Event Trigger (Production-Ready)

```sql
-- Install in a dedicated schema
CREATE SCHEMA IF NOT EXISTS postgrest;

-- Create event trigger function for DDL changes
CREATE OR REPLACE FUNCTION postgrest.pgrst_ddl_watch()
RETURNS event_trigger AS $$
DECLARE
  cmd record;
BEGIN
  FOR cmd IN SELECT * FROM pg_event_trigger_ddl_commands()
  LOOP
    IF cmd.command_tag IN (
      'CREATE TABLE', 'ALTER TABLE', 'CREATE VIEW', 'ALTER VIEW',
      'CREATE FUNCTION', 'ALTER FUNCTION', 'CREATE TYPE', 'ALTER TYPE'
    )
    AND cmd.schema_name IS DISTINCT FROM 'pg_temp'
    THEN
      NOTIFY pgrst, 'reload schema';
    END IF;
  END LOOP;
END;
$$ LANGUAGE plpgsql;

-- Create event trigger function for DROP changes
CREATE OR REPLACE FUNCTION postgrest.pgrst_drop_watch()
RETURNS event_trigger AS $$
DECLARE
  obj record;
BEGIN
  FOR obj IN SELECT * FROM pg_event_trigger_dropped_objects()
  LOOP
    IF obj.object_type IN ('table', 'view', 'function', 'type')
    AND obj.is_temporary IS FALSE
    THEN
      NOTIFY pgrst, 'reload schema';
    END IF;
  END LOOP;
END;
$$ LANGUAGE plpgsql;

-- Register triggers
CREATE EVENT TRIGGER pgrst_ddl_watch
  ON ddl_command_end
  EXECUTE FUNCTION postgrest.pgrst_ddl_watch();

CREATE EVENT TRIGGER pgrst_drop_watch
  ON sql_drop
  EXECUTE FUNCTION postgrest.pgrst_drop_watch();
```

### 5.2 pg_graphql Query via SQL

```sql
-- Create a table
CREATE TABLE account (
    id serial PRIMARY KEY,
    email text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

-- Query via graphql.resolve
SELECT graphql.resolve($$
  query {
    accountCollection {
      edges {
        node {
          id
          email
          createdAt
        }
      }
    }
  }
$$);

-- Result: {"data": {"accountCollection": {"edges": [...]}}}

-- Insert via mutation
SELECT graphql.resolve($$
  mutation {
    insertIntoAccountCollection(objects: [
      { email: "user@example.com" }
    ]) {
      records {
        id
        email
      }
    }
  }
$$);
```

### 5.3 postgres-meta Table Creation via API

```bash
# Create a table via postgres-meta REST API
curl -X POST http://localhost:8080/tables   -H "Content-Type: application/json"   -d '{"name": "products", "schema": "public", "comment": "Product catalog"}'

# Add a column via postgres-meta
curl -X POST http://localhost:8080/columns   -H "Content-Type: application/json"   -d '{
    "table_id": 12345,
    "name": "price",
    "type": "numeric",
    "is_nullable": false,
    "check": "price > 0"
  }'
```

---

## 6. References

1. **PostgREST Source Code**: `PostgREST/PostgREST` on GitHub — `src/PostgREST/SchemaCache.hs`
2. **PostgREST Documentation**: https://docs.postgrest.org/en/v13/references/schema_cache.html
3. **postgres-meta Source Code**: `supabase/postgres-meta` on GitHub — `src/lib/sql/`
4. **postgres-meta API Docs**: https://supabase.github.io/postgres-meta/
5. **pg_graphql Source Code**: `supabase/pg_graphql` on GitHub — `src/sql_types.rs`, `src/resolve.rs`, `sql/load_sql_context.sql`
6. **pg_graphql Documentation**: https://supabase.github.io/pg_graphql
7. **PostgreSQL Catalog Documentation**: https://www.postgresql.org/docs/current/catalogs.html
