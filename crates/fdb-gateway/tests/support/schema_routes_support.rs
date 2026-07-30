//! Shared harness for the `/schema/v1` route tests (p17-c004).
//!
//! Builds the real `SchemaApiState` — live `StateManager` (mirroring
//! `bootstrap.rs`, same construction as `gateway_startup_live_pg.rs`), real
//! `PgProvisioner` — and drives the actual handlers via
//! `tower::ServiceExt::oneshot`. No mocks on the path under test.

#![allow(clippy::expect_used)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use axum::routing::{get, post};
use axum::Router;
use fdb_gateway::schema_api::{self, SchemaApiState};
use fdb_postgres::{PgProvisioner, PgRest};
use fdb_reflection::{MutationGates, ReflectionEngine, StateManager};
use futures::{stream, StreamExt};
use sqlx::PgPool;
use tokio_postgres::NoTls;

pub fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

pub struct TestEnv {
    pub db_url: String,
    pub service_key: String,
    pub anon_key: Option<String>,
    pub state_manager: Arc<StateManager>,
}

impl TestEnv {
    /// Gate on `DATABASE_URL` + migration-0015 artifacts; `service_key` is a
    /// placeholder in this tier (auth-failure cases only).
    pub async fn database_only() -> Option<Self> {
        let db_url = env_nonempty("DATABASE_URL").or_else(|| {
            eprintln!("skipping: DATABASE_URL not set");
            None
        })?;

        let admin = admin_client(&db_url).await;
        let ledger: bool = admin
            .query_one(
                "SELECT to_regclass('flint_schema.provision_ledger') IS NOT NULL",
                &[],
            )
            .await
            .expect("regclass query")
            .get(0);
        if !ledger {
            eprintln!("skipping: migration 0015 artifacts absent");
            return None;
        }

        let pool = PgPool::connect(&db_url).await.expect("sqlx pool connect");
        let rest_pool = {
            let mut cfg = deadpool_postgres::Config::new();
            cfg.url = Some(db_url.clone());
            cfg.create_pool(
                Some(deadpool_postgres::Runtime::Tokio1),
                tokio_postgres::NoTls,
            )
            .expect("rest pool create")
        };
        let executor: Arc<dyn fdb_ports::SqlExecutor> = Arc::new(PgRest::new(rest_pool));
        let engine = ReflectionEngine::new(pool);
        let empty_factory = Arc::new(|_spec, _meta, _who| {
            stream::empty::<async_graphql::Result<async_graphql::Value>>().boxed()
        });
        let state_manager = Arc::new(
            StateManager::new_with_gates(
                engine,
                executor,
                db_url.clone(),
                MutationGates::default(),
                empty_factory,
            )
            .await
            .expect("initial schema compile"),
        );

        Some(Self {
            db_url,
            service_key: "unset".to_owned(),
            anon_key: None,
            state_manager,
        })
    }

    /// Additionally gate on the real Sansaba keys + a served JWKS
    /// (FFS-001 §9: real credentials, not synthetic tokens).
    pub async fn with_keys() -> Option<Self> {
        let mut env = Self::database_only().await?;
        let (Some(service_key), Some(_jwks)) = (
            env_nonempty("FLINT_SERVICE_ROLE_KEY"),
            env_nonempty("FLINT_GATE_JWKS_URL"),
        ) else {
            eprintln!("skipping: FLINT_SERVICE_ROLE_KEY / FLINT_GATE_JWKS_URL not set");
            return None;
        };
        env.service_key = service_key;
        env.anon_key = env_nonempty("FLINT_ANON_KEY");
        Some(env)
    }

    fn router_with(&self, provisioner: Option<Arc<dyn fdb_ports::SchemaProvisioner>>, namespaces: &[&str]) -> Router {
        let state = SchemaApiState {
            provisioner,
            namespaces: Arc::new(namespaces.iter().map(|s| (*s).to_owned()).collect()),
            state_manager: Arc::clone(&self.state_manager),
        };
        Router::new()
            .route("/schema/v1/plan", post(schema_api::plan::plan))
            .route("/schema/v1/apply", post(schema_api::apply::apply))
            .route("/schema/v1/status", get(schema_api::status::status))
            .with_state(state)
    }

    pub fn router_disabled(&self) -> Router {
        self.router_with(None, &[])
    }

    pub fn router_enabled(&self, namespaces: &[&str]) -> Router {
        let provisioner =
            PgProvisioner::from_url(&self.db_url).expect("provisioner pool from DATABASE_URL");
        self.router_with(Some(Arc::new(provisioner)), namespaces)
    }

    pub async fn reset_namespace(&self, ns: &str) {
        let admin = admin_client(&self.db_url).await;
        admin
            .batch_execute(&format!("DROP SCHEMA IF EXISTS {ns} CASCADE"))
            .await
            .expect("drop fixture schema");
        admin
            .execute(
                "DELETE FROM flint_schema.provision_ledger WHERE namespace = $1",
                &[&ns],
            )
            .await
            .expect("clear fixture ledger rows");
        admin
            .batch_execute(&format!(
                "CREATE SCHEMA {ns}; GRANT USAGE, CREATE ON SCHEMA {ns} TO flint_provisioner;"
            ))
            .await
            .expect("create granted fixture schema");
    }

    pub async fn table_exists(&self, ns: &str, table: &str) -> bool {
        let admin = admin_client(&self.db_url).await;
        admin
            .query_one(
                &format!("SELECT to_regclass('{ns}.{table}') IS NOT NULL"),
                &[],
            )
            .await
            .expect("regclass query")
            .get(0)
    }

    /// Simulate out-of-band drift: create the table behind the API's back.
    pub async fn create_out_of_band_table(&self, ns: &str, table: &str) {
        let admin = admin_client(&self.db_url).await;
        admin
            .batch_execute(&format!(
                "CREATE TABLE {ns}.{table} (id text PRIMARY KEY, tenant_id text NOT NULL, \
                 payload jsonb NOT NULL DEFAULT '{{}}'); \
                 ALTER TABLE {ns}.{table} ENABLE ROW LEVEL SECURITY;"
            ))
            .await
            .expect("out-of-band table");
    }
}

async fn admin_client(url: &str) -> tokio_postgres::Client {
    let (client, conn) = tokio_postgres::connect(url, NoTls)
        .await
        .expect("connect admin");
    tokio::spawn(async move {
        let _ = conn.await;
    });
    client
}

pub fn request(method: &str, uri: &str, bearer: Option<&str>, body: &[u8]) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(token) = bearer {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    builder
        .body(Body::from(body.to_vec()))
        .expect("request build")
}

pub async fn body_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    serde_json::from_slice(&bytes).expect("body is JSON")
}

/// A minimal tenant-scoped spec body for `ns.table`.
pub fn spec_body(ns: &str, table: &str) -> String {
    format!(
        r#"{{"namespace":"{ns}","tables":[{{"name":"{table}","tenantScoped":true,
            "columns":[
              {{"name":"id","type":"text","nullable":false,"primaryKey":true}},
              {{"name":"payload","type":"jsonb","nullable":false,"default":"'{{}}'"}}
            ]}}]}}"#
    )
}
