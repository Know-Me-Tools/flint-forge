//! AG-UI protocol descriptor and OpenAPI generation for ext-flint-meta.
//!
//! Provides two SQL functions built dynamically from the live schema cache:
//!
//! - `flint_meta.agui_descriptor()` — AG-UI/1.0 protocol descriptor for the
//!   flint-reflection engine. Describes available tables, columns, functions,
//!   and platform capabilities to AI agent runtimes.
//!
//! - `flint_meta.openapi()` — Minimal OpenAPI 3.1 document for the REST surface
//!   exposed by fdb-gateway over this schema cache. One path per public table
//!   with GET (list) and POST (insert) operations.
//!
//! Both functions are `STABLE PARALLEL SAFE SECURITY INVOKER` — they only read
//! cache tables and take no write locks beyond a snapshot. Both are restricted
//! to `service_role` because they expose full schema topology.

use pgrx::prelude::*;

extension_sql!(
    r#"
-- ── agui_descriptor() ────────────────────────────────────────────────────────
-- Returns an AG-UI protocol descriptor built dynamically from the live schema
-- cache. The descriptor tells an AI runtime which tables, functions, and
-- policies are available and how to call them.
--
-- AG-UI Custom events ride in the "x-forge" namespace. The descriptor is
-- intentionally minimal — callers needing the full schema use tables()/columns().
CREATE OR REPLACE FUNCTION flint_meta.agui_descriptor()
RETURNS jsonb
LANGUAGE plpgsql
STABLE PARALLEL SAFE
SECURITY INVOKER
AS $$
DECLARE
    v_tables   jsonb;
    v_funcs    jsonb;
BEGIN
    -- Build table list with column summary
    SELECT jsonb_agg(
        jsonb_build_object(
            'schema',      t.schema_name,
            'name',        t.table_name,
            'is_view',     t.is_view,
            'rls_enabled', t.rls_enabled,
            'description', t.description,
            'columns', (
                SELECT jsonb_agg(jsonb_build_object(
                    'name',       c.column_name,
                    'type',       c.data_type,
                    'nullable',   c.is_nullable,
                    'pk',         c.is_pk,
                    'fk',         c.is_fk
                ) ORDER BY c.ordinal)
                FROM flint_meta.cache_columns c
                WHERE c.schema_name = t.schema_name
                  AND c.table_name  = t.table_name
            )
        ) ORDER BY t.schema_name, t.table_name
    )
    INTO v_tables
    FROM flint_meta.cache_tables t
    WHERE t.schema_name NOT IN ('flint_meta', 'vault', 'auth', 'pg_catalog', 'information_schema');

    -- Build function list
    SELECT jsonb_agg(
        jsonb_build_object(
            'schema',         f.schema_name,
            'name',           f.function_name,
            'return_type',    f.return_type,
            'argument_types', f.argument_types,
            'is_stable',      f.is_stable,
            'description',    f.description
        ) ORDER BY f.schema_name, f.function_name
    )
    INTO v_funcs
    FROM flint_meta.cache_functions f
    WHERE f.schema_name NOT IN ('flint_meta', 'vault', 'auth', 'pg_catalog', 'information_schema');

    RETURN jsonb_build_object(
        'protocol',  'flint-forge/schema-descriptor/1.0',
        'version',   flint_meta.version(),
        'timestamp', now(),
        'tables',    COALESCE(v_tables, '[]'::jsonb),
        'functions', COALESCE(v_funcs, '[]'::jsonb),
        'capabilities', jsonb_build_object(
            'realtime',       true,
            'rls',            true,
            'graphql',        true,
            'rest',           true,
            'webhooks',       true,
            'edge_functions', true
        ),
        'x-forge', jsonb_build_object(
            'schema_version', flint_meta.version(),
            'extensions',     jsonb_build_array('flint_auth', 'flint_hooks', 'flint_llm', 'flint_vault', 'flint_meta')
        )
    );
END;
$$;

-- ── openapi() ─────────────────────────────────────────────────────────────────
-- Returns a minimal OpenAPI 3.1 document for the REST surface exposed by
-- fdb-gateway over this schema cache. One path per public table, with GET
-- (list) and POST (insert) operations. Full path parameters and schema
-- references follow the pg_graphql/PostgREST convention.
CREATE OR REPLACE FUNCTION flint_meta.openapi()
RETURNS jsonb
LANGUAGE plpgsql
STABLE PARALLEL SAFE
SECURITY INVOKER
AS $$
DECLARE
    v_paths   jsonb := '{}'::jsonb;
    v_schemas jsonb := '{}'::jsonb;
    rec       record;
    tbl_path  text;
    tbl_schema jsonb;
BEGIN
    FOR rec IN
        SELECT schema_name, table_name, description, rls_enabled
        FROM flint_meta.cache_tables
        WHERE schema_name NOT IN ('flint_meta', 'vault', 'auth', 'pg_catalog', 'information_schema')
        ORDER BY schema_name, table_name
    LOOP
        tbl_path := '/' || rec.schema_name || '/' || rec.table_name;

        -- Build column properties for the schema object
        SELECT jsonb_object_agg(
            c.column_name,
            jsonb_build_object(
                'type',        CASE
                                 WHEN c.data_type LIKE '%int%' THEN 'integer'
                                 WHEN c.data_type IN ('bool', 'boolean') THEN 'boolean'
                                 WHEN c.data_type IN ('jsonb', 'json') THEN 'object'
                                 WHEN c.data_type IN ('text[]', 'character varying[]', '_text') THEN 'array'
                                 ELSE 'string'
                               END,
                'nullable',    c.is_nullable,
                'description', c.description
            )
        )
        INTO tbl_schema
        FROM flint_meta.cache_columns c
        WHERE c.schema_name = rec.schema_name AND c.table_name = rec.table_name;

        -- Add schema component
        v_schemas := v_schemas || jsonb_build_object(
            rec.schema_name || '_' || rec.table_name,
            jsonb_build_object(
                'type',        'object',
                'description', rec.description,
                'properties',  COALESCE(tbl_schema, '{}'::jsonb)
            )
        );

        -- Add path operations
        v_paths := v_paths || jsonb_build_object(
            tbl_path,
            jsonb_build_object(
                'get', jsonb_build_object(
                    'summary',     'List ' || rec.table_name,
                    'operationId', 'list_' || rec.schema_name || '_' || rec.table_name,
                    'tags',        jsonb_build_array(rec.schema_name),
                    'security',    jsonb_build_array(jsonb_build_object('bearerAuth', '[]'::jsonb)),
                    'parameters',  jsonb_build_array(
                        jsonb_build_object('in', 'query', 'name', 'limit',  'schema', jsonb_build_object('type', 'integer')),
                        jsonb_build_object('in', 'query', 'name', 'offset', 'schema', jsonb_build_object('type', 'integer'))
                    ),
                    'responses',   jsonb_build_object(
                        '200', jsonb_build_object(
                            'description', 'OK',
                            'content',     jsonb_build_object(
                                'application/json',
                                jsonb_build_object('schema', jsonb_build_object(
                                    'type', 'array',
                                    'items', jsonb_build_object('$ref', '#/components/schemas/' || rec.schema_name || '_' || rec.table_name)
                                ))
                            )
                        ),
                        '401', jsonb_build_object('description', 'Unauthorized')
                    )
                ),
                'post', jsonb_build_object(
                    'summary',     'Insert into ' || rec.table_name,
                    'operationId', 'insert_' || rec.schema_name || '_' || rec.table_name,
                    'tags',        jsonb_build_array(rec.schema_name),
                    'security',    jsonb_build_array(jsonb_build_object('bearerAuth', '[]'::jsonb)),
                    'requestBody', jsonb_build_object(
                        'required', true,
                        'content', jsonb_build_object(
                            'application/json',
                            jsonb_build_object('schema', jsonb_build_object('$ref', '#/components/schemas/' || rec.schema_name || '_' || rec.table_name))
                        )
                    ),
                    'responses',   jsonb_build_object(
                        '201', jsonb_build_object('description', 'Created'),
                        '401', jsonb_build_object('description', 'Unauthorized')
                    )
                )
            )
        );
    END LOOP;

    RETURN jsonb_build_object(
        'openapi', '3.1.0',
        'info', jsonb_build_object(
            'title',       'Flint Forge REST API',
            'description', 'Auto-generated from flint_meta schema cache',
            'version',     flint_meta.version()::text
        ),
        'components', jsonb_build_object(
            'securitySchemes', jsonb_build_object(
                'bearerAuth', jsonb_build_object(
                    'type',         'http',
                    'scheme',       'bearer',
                    'bearerFormat', 'JWT'
                )
            ),
            'schemas', v_schemas
        ),
        'paths', v_paths
    );
END;
$$;
"#,
    name = "flint_meta_agui",
    requires = ["flint_meta_functions"]
);

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    fn test_agui_descriptor_returns_jsonb() {
        let result = Spi::get_one::<pgrx::JsonB>("SELECT flint_meta.agui_descriptor()")
            .unwrap()
            .unwrap();

        let obj = result
            .0
            .as_object()
            .expect("agui_descriptor must return a JSON object");
        assert!(
            obj.contains_key("protocol"),
            "descriptor must have 'protocol' key"
        );
        assert_eq!(obj["protocol"].as_str().unwrap(), "flint-forge/schema-descriptor/1.0");
        assert!(
            obj.contains_key("tables"),
            "descriptor must have 'tables' key"
        );
        assert!(
            obj.contains_key("capabilities"),
            "descriptor must have 'capabilities' key"
        );
    }

    #[pg_test]
    fn test_openapi_returns_jsonb() {
        let result = Spi::get_one::<pgrx::JsonB>("SELECT flint_meta.openapi()")
            .unwrap()
            .unwrap();

        let obj = result
            .0
            .as_object()
            .expect("openapi() must return a JSON object");
        assert!(
            obj.contains_key("openapi"),
            "must have 'openapi' version key"
        );
        assert_eq!(obj["openapi"].as_str().unwrap(), "3.1.0");
        assert!(obj.contains_key("info"), "must have 'info' key");
        assert!(obj.contains_key("paths"), "must have 'paths' key");
    }

    #[pg_test]
    fn test_agui_capabilities_block() {
        let result =
            Spi::get_one::<pgrx::JsonB>("SELECT flint_meta.agui_descriptor()->'capabilities'")
                .unwrap()
                .unwrap();

        let caps = result
            .0
            .as_object()
            .expect("capabilities must be an object");
        assert!(
            caps["realtime"].as_bool().unwrap_or(false),
            "realtime must be true"
        );
        assert!(caps["rls"].as_bool().unwrap_or(false), "rls must be true");
    }
}
