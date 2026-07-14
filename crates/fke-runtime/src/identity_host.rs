//! `flint:host/identity` — verified origin JWT context, sourced from the
//! `RlsContext` already established for this invocation (see
//! `EdgeRuntime::handle_with_telemetry`).

use crate::host_bindings::flint::host::identity::Host;
use crate::KilnHostState;
use fke_domain::Capability;

impl Host for KilnHostState {
    /// `None` unless `Capability::Identity` is granted. The WIT contract
    /// additionally calls for a finer "may see the *raw* JWT" sub-grant
    /// distinct from `claims()` visibility (default-deny) — the domain model
    /// doesn't yet have a bit for that distinction, so this checks the same
    /// `Identity` capability `claims()` does. Revisit if/when a dedicated
    /// grant is added.
    fn origin_jwt(&mut self) -> Option<String> {
        if !self.granted.contains(&Capability::Identity) {
            return None;
        }
        self.identity.as_ref().map(|rls| rls.raw_bearer.clone())
    }

    /// Always returns a value (never traps) — `{}` when no caller identity
    /// was established for this invocation (e.g. BGW / system-level calls).
    fn claims(&mut self) -> String {
        self.identity
            .as_ref()
            .map_or_else(|| "{}".to_owned(), |rls| rls.claims_json.clone())
    }
}
