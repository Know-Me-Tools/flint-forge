//! Exercise the same router returned by gateway startup against migrated CI Postgres.
//! Authentication settings and the verifier cache are process-global. A child
//! test process gives this fixture its own JWKS, pool and listener lifetimes.
use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode},
    response::Response,
    routing::get,
    Json, Router,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::{
    env,
    net::SocketAddr,
    time::{SystemTime, UNIX_EPOCH},
};
use tower::ServiceExt;
use uuid::Uuid;
#[path = "gateway_data_routes.rs"]
mod data;
use crate::gateway_fixture_database as fixture;
#[path = "../../../forge-identity/tests/common/mod.rs"]
mod keys;
#[path = "gateway_protocols.rs"]
mod protocols;
#[path = "gateway_schema_cycle.rs"]
mod schema;
#[path = "gateway_views.rs"]
mod views;
#[path = "gateway_websocket.rs"]
mod websocket;

struct Client {
    app: Router,
    token: String,
}
impl Client {
    async fn request(
        &self,
        method: &str,
        path: &str,
        body: &Value,
        token: Option<&str>,
    ) -> Response {
        let mut req = Request::builder()
            .method(method)
            .uri(path)
            .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 10000))))
            .header("content-type", "application/json");
        if let Some(token) = token {
            req = req.header("authorization", format!("Bearer {token}"));
        }
        self.app
            .clone()
            .oneshot(req.body(Body::from(body.to_string())).expect("request"))
            .await
            .expect("serve")
    }
    async fn json(&self, method: &str, path: &str, body: &Value, status: StatusCode) -> Value {
        let response = self.request(method, path, body, Some(&self.token)).await;
        let actual = response.status();
        let data = axum::body::to_bytes(response.into_body(), 16 * 1024 * 1024)
            .await
            .expect("body");
        assert_eq!(
            actual,
            status,
            "{method} {path}: {}",
            String::from_utf8_lossy(&data)
        );
        serde_json::from_slice(&data).expect("JSON")
    }
    async fn rpc(&self, path: &str, method: &str, params: Value) -> Value {
        let response = self
            .json(
                "POST",
                path,
                &json!({"jsonrpc":"2.0","id":7,"method":method,"params":params}),
                StatusCode::OK,
            )
            .await;
        assert_eq!(response["id"], 7);
        response
    }
}

#[tokio::test]
async fn serves_compiled_and_authenticated_routes() {
    let Ok(url) = env::var("DATABASE_URL") else {
        return;
    };
    if env::var_os("FORGE_GATEWAY_ROUTER_TEST_CHILD").is_none() {
        let query = url.split_once('?').map(|(_, query)| query);
        fixture::with_database(&url, |options| async move {
            use sqlx::ConnectOptions as _;
            let mut database_url = options.to_url_lossy();
            // Keep the operator's options. SQLx adds client-only parameters
            // on serialization that the gateway's tokio-postgres pool rejects.
            database_url.set_query(query);
            let status = tokio::process::Command::new(env::current_exe().expect("test binary"))
                .args([
                    "--exact",
                    "gateway_http_tests::serves_compiled_and_authenticated_routes",
                    "--nocapture",
                ])
                .env(
                    "FORGE_GATEWAY_ROUTER_TEST_CHILD",
                    options.get_database().expect("fixture database name"),
                )
                .env("DATABASE_URL", database_url.as_str())
                .env_remove("OTEL_EXPORTER_OTLP_ENDPOINT")
                .env("FLINT_A2UI_EMBED_MODEL", "tongyi-embedding-vision-plus")
                .env("FLINT_A2UI_EMBED_DIMENSIONS", "1152")
                .env("FLINT_AUTHZ_MODE", "rls")
                .env("FLINT_RATE_LIMIT_REST", "100")
                .env("FLINT_RATE_LIMIT_BURST", "500")
                .env("FLINT_FILE_METADATA_ENABLED", "true")
                .env("FLINT_CHANGE_SOURCE", "listen")
                .status()
                .await
                .expect("isolated router test");
            assert!(status.success());
        })
        .await;
        return;
    }
    exercise_gateway(&url).await;
}

async fn exercise_gateway(url: &str) {
    let key = keys::generate_es256_key("gateway-ci");
    let jwks = keys::jwks_response(&[&key]);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("JWKS listener");
    let issuer = format!("http://{}", listener.local_addr().expect("JWKS address"));
    env::set_var("FLINT_GATE_JWKS_URL", format!("{issuer}/jwks"));
    env::set_var("FLINT_GATE_ISSUER", &issuer);
    env::set_var("FLINT_GATE_AUDIENCE", "gateway-test");
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new().route("/jwks", get(move || async move { Json(jwks) })),
        )
        .await
        .expect("JWKS");
    });
    let sub = Uuid::new_v4();
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::ES256);
    header.kid = Some(key.kid);
    let exp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_secs()
        + 300;
    let token = |role: &str| {
        let claims = json!({"sub":sub,"role":role,"tenant_id":"fixture","aud":"gateway-test","iss":issuer,"exp":exp});
        jsonwebtoken::encode(&header, &claims, &key.encoding_key).expect("test JWT")
    };
    let namespace = format!("ci_{}", sub.simple());
    env::set_var("FLINT_PROVISION_NAMESPACES", &namespace);
    env::set_var("PROVISIONER_DATABASE_URL", url);
    let pool = PgPool::connect(url).await.expect("migrated database");
    let actual: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(&pool)
        .await
        .expect("current database");
    let expected = env::var("FORGE_GATEWAY_ROUTER_TEST_CHILD").expect("fixture database marker");
    assert_eq!(actual, expected);
    assert!(actual
        .strip_prefix("forge_test_")
        .is_some_and(|suffix| Uuid::parse_str(suffix).is_ok()));
    data::install_extensions(&pool).await;
    check_bootstrap_owner(url).await;
    let _tracing = crate::telemetry::init_tracing();
    let app = crate::bootstrap::build_router(pool.clone(), url.to_owned())
        .await
        .expect("gateway router");
    let service = Client {
        app: app.clone(),
        token: token("service_role"),
    };
    let client = Client {
        app,
        token: token("authenticated"),
    };
    schema::provision(&service, &pool, &namespace).await;
    data::query_and_discover(&client, &pool).await;
    websocket::subscriptions(&client, &pool).await;
    check_discovery_routes(&client).await;
    let table = format!("ci_{}", sub.simple());
    let binding:Uuid=sqlx::query_scalar("INSERT INTO flint_a2ui.bindings(table_schema,table_name,binding_type,component_id,config) SELECT 'public',$1,'grid',id,$2 FROM flint_a2ui.components WHERE slug='data-grid' RETURNING id")
        .bind(&table).bind(json!({"columns":["name"],"fields":["name"]})).fetch_one(&pool).await.expect("private fixture binding");
    sqlx::query("INSERT INTO flint_a2ui.applications(id,slug,name,owner_id) VALUES($1,$2,'Catalog fixture',$1)")
        .bind(sub).bind(&table).execute(&pool).await.expect("owned application");
    sqlx::query("INSERT INTO flint_a2ui.assembly_rules(application_id,event_type,assembly_config) VALUES($1,'catalog.preview',$2)")
        .bind(sub).bind(json!({"component_slug":"button","props":{"label":"Review draft"}}))
        .execute(&pool).await.expect("preview assembly rule");
    catalog_protocols(&client, &table).await;
    views::surface_preview(&client, sub).await;
    sqlx::query("DELETE FROM flint_a2ui.assembly_rules WHERE application_id=$1")
        .bind(sub)
        .execute(&pool)
        .await
        .expect("remove preview rule");
    sqlx::query("DELETE FROM flint_a2ui.applications WHERE id=$1")
        .bind(sub)
        .execute(&pool)
        .await
        .expect("remove owned application");
    views::design_import_and_htmx(&client, &pool).await;
    run_lifecycle(&client, &table).await;
    tokio::join!(
        data::scrape_pool_metrics(&client),
        protocols::receive_mcp_ping(&client)
    );
    sqlx::query("DELETE FROM flint_a2ui.bindings WHERE id=$1")
        .bind(binding)
        .execute(&pool)
        .await
        .expect("remove fixture binding");
    server.abort();
}

async fn check_bootstrap_owner(url: &str) {
    let reader = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query("SET ROLE authenticated")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(url)
        .await
        .expect("preflight reader connection");
    let preflight = crate::role_preflight::check(&reader)
        .await
        .expect_err("migration owner required");
    assert!(preflight.to_string().contains("does not own the"));
    reader.close().await;
}

async fn check_discovery_routes(client: &Client) {
    let health = client
        .json("GET", "/healthz", &Value::Null, StatusCode::OK)
        .await;
    assert_eq!(health["service"], "flint-quarry");
    let docs = client
        .json("GET", "/openapi.json", &Value::Null, StatusCode::OK)
        .await;
    assert!(docs["paths"].is_object());
    for token in [None, Some("expired")] {
        for (method, path, body) in [
            ("GET", "/a2ui/v1/components", Value::Null),
            ("POST", "/graphql", json!({"query":"{__typename}"})),
        ] {
            assert_eq!(
                client.request(method, path, &body, token).await.status(),
                StatusCode::UNAUTHORIZED
            );
        }
    }
    let files = client
        .json(
            "GET",
            "/files/v1/capabilities",
            &Value::Null,
            StatusCode::OK,
        )
        .await;
    assert_eq!(files["data"]["api_version"], "files/v1");
}

async fn catalog_protocols(client: &Client, table: &str) {
    let bindings = client
        .json(
            "GET",
            &format!("/a2ui/v1/components/bindings/public/{table}"),
            &Value::Null,
            StatusCode::OK,
        )
        .await;
    assert_eq!(bindings["bindings"][0]["table_name"], table);
    let context = json!({"data_source":{"schema":"public","table":table}});
    let applications = client
        .json("GET", "/a2ui/v1/applications", &Value::Null, StatusCode::OK)
        .await;
    let app = applications["applications"]
        .as_array()
        .expect("applications")
        .iter()
        .find(|a| a["slug"] == table)
        .expect("owned application");
    let id = app["id"].as_str().expect("application id");
    assert_eq!(
        client
            .json(
                "GET",
                &format!("/a2ui/v1/applications/{id}"),
                &Value::Null,
                StatusCode::OK
            )
            .await["application"]["slug"],
        table
    );
    protocols::mcp_tools(client, table, id, &context).await;
    protocols::a2a_tasks(client, id, &context).await;
    protocols::request_errors(client).await;
}

async fn run_lifecycle(client: &Client, table: &str) {
    let run = client
        .json("POST", "/agents/v1/runs", &Value::Null, StatusCode::OK)
        .await;
    let id = run["run_id"].as_str().expect("run id");
    let events = run["events_url"].as_str().expect("events URL");
    client
        .json(
            "POST",
            events,
            &json!({"event":{"type":"RunStarted","run_id":"other-run"}}),
            StatusCode::BAD_REQUEST,
        )
        .await;
    let stream = client
        .request("GET", events, &Value::Null, Some(&client.token))
        .await;
    assert_eq!(stream.headers()["content-type"], "text/event-stream");
    let surface = format!("/agents/v1/{id}/surfaces/assemble");
    let missing = client
        .json(
            "POST",
            "/agents/v1/absent-fixture/surfaces/assemble",
            &json!({"event_type":"mount"}),
            StatusCode::NOT_FOUND,
        )
        .await;
    assert_eq!(missing["error"], "run not found");
    client
        .json(
            "POST",
            &surface,
            &json!({"event_type":"mount"}),
            StatusCode::BAD_REQUEST,
        )
        .await;
    let assembled = client
        .json(
            "POST",
            &surface,
            &json!({"event_type":"mount","event_context":{"data_source":{"table":table}}}),
            StatusCode::OK,
        )
        .await;
    assert_eq!(assembled["status"], "emitted");
    for event in [
        json!({"type":"RunStarted","run_id":id}),
        json!({"type":"StateDelta","run_id":id,"delta":[]}),
        json!({"type":"StateSnapshot","run_id":id,"state":{"revision":2}}),
        json!({"type":"RunFinished","run_id":id}),
    ] {
        client
            .json("POST", events, &json!({"event":event}), StatusCode::OK)
            .await;
    }
    let data = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        axum::body::to_bytes(stream.into_body(), 64 * 1024),
    )
    .await
    .expect("stream terminates")
    .expect("stream body");
    let body = std::str::from_utf8(&data).expect("SSE text");
    assert!(body.contains("a2ui:surface"));
    for kind in ["RunStarted", "StateDelta", "StateSnapshot"] {
        assert!(body.contains(kind), "{body}");
    }
}
