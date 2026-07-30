//! Flint Quarry library surface — exposed for integration tests and binary reuse.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod a2ui_embedder;
/// Keto relation-tuple cache and its `flint_meta.keto_tuples` sync task.
///
/// Exposed on the library target (rather than staying private to the binary)
/// so `prime()`'s real SQL can be exercised by a `DATABASE_URL`-gated
/// integration test — the same reason [`realtime_source`] was extracted in
/// p16-c004. The column mapping this module fixes could otherwise only fail
/// against a live database, on a write.
pub mod keto_sync;
pub mod realtime_source;
/// The `/schema/v1` provisioning API (FFS-001, p17-c004).
///
/// On the library target for the same reason as [`keto_sync`]: the route
/// group and its `SchemaApiState` must be constructible by
/// `DATABASE_URL`-gated integration tests, which drive the real handlers
/// through `tower::ServiceExt::oneshot`.
pub mod schema_api;
