//! GraphQL subscription live-stream factory + WebSocket upgrade handling
//! (`graphql-transport-ws`, GET /graphql).
//!
//! Relocated from `main.rs` (p16 file-size split) — behavior unchanged.
//! Query/mutation HTTP handling lives in `crate::graphql`.

use std::sync::Arc;

use async_graphql_axum::{GraphQLProtocol, GraphQLWebSocket};
use axum::{
    extract::{State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde_json::json;

use fdb_app::Quarry;
use fdb_auth::rls_from_bearer;
use fdb_domain::{SubscriptionSpec, TableMeta};
use fdb_gateway::realtime_source::{resolve_change_source, ChangeSourceKind};
use fdb_postgres::{PgGraphQl, PgRest};
use fdb_realtime::{FabricChangeSource, FrfConfig, KetoConfig, ListenChangeSource, ListenConfig};
use fdb_reflection::compilers::graphql::SubStreamFactory;
use forge_identity::RlsContext;
use futures::stream::StreamExt;

use crate::GatewayState;

/// Build the GraphQL subscription live-stream factory.
///
/// Composes a [`Quarry`] from `PgRest` (RLS re-query), `PgGraphQl` (required by the
/// constructor; unused on this path), and a `ChangeStreamSource` — by default
/// `ListenChangeSource` (Postgres LISTEN/NOTIFY, real events). `FabricChangeSource`
/// (FRF gRPC) remains available via `FLINT_CHANGE_SOURCE=fabric` but fails closed
/// (`StreamError::Unavailable`) pending OQ-FRF-1 (FRF's `WatchEntityType` RPC
/// hasn't landed) — see `p16-c004`. Keto is threaded so the subscribe-time
/// coarse check runs inside whichever source is selected.
///
/// The returned factory, given a table's spec/meta and the subscriber's `RlsContext`,
/// yields the RLS-filtered, GraphQL-projected event stream. `spec.tenant` is filled
/// from the subscriber's claims here (the compiler has no per-subscriber context).
pub(crate) async fn build_subscription_factory(
    database_url: &str,
    keto_adapter: Arc<dyn fdb_ports::KetoCheck>,
) -> SubStreamFactory {
    let make_pool = || {
        let mut cfg = deadpool_postgres::Config::new();
        cfg.url = Some(database_url.to_owned());
        cfg.create_pool(
            Some(deadpool_postgres::Runtime::Tokio1),
            tokio_postgres::NoTls,
        )
        .expect("subscription pool create")
    };

    let sub_rest = Arc::new(PgRest::new(make_pool()));
    let sub_graphql = Arc::new(PgGraphQl::new(make_pool()));
    let realtime_keto_cfg = KetoConfig {
        base_url: std::env::var("KETO_BASE_URL").unwrap_or_else(|_| "http://keto:4466".into()),
    };

    // Select the change-stream backend. Default `listen` (in-process Postgres
    // LISTEN/NOTIFY — real events, no external dependency). `FLINT_CHANGE_SOURCE=fabric`
    // opts into the FRF gRPC adapter, which fails closed (StreamError::Unavailable)
    // pending OQ-FRF-1 (FRF's WatchEntityType RPC hasn't landed) — opt in only
    // once that RPC is confirmed available upstream. The default-vs-opt-in
    // decision itself is unit-tested in `realtime_source`.
    let change_source_env = std::env::var("FLINT_CHANGE_SOURCE").ok();
    let change_source: Arc<dyn fdb_ports::ChangeStreamSource> =
        match resolve_change_source(change_source_env.as_deref()) {
            ChangeSourceKind::Listen => {
                let cfg = ListenConfig {
                    database_url: database_url.to_owned(),
                    broadcast_capacity: std::env::var("FLINT_LISTEN_CAPACITY")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(1024),
                };
                Arc::new(
                    ListenChangeSource::new(cfg, realtime_keto_cfg)
                        .await
                        .expect("listen change source"),
                )
            }
            ChangeSourceKind::Fabric => {
                let frf_cfg = FrfConfig {
                    endpoint: std::env::var("FRF_ENDPOINT")
                        .unwrap_or_else(|_| "http://frf:50051".into()),
                };
                Arc::new(
                    FabricChangeSource::new(frf_cfg, realtime_keto_cfg)
                        .expect("fabric change source"),
                )
            }
        };
    let quarry =
        Arc::new(Quarry::new(sub_rest, sub_graphql, change_source).with_keto(keto_adapter));

    Arc::new(
        move |mut spec: SubscriptionSpec, table_meta: TableMeta, who: RlsContext| {
            let quarry = Arc::clone(&quarry);
            spec.tenant = who.tenant().map(|t| t.0).unwrap_or_default();
            // Defer to the use-case; flatten the "failed to open stream" error into a
            // single GraphQL error event so the field terminates cleanly.
            futures::stream::once(async move {
                quarry
                    .subscribe_graphql_values(spec, table_meta, &who)
                    .await
            })
            .flat_map(|opened| match opened {
                Ok(s) => s,
                Err(e) => {
                    futures::stream::once(
                        async move { Err(async_graphql::Error::new(e.to_string())) },
                    )
                    .boxed()
                }
            })
            .boxed()
        },
    )
}

/// GET /graphql — WebSocket upgrade for GraphQL subscriptions (graphql-transport-ws).
///
/// Reads the current compiled `subscription_schema` from `StateManager` on each
/// connection. The schema is hot-swapped on DDL changes; each new WS connection
/// gets the schema that was current at upgrade time.
///
/// Returns `503 Service Unavailable` when no subscription schema is available
/// (schema compile failed on startup or after DDL change).
pub(crate) async fn graphql_ws_handler(
    State(state): State<GatewayState>,
    protocol: GraphQLProtocol,
    ws: WebSocketUpgrade,
) -> Response {
    let compiled = state.state_manager.current();
    let Some(schema) = compiled.subscription_schema.clone() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "subscription schema not available"})),
        )
            .into_response();
    };

    ws.protocols(async_graphql::http::ALL_WEBSOCKET_PROTOCOLS)
        .on_upgrade(move |socket| async move {
            GraphQLWebSocket::new(socket, schema, protocol)
                // Authenticate at connection_init: extract the bearer from the
                // init payload, verify it, and inject the resulting RlsContext
                // into the connection Data. The subscription resolver reads it via
                // `ctx.data::<RlsContext>()`. Fail CLOSED — a missing or invalid
                // token returns Err, which rejects the WebSocket connection.
                .on_connection_init(connection_init_rls)
                .serve()
                .await;
        })
}

/// `graphql-transport-ws` `connection_init` handler: authenticate the socket.
///
/// Reads a bearer token from the init payload (`{"authorization": "Bearer …"}`
/// or `{"Authorization": "…"}`) and verifies it into an `RlsContext` installed in
/// the connection `Data`. Returns `Err` (rejecting the connection) when the token
/// is absent or invalid — this is the fail-closed auth gate for subscriptions.
///
/// SECURITY: never log the token or the derived `keto_subject`.
async fn connection_init_rls(
    payload: serde_json::Value,
) -> async_graphql::Result<async_graphql::Data> {
    let bearer = payload
        .get("authorization")
        .or_else(|| payload.get("Authorization"))
        .and_then(serde_json::Value::as_str)
        .map(|s| s.strip_prefix("Bearer ").unwrap_or(s).to_owned())
        .ok_or_else(|| async_graphql::Error::new("missing authorization in connection_init"))?;

    let rls = rls_from_bearer(&bearer)
        .await
        .map_err(|_| async_graphql::Error::new("invalid or expired token"))?;

    let mut data = async_graphql::Data::default();
    data.insert(rls);
    Ok(data)
}
