//! Cedar policy engine — concrete implementation of [`Pep`] over `cedar-policy`.
//!
//! # Architecture
//!
//! `forge-policy` is a library crate at Layer 0/1. It defines the [`PolicySource`]
//! trait so the DB adapter (in `fdb-gateway`) provides the policy text without
//! pulling `sqlx` into this crate.
//!
//! # Fail-closed
//!
//! Policy load failure, policy parse failure, or evaluation error →
//! [`Decision::Deny`]. Never [`Decision::Allow`] on an error path.
//!
//! # Security
//!
//! Policy bodies and principal identifiers MUST NOT appear in tracing spans.
//! The `#[instrument]` on `check` skips `self` and `who`.

#![forbid(unsafe_code)]

use std::sync::Arc;

use async_trait::async_trait;
use cedar_policy::{
    Authorizer, Context, Decision as CedarDecision, Entities, EntityId, EntityTypeName, EntityUid,
    Policy, PolicyId, PolicySet, Request as CedarRequest,
};
use forge_identity::RlsContext;
use tracing::instrument;

use crate::{Decision, Pep, Request};

/// A single policy row loaded from `flint_meta.cedar_policies`.
#[derive(Debug, Clone)]
pub struct PolicyEntry {
    /// Stable unique ID used as the Cedar `PolicyId`.
    pub id: String,
    /// Cedar policy source text.
    pub text: String,
    /// Disabled policies are skipped during compilation.
    pub enabled: bool,
}

/// Error returned when loading or compiling policies. Never carries policy
/// body text or principal identifiers (security: no PII in errors).
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum PolicyLoadError {
    #[error("policy source unavailable")]
    SourceUnavailable,
    #[error("policy parse failed for id {id}")]
    ParseFailed { id: String },
    #[error("policy set assembly failed")]
    AssemblyFailed,
}

/// Source of Cedar policy bundles. Implemented by the adapter layer
/// (e.g. `DbPolicySource` reading `flint_meta.cedar_policies` via the
/// privileged pool). Keeps `forge-policy` free of `sqlx`.
#[async_trait]
pub trait PolicySource: Send + Sync {
    /// Load all enabled policy entries. Returning an error causes the engine
    /// to fall back to the last-known-good `PolicySet` (or an empty set on
    /// first load — which denies everything).
    async fn load(&self) -> Result<Vec<PolicyEntry>, PolicyLoadError>;
}

/// Cedar policy engine implementing [`Pep`].
///
/// Policies are stored in an [`ArcSwap`] for lock-free hot-reload.
/// On `reload()`, the entire `PolicySet` is replaced atomically; in-flight
/// evaluations hold an `Arc` guard to the previous set.
pub struct CedarPolicyEngine {
    source: Arc<dyn PolicySource>,
    policies: arc_swap::ArcSwap<PolicySet>,
}

impl CedarPolicyEngine {
    /// Create the engine and perform an initial policy load.
    ///
    /// If the initial load fails, the engine starts with an empty `PolicySet`
    /// (denies everything). Call `reload()` once the source becomes available.
    pub async fn new(source: Arc<dyn PolicySource>) -> Self {
        let engine = Self {
            source,
            policies: arc_swap::ArcSwap::new(Arc::new(PolicySet::new())),
        };
        // Best-effort initial load; a failure here is fail-closed-safe (the
        // engine already starts deny-all), but it's an operational blind spot
        // if it goes unnoticed — a down/misconfigured policy source would
        // otherwise leave the gateway silently denying every request with no
        // trace of why. `PolicyLoadError` never carries policy text or
        // principal identifiers, so it's safe to log.
        if let Err(e) = engine.reload().await {
            tracing::warn!(error = %e, "initial Cedar policy load failed; starting deny-all");
        }
        engine
    }
    /// Create an engine with a static policy set (for tests).
    pub fn from_policies(policies: PolicySet) -> Self {
        Self {
            // A no-op source that always returns empty — the test set is already loaded.
            source: Arc::new(NoopSource),
            policies: arc_swap::ArcSwap::new(Arc::new(policies)),
        }
    }

    /// Reload policies from the source. On success, atomically swaps the
    /// active `PolicySet`. On failure, the previous set is retained.
    ///
    /// SECURITY: policy text and principal identifiers MUST NOT appear in spans.
    #[instrument(skip(self), fields(action = "reload"))]
    pub async fn reload(&self) -> Result<(), PolicyLoadError> {
        let entries = self.source.load().await?;

        let mut set = PolicySet::new();
        for entry in entries.into_iter().filter(|e| e.enabled) {
            let policy = Policy::parse(Some(PolicyId::new(entry.id.clone())), &entry.text)
                .map_err(|_| PolicyLoadError::ParseFailed {
                    id: entry.id.clone(),
                })?;
            set.add(policy)
                .map_err(|_| PolicyLoadError::AssemblyFailed)?;
        }

        let count = set.policies().count();
        self.policies.store(Arc::new(set));
        tracing::debug!(count, "cedar policy set reloaded");
        Ok(())
    }

    /// Evaluate a Cedar request against the current policy set.
    ///
    /// Constructs `EntityUid`s for principal (from RlsContext subject),
    /// action, and resource, then runs the authorizer.
    /// Any construction or evaluation error → `Decision::Deny` (fail-closed).
    fn evaluate(&self, who: &RlsContext, req: &Request) -> Decision {
        let Some(principal) = who.subject().map(|sid| sid.0) else {
            return Decision::Deny; // no subject → deny
        };

        let Some(principal_uid) = build_entity_uid("User", &principal) else {
            return Decision::Deny;
        };
        let Some(action_uid) = build_entity_uid("Action", &req.action) else {
            return Decision::Deny;
        };
        let Some(resource_uid) = build_entity_uid("Resource", &req.resource) else {
            return Decision::Deny;
        };

        let Ok(cedar_req) = CedarRequest::new(
            principal_uid,
            action_uid,
            resource_uid,
            Context::empty(),
            None, // schemaless mode
        ) else {
            return Decision::Deny;
        };

        let authorizer = Authorizer::new();
        let policies = self.policies.load();
        let response = authorizer.is_authorized(&cedar_req, &policies, &Entities::empty());

        match response.decision() {
            CedarDecision::Allow => Decision::Allow,
            CedarDecision::Deny => Decision::Deny,
        }
    }
}

/// Build a Cedar `EntityUid` from a type name and id string.
/// Returns `None` if either component fails to parse (fail-closed caller).
fn build_entity_uid(type_name: &str, id: &str) -> Option<EntityUid> {
    let tn = type_name.parse::<EntityTypeName>().ok()?;
    let eid = EntityId::new(id);
    Some(EntityUid::from_type_name_and_id(tn, eid))
}

/// No-op policy source for test construction.
struct NoopSource;

#[async_trait]
impl PolicySource for NoopSource {
    async fn load(&self) -> Result<Vec<PolicyEntry>, PolicyLoadError> {
        Ok(vec![])
    }
}

#[async_trait]
impl Pep for CedarPolicyEngine {
    /// Evaluate the request. Fail-closed: any error → [`Decision::Deny`].
    ///
    /// SECURITY: `who` contains PII (keto_subject, raw_bearer). The
    /// `#[instrument(skip(self, who))]` ensures neither appears in spans.
    #[instrument(skip(self, who))]
    async fn check(&self, who: &RlsContext, req: &Request) -> Decision {
        self.evaluate(who, req)
    }
}

// ─── unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use forge_identity::RlsContext;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn rls_for(subject: &str) -> RlsContext {
        RlsContext {
            role: "authenticated".into(),
            claims_json: format!(r#"{{"sub":"{subject}"}}"#),
            raw_bearer: String::new(),
            keto_subject: subject.into(),
            vault_key_id: None,
        }
    }

    fn req(action: &str, resource: &str) -> Request {
        Request {
            action: action.into(),
            resource: resource.into(),
            context: forge_domain::Json::Null,
        }
    }

    /// An allow-all policy for tests.
    const ALLOW_ALL: &str = r#"permit(principal, action == Action::"view", resource);"#;

    /// A deny-all policy for tests.
    const DENY_ALL: &str = "forbid(principal, action, resource);";

    #[tokio::test]
    async fn allow_when_policy_permits() {
        let mut set = PolicySet::new();
        set.add(
            Policy::parse(Some(PolicyId::new("allow-view")), ALLOW_ALL).expect("parse allow-all"),
        )
        .expect("add");
        let engine = CedarPolicyEngine::from_policies(set);

        let decision = engine.check(&rls_for("alice"), &req("view", "doc:1")).await;
        assert_eq!(decision, Decision::Allow);
    }

    #[tokio::test]
    async fn deny_when_no_matching_policy() {
        let engine = CedarPolicyEngine::from_policies(PolicySet::new());
        let decision = engine.check(&rls_for("alice"), &req("view", "doc:1")).await;
        assert_eq!(decision, Decision::Deny);
    }

    #[tokio::test]
    async fn deny_when_policy_forbids() {
        let mut set = PolicySet::new();
        set.add(Policy::parse(Some(PolicyId::new("deny-all")), DENY_ALL).expect("parse deny-all"))
            .expect("add");
        let engine = CedarPolicyEngine::from_policies(set);

        let decision = engine.check(&rls_for("alice"), &req("view", "doc:1")).await;
        assert_eq!(decision, Decision::Deny);
    }

    #[tokio::test]
    async fn deny_when_principal_missing() {
        let engine = CedarPolicyEngine::from_policies(PolicySet::new());
        let who = RlsContext {
            role: "authenticated".into(),
            claims_json: r#"{"role":"anon"}"#.into(), // no "sub"
            raw_bearer: String::new(),
            keto_subject: String::new(),
            vault_key_id: None,
        };
        let decision = engine.check(&who, &req("view", "doc:1")).await;
        assert_eq!(decision, Decision::Deny);
    }

    #[tokio::test]
    async fn malformed_policy_in_source_returns_deny() {
        struct BadSource;
        #[async_trait]
        impl PolicySource for BadSource {
            async fn load(&self) -> Result<Vec<PolicyEntry>, PolicyLoadError> {
                Ok(vec![PolicyEntry {
                    id: "bad".into(),
                    text: "this is not valid cedar".into(),
                    enabled: true,
                }])
            }
        }
        // On reload, the bad policy causes ParseFailed → engine retains
        // empty set → everything denies.
        let engine = CedarPolicyEngine::new(Arc::new(BadSource)).await;
        let decision = engine.check(&rls_for("alice"), &req("view", "doc:1")).await;
        assert_eq!(decision, Decision::Deny);
    }

    #[tokio::test]
    async fn reload_swaps_policy_set() {
        struct CountingSource {
            calls: AtomicUsize,
        }
        #[async_trait]
        impl PolicySource for CountingSource {
            async fn load(&self) -> Result<Vec<PolicyEntry>, PolicyLoadError> {
                let n = self.calls.fetch_add(1, Ordering::SeqCst);
                if n == 0 {
                    Ok(vec![]) // first load: empty → deny all
                } else {
                    Ok(vec![PolicyEntry {
                        id: "allow-view".into(),
                        text: ALLOW_ALL.into(),
                        enabled: true,
                    }])
                }
            }
        }
        let source: Arc<dyn PolicySource> = Arc::new(CountingSource {
            calls: AtomicUsize::new(0),
        });
        let engine = CedarPolicyEngine::new(Arc::clone(&source)).await;

        // First load was empty → deny.
        let d1 = engine.check(&rls_for("alice"), &req("view", "doc:1")).await;
        assert_eq!(d1, Decision::Deny);

        // Reload picks up the allow policy.
        engine.reload().await.expect("reload");

        let d2 = engine.check(&rls_for("alice"), &req("view", "doc:1")).await;
        assert_eq!(d2, Decision::Allow);
    }
}
