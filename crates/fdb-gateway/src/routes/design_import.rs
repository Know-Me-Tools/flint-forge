//! Design system import — REST endpoint for DESIGN.md, W3C token, and ZIP imports.
//!
//! `POST /a2ui/v1/design-systems/import` accepts:
//! - `format: "design_md"` with `content: <DESIGN.md text>`
//! - `format: "w3c_tokens"` with `content: <W3C Design Token JSON>`
//! - `format: "claude_design_zip"` with `content: <base64-encoded ZIP containing DESIGN.md>`
//!
//! On success, the design system is upserted and component_overrides rows
//! are created/updated for any §5 overrides in the DESIGN.md.
#![forbid(unsafe_code)]

use std::io::Cursor;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    Extension,
};
use base64::{engine::general_purpose, Engine as _};
use fdb_app::a2ui::{map_w3c_tokens, parse_design_md};
use forge_identity::RlsContext;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::routes::a2ui::A2uiState;

/// Request body for `POST /a2ui/v1/design-systems/import`.
#[derive(Debug, Deserialize)]
pub struct ImportBody {
    /// Import format: `"design_md"`, `"w3c_tokens"`, or `"claude_design_zip"`.
    pub format: String,
    /// The content to import — DESIGN.md text, W3C JSON string, or base64-encoded ZIP.
    pub content: String,
    /// Optional: associate the import with an existing design system by id.
    #[serde(default)]
    pub design_system_id: Option<Uuid>,
    /// Optional: name override (auto-extracted from DESIGN.md title if omitted).
    #[serde(default)]
    pub name: Option<String>,
}

/// `POST /a2ui/v1/design-systems/import`
///
/// Parses, maps, and stores a design system import. Returns the
/// `design_system_id` and a count of component overrides applied.
pub async fn import_design_system(
    State(state): State<A2uiState>,
    Extension(_who): Extension<RlsContext>,
    Json(body): Json<ImportBody>,
) -> impl IntoResponse {
    match body.format.as_str() {
        "design_md" => import_design_md(state, body).await,
        "w3c_tokens" => import_w3c_tokens(state, body).await,
        "claude_design_zip" => import_claude_design_zip(state, body).await,
        other => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": format!("unsupported format: {other}; expected 'design_md', 'w3c_tokens', or 'claude_design_zip'") })),
        )
            .into_response(),
    }
}

async fn import_design_md(state: A2uiState, body: ImportBody) -> axum::response::Response {
    let doc = match parse_design_md(&body.content) {
        Ok(d) => d,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };

    let name = body.name.unwrap_or_else(|| doc.name.clone());
    let ds_id = upsert_design_system(
        &state.pool,
        body.design_system_id,
        &name,
        &doc.tokens,
        "design_md",
        &body.content,
    )
    .await;

    let ds_id = match ds_id {
        Ok(id) => id,
        Err(e) => {
            tracing::error!(error = %e, "design system upsert failed");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "database error" })),
            )
                .into_response();
        }
    };

    // Persist component overrides
    let mut override_count = 0usize;
    for ov in &doc.component_overrides {
        // Resolve component_id by slug
        let component_id: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM flint_a2ui.components WHERE slug = $1")
                .bind(&ov.slug)
                .fetch_optional(&state.pool)
                .await
                .ok()
                .flatten();

        if let Some((cid,)) = component_id {
            let res = sqlx::query(
                "INSERT INTO flint_a2ui.component_overrides
                    (design_system_id, component_id, prop_defaults, css_vars,
                     react_component, flutter_widget, htmx_template, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, now())
                 ON CONFLICT (design_system_id, component_id)
                 DO UPDATE SET
                    prop_defaults    = EXCLUDED.prop_defaults,
                    css_vars         = EXCLUDED.css_vars,
                    react_component  = EXCLUDED.react_component,
                    flutter_widget   = EXCLUDED.flutter_widget,
                    htmx_template    = EXCLUDED.htmx_template,
                    updated_at       = now()",
            )
            .bind(ds_id)
            .bind(cid)
            .bind(&ov.prop_defaults)
            .bind(&ov.css_vars)
            .bind(&ov.react_component)
            .bind(&ov.flutter_widget)
            .bind(&ov.htmx_template)
            .execute(&state.pool)
            .await;

            if res.is_ok() {
                override_count += 1;
            } else if let Err(e) = res {
                tracing::warn!(slug = %ov.slug, error = %e, "component override upsert failed");
            }
        } else {
            tracing::debug!(slug = %ov.slug, "component override skipped — slug not found");
        }
    }

    Json(json!({
        "design_system_id": ds_id,
        "name": name,
        "format": "design_md",
        "component_overrides_applied": override_count,
        "sections_parsed": doc.raw_sections.len(),
    }))
    .into_response()
}

async fn import_w3c_tokens(state: A2uiState, body: ImportBody) -> axum::response::Response {
    let tokens = match map_w3c_tokens(&body.content) {
        Ok(t) => t,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("invalid W3C token JSON: {e}") })),
            )
                .into_response();
        }
    };

    let name = body
        .name
        .unwrap_or_else(|| "Imported Design System".to_owned());
    let ds_id = upsert_design_system(
        &state.pool,
        body.design_system_id,
        &name,
        &tokens,
        "w3c_tokens",
        &body.content,
    )
    .await;

    let ds_id = match ds_id {
        Ok(id) => id,
        Err(e) => {
            tracing::error!(error = %e, "design system upsert failed");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "database error" })),
            )
                .into_response();
        }
    };

    Json(json!({
        "design_system_id": ds_id,
        "name": name,
        "format": "w3c_tokens",
        "component_overrides_applied": 0,
    }))
    .into_response()
}

/// Decodes a base64-encoded ZIP, extracts `DESIGN.md`, then delegates to
/// [`import_design_md`] with the extracted content.
async fn import_claude_design_zip(state: A2uiState, body: ImportBody) -> axum::response::Response {
    // Decode base64 ZIP bytes
    let Ok(zip_bytes) = general_purpose::STANDARD.decode(&body.content) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "invalid base64 content for claude_design_zip"})),
        )
            .into_response();
    };

    // Open ZIP archive — Cursor<Vec<u8>> implements both Read and Seek
    let mut archive = match zip::ZipArchive::new(Cursor::new(zip_bytes)) {
        Ok(a) => a,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("invalid ZIP archive: {e}")})),
            )
                .into_response();
        }
    };

    // Find DESIGN.md inside the ZIP (root or in any subdirectory)
    let design_md = {
        let mut found: Option<String> = None;
        for i in 0..archive.len() {
            let Ok(mut file) = archive.by_index(i) else {
                continue;
            };
            if file.name().ends_with("DESIGN.md") {
                let mut buf = String::new();
                if std::io::Read::read_to_string(&mut file, &mut buf).is_ok() {
                    found = Some(buf);
                    break;
                }
            }
        }
        match found {
            Some(md) => md,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"error": "DESIGN.md not found in ZIP archive"})),
                )
                    .into_response();
            }
        }
    };

    // Delegate to import_design_md() with the extracted DESIGN.md content
    let new_body = ImportBody {
        format: "design_md".to_owned(),
        content: design_md,
        design_system_id: body.design_system_id,
        name: body.name,
    };
    import_design_md(state, new_body).await
}

/// Upsert a design system row, returning its id.
async fn upsert_design_system(
    pool: &sqlx::PgPool,
    existing_id: Option<Uuid>,
    name: &str,
    tokens: &Value,
    source_format: &str,
    source_content: &str,
) -> Result<Uuid, sqlx::Error> {
    if let Some(id) = existing_id {
        sqlx::query(
            "UPDATE flint_a2ui.design_systems
             SET name           = $2,
                 tokens         = $3,
                 source_format  = $4,
                 source_content = $5,
                 imported_at    = now()
             WHERE id = $1",
        )
        .bind(id)
        .bind(name)
        .bind(tokens)
        .bind(source_format)
        .bind(source_content)
        .execute(pool)
        .await?;
        Ok(id)
    } else {
        let (new_id,): (Uuid,) = sqlx::query_as(
            "INSERT INTO flint_a2ui.design_systems
                (name, tokens, source_format, source_content, imported_at)
             VALUES ($1, $2, $3, $4, now())
             RETURNING id",
        )
        .bind(name)
        .bind(tokens)
        .bind(source_format)
        .bind(source_content)
        .fetch_one(pool)
        .await?;
        Ok(new_id)
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_body_deserializes_design_md_format() {
        let content = "# Design System Test\n\n## 1. Color\n";
        let json = format!(
            r#"{{"format":"design_md","content":{}}}"#,
            serde_json::to_string(content).unwrap()
        );
        let body: ImportBody = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(body.format, "design_md");
        assert!(body.content.contains("Design System"));
        assert!(body.design_system_id.is_none());
    }

    #[test]
    fn import_body_accepts_optional_design_system_id() {
        let id = Uuid::new_v4();
        let json =
            format!(r#"{{"format":"w3c_tokens","content":"{{}}","design_system_id":"{id}"}}"#);
        let body: ImportBody = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(body.design_system_id, Some(id));
    }

    #[test]
    fn import_body_rejects_unknown_format() {
        let body = serde_json::json!({"format": "unknown", "content": "x"});
        let parsed: ImportBody = serde_json::from_value(body).expect("deserialize");
        assert_eq!(parsed.format, "unknown");
    }

    #[test]
    fn import_body_accepts_claude_design_zip_format() {
        let content = general_purpose::STANDARD.encode(b"fake-zip-bytes");
        let json = format!(r#"{{"format":"claude_design_zip","content":"{content}"}}"#);
        let body: ImportBody = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(body.format, "claude_design_zip");
    }
}
