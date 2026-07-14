//! A2A task dispatch and handlers — delegate to the A2UI inner functions so
//! REST, MCP, and A2A surfaces share a single SQL authority.

use forge_identity::RlsContext;
use serde_json::{json, Value};

use crate::routes::a2ui::{self, AssembleSurfaceBody, ListComponentsQuery, SearchComponentsBody};

use super::helpers::{http_to_a2a_error, parse_uuid_opt};
use super::types::{A2aError, TaskState, INVALID_PARAMS, TASK_NOT_FOUND};
use super::A2aState;

/// Return the list of supported task definitions.
pub(super) fn task_list() -> Vec<Value> {
    vec![
        json!({
            "name": "a2ui.component.discover",
            "description": "Find a UI component by natural language description",
        }),
        json!({
            "name": "a2ui.component.assemble",
            "description": "Assemble an A2UI surface from an event context",
        }),
        json!({
            "name": "a2ui.search.semantic",
            "description": "Semantic vector search for UI components",
        }),
    ]
}

pub(super) async fn dispatch_task(
    state: &A2aState,
    who: &RlsContext,
    task_id: &str,
    name: &str,
    input: &Value,
) -> Result<Value, A2aError> {
    let output = match name {
        "a2ui.component.discover" => task_component_discover(state, who, input).await?,
        "a2ui.component.assemble" => task_component_assemble(state, who, input).await?,
        "a2ui.search.semantic" => task_search_semantic(state, who, input).await?,
        other => {
            return Err(A2aError::new(
                TASK_NOT_FOUND,
                format!("unknown task: {other}"),
            ));
        }
    };
    // Wrap the output in the A2A Task envelope per the spec.
    Ok(json!({
        "task": {
            "id": task_id,
            "name": name,
            "state": TaskState::Completed.as_str(),
            "output": output,
        }
    }))
}

async fn task_component_discover(
    state: &A2aState,
    who: &RlsContext,
    input: &Value,
) -> Result<Value, A2aError> {
    // Component discovery uses list_components + optional semantic search.
    // When `query` is provided, semantic search is preferred; otherwise list.
    if let Some(query) = input.get("query").and_then(Value::as_str) {
        let limit = input
            .get("limit")
            .and_then(Value::as_i64)
            .map_or(10, |i| i32::try_from(i).unwrap_or(10));
        let app_id = parse_uuid_opt(input, "app_id")?;
        let body = SearchComponentsBody {
            query: query.to_owned(),
            limit,
            app_id,
        };
        let result = a2ui::search_components_value(&state.a2ui.pool, who, &body)
            .await
            .map_err(http_to_a2a_error)?;
        // Adapt the REST shape to the A2A task output schema.
        let components = result
            .0
            .get("results")
            .cloned()
            .unwrap_or(Value::Array(vec![]));
        Ok(json!({ "components": components }))
    } else {
        let app_id = parse_uuid_opt(input, "app_id")?;
        let query = ListComponentsQuery {
            app_id,
            category: None,
        };
        let result = a2ui::list_components_value(&state.a2ui.pool, who, &query)
            .await
            .map_err(http_to_a2a_error)?;
        let components = result
            .0
            .get("components")
            .cloned()
            .unwrap_or(Value::Array(vec![]));
        Ok(json!({ "components": components }))
    }
}

async fn task_component_assemble(
    state: &A2aState,
    who: &RlsContext,
    input: &Value,
) -> Result<Value, A2aError> {
    let event_type = input
        .get("event_type")
        .and_then(Value::as_str)
        .ok_or_else(|| A2aError::new(INVALID_PARAMS, "event_type required"))?
        .to_owned();
    let event_payload = input.get("event_payload").cloned().unwrap_or(Value::Null);
    let application_id = parse_uuid_opt(input, "application_id")?;
    let body = AssembleSurfaceBody {
        event_type,
        event_context: event_payload,
        application_id,
    };
    let surface = a2ui::assemble_surface_value(&state.a2ui.pool, who, &body)
        .await
        .map_err(http_to_a2a_error)?;
    Ok(json!({ "surface": surface.0 }))
}

async fn task_search_semantic(
    state: &A2aState,
    who: &RlsContext,
    input: &Value,
) -> Result<Value, A2aError> {
    let query = input
        .get("query")
        .and_then(Value::as_str)
        .ok_or_else(|| A2aError::new(INVALID_PARAMS, "query required"))?
        .to_owned();
    let limit = input
        .get("limit")
        .and_then(Value::as_i64)
        .map_or(10, |i| i32::try_from(i).unwrap_or(10));
    let app_id = parse_uuid_opt(input, "app_id")?;
    let body = SearchComponentsBody {
        query,
        limit,
        app_id,
    };
    let result = a2ui::search_components_value(&state.a2ui.pool, who, &body)
        .await
        .map_err(http_to_a2a_error)?;
    let results = result
        .0
        .get("results")
        .cloned()
        .unwrap_or(Value::Array(vec![]));
    Ok(json!({ "results": results }))
}
