//! Flint Quarry composition root: REST, GraphQL (Q/M/Sub), /rpc, /healthz.
#![forbid(unsafe_code)]

mod agui_hook_dispatcher;
mod bootstrap;
mod graphql;
mod handlers;
mod keto_sync;
mod policy_source;
mod rls_layer;
mod routes;
mod subscriptions;
mod telemetry;

#[cfg(test)]
mod gateway_tests;

use std::sync::Arc;

use fdb_postgres::{PgGraphQl, PgVectorRpc};
use fdb_reflection::StateManager;
use sqlx::PgPool;

/// Shared gateway state — available in all route handlers via `State<GatewayState>`.
#[derive(Clone)]
pub(crate) struct GatewayState {
    state_manager: Arc<StateManager>,
    graphql_executor: Arc<PgGraphQl>,
    vector_rpc: Arc<PgVectorRpc>,
    /// Privileged reflection pool used by A2UI routes and background tasks.
    /// SECURITY: never expose this pool to per-user RLS contexts.
    pool: PgPool,
    /// Keto relation-check adapter backed by the background sync cache.
    //
    // p16-c006: corrected — this field itself is genuinely never read by any
    // handler (verified: `state.keto`/`.keto.as_ref()` has no call sites).
    // That is NOT a gap: REST/GraphQL mutation Keto enforcement is wired
    // through `MutationGates` (see `gates`/`with_keto()` at construction),
    // which handlers reach via the reflection compiler, not via this field
    // directly. Retained as a construction-site convenience for any future
    // handler needing direct Keto access outside the gates path.
    #[allow(dead_code)]
    keto: Option<Arc<dyn fdb_ports::KetoCheck>>,
}

// Composition root: the actual binary entry point. All wiring of pools,
// adapters, gates, and routes lives in `bootstrap::run()` (p16 file-size
// split of what used to be a single long `main()` body) — behavior unchanged.
#[tokio::main]
async fn main() {
    bootstrap::run().await;
}
