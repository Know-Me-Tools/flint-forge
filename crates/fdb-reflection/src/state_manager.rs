use std::sync::Arc;

use arc_swap::ArcSwap;
use fdb_ports::{KetoCheck, SqlExecutor};
use forge_policy::Pep;
use tokio::sync::watch;

use crate::{
    compiled::CompiledState,
    compilers::{
        graphql::{GraphQlCompiler, SubStreamFactory},
        openapi::OpenApiCompiler,
        rest::RestCompiler,
    },
    engine::ReflectionEngine,
    error::ReflectionError,
};

/// Optional authorization gates threaded into every REST recompile so that
/// hot-swapped routers keep enforcing Keto + Cedar on mutations.
#[derive(Clone, Default)]
pub struct MutationGates {
    /// Keto relationship-check client. `None` disables the coarse
    /// relationship gate on mutations (used only where Keto is not deployed,
    /// e.g. local dev).
    pub keto: Option<Arc<dyn KetoCheck>>,
    /// Cedar policy enforcement point. `None` disables the action/capability
    /// gate on mutations.
    pub pep: Option<Arc<dyn Pep>>,
}

/// Hot-swappable schema state manager.
///
/// Owns an `ArcSwap<CompiledState>` so readers always get a consistent snapshot
/// while the background listener atomically installs new compiled state on DDL change.
pub struct StateManager {
    compiled: Arc<ArcSwap<CompiledState>>,
    engine: Arc<ReflectionEngine>,
    /// REST/`rpc` executor — runs every reflection-compiler statement inside
    /// an RLS-scoped transaction. MUST be backed by a non-owner Postgres role
    /// distinct from `engine`'s privileged introspection/catalog pool; never
    /// the migration-owner pool (see `p16-c001`).
    executor: Arc<dyn SqlExecutor>,
    db_url: String,
    gates: MutationGates,
    /// Live-stream seam for GraphQL subscriptions, injected by the composition
    /// root. Every recompile (initial + hot-swap) threads this into
    /// `GraphQlCompiler::compile` so the subscription schema keeps its live
    /// stream body across DDL changes.
    sub_stream_factory: SubStreamFactory,
    /// Watch sender — notifies listeners whenever a new `CompiledState` is
    /// installed. Receivers see the new schema version. Used by the AG-UI
    /// state propagation task (p7-c007) to emit `StateSnapshot` events.
    version_tx: watch::Sender<u64>,
}

impl StateManager {
    /// Build a `StateManager` with authorization gates that are applied to
    /// every REST recompile (initial and hot-swap), performing the initial
    /// compile before returning. The process must not accept requests until
    /// this returns successfully.
    ///
    /// `sub_stream_factory` is the GraphQL subscription live-stream seam. It is
    /// mandatory and never mutated afterwards, so every served subscription has
    /// its live stream; see `GraphQlCompiler::compile`.
    ///
    /// # Errors
    ///
    /// Returns [`ReflectionError`] when the initial `ReflectionEngine::reflect`
    /// query fails, or when the reflected model fails the validation pass
    /// (`ReflectionError::Query`/`ReflectionError::Validation`). REST/GraphQL/
    /// MCP/A2UI compilation failures are logged and degrade gracefully
    /// (empty catalog, no subscription schema) rather than failing this call.
    pub async fn new_with_gates(
        engine: ReflectionEngine,
        executor: Arc<dyn SqlExecutor>,
        db_url: String,
        gates: MutationGates,
        sub_stream_factory: SubStreamFactory,
    ) -> Result<Self, ReflectionError> {
        let initial =
            Self::do_compile(&engine, Arc::clone(&executor), &gates, &sub_stream_factory).await?;
        let (version_tx, _) = watch::channel(initial.version);
        Ok(Self {
            compiled: Arc::new(ArcSwap::from_pointee(initial)),
            engine: Arc::new(engine),
            executor,
            db_url,
            gates,
            sub_stream_factory,
            version_tx,
        })
    }

    /// Return the currently active `CompiledState`.
    /// The returned `Arc` keeps the old state alive for the caller's lifetime —
    /// safe to hold across an `await` point inside a request handler.
    pub fn current(&self) -> Arc<CompiledState> {
        self.compiled.load_full()
    }

    /// Subscribe to schema version updates.
    ///
    /// The receiver fires whenever a new `CompiledState` is installed after a
    /// DDL change. Use this to emit AG-UI `StateSnapshot` events (p7-c007).
    pub fn subscribe_version(&self) -> watch::Receiver<u64> {
        self.version_tx.subscribe()
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
                    match Self::do_compile(
                        &self.engine,
                        Arc::clone(&self.executor),
                        &self.gates,
                        &self.sub_stream_factory,
                    )
                    .await
                    {
                        Ok(state) => {
                            let version = state.version;
                            self.compiled.store(Arc::new(state));
                            // `watch::Sender::send` errors only when there are no
                            // receivers; the authoritative state is already swapped
                            // into `self.compiled` above, so a missing subscriber
                            // just means no one needed the change notification.
                            let _ = self.version_tx.send(version);
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
        let mut listener = sqlx::postgres::PgListener::connect(&self.db_url).await?;
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

            match Self::do_compile(
                &self.engine,
                Arc::clone(&self.executor),
                &self.gates,
                &self.sub_stream_factory,
            )
            .await
            {
                Ok(state) => {
                    // ArcSwap::store is atomic — in-flight requests keep their old guard.
                    let version = state.version;
                    self.compiled.store(Arc::new(state));
                    // Same reasoning as the reconnect path above: `send` failing
                    // just means no subscriber is listening for the version bump;
                    // the swapped-in state is already authoritative.
                    let _ = self.version_tx.send(version);
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
        executor: Arc<dyn SqlExecutor>,
        gates: &MutationGates,
        sub_stream_factory: &SubStreamFactory,
    ) -> Result<CompiledState, ReflectionError> {
        let model = engine.reflect().await?;
        let router = RestCompiler::compile_with_gates(
            &model,
            executor,
            gates.keto.clone(),
            gates.pep.clone(),
        );
        let openapi_doc = OpenApiCompiler::compile(&model);
        let mcp_tools_doc = crate::compilers::mcp::McpCompiler::compile(&model);
        let mcp_count = mcp_tools_doc
            .get("tools")
            .and_then(|t| t.as_array())
            .map(Vec::len)
            .unwrap_or(0);
        tracing::info!(mcp_tools = mcp_count, "MCP tools compiled");
        let subscription_schema = match GraphQlCompiler::compile(&model, sub_stream_factory.clone())
        {
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
            mcp_tools_doc,
            subscription_schema,
            a2ui_catalog,
        })
    }
}
