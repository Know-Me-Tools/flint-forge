//! `/schema/v1` — the FFS-001 schema-provisioning API (p17-c004).
//!
//! Mounted UNCONDITIONALLY in `bootstrap.rs`; the disabled-state contract
//! lives in the handlers: when `FLINT_PROVISION_NAMESPACES` is empty/unset
//! (`SchemaApiState::provisioner` is `None`) every endpoint returns
//! `503 schema provisioning is not enabled`. FFS-001 task 3.3's literal
//! "feature-gate mounting" wording is deliberately overridden — an unmounted
//! route would 404 where §4.1 requires 503 (plan.md, Unresolved Review
//! Findings #1).

pub mod apply;
pub mod ddl;
pub mod plan;
pub mod status;

use std::sync::Arc;

use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use fdb_ports::SchemaProvisioner;
use fdb_reflection::StateManager;
use forge_identity::RlsContext;
use serde_json::json;

/// The JWT role allowed to call provisioning routes — the same privileged
/// role convention as `ext-flint-meta` and fke-server's `/admin/functions`.
const PROVISIONER_ROLE: &str = "service_role";

/// Extract the `Bearer` token from an `Authorization` header, if present.
/// (Deliberately local: `schema_api` lives on the library target so its
/// router is constructible by integration tests, and must not reach into
/// binary-target modules.)
fn extract_bearer(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::to_owned)
}

/// Plans expire 24h after creation (FFS-001 §4.2) so a stale plan cannot be
/// applied against a drifted schema by default; the hash drift guard remains
/// the authoritative check inside that window.
pub const PLAN_TTL_SECS: i64 = 24 * 60 * 60;

/// State for the `/schema/v1` route group.
#[derive(Clone)]
pub struct SchemaApiState {
    /// `None` when `FLINT_PROVISION_NAMESPACES` is empty/unset — the feature
    /// is off and every handler answers 503.
    pub provisioner: Option<Arc<dyn SchemaProvisioner>>,
    /// The parsed operator allowlist.
    pub namespaces: Arc<Vec<String>>,
    /// For sampling the reflection schema version (`/status`, apply
    /// versionBefore/After).
    pub state_manager: Arc<StateManager>,
}

/// JSON error body helper.
pub(crate) fn error_response(code: StatusCode, message: &str) -> Response {
    (code, Json(json!({ "error": message }))).into_response()
}

/// The 503 disabled-state gate (checked before auth: the response leaks only
/// that the feature is off, which `/status` intentionally reports anyway).
pub(crate) fn require_enabled(
    state: &SchemaApiState,
) -> Result<Arc<dyn SchemaProvisioner>, Box<Response>> {
    state.provisioner.clone().ok_or_else(|| {
        Box::new(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "schema provisioning is not enabled",
        ))
    })
}

/// Verify the bearer and require the `service_role` role — the
/// `require_admin` idiom from fke-server's `/admin/functions`, applied to
/// the provisioning surface (FFS-001 §4.1).
pub(crate) async fn require_provisioner(headers: &HeaderMap) -> Result<RlsContext, Response> {
    let Some(bearer) = extract_bearer(headers) else {
        return Err(error_response(
            StatusCode::UNAUTHORIZED,
            "missing Authorization header",
        ));
    };
    let Ok(caller) = fdb_auth::rls_from_bearer(&bearer).await else {
        return Err(error_response(
            StatusCode::UNAUTHORIZED,
            "invalid or expired token",
        ));
    };
    if caller.role != PROVISIONER_ROLE {
        return Err(error_response(
            StatusCode::FORBIDDEN,
            "provisioner role required",
        ));
    }
    Ok(caller)
}

/// The caller's JWT `sub`, for ledger attribution (never the bearer).
pub(crate) fn subject_of(ctx: &RlsContext) -> String {
    serde_json::from_str::<serde_json::Value>(&ctx.claims_json)
        .ok()
        .and_then(|claims| {
            claims
                .get("sub")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "unknown".to_owned())
}

/// Reserved-then-allowlist namespace gate (FFS-001 D4): the reserved refusal
/// is unconditional and cheap — it runs before any database access, and the
/// deeper spec validation repeats it defense-in-depth inside `generate`.
pub(crate) fn require_allowlisted(
    state: &SchemaApiState,
    ns: &str,
) -> Result<(), Box<Response>> {
    let reserved = ns == "public"
        || ns == "information_schema"
        || ns.starts_with("flint_")
        || ns.starts_with("pg_");
    if reserved {
        return Err(Box::new(error_response(
            StatusCode::FORBIDDEN,
            "namespace is reserved and can never be provisioned",
        )));
    }
    if !state.namespaces.iter().any(|allowed| allowed == ns) {
        return Err(Box::new(error_response(
            StatusCode::FORBIDDEN,
            "namespace is not in the provisioning allowlist",
        )));
    }
    Ok(())
}

/// Current Unix time in seconds.
pub(crate) fn now_epoch() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

/// Render a Unix timestamp as `YYYY-MM-DDTHH:MM:SSZ` without a date-time
/// dependency (Howard Hinnant's `civil_from_days` algorithm).
pub(crate) fn epoch_to_iso8601(epoch: i64) -> String {
    let days = epoch.div_euclid(86_400);
    let secs_of_day = epoch.rem_euclid(86_400);
    let (hh, mm, ss) = (
        secs_of_day / 3600,
        (secs_of_day % 3600) / 60,
        secs_of_day % 60,
    );

    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

#[cfg(test)]
mod iso_tests {
    use super::epoch_to_iso8601;

    #[test]
    fn renders_known_timestamps() {
        assert_eq!(epoch_to_iso8601(0), "1970-01-01T00:00:00Z");
        // 2026-07-23T10:00:00Z (= 1_782_864_000 for Jul 1 + 22 days + 10h)
        assert_eq!(epoch_to_iso8601(1_784_800_800), "2026-07-23T10:00:00Z");
        // Leap-day 2024-02-29T12:00:00Z
        assert_eq!(epoch_to_iso8601(1_709_208_000), "2024-02-29T12:00:00Z");
    }
}
