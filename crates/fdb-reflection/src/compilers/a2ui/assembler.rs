//! The A2UI surface assembler.

use std::sync::Arc;

use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use super::error::{A2uiPublisher, AssemblerError};
use super::helpers::{catalog_id_for, data_source, matches_filter};
use super::rows::{BindingRow, ComponentRow, RuleRow};
use super::types::{A2uiMessage, A2uiSurface, AssemblyContext};

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
    ///
    /// # Errors
    ///
    /// Returns [`AssemblerError::Database`] if any `flint_a2ui.*` query
    /// fails; [`AssemblerError::InvalidConfig`] if a matched assembly rule's
    /// config is malformed or names a non-existent component, or if
    /// serializing the surface for the optional publisher fails;
    /// [`AssemblerError::NoBinding`] if no rule matches and no default table
    /// binding exists; or an error from `publisher.publish` if a publisher
    /// is attached and publishing fails.
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
