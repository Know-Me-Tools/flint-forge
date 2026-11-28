use super::{
    json, Body, Client, ConnectInfo, PgPool, Request, Router, ServiceExt, SocketAddr, StatusCode,
    Uuid, Value,
};

pub(super) async fn design_import_and_htmx(client: &Client, pool: &PgPool) {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO flint_a2ui.design_systems(id,slug,name,tokens) VALUES($1,$2,'Fixture',$3)",
    )
    .bind(id)
    .bind(id.to_string())
    .bind(json!({"color":{"primary":"navy"}}))
    .execute(pool)
    .await
    .expect("private design fixture");
    let path = "/a2ui/v1/design-systems/import";
    let md = "# Design System: Sample\n\n## 1. Color\nprimary: blue\n";
    let zip = encoded_archive("theme/DESIGN.md", md.as_bytes());
    let readme = encoded_archive("theme/notes.md", b"Theme details");
    // A closed pool simulates a storage outage for both insert and update.
    // Every parser must reach persistence and return a stable, redacted error.
    let unavailable = PgPool::connect_lazy("postgres://localhost/unavailable").expect("lazy pool");
    unavailable.close().await;
    let app = Router::new()
        .route(
            path,
            axum::routing::post(crate::routes::design_import::import_design_system),
        )
        .with_state(crate::routes::a2ui::A2uiState { pool: unavailable })
        .layer(axum::middleware::from_fn(crate::rls_layer::require_rls));
    let importer = Client {
        app,
        token: client.token.clone(),
    };
    let tokens = json!({"color":{"primary":{"$value":"navy"}}});
    for (fmt, content, system) in [
        ("design_md", md.to_owned(), Some(id)),
        ("claude_design_zip", zip, None),
        ("w3c_tokens", tokens.to_string(), None),
    ] {
        let result = importer
            .json(
                "POST",
                path,
                &json!({"format":fmt, "content":content,"design_system_id":system}),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
            .await;
        assert_eq!(result, json!({"error":"database error"}));
    }
    assert_eq!(
        client
            .json(
                "GET",
                &format!("/a2ui/v1/design-systems/{id}/tokens"),
                &Value::Null,
                StatusCode::OK
            )
            .await["color"]["primary"],
        "navy"
    );
    for (fmt, content) in [
        ("w3c_tokens", "{"),
        ("design_md", "untitled"),
        ("other", ""),
        ("claude_design_zip", "?"),
        ("claude_design_zip", "eA=="),
        ("claude_design_zip", readme.as_str()),
    ] {
        assert!(client
            .json(
                "POST",
                path,
                &json!({"format":fmt,"content":content}),
                StatusCode::BAD_REQUEST
            )
            .await["error"]
            .is_string());
    }
    render_previews(client).await;
    sqlx::query("DELETE FROM flint_a2ui.design_systems WHERE id=$1")
        .bind(id)
        .execute(pool)
        .await
        .expect("remove design fixture");
}

async fn render_previews(client: &Client) {
    for (method, path, props, needle) in [
        (
            "GET",
            "/htmx/admin/registry",
            Value::Null,
            "Component Registry",
        ),
        (
            "GET",
            "/htmx/components/button",
            Value::Null,
            "data-flint-component=\"button\"",
        ),
        (
            "POST",
            "/htmx/components/button",
            json!({"label":"Send item"}),
            "Send item",
        ),
    ] {
        let response = client
            .request(method, path, &props, Some(&client.token))
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        let data = axum::body::to_bytes(response.into_body(), 2 * 1024 * 1024)
            .await
            .expect("HTML body");
        assert!(std::str::from_utf8(&data)
            .expect("UTF-8 HTML")
            .contains(needle));
    }
}

pub(super) async fn surface_preview(client: &Client, app_id: Uuid) {
    let url = format!("/htmx/surfaces/assemble?event_type=catalog.preview&application_id={app_id}");
    for (method, path, status, content) in [
        ("GET", url.as_str(), StatusCode::OK, "Review draft"),
        (
            "GET",
            "/htmx/surfaces/assemble?event_type=no.rule",
            StatusCode::BAD_REQUEST,
            "missing field",
        ),
        (
            "GET",
            "/htmx/components/absent-fixture",
            StatusCode::NOT_FOUND,
            "not found",
        ),
        (
            "POST",
            "/htmx/components/absent-fixture",
            StatusCode::NOT_FOUND,
            "not found",
        ),
        (
            "POST",
            "/htmx/components/button",
            StatusCode::OK,
            "Preview button",
        ),
    ] {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 10000))))
            .header("HX-Request", "true")
            .header("authorization", format!("Bearer {}", client.token))
            .header("content-type", "application/json")
            .body(Body::from(json!({"label":"Preview button"}).to_string()))
            .expect("HTMX request");
        let response = client
            .app
            .clone()
            .oneshot(request)
            .await
            .expect("render fragment");
        assert_eq!(response.status(), status, "{path}");
        let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("HTML");
        let html = std::str::from_utf8(&bytes).expect("UTF-8");
        assert!(html.contains(content), "{path}: {html}");
        assert!(
            !html.contains("<!DOCTYPE html>"),
            "HTMX clients expect a fragment"
        );
    }
}

fn encoded_archive(name: &str, contents: &[u8]) -> String {
    use base64::Engine as _;
    use std::io::{Cursor, Write};
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    archive
        .start_file(name, zip::write::SimpleFileOptions::default())
        .expect("archive entry");
    archive.write_all(contents).expect("document");
    base64::engine::general_purpose::STANDARD
        .encode(archive.finish().expect("archive").into_inner())
}
