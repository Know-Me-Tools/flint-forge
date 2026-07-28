//! Authorization-model selection for the gateway.
//!
//! flint-forge's authorization story is **Postgres grants + RLS**, enforced on
//! every route by `rls_layer` and `SET LOCAL ROLE` / `request.jwt.claims`
//! (`fdb-postgres::backend`), plus the Cedar capability gate. That is the same
//! model Supabase implements, and it is complete on its own.
//!
//! Ory Keto (Zanzibar-style relation tuples) is an **additional, coarse**
//! pre-filter in front of that. Some deployments need it; many do not. An
//! application whose model is "every authenticated principal may mutate" has no
//! per-subject tuples to seed, and because the Keto gate is fail-closed, an
//! empty tuple set denies **every** mutation while reads keep working — a
//! configuration mismatch that presents as a policy bug. Making the mode
//! explicit is what stops that.
//!
//! A pure function over the env value (rather than reading the variable inline)
//! so the decision is unit-testable without racing on process environment across
//! parallel test threads — the same reason `realtime_source::resolve_change_source`
//! is shaped this way.

/// The authorization model this gateway enforces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthzMode {
    /// Postgres grants + RLS, plus the Cedar capability gate. **The default.**
    ///
    /// No Keto anywhere: no relation-tuple gate on mutations, no tuple-sync
    /// background task, and no HTTP dependency on a Keto service for
    /// subscriptions.
    Rls,
    /// `Rls`, plus Keto relation checks on mutations and subscriptions.
    ///
    /// Requires seeded tuples in `flint_meta.keto_tuples`; with an empty tuple
    /// set every mutation is denied, so the gateway refuses to start rather than
    /// serving traffic that would 403 uniformly.
    RlsKeto,
}

impl AuthzMode {
    /// Whether Keto relation checks participate in authorization.
    #[must_use]
    pub fn keto_enabled(self) -> bool {
        matches!(self, AuthzMode::RlsKeto)
    }
}

/// The env var selecting the authorization model.
pub const AUTHZ_MODE_VAR: &str = "FLINT_AUTHZ_MODE";

/// Deprecated predecessor. Named only the mutation half, so a deployment that
/// set it still could not open a subscription without a reachable Keto service.
pub const LEGACY_KETO_GATE_VAR: &str = "FLINT_KETO_MUTATION_GATE";

/// Resolve `FLINT_AUTHZ_MODE` to an [`AuthzMode`].
///
/// Unset or empty ⇒ [`AuthzMode::Rls`].
///
/// # Errors
///
/// Returns the offending value when it is not recognized. An unrecognized value
/// is deliberately **not** coerced to a default: silently falling back would
/// mean a typo (`rls+ketto`) either quietly disables an authorization layer or
/// quietly enables one that denies every write. Both are worse than refusing to
/// start.
pub fn resolve_authz_mode(env_value: Option<&str>) -> Result<AuthzMode, String> {
    match env_value.map(str::trim) {
        None | Some("") => Ok(AuthzMode::Rls),
        Some(v) if v.eq_ignore_ascii_case("rls") => Ok(AuthzMode::Rls),
        Some(v) if v.eq_ignore_ascii_case("rls+keto") => Ok(AuthzMode::RlsKeto),
        Some(other) => Err(other.to_string()),
    }
}

/// Apply the deprecated `FLINT_KETO_MUTATION_GATE` to an already-resolved mode.
///
/// `off` / `0` map to [`AuthzMode::Rls`]. Honoured only when `FLINT_AUTHZ_MODE`
/// is unset, so the current variable always wins and a deployment carrying both
/// is not silently governed by the older one.
///
/// Returns the mode and whether the legacy variable actually took effect, so the
/// caller can emit a deprecation warning exactly when it mattered.
#[must_use]
pub fn apply_legacy_gate_var(
    mode: AuthzMode,
    authz_mode_set: bool,
    legacy_value: Option<&str>,
) -> (AuthzMode, bool) {
    if authz_mode_set {
        return (mode, false);
    }
    let disables = legacy_value
        .map(str::trim)
        .is_some_and(|v| v.eq_ignore_ascii_case("off") || v == "0");

    if disables {
        (AuthzMode::Rls, true)
    } else {
        (mode, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_rls_when_unset() {
        // The Supabase default: grants + RLS are the whole story.
        assert_eq!(resolve_authz_mode(None), Ok(AuthzMode::Rls));
    }

    #[test]
    fn defaults_to_rls_when_empty_or_whitespace() {
        // An env var set to "" by a compose template must not be an error.
        assert_eq!(resolve_authz_mode(Some("")), Ok(AuthzMode::Rls));
        assert_eq!(resolve_authz_mode(Some("   ")), Ok(AuthzMode::Rls));
    }

    #[test]
    fn parses_both_modes_case_insensitively() {
        assert_eq!(resolve_authz_mode(Some("rls")), Ok(AuthzMode::Rls));
        assert_eq!(resolve_authz_mode(Some("RLS")), Ok(AuthzMode::Rls));
        assert_eq!(resolve_authz_mode(Some("rls+keto")), Ok(AuthzMode::RlsKeto));
        assert_eq!(resolve_authz_mode(Some("RLS+Keto")), Ok(AuthzMode::RlsKeto));
    }

    #[test]
    fn rejects_an_unrecognized_value_instead_of_defaulting() {
        // A typo must not silently pick an authorization model. This is the
        // whole reason the function returns Result.
        assert_eq!(
            resolve_authz_mode(Some("rls+ketto")),
            Err("rls+ketto".into())
        );
        assert_eq!(resolve_authz_mode(Some("none")), Err("none".into()));
        assert_eq!(resolve_authz_mode(Some("off")), Err("off".into()));
    }

    #[test]
    fn keto_is_enabled_only_in_the_combined_mode() {
        assert!(!AuthzMode::Rls.keto_enabled());
        assert!(AuthzMode::RlsKeto.keto_enabled());
    }

    #[test]
    fn legacy_var_disables_keto_when_authz_mode_is_unset() {
        let (mode, used) = apply_legacy_gate_var(AuthzMode::RlsKeto, false, Some("off"));
        assert_eq!(mode, AuthzMode::Rls);
        assert!(used, "caller needs this to emit the deprecation warning");

        let (mode, used) = apply_legacy_gate_var(AuthzMode::RlsKeto, false, Some("0"));
        assert_eq!(mode, AuthzMode::Rls);
        assert!(used);
    }

    #[test]
    fn authz_mode_wins_over_the_legacy_var() {
        // Both set: the current variable governs, so a stale legacy value in a
        // Helm chart cannot silently override an explicit choice.
        let (mode, used) = apply_legacy_gate_var(AuthzMode::RlsKeto, true, Some("off"));
        assert_eq!(mode, AuthzMode::RlsKeto);
        assert!(!used);
    }

    #[test]
    fn legacy_var_is_inert_when_absent_or_not_disabling() {
        let (mode, used) = apply_legacy_gate_var(AuthzMode::RlsKeto, false, None);
        assert_eq!(mode, AuthzMode::RlsKeto);
        assert!(!used);

        let (mode, used) = apply_legacy_gate_var(AuthzMode::RlsKeto, false, Some("on"));
        assert_eq!(mode, AuthzMode::RlsKeto);
        assert!(!used);
    }
}
