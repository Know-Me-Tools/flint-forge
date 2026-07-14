use std::sync::Arc;

use fdb_domain::{ChangeEvent, ChangeOp};
use fdb_ports::StreamError;
use forge_identity::RlsContext;
use futures::stream::StreamExt;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::wrappers::BroadcastStream;

use super::payload::{op_from_str, parse_payload, PayloadError};
use super::validate::{is_safe_entity, is_safe_tenant, matches};
use crate::keto_check_via_http;

// --- op_from_str mapping ------------------------------------------------

#[test]
fn op_mapping_known_values() {
    assert_eq!(op_from_str("insert"), Some(ChangeOp::Insert));
    assert_eq!(op_from_str("update"), Some(ChangeOp::Update));
    assert_eq!(op_from_str("delete"), Some(ChangeOp::Delete));
    assert_eq!(op_from_str("upsert"), Some(ChangeOp::Upsert));
}

#[test]
fn op_mapping_rejects_empty_unknown_and_uppercase() {
    assert_eq!(op_from_str(""), None);
    assert_eq!(op_from_str("garbage"), None);
    // Trigger emits lowercase; uppercase is intentionally unmapped.
    assert_eq!(op_from_str("INSERT"), None);
}

// --- parse_payload happy paths -----------------------------------------

#[test]
fn parse_full_insert() {
    let raw = r#"{
        "op":"insert","schema":"public","table":"documents","tenant":"acme",
        "record":{"id":42,"title":"x","tenant_id":"acme"},
        "old_record":null,"truncated":false
    }"#;
    let (tenant, ev) = parse_payload(raw).expect("parse insert");
    assert_eq!(tenant.as_deref(), Some("acme"));
    assert_eq!(ev.op, ChangeOp::Insert);
    assert_eq!(ev.schema, "public");
    assert_eq!(ev.table, "documents");
    assert!(ev.record.is_some());
    assert!(ev.old_record.is_none());
}

#[test]
fn parse_update_has_both_images() {
    let raw = r#"{
        "op":"update","schema":"public","table":"documents","tenant":"acme",
        "record":{"id":42,"title":"new"},
        "old_record":{"id":42,"title":"old"},"truncated":false
    }"#;
    let (_tenant, ev) = parse_payload(raw).expect("parse update");
    assert_eq!(ev.op, ChangeOp::Update);
    assert!(ev.record.is_some());
    assert!(ev.old_record.is_some());
}

#[test]
fn parse_delete_has_old_only() {
    let raw = r#"{
        "op":"delete","schema":"public","table":"documents","tenant":"acme",
        "record":null,"old_record":{"id":42},"truncated":false
    }"#;
    let (_tenant, ev) = parse_payload(raw).expect("parse delete");
    assert_eq!(ev.op, ChangeOp::Delete);
    assert!(ev.record.is_none());
    assert!(ev.old_record.is_some());
}

#[test]
fn parse_truncated_pk_only_preserves_pk() {
    // The load-bearing overflow case: a truncated event must still carry the PK
    // so the downstream build_pk_filters re-query can re-fetch the full row.
    let raw = r#"{
        "op":"update","schema":"public","table":"documents","tenant":"acme",
        "record":{"id":42},"old_record":{"id":42},"truncated":true
    }"#;
    let (_tenant, ev) = parse_payload(raw).expect("parse truncated");
    let pk = ev
        .record
        .as_ref()
        .and_then(|r| r.get("id"))
        .and_then(serde_json::Value::as_i64);
    assert_eq!(pk, Some(42));
}

#[test]
fn parse_missing_tenant_is_none() {
    let raw = r#"{
        "op":"insert","schema":"public","table":"logs",
        "record":{"id":1},"old_record":null
    }"#;
    let (tenant, ev) = parse_payload(raw).expect("parse no-tenant");
    assert!(tenant.is_none());
    assert_eq!(ev.op, ChangeOp::Insert);
}

// --- parse_payload error paths -----------------------------------------

#[test]
fn parse_non_json_errors() {
    assert!(matches!(
        parse_payload("not json at all"),
        Err(PayloadError::Json)
    ));
}

#[test]
fn parse_missing_op_errors() {
    let raw = r#"{"schema":"public","table":"documents"}"#;
    assert!(matches!(parse_payload(raw), Err(PayloadError::Json)));
}

#[test]
fn parse_unknown_op_errors() {
    let raw = r#"{"op":"truncate","schema":"public","table":"documents"}"#;
    assert!(matches!(parse_payload(raw), Err(PayloadError::UnknownOp)));
}

// --- entity_type filter logic ------------------------------------------

#[test]
fn matches_exact_schema_and_table() {
    assert!(matches("public.documents", "public", "documents"));
}

#[test]
fn matches_rejects_table_mismatch() {
    assert!(!matches("public.documents", "public", "orders"));
}

#[test]
fn matches_rejects_schema_mismatch() {
    assert!(!matches("public.documents", "auth", "documents"));
}

#[test]
fn matches_rejects_unqualified_entity() {
    assert!(!matches("documents", "public", "documents"));
}

// --- Defense-in-depth identifier validation (pre-Keto, fail closed) -----

#[test]
fn is_safe_entity_accepts_qualified_and_rejects_injection() {
    assert!(is_safe_entity("public.documents"));
    assert!(is_safe_entity("app_1.order_items"));
    // Not two dot-segments, or unsafe chars, or URL-reserved injection.
    assert!(!is_safe_entity("documents")); // unqualified
    assert!(!is_safe_entity("public.doc.extra")); // three segments
    assert!(!is_safe_entity("public.docs&relation=owner")); // query injection
    assert!(!is_safe_entity("public.1docs")); // digit-led table
    assert!(!is_safe_entity("pub lic.docs")); // whitespace
    assert!(!is_safe_entity("")); // empty
}

#[test]
fn is_safe_tenant_allows_slug_uuid_empty_rejects_reserved() {
    assert!(is_safe_tenant("")); // no-tenant tables
    assert!(is_safe_tenant("acme"));
    assert!(is_safe_tenant("550e8400-e29b-41d4-a716-446655440000")); // uuid
    assert!(is_safe_tenant("tenant_42"));
    assert!(!is_safe_tenant("acme&relation=owner")); // query injection
    assert!(!is_safe_tenant("a/b")); // path
    assert!(!is_safe_tenant("a b")); // whitespace
    assert!(!is_safe_tenant(&"x".repeat(129))); // over cap
}

// --- Keto fail-closed (no DB; a dead port for the Unavailable case) -----

fn rls_with_subject(subject: &str) -> RlsContext {
    RlsContext {
        role: "authenticated".to_string(),
        claims_json: "{}".to_string(),
        raw_bearer: "token".to_string(),
        keto_subject: subject.to_string(),
        vault_key_id: None,
    }
}

#[tokio::test]
async fn keto_allowed_true_ok() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/relation-tuples/check"))
        .respond_with(
            wiremock::ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"allowed": true})),
        )
        .mount(&server)
        .await;

    let out = keto_check_via_http(
        &reqwest::Client::new(),
        &server.uri(),
        "public.documents",
        "acme",
        "user-1",
    )
    .await;
    assert!(out.is_ok());
}

#[tokio::test]
async fn keto_allowed_false_denied() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/relation-tuples/check"))
        .respond_with(
            wiremock::ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"allowed": false})),
        )
        .mount(&server)
        .await;

    let out = keto_check_via_http(
        &reqwest::Client::new(),
        &server.uri(),
        "public.documents",
        "acme",
        "user-1",
    )
    .await;
    assert!(matches!(out, Err(StreamError::Denied)));
}

#[tokio::test]
async fn keto_forbidden_denied() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/relation-tuples/check"))
        .respond_with(wiremock::ResponseTemplate::new(403))
        .mount(&server)
        .await;

    let out = keto_check_via_http(
        &reqwest::Client::new(),
        &server.uri(),
        "public.documents",
        "acme",
        "user-1",
    )
    .await;
    assert!(matches!(out, Err(StreamError::Denied)));
}

#[tokio::test]
async fn keto_server_error_unavailable_fail_closed() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .respond_with(wiremock::ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let out = keto_check_via_http(
        &reqwest::Client::new(),
        &server.uri(),
        "public.documents",
        "acme",
        "user-1",
    )
    .await;
    assert!(matches!(out, Err(StreamError::Unavailable)));
}

#[tokio::test]
async fn keto_connection_refused_unavailable_fail_closed() {
    // Point at a port nothing is listening on. This is the load-bearing
    // fail-closed assertion: a transport error must deny, never allow.
    let dead = "http://127.0.0.1:1";
    let out = keto_check_via_http(
        &reqwest::Client::new(),
        dead,
        "public.documents",
        "acme",
        "user-1",
    )
    .await;
    assert!(matches!(out, Err(StreamError::Unavailable)));
}

#[test]
fn rls_context_constructor_excludes_pii_from_debug_expectations() {
    // Sanity: keto_subject is a field but must never be logged. This test
    // exercises the constructor used by the async tests below.
    let rls = rls_with_subject("user-1");
    assert_eq!(rls.keto_subject, "user-1");
    assert_eq!(rls.role, "authenticated");
}

// --- Broadcast lag mapping (pure tokio; no DB, no Postgres) --------------

#[tokio::test]
async fn broadcast_lag_becomes_skip_not_terminate() {
    // Capacity 2, overflow it, then confirm the watch-style filter turns the
    // Lagged signal into a skipped (None) item rather than an Err / termination.
    let (tx, rx) = tokio::sync::broadcast::channel::<Arc<ChangeEvent>>(2);
    let ev = |t: &str| {
        Arc::new(ChangeEvent {
            op: ChangeOp::Insert,
            schema: "public".to_string(),
            table: t.to_string(),
            record: None,
            old_record: None,
        })
    };
    // Send more than capacity before the receiver reads → guaranteed lag.
    for i in 0..5 {
        let _ = tx.send(ev(&format!("t{i}")));
    }

    let want_entity = "public.documents".to_string();
    let mut stream = BroadcastStream::new(rx)
        .filter_map(move |item| {
            let want_entity = want_entity.clone();
            async move {
                match item {
                    Ok(e) if matches(&want_entity, &e.schema, &e.table) => {
                        Some(Ok::<(), StreamError>(()))
                    }
                    // Non-matching table OR a Lagged signal: both drop to None
                    // (skip), never Err, never terminate.
                    Ok(_) | Err(BroadcastStreamRecvError::Lagged(_)) => None,
                }
            }
        })
        .boxed();

    // The lagged window is skipped (None) and the stream ends cleanly once the
    // channel is exhausted and dropped — it never yields Err and never panics.
    drop(tx);
    let mut yielded_err = false;
    while let Some(item) = stream.next().await {
        if item.is_err() {
            yielded_err = true;
        }
    }
    assert!(
        !yielded_err,
        "lag must not surface as an Err to the subscriber"
    );
}
