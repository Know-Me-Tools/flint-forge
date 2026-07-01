use std::sync::Arc;

use arc_swap::ArcSwap;
use fdb_ports::KetoCheck;
use forge_policy::Pep;

use crate::{
    compiled::CompiledState,
    compilers::{graphql::GraphQlCompiler, openapi::OpenApiCompiler, rest::RestCompiler},
    engine::ReflectionEngine,
    error::ReflectionError,
};

/// Optional authorization gates threaded into every REST recompile so that
/// hot-swapped routers keep enforcing Keto + Cedar on mutations.
#[derive(Clone, Default)]
pub struct MutationGates {
    pub keto: Option<Arc<dyn KetoCheck>>,
    pub pep: Option<Arc<dyn Pep>>,
}

/// Hot-swappable schema state manager.
///
/// Owns an `ArcSwap<CompiledState>` so readers always get a consistent snapshot
/// while the background listener atomically installs new compiled state on DDL change.
pub struct StateManager {
    compiled: Arc<ArcSwap<CompiledState>>,
    engine: Arc<ReflectionEngine>,
    pool: sqlx::PgPool,
    db_url: String,
    gates: MutationGates,
}

impl StateManager {
    /// Build a `StateManager`, performing the initial compile before returning.
    /// The process must not accept requests until this returns successfully.
    pub async fn new(
        engine: ReflectionEngine,
        pool: sqlx::PgPool,
        db_url: String,
    ) -> Result<Self, ReflectionError> {
        Self::new_with_gates(engine, pool, db_url, MutationGates::default()).await
    }

    /// Build a `StateManager` with authorization gates that are applied to
    /// every REST recompile (initial and hot-swap).
    pub async fn new_with_gates(
        engine: ReflectionEngine,
        pool: sqlx::PgPool,
        db_url: String,
        gates: MutationGates,
    ) -> Result<Self, ReflectionError> {
        let initial = Self::do_compile(&engine, pool.clone(), &gates).await?;
        Ok(Self {
            compiled: Arc::new(ArcSwap::from_pointee(initial)),
            engine: Arc::new(engine),
            pool,
            db_url,
            gates,
        })
    }

    /// Return the currently active `CompiledState`.
    /// The returned `Arc` keeps the old state alive for the caller's lifetime —
    /// safe to hold across an `await` point inside a request handler.
    pub fn current(&self) -> Arc<CompiledState> {
        self.compiled.load_full()
    }

    /// Spawn the background `PgListener` loop.
    /// Returns a `JoinHandle` the caller can abort on graceful shutdown.
    pub fn start_listener(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            self.listen_loop().await;
        })
    }

    async fn listen_loop(&self) {
        loop {
            match self.run_listener().await {
                Ok(()) => break,
                Err(e) => {
                    // SECURITY: log the error code only; do not log JWT payloads or claim values.
                    tracing::error!(error = %e, "PgListener disconnected; recompiling in 2s");
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                    // Force full recompile on reconnect — notifications may have been missed.
                    match Self::do_compile(&self.engine, self.pool.clone(), &self.gates).await {
                        Ok(state) => {
                            self.compiled.store(Arc::new(state));
                            tracing::info!("schema recompiled after PgListener reconnect");
                        }
                        Err(compile_err) => {
                            tracing::error!(
                                error = %compile_err,
                                "recompile failed after reconnect; serving stale state"
                            );
                            // Do NOT swap in a failed state — keep serving stale.
                        }
                    }
                }
            }
        }
    }

    async fn run_listener(&self) -> Result<(), sqlx::Error> {
        let mut listener =
            sqlx::postgres::PgListener::connect(&self.db_url).await?;
        listener.listen("meta_runtime").await?;
        tracing::info!("PgListener connected; listening on meta_runtime");

        loop {
            // `recv()` returns `Err` on connection loss — outer loop reconnects.
            let notification = listener.recv().await?;
            tracing::debug!(
                channel = notification.channel(),
                payload = notification.payload(),
                "meta_runtime notification — triggering recompile"
            );

            match Self::do_compile(&self.engine, self.pool.clone(), &self.gates).await {
                Ok(state) => {
                    // ArcSwap::store is atomic — in-flight requests keep their old guard.
                    self.compiled.store(Arc::new(state));
                    tracing::info!("schema hot-swap complete");
                }
                Err(e) => {
                    tracing::error!(error = %e, "recompile failed; serving previous state");
                }
            }
        }
    }

    async fn do_compile(
        engine: &ReflectionEngine,
        pool: sqlx::PgPool,
        gates: &MutationGates,
    ) -> Result<CompiledState, ReflectionError> {
        let model = engine.reflect().await?;
        let router = RestCompiler::compile_with_gates(
            &model,
            pool,
            gates.keto.clone(),
            gates.pep.clone(),
        );
        let openapi_doc = OpenApiCompiler::compile(&model);
        let subscription_schema = match GraphQlCompiler::compile(&model) {
            Ok(schema) => Some(schema),
            Err(e) => {
                tracing::warn!(error = %e, "GraphQlCompiler failed; subscription schema unavailable");
                None
            }
        };
        let a2ui_catalog = match engine.load_a2ui_catalog().await {
            Ok(catalog) => {
                tracing::info!(components = catalog.components.len(), "A2UI catalog loaded");
                Arc::new(catalog)
            }
            Err(e) => {
                tracing::warn!(error = %e, "A2UI catalog load failed; serving empty catalog");
                Arc::new(crate::compiled::A2uiCatalog::empty())
            }
        };
        Ok(CompiledState {
            version: model.version,
            database_model: Arc::new(model),
            router: Arc::new(router),
            openapi_doc,
            subscription_schema,
            a2ui_catalog,
        })
    }
}
