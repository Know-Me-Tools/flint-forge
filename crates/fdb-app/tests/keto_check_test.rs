//! Unit tests for `Quarry` Keto-check injection (p3-c011).
//!
//! These tests verify the hexagonal composition pattern:
//! - `Quarry::with_keto()` stores the adapter.
//! - `check_keto()` delegates to the adapter.
//! - Denial returns `ForbiddenError`.
//! - `Quarry` without keto configured always allows.

#![forbid(unsafe_code)]

use std::sync::Arc;

use async_trait::async_trait;
use fdb_app::Quarry;
use fdb_ports::KetoCheck;

/// Mock that allows or denies based on a configurable flag.
struct MockKetoCheck {
    allow: bool,
}

#[async_trait]
impl KetoCheck for MockKetoCheck {
    async fn check(
        &self,
        _namespace: &str,
        _object: &str,
        _relation: &str,
        _subject: &str,
    ) -> bool {
        self.allow
    }
}

// ── Stubs ───────────────────────────────────────────────────────────────────
// Quarry::new requires three ports. These are zero-effort stubs that are never
// called by check_keto — they exist only to satisfy the constructor.

struct StubRest;
#[async_trait::async_trait]
impl fdb_ports::RestExecutor for StubRest {
    async fn execute(
        &self,
        _: fdb_domain::RestQuery,
        _: &forge_identity::RlsContext,
    ) -> Result<fdb_domain::RestResult, fdb_ports::BackendError> {
        unreachable!("stub")
    }
}

struct StubGraphQl;
#[async_trait::async_trait]
impl fdb_ports::GraphQlExecutor for StubGraphQl {
    async fn execute(
        &self,
        _: fdb_domain::GraphQlRequest,
        _: &forge_identity::RlsContext,
    ) -> Result<forge_domain::Json, fdb_ports::BackendError> {
        unreachable!("stub")
    }
}

struct StubChanges;
#[async_trait::async_trait]
impl fdb_ports::ChangeStreamSource for StubChanges {
    async fn watch(
        &self,
        _: fdb_domain::SubscriptionSpec,
        _: &forge_identity::RlsContext,
    ) -> Result<
        futures::stream::BoxStream<
            'static,
            Result<fdb_domain::ChangeEvent, fdb_ports::StreamError>,
        >,
        fdb_ports::StreamError,
    > {
        unreachable!("stub")
    }
}

fn make_quarry() -> Quarry {
    Quarry::new(
        Arc::new(StubRest),
        Arc::new(StubGraphQl),
        Arc::new(StubChanges),
    )
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn quarry_without_keto_always_allows() {
    let q = make_quarry();
    assert!(q.check_keto("ns", "obj", "rel", "subj").await.is_ok());
}

#[tokio::test]
async fn quarry_with_keto_allow_passes() {
    let q = make_quarry().with_keto(Arc::new(MockKetoCheck { allow: true }));
    assert!(q.check_keto("ns", "obj", "rel", "subj").await.is_ok());
}

#[tokio::test]
async fn quarry_with_keto_deny_returns_forbidden() {
    let q = make_quarry().with_keto(Arc::new(MockKetoCheck { allow: false }));
    assert!(q.check_keto("ns", "obj", "rel", "subj").await.is_err());
}

#[tokio::test]
async fn keto_check_error_never_logs_subject() {
    // Static-assertion test: ForbiddenError carries no subject/relation fields
    // by design. We obtain the error from a denial and verify its Display impl
    // contains no PII. If someone adds a field, this test catches the leak.
    let q = make_quarry().with_keto(Arc::new(MockKetoCheck { allow: false }));
    let err = q
        .check_keto("entities", "orders", "view", "user-sensitive-pii")
        .await
        .expect_err("should deny");
    let msg = format!("{err}");
    assert!(
        !msg.contains("user-sensitive-pii") && !msg.contains("subj"),
        "ForbiddenError must not contain subject identifiers: {msg}"
    );
}
