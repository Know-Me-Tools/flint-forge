//! Change-stream backend selection for GraphQL subscriptions (p16-c004).
//!
//! Pulled out of `main.rs`'s `build_subscription_factory` into a pure function so
//! the default-vs-opt-in decision is unit-testable without touching real
//! environment variables (which would race across parallel test threads) or a
//! live database. `ListenChangeSource`'s actual event delivery is proven
//! separately by `fdb-realtime`'s `listen_change_source_watch_delivers_event`
//! (DATABASE_URL-gated); this module proves the OTHER half of the p16-c004
//! verification gate — that the default, with no env var set, resolves to it.

/// Selected change-stream backend for GraphQL subscriptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeSourceKind {
    /// Postgres LISTEN/NOTIFY — real events, no external dependency. The default.
    Listen,
    /// FRF gRPC — as of p16-c004 still returns an empty stream pending OQ-FRF-1
    /// (FRF's `WatchEntityType` RPC hasn't landed). Opt-in only.
    Fabric,
}

/// Resolve a `FLINT_CHANGE_SOURCE` env value to a [`ChangeSourceKind`].
///
/// Defaults to [`ChangeSourceKind::Listen`] unless the value is exactly
/// `"fabric"` — matching `main.rs`'s prior inline `!= Ok("fabric")` check, so
/// any unset, empty, or unrecognized value stays on the working default rather
/// than silently falling through to the empty-stream fabric adapter.
#[must_use]
pub fn resolve_change_source(env_value: Option<&str>) -> ChangeSourceKind {
    if env_value == Some("fabric") {
        ChangeSourceKind::Fabric
    } else {
        ChangeSourceKind::Listen
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_listen_when_unset() {
        assert_eq!(resolve_change_source(None), ChangeSourceKind::Listen);
    }

    #[test]
    fn defaults_to_listen_for_any_non_fabric_value() {
        assert_eq!(resolve_change_source(Some("")), ChangeSourceKind::Listen);
        assert_eq!(
            resolve_change_source(Some("bogus")),
            ChangeSourceKind::Listen
        );
        assert_eq!(
            resolve_change_source(Some("Fabric")),
            ChangeSourceKind::Listen
        );
    }

    #[test]
    fn opts_into_fabric_only_on_exact_lowercase_match() {
        assert_eq!(
            resolve_change_source(Some("fabric")),
            ChangeSourceKind::Fabric
        );
    }
}
