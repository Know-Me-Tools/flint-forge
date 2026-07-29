//! Classify each reflected table's RLS posture and report it at startup.
//!
//! # Why this is observability, not a gate
//!
//! A table without policies is a normal mid-development state: you create it,
//! exercise it through the API, and learn what the policies should be from what
//! you find. A gate that refuses to mount such a table breaks that loop, gets
//! switched off, and then protects nothing. Supabase reaches the same
//! conclusion — labels, alerts, and a lint, but it never refuses to serve.
//!
//! So this pass only reports. The corollary is that the report must be
//! *accurate*: a warning that fires on correctly-configured tables is noise,
//! and noise gets filtered out, which is the same failure as a disabled gate.
//! That is why [`RlsPosture`] distinguishes four conditions instead of keying
//! on `rls_enabled` alone.
//!
//! # The two legitimate ways to have no policy
//!
//! 1. **Genuinely public data** (countries, feature flags, published posts):
//!    enable RLS and write `USING (true)`. The permissive policy records the
//!    decision; RLS-off is indistinguishable from having forgotten.
//! 2. **Table that should not be on the API** (job queues, outboxes): `REVOKE`
//!    the API roles. It never reaches the Data API, so RLS is moot — this repo
//!    does exactly that for `flint_meta`.
//!
//! Both have a correct expression, so a table that *is* API-reachable with no
//! policy is always worth surfacing.

use crate::model::{DatabaseModel, Table};

/// A table's row-security posture, as reported at startup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RlsPosture {
    /// RLS enabled with at least one policy, and forced. Nothing to report.
    Protected,
    /// Not reachable through the Data API (`authenticated`/`anon` hold no
    /// privilege), so row security is moot. The `REVOKE` pattern — correct, and
    /// deliberately silent.
    NotExposed,
    /// **Exposed with no row security at all.** API-granted and RLS off: every
    /// authenticated principal can read and write every row. The real Broken
    /// Access Control case.
    Unprotected,
    /// RLS enabled but **zero policies**. Denies everything for non-owners, so
    /// it fails closed rather than leaking — but it is almost always either a
    /// mid-development state or an oversight, and it is the shape that presents
    /// as "every write 403s while reads succeed".
    NoPolicies,
    /// RLS enabled with policies, but **not `FORCE`d**. The table owner bypasses
    /// every policy. Harmless when the API roles are not the owner, misleading
    /// when they are — and `rls_enabled` alone would have called this protected.
    NotForced,
}

impl RlsPosture {
    /// Whether this posture is worth telling the operator about.
    #[must_use]
    pub fn is_reportable(self) -> bool {
        !matches!(self, RlsPosture::Protected | RlsPosture::NotExposed)
    }
}

/// Classify one table.
///
/// Pure over [`Table`]'s reflected flags so the decision is unit-testable
/// without a database — the same shape as `authz_mode::resolve_authz_mode`.
///
/// Order matters: reachability is checked first, because a table the API cannot
/// touch has no exposure regardless of its RLS flags.
#[must_use]
pub fn classify(table: &Table) -> RlsPosture {
    if !table.api_granted {
        return RlsPosture::NotExposed;
    }
    if !table.rls_enabled {
        return RlsPosture::Unprotected;
    }
    if table.policy_count == 0 {
        return RlsPosture::NoPolicies;
    }
    if !table.rls_forced {
        return RlsPosture::NotForced;
    }
    RlsPosture::Protected
}

/// The operator-facing message for a posture, naming both legitimate fixes.
///
/// A warning that only states a problem gets ignored; one that names the
/// resolutions gets acted on.
fn advice(posture: RlsPosture) -> &'static str {
    match posture {
        RlsPosture::Unprotected => {
            "exposed to the API with row security OFF — every authenticated \
             principal can read and write every row. Add a policy (use \
             `USING (true)` if the data really is public, so the decision is \
             recorded), or REVOKE the API roles if it should not be on the API"
        }
        RlsPosture::NoPolicies => {
            "row security is ON but the table has NO policies — every row is \
             denied to non-owners, so writes will 403 while reads appear to \
             work. Add a policy, or REVOKE the API roles if it should not be \
             on the API"
        }
        RlsPosture::NotForced => {
            "row security is ON but not FORCEd — the table owner bypasses every \
             policy. Run `ALTER TABLE … FORCE ROW LEVEL SECURITY` if the API \
             roles may own this table"
        }
        RlsPosture::Protected | RlsPosture::NotExposed => "",
    }
}

/// How many tables have a reportable posture, without logging anything.
///
/// Separate from [`run`] so a metrics publisher can re-read the count on every
/// schema reload without re-emitting the warnings — a gauge that logged on each
/// refresh would turn one honest startup warning into a repeating stream, which
/// is how warnings get filtered out and stop being read.
#[must_use]
pub fn count_reportable(model: &DatabaseModel) -> usize {
    model
        .tables
        .iter()
        .filter(|t| classify(t).is_reportable())
        .count()
}

/// Report every reportable table in one aggregated warning per posture.
///
/// One line per posture rather than one per table: a per-table loop over a
/// large schema scrolls the actionable lines off the screen.
///
/// Returns the count of reportable tables. The composition root publishes it as
/// a gauge so an exposed table is alertable rather than only greppable; this
/// crate deliberately takes no metrics dependency, keeping the pass a pure
/// function over the model.
pub fn run(model: &DatabaseModel) -> usize {
    let mut reportable = 0_usize;

    for posture in [
        RlsPosture::Unprotected,
        RlsPosture::NoPolicies,
        RlsPosture::NotForced,
    ] {
        let hits: Vec<String> = model
            .tables
            .iter()
            .filter(|t| classify(t) == posture)
            .map(|t| format!("{}.{}", t.schema, t.name))
            .collect();

        if hits.is_empty() {
            continue;
        }
        reportable += hits.len();
        tracing::warn!(
            count = hits.len(),
            tables = %hits.join(", "),
            "{}",
            advice(posture)
        );
    }

    reportable
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Table;

    fn table(rls_enabled: bool, rls_forced: bool, api_granted: bool, policy_count: i32) -> Table {
        Table {
            schema: "public".into(),
            name: "t".into(),
            columns: vec![],
            pk: vec![],
            fk: vec![],
            rls_enabled,
            rls_forced,
            api_granted,
            policy_count,
            vault_key: None,
        }
    }

    #[test]
    fn ungranted_table_is_never_reported() {
        // The REVOKE pattern: not on the API, so row security is moot. Reporting
        // it would punish the operator for doing the right thing — and a warning
        // that fires on correct configuration is one nobody reads.
        assert_eq!(
            classify(&table(false, false, false, 0)),
            RlsPosture::NotExposed
        );
        assert_eq!(
            classify(&table(true, true, false, 3)),
            RlsPosture::NotExposed
        );
        assert!(!RlsPosture::NotExposed.is_reportable());
    }

    #[test]
    fn granted_without_rls_is_the_real_exposure() {
        assert_eq!(
            classify(&table(false, false, true, 0)),
            RlsPosture::Unprotected
        );
        assert!(RlsPosture::Unprotected.is_reportable());
    }

    #[test]
    fn rls_on_with_zero_policies_is_distinct_from_unprotected() {
        // Fails CLOSED, unlike `Unprotected` — different urgency, so it must not
        // collapse into the same message. This is San Saba's 403-everything shape.
        assert_eq!(
            classify(&table(true, true, true, 0)),
            RlsPosture::NoPolicies
        );
    }

    #[test]
    fn rls_on_but_not_forced_is_caught() {
        // `rls_enabled` alone would have called this protected.
        assert_eq!(
            classify(&table(true, false, true, 2)),
            RlsPosture::NotForced
        );
    }

    #[test]
    fn fully_protected_table_is_silent() {
        assert_eq!(classify(&table(true, true, true, 1)), RlsPosture::Protected);
        assert!(!RlsPosture::Protected.is_reportable());
    }

    #[test]
    fn run_counts_only_reportable_tables() {
        let model = DatabaseModel {
            tables: vec![
                table(true, true, true, 1),   // protected
                table(true, true, false, 0),  // not exposed
                table(false, false, true, 0), // unprotected
                table(true, true, true, 0),   // no policies
            ],
            functions: vec![],
            views: vec![],
            version: 1,
        };
        assert_eq!(run(&model), 2, "protected and not-exposed must not count");
    }

    #[test]
    fn count_reportable_agrees_with_run() {
        // The gauge reads `count_reportable` while the log reads `run`. If they
        // ever disagree, the metric and the warning tell different stories.
        let model = DatabaseModel {
            tables: vec![
                table(true, true, true, 1),   // protected
                table(true, true, false, 0),  // not exposed
                table(false, false, true, 0), // unprotected
                table(true, true, true, 0),   // no policies
                table(true, false, true, 2),  // not forced
            ],
            functions: vec![],
            views: vec![],
            version: 1,
        };
        assert_eq!(count_reportable(&model), run(&model));
        assert_eq!(count_reportable(&model), 3);
    }

    #[test]
    fn every_reportable_posture_has_advice() {
        for posture in [
            RlsPosture::Unprotected,
            RlsPosture::NoPolicies,
            RlsPosture::NotForced,
        ] {
            assert!(
                !advice(posture).is_empty(),
                "reportable posture {posture:?} must name a fix"
            );
        }
    }
}
