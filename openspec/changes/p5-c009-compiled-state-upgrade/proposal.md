# p5-c009 — CompiledState Upgrade: flint_a2ui Registry Integration

**Phase:** 5 — Flint A2UI Component Registry  
**Priority:** P0 (correctness — fixes Phase 2 design before Phase 7 builds on it)  
**Depends on:** p5-c001, p5-c005 (application model), Phase 2 `CompiledState` (p2-c003)  
**Blocks:** p7-c005 (A2UI surface emitter), p7-c007 (state propagation)

---

## What this change delivers

Three coordinated fixes to the Phase 2 → Phase 7 data flow:

### 1. CompiledState.agui_descriptors source change

In Phase 2, `agui_descriptors` is populated by calling `flint_meta.agui_descriptor()` directly (a single JSONB blob describing the database schema). After Phase 5, the source of truth is `flint_a2ui.resolve_components()` — a permission-filtered list of components from the registry.

```rust
// fdb-reflection/src/compiled.rs

pub struct CompiledState {
    pub version: u64,
    pub database_model: Arc<DatabaseModel>,
    pub router: Arc<axum::Router<()>>,
    pub openapi_doc: Arc<utoipa::openapi::OpenApi>,
    pub mcp_tools: Arc<Vec<McpToolDef>>,
    // Phase 2: HashMap<slug, flint_meta.agui_descriptor() output>
    // Phase 5 (this change): Vec of resolved A2UI catalog entries from flint_a2ui
    pub a2ui_catalog: Arc<A2uiCatalog>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct A2uiCatalog {
    pub catalog_id: String,    // URI: e.g. "/a2ui/v1/catalog/flint-base/1.0"
    pub version: String,
    pub components: Vec<A2uiCatalogEntry>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct A2uiCatalogEntry {
    pub slug: String,
    pub primitive_type: String,
    pub category: String,
    pub schema: serde_json::Value,
    pub description: Option<String>,
}
```

The `ReflectionEngine` now queries `flint_a2ui.components` to populate `a2ui_catalog`:

```rust
// fdb-reflection/src/engine.rs

impl ReflectionEngine {
    pub async fn reflect(&self, pool: &PgPool) -> Result<CompiledState, ReflectionError> {
        // ... existing reflection logic ...

        // NEW: Load A2UI catalog from registry
        let catalog = self.load_a2ui_catalog(pool).await?;

        Ok(CompiledState {
            version,
            database_model: Arc::new(database_model),
            router: Arc::new(router),
            openapi_doc: Arc::new(openapi_doc),
            mcp_tools: Arc::new(mcp_tools),
            a2ui_catalog: Arc::new(catalog),
        })
    }

    async fn load_a2ui_catalog(&self, pool: &PgPool) -> Result<A2uiCatalog, ReflectionError> {
        // If flint_a2ui schema doesn't exist yet (Phase 5 not deployed),
        // fall back to empty catalog — graceful degradation
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables
             WHERE table_schema = 'flint_a2ui' AND table_name = 'components')"
        )
        .fetch_one(pool)
        .await
        .unwrap_or(false);

        if !exists {
            tracing::warn!("flint_a2ui schema not found; A2UI catalog will be empty");
            return Ok(A2uiCatalog {
                catalog_id: String::new(),
                version: "0.0.0".into(),
                components: vec![],
            });
        }

        let components = sqlx::query_as!(
            A2uiCatalogEntry,
            r#"SELECT slug, primitive_type, category, schema as "schema: serde_json::Value", description
               FROM flint_a2ui.components
               WHERE is_base = true OR application_id IS NULL
               ORDER BY category, slug"#
        )
        .fetch_all(pool)
        .await
        .map_err(ReflectionError::DatabaseQuery)?;

        Ok(A2uiCatalog {
            catalog_id: "/a2ui/v1/catalog/flint-base/1.0".into(),
            version: "1.0.0".into(),
            components,
        })
    }
}
```

### 2. Cedar a2ui:emit capability check in fke-server

Before any Kiln WASM function can emit an A2UI surface, `fke-server` must verify the Cedar `a2ui:emit` capability:

```rust
// fke-server/src/handler.rs (or wherever WASM output is processed)

async fn handle_wasm_output(
    output: WasmOutput,
    state: &CompiledState,
    cedar: &CedarEngine,
    jwt_claims: &JwtClaims,
) -> Result<(), PolicyError> {
    if let WasmOutput::A2uiSurface(surface) = &output {
        // Cedar policy check: does this function have a2ui:emit capability?
        let decision = cedar.is_authorized(
            &jwt_claims.principal(),
            "a2ui:emit",
            &surface.catalog_id,
        )?;
        if !decision.is_allow() {
            return Err(PolicyError::A2uiEmitDenied {
                function: jwt_claims.sub.clone(),
                catalog: surface.catalog_id.clone(),
            });
        }
    }
    Ok(())
}
```

### 3. `flint_meta.agui_descriptor()` GRANT review

Keeping the current `GRANT EXECUTE TO service_role` (not `authenticated, anon`). The full schema topology is sensitive. Agents discover components through `flint_a2ui.resolve_components()` (which is permission-filtered) not through `agui_descriptor()`.

---

## Backward compatibility

The field rename from `agui_descriptors: HashMap<String, Value>` to `a2ui_catalog: A2uiCatalog` is a breaking change within `CompiledState`. Since `CompiledState` is an internal type (not part of a public API), this is acceptable. All internal callers must be updated in this change.

Known callers to update:
- `fdb-gateway/src/routes/` — any handler reading `agui_descriptors`
- `fke-server/src/` — WASM output handler (Phase 6/7)
- Phase 7 `p7-c005-a2ui-surface-emitter` — will read `state.a2ui_catalog` instead

---

## Gate tests

- `CompiledState` compiles with `a2ui_catalog` field
- `ReflectionEngine::reflect()` populates `a2ui_catalog` from `flint_a2ui.components`
- When `flint_a2ui` schema is absent, `a2ui_catalog` is empty (graceful degradation)
- Cedar `a2ui:emit` check correctly blocks WASM functions without the capability
- `flint_meta.agui_descriptor()` protocol label is `'flint-forge/schema-descriptor/1.0'` (set in p5-c003, verified here)
