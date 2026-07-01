//! Flint Quarry application layer — use-cases composed against ports.
#![forbid(unsafe_code)]

pub mod a2ui;
pub mod graphql;

use fdb_ports::{ChangeStreamSource, GraphQlExecutor, KetoCheck, RestExecutor};
use forge_policy::{Decision, Pep, Request as PolicyRequest};
use std::sync::Arc;

/// Wires the use-cases over whatever adapters the interface layer injects.
///
/// `keto` is `Option<Arc<dyn KetoCheck>>` so that the Quarry can operate
/// without a Keto gate during early boot or test scaffolding. When `Some`,
/// mutation use-cases call `KetoCheck::check()` before delegating to the
/// executor and return a typed 403 on denial.
///
/// `pep` is `Option<Arc<dyn Pep>>` — the Cedar policy enforcement point.
/// When `Some`, mutation use-cases call `Pep::check()` after the Keto gate.
pub struct Quarry {
    pub rest: Arc<dyn RestExecutor>,
    pub graphql: Arc<dyn GraphQlExecutor>,
    pub changes: Arc<dyn ChangeStreamSource>,
    pub keto: Option<Arc<dyn KetoCheck>>,
    pub pep: Option<Arc<dyn Pep>>,
}

/// Typed mutation-denial error surfaced when `KetoCheck::check()` returns `false`.
#[derive(Debug, thiserror::Error)]
#[error("forbidden: Keto relation check denied")]
pub struct ForbiddenError;

impl Quarry {
    pub fn new(
        rest: Arc<dyn RestExecutor>,
        graphql: Arc<dyn GraphQlExecutor>,
        changes: Arc<dyn ChangeStreamSource>,
    ) -> Self {
        Self {
            rest,
            graphql,
            changes,
            keto: None,
            pep: None,
        }
    }

    /// Attach a Keto check adapter. Called once at gateway composition time.
    #[must_use]
    pub fn with_keto(mut self, keto: Arc<dyn KetoCheck>) -> Self {
        self.keto = Some(keto);
        self
    }

    /// Attach a Cedar policy enforcement point. Called once at gateway
    /// composition time.
    #[must_use]
    pub fn with_pep(mut self, pep: Arc<dyn Pep>) -> Self {
        self.pep = Some(pep);
        self
    }

    /// Mutation-time Keto gate. Returns `Ok(())` when the check passes (or
    /// when no Keto adapter is configured), `Err(ForbiddenError)` when denied.
    ///
    /// SECURITY: `subject` is PII and MUST NOT be logged. The error variant
    /// carries no PII by design.
    pub async fn check_keto(
        &self,
        namespace: &str,
        object: &str,
        relation: &str,
        subject: &str,
    ) -> Result<(), ForbiddenError> {
        if let Some(keto) = &self.keto {
            if !keto.check(namespace, object, relation, subject).await {
                return Err(ForbiddenError);
            }
        }
        Ok(())
    }

    /// Capability-time Cedar policy gate. Returns `Ok(())` when the policy
    /// allows (or when no PEP is configured), `Err(ForbiddenError)` when denied.
    ///
    /// SECURITY: `who` contains PII. The error variant carries no PII by design.
    pub async fn check_pep(
        &self,
        who: &forge_identity::RlsContext,
        action: &str,
        resource: &str,
    ) -> Result<(), ForbiddenError> {
        if let Some(pep) = &self.pep {
            let req = PolicyRequest {
                action: action.into(),
                resource: resource.into(),
                context: forge_domain::Json::Null,
            };
            if pep.check(who, &req).await == Decision::Deny {
                return Err(ForbiddenError);
            }
        }
        Ok(())
    }
}
