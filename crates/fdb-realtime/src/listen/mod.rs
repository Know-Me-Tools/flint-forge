//! In-process `ChangeStreamSource` over Postgres `LISTEN`/`NOTIFY` — the OQ-FRF-1
//! workaround for the missing FRF `WatchEntityType` gRPC RPC.
//!
//! This adapter has EXACTLY two jobs:
//!
//! 1. **Subscribe-time Keto coarse check** (fail closed) — reuses the crate's
//!    `keto_check_via_http` helper verbatim.
//! 2. **Producing the raw `ChangeEvent` stream** from `LISTEN`/`NOTIFY`.
//!
//! It does NOT perform the per-event RLS re-query. That is owned by the use-case
//! layer (`fdb_app::Quarry::subscribe_rls_filtered`), which layers `build_pk_filters`
//! plus a `RestExecutor` round-trip on top of whatever `watch()` returns. No
//! `fdb-app` dependency is added here.
//!
//! # Critical downstream contract
//!
//! Because the use-case rebuilds primary-key filters from `record`/`old_record` to
//! re-fetch the current row under RLS, the raw `ChangeEvent` MUST always carry the
//! primary-key column values — even when a wide row is truncated to fit the 8000-byte
//! `NOTIFY` limit. The migration's trigger degrades to a PK-only image in that case
//! (see `migrations/0007_change_notify.sql`). The full `record` in the untruncated
//! case is an optimization the RLS re-query overwrites; it is NEVER trusted as the
//! delivered row.
//!
//! # Security invariants
//!
//! - `who.keto_subject` is PII and MUST NOT appear in any tracing span/log or error.
//! - The subscribe-time Keto check FAILS CLOSED: Keto unreachable => deny, never allow.
//! - Every failure mode here can only cause a *missed* event, never an *unauthorized*
//!   one, because the downstream RLS re-query is authoritative on every delivered event.
//!
//! # Cargo changes required
//!
//! In `crates/fdb-realtime/Cargo.toml`, add under `[dependencies]`:
//!
//! ```toml
//! sqlx = { workspace = true }
//! tokio = { workspace = true }
//! tokio-stream = { workspace = true }
//! ```
//!
//! `sqlx` (0.8, features postgres + runtime-tokio + json) and `tokio` (features full)
//! already exist in the root `[workspace.dependencies]`. `tokio-stream` does NOT — add
//! it to the root `Cargo.toml` `[workspace.dependencies]`:
//!
//! ```toml
//! tokio-stream = { version = "0.1", features = ["sync"] }
//! ```
//!
//! (The `sync` feature gates `tokio_stream::wrappers::BroadcastStream`.)
//!
//! The unit tests use `wiremock` for the Keto happy/denied HTTP paths. Add it to
//! `crates/fdb-realtime/Cargo.toml` under `[dev-dependencies]` (not needed at
//! runtime): `wiremock = "0.6"` (or `wiremock = { workspace = true }` if pinned in
//! the root `[workspace.dependencies]`). The load-bearing `Unavailable` fail-closed
//! test uses a dead port and needs no extra dependency.
//!
//! # Module layout
//!
//! This module is split across files by seam, pure relocation with no behavior
//! change:
//!
//! - [`config`] — `ListenConfig`.
//! - [`error`] — `ListenError` and the DSN-safe `redact` helper.
//! - [`source`] — the `ListenChangeSource` struct, its background-task guard, and
//!   [`ListenChangeSource::new`].
//! - [`watch`] — the `ChangeStreamSource for ListenChangeSource` trait impl.
//! - [`listen_loop`] — the background task that owns the `PgListener` and fans
//!   decoded events out over the broadcast channel.
//! - [`payload`] — the `NOTIFY` payload wire shape and its parser.
//! - [`validate`] — defense-in-depth identifier validation and the fan-out filter
//!   predicate.

use std::time::Duration;

mod config;
mod error;
mod listen_loop;
mod payload;
mod source;
mod validate;
mod watch;

pub use config::ListenConfig;
pub use error::ListenError;
pub use source::ListenChangeSource;

/// The single fixed `LISTEN`/`NOTIFY` channel. MUST match `pg_notify(...)` in
/// `migrations/0007_change_notify.sql`.
const CHANNEL: &str = "flint_change";

/// Migration-side threshold (bytes) at which the trigger degrades to a PK-only
/// image. Documented here only to keep the Rust and SQL sides in sync; the Rust
/// adapter never enforces it (Postgres does).
const _MAX_NOTIFY_BYTES: usize = 7500;

/// Backoff between reconnect attempts in the listen loop, to avoid a hot spin
/// when the connection is flapping.
const RECONNECT_BACKOFF: Duration = Duration::from_millis(500);

/// Default broadcast capacity when the caller does not care to tune it.
const DEFAULT_BROADCAST_CAPACITY: usize = 1024;

#[cfg(test)]
mod tests;
