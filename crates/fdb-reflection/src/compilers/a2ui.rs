//! A2UI surface assembler.
//!
//! Turns an incoming event context into an A2UI v0.9.1 message sequence by
//! applying application-specific assembly rules, then falling back to the
//! default table → component binding in `flint_a2ui.bindings`.
#![forbid(unsafe_code)]

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{types::Json as SqlxJson, FromRow, PgPool};
use uuid::Uuid;

/// Errors produced by the A2UI assembler.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum AssemblerError {
    #[error("database query failed")]
    Database(#[from] sqlx::Error),

    #[error("no assembly rule matched and no default binding found for {0}.{1}")]
    NoBinding(String, String),

    #[error("invalid assembly configuration: {0}")]
    InvalidConfig(String),

    #[error("event payload missing required field {0}")]
    MissingField(String),
}

/// Optional publisher for assembled surfaces (e.g. FRF Iggy topic).
#[async_trait::async_trait]
pub trait A2uiPublisher: Send + Sync {
    /// Publish a serialized surface to the given topic.
    async fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), AssemblerError>;
}

/// All inputs needed to assemble a surface.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssemblyContext {
    /// Event type name, e.g. `tool_call_completed`.
    pub event_type: String,
    /// Event payload. Default binding resolution looks for `data_source.schema`
    /// and `data_source.table`.
    pub event_payload: Value,
    /// Application that should own the assembled surface.
    pub application_id: Option<Uuid>,
    /// JWT claims of the caller, used for any permission-filtered resolution.
    pub jwt_claims: Value,
    /// Optional explicit surface id; otherwise a new UUID is generated.
    pub surface_id: Option<Uuid>,
}

/// A fully assembled A2UI surface, represented as a sequence of messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2uiSurface {
    pub surface_id: Uuid,
    pub catalog_id: String,
    pub messages: Vec<A2uiMessage>,
}

impl A2uiSurface {
    /// Serialize the whole surface to a JSON value.
    pub fn to_json(&self) -> Value {
        json!({
            "surfaceId": self.surface_id,
            "catalogId": self.catalog_id,
            "messages": self.messages,
        })
    }
}

/// A single A2UI message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2uiMessage {
    pub op: String,
    #[serde(flatten)]
    pub payload: Value,
}

/// Builds A2UI surfaces from events.
#[derive(Clone)]
pub struct A2uiAssembler {
    pool: PgPool,
    publisher: Option<Arc<dyn A2uiPublisher>>,
}

impl A2uiAssembler {
    /// Create a new assembler backed by a privileged pool.
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            publisher: None,
        }
    }

    /// Attach an optional publisher for assembled surfaces.
    pub fn with_publisher(mut self, publisher: Arc<dyn A2uiPublisher>) -> Self {
        self.publisher = Some(publisher);
        self
    }

    /// Assemble a surface for the given context.
    pub async fn assemble(&self, ctx: &AssemblyContext) -> Result<A2uiSurface, AssemblerError> {
        let surface_id = ctx.surface_id.unwrap_or_else(Uuid::new_v4);
        let catalog_id = catalog_id_for(ctx);

        let (primitive_type, props) = self.resolve_component(ctx).await?;

        let mut messages = vec![
            A2uiMessage {
                op: "createSurface".to_string(),
                payload: json!({
                    "surfaceId": surface_id.to_string(),
                    "catalogId": catalog_id,
                }),
            },
            A2uiMessage {
                op: "updateComponents".to_string(),
                payload: json!({
                    "surfaceId": surface_id.to_string(),
                    "components": [
                        {
                            "id": "main",
                            "component": primitive_type,
                            "props": props,
                        }
                    ]
                }),
            },
            A2uiMessage {
                op: "updateDataModel".to_string(),
                payload: json!({
                    "surfaceId": surface_id.to_string(),
                    "path": "/data",
                    "value": ctx.event_payload,
                }),
            },
        ];

        // Optional footer actions when the event carries a primary action.
        if let Some(actions) = ctx.event_payload.get("actions") {
            messages.push(A2uiMessage {
                op: "updateActions".to_string(),
                payload: json!({
                    "surfaceId": surface_id.to_string(),
                    "actions": actions,
                }),
            });
        }

        let surface = A2uiSurface {
            surface_id,
            catalog_id,
            messages,
        };

        if let Some(publisher) = &self.publisher {
            let payload = serde_json::to_vec(&surface).map_err(|e| {
                AssemblerError::InvalidConfig(format!("failed to serialize surface: {e}"))
            })?;
            publisher.publish("a2ui.surfaces", &payload).await?;
        }

        Ok(surface)
    }

    /// Pick the component primitive and props for this context.
    async fn resolve_component(
        &self,
        ctx: &AssemblyContext,
    ) -> Result<(String, Value), AssemblerError> {
        // 1. Try assembly rules first.
        if let Some(app_id) = ctx.application_id {
            if let Some((primitive_type, props)) = self.try_rules(ctx, app_id).await? {
                return Ok((primitive_type, props));
            }
        }

        // 2. Fall back to default table binding.
        self.default_binding(ctx).await
    }

    /// Query `flint_a2ui.assembly_rules` and return the first matching rule's
    /// configured component, or `None` if no rule matches.
    async fn try_rules(
        &self,
        ctx: &AssemblyContext,
        app_id: Uuid,
    ) -> Result<Option<(String, Value)>, AssemblerError> {
        let rules: Vec<RuleRow> = sqlx::query_as(
            "SELECT event_filter, assembly_config, priority
             FROM flint_a2ui.assembly_rules
             WHERE application_id = $1 AND event_type = $2 AND is_active = true
             ORDER BY priority ASC",
        )
        .bind(app_id)
        .bind(&ctx.event_type)
        .fetch_all(&self.pool)
        .await?;

        for rule in rules {
            if matches_filter(&ctx.event_payload, &rule.event_filter.0) {
                let config = &rule.assembly_config.0;
                let (primitive_type, mut props) = self.resolve_from_config(config).await?;

                // Merge any props declared directly in the assembly config.
                if let Some(configured_props) = config.get("props").and_then(Value::as_object) {
                    if let Some(props_obj) = props.as_object_mut() {
                        for (k, v) in configured_props {
                            props_obj.insert(k.clone(), v.clone());
                        }
                    }
                }

                return Ok(Some((primitive_type, props)));
            }
        }

        Ok(None)
    }

    /// Resolve a component from an explicit assembly config object.
    async fn resolve_from_config(&self, config: &Value) -> Result<(String, Value), AssemblerError> {
        let slug = config
            .get("component_slug")
            .or_else(|| config.get("component"))
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AssemblerError::InvalidConfig(
                    "assembly_config must contain component_slug or component".to_string(),
                )
            })?;

        let row: ComponentRow = sqlx::query_as(
            "SELECT slug, primitive_type, schema
             FROM flint_a2ui.components
             WHERE slug = $1",
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AssemblerError::InvalidConfig(format!("component not found: {slug}")))?;

        Ok((
            row.primitive_type,
            config.get("props").cloned().unwrap_or(Value::Null),
        ))
    }

    /// Fall back to `flint_a2ui.bindings` for the source table named in the
    /// event payload.
    async fn default_binding(
        &self,
        ctx: &AssemblyContext,
    ) -> Result<(String, Value), AssemblerError> {
        let (schema, table) = data_source(ctx)?;

        let binding: Option<BindingRow> = sqlx::query_as(
            "SELECT b.table_schema, b.table_name, b.binding_type, b.config,
                    c.slug, c.primitive_type
             FROM flint_a2ui.bindings b
             JOIN flint_a2ui.components c ON c.id = b.component_id
             WHERE b.table_schema = $1 AND b.table_name = $2
             ORDER BY CASE b.binding_type
                 WHEN 'grid' THEN 1
                 WHEN 'form' THEN 2
                 WHEN 'detail' THEN 3
                 WHEN 'card' THEN 4
                 ELSE 5
             END
             LIMIT 1",
        )
        .bind(&schema)
        .bind(&table)
        .fetch_optional(&self.pool)
        .await?;

        let Some(binding) = binding else {
            return Err(AssemblerError::NoBinding(schema, table));
        };

        let data_source = format!("{schema}.{table}");
        let mut props = binding.config.0.clone();
        if let Some(props_obj) = props.as_object_mut() {
            props_obj.insert("data_source".to_string(), Value::String(data_source));
        } else if props.is_null() {
            props = json!({"data_source": data_source});
        }

        Ok((binding.primitive_type, props))
    }
}

/// Build the canonical catalog id for a context.
fn catalog_id_for(ctx: &AssemblyContext) -> String {
    ctx.event_payload
        .get("catalog_id")
        .and_then(Value::as_str)
        .map_or_else(
            || "https://forge.example.com/a2ui/v1/catalog/flint-base/1.0.0".to_string(),
            ToOwned::to_owned,
        )
}

/// Extract `data_source.schema` / `data_source.table` from the event payload.
fn data_source(ctx: &AssemblyContext) -> Result<(String, String), AssemblerError> {
    let ds = ctx
        .event_payload
        .get("data_source")
        .ok_or_else(|| AssemblerError::MissingField("data_source".to_string()))?;

    let schema = ds
        .get("schema")
        .and_then(Value::as_str)
        .unwrap_or("public")
        .to_string();
    let table = ds
        .get("table")
        .and_then(Value::as_str)
        .ok_or_else(|| AssemblerError::MissingField("data_source.table".to_string()))?
        .to_string();

    Ok((schema, table))
}

/// Check whether `payload` satisfies all predicates in `filter`.
///
/// Filter keys may be dotted paths (e.g. `data_source.table`). An empty filter
/// object matches every payload.
fn matches_filter(payload: &Value, filter: &Value) -> bool {
    let Some(predicates) = filter.as_object() else {
        return true;
    };

    for (key, expected) in predicates {
        let actual = navigate(payload, key);
        if !value_matches(actual, expected) {
            return false;
        }
    }

    true
}

/// Navigate a dotted path through a JSON object. Missing segments yield Null.
fn navigate<'v>(value: &'v Value, path: &str) -> &'v Value {
    let mut current = value;
    for segment in path.split('.') {
        current = current.get(segment).unwrap_or(&Value::Null);
        if current.is_null() {
            break;
        }
    }
    current
}

/// Compare an actual JSON value to an expected predicate value. Null expected
/// values are interpreted as "missing or null".
fn value_matches(actual: &Value, expected: &Value) -> bool {
    if expected.is_null() {
        return actual.is_null();
    }
    actual == expected
}

#[derive(Debug, FromRow)]
struct RuleRow {
    event_filter: SqlxJson<Value>,
    assembly_config: SqlxJson<Value>,
    #[allow(dead_code)]
    priority: i32,
}

#[derive(Debug, FromRow)]
struct ComponentRow {
    #[allow(dead_code)]
    slug: String,
    primitive_type: String,
    #[allow(dead_code)]
    schema: SqlxJson<Value>,
}

#[derive(Debug, FromRow)]
struct BindingRow {
    #[allow(dead_code)]
    table_schema: String,
    #[allow(dead_code)]
    table_name: String,
    #[allow(dead_code)]
    binding_type: String,
    config: SqlxJson<Value>,
    #[allow(dead_code)]
    slug: String,
    primitive_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn connect() -> Option<PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        PgPool::connect(&url).await.ok()
    }

    /// Seed the minimal catalog + binding needed by the gate tests.
    async fn seed_grid_binding(pool: &PgPool) -> sqlx::Result<()> {
        let component_id = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO flint_a2ui.components
             (slug, category, primitive_type, schema, is_base)
             VALUES ('data-grid', 'data-display', 'DataGrid', '{}', true)
             ON CONFLICT (slug) DO UPDATE SET primitive_type = EXCLUDED.primitive_type
             RETURNING id",
        )
        .fetch_one(pool)
        .await?;

        sqlx::query(
            "INSERT INTO flint_a2ui.bindings
             (table_schema, table_name, component_id, binding_type, config)
             VALUES ('public', 'orders', $1, 'grid', '{\"columns\": [{\"field\": \"id\", \"header\": \"ID\"}]}')
             ON CONFLICT (table_schema, table_name, binding_type) DO UPDATE
                 SET config = EXCLUDED.config",
        )
        .bind(component_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_assemble_falls_back_to_grid_binding() {
        let Some(pool) = connect().await else { return };
        seed_grid_binding(&pool).await.expect("seed binding");

        let assembler = A2uiAssembler::new(pool);
        let ctx = AssemblyContext {
            event_type: "tool_call_completed".to_string(),
            event_payload: json!({
                "data_source": {"schema": "public", "table": "orders"},
                "data": [{"id": 1}]
            }),
            application_id: None,
            jwt_claims: json!({"flint": {"user_id": "anonymous-user"}}),
            surface_id: None,
        };

        let surface = assembler.assemble(&ctx).await.expect("assemble");
        let update_components = surface
            .messages
            .iter()
            .find(|m| m.op == "updateComponents")
            .expect("updateComponents message");
        let components = update_components
            .payload
            .get("components")
            .and_then(Value::as_array)
            .expect("components array");
        assert_eq!(components[0]["component"], "DataGrid");
        assert_eq!(components[0]["props"]["data_source"], "public.orders");
    }

    #[tokio::test]
    async fn test_assemble_latency_sla() {
        let Some(pool) = connect().await else { return };
        seed_grid_binding(&pool).await.expect("seed binding");

        let assembler = A2uiAssembler::new(pool);
        let ctx = AssemblyContext {
            event_type: "tool_call_completed".to_string(),
            event_payload: json!({
                "data_source": {"schema": "public", "table": "orders"},
                "data": [{"id": 1}]
            }),
            application_id: None,
            jwt_claims: json!({"flint": {"user_id": "anonymous-user"}}),
            surface_id: None,
        };

        let start = std::time::Instant::now();
        let _surface = assembler.assemble(&ctx).await.expect("assemble");
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 500,
            "assembly took {elapsed:?}, exceeds 500ms SLA"
        );
    }

    #[tokio::test]
    async fn test_no_binding_returns_typed_error() {
        let Some(pool) = connect().await else { return };

        let assembler = A2uiAssembler::new(pool);
        let ctx = AssemblyContext {
            event_type: "tool_call_completed".to_string(),
            event_payload: json!({
                "data_source": {"schema": "public", "table": "does_not_exist"}
            }),
            application_id: None,
            jwt_claims: json!({"flint": {"user_id": "anonymous-user"}}),
            surface_id: None,
        };

        let err = assembler.assemble(&ctx).await.unwrap_err();
        assert!(matches!(err, AssemblerError::NoBinding(_, _)));
    }
}
