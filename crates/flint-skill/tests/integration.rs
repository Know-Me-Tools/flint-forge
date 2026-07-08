//! Integration tests for `flint-skill`.
#![allow(refining_impl_trait)]
//!
//! These tests exercise the public API surface against in-process mock
//! implementations of the trait abstractions. They do **not** invoke any
//! WIT bindings — the SDK crate intentionally contains no host calls —
//! so the tests run on the host target and prove that skill code using
//! these traits and types compiles and behaves correctly end-to-end.

use flint_skill::{
    CompletionResult, Database, DbRow, EmbeddingResult, HostInterface, Identity, Kv, Llm,
    LlmOptions, SecretHandle, Secrets, SkillError,
};
use serde_json::Value;

// ---------------------------------------------------------------------------
// Mock implementations — these mirror the shape of a real skill-author
// adapter over a `bindings::flint::host::*` module, but return canned data
// instead of crossing the WIT boundary.
// ---------------------------------------------------------------------------

struct MockDb {
    rows: Vec<String>,
}

impl Database for MockDb {
    async fn query<'a>(
        &'a self,
        _sql: &'a str,
        _params: &'a [String],
    ) -> flint_skill::SkillResult<Vec<DbRow>> {
        self.rows
            .iter()
            .map(|s| DbRow::from_json_str(s))
            .collect()
    }
}

struct MockLlm;
impl Llm for MockLlm {
    async fn complete<'a>(
        &'a self,
        _prompt: &'a str,
        opts: &'a LlmOptions,
    ) -> flint_skill::SkillResult<CompletionResult> {
        let _ = opts.to_json()?; // exercises serialization path
        Ok(CompletionResult {
            text: String::from("hello"),
        })
    }

    async fn embed<'a>(
        &'a self,
        _input: &'a str,
        _model: Option<&'a str>,
    ) -> flint_skill::SkillResult<EmbeddingResult> {
        Ok(EmbeddingResult {
            vector: vec![0.1, 0.2, 0.3],
        })
    }
}

struct MockKv {
    store: std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>,
}
impl Kv for MockKv {
    fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.store.lock().unwrap().get(key).cloned()
    }
    fn set(&self, key: &str, value: &[u8]) {
        self.store
            .lock()
            .unwrap()
            .insert(key.to_string(), value.to_vec());
    }
}

struct MockIdentity;
impl Identity for MockIdentity {
    fn origin_jwt(&self) -> Option<String> {
        None // publisher did not grant origin-jwt visibility
    }
    fn claims_json(&self) -> String {
        r#"{"iss":"flint:kiln","sub":"did:web:example.com","aud":"kiln"}"#.to_string()
    }
}

#[derive(Debug)]
struct MockSecret;
impl SecretHandle for MockSecret {
    async fn reveal(&self) -> flint_skill::SkillResult<String> {
        Ok(String::from("super-secret-value"))
    }
}

struct MockSecrets;
impl Secrets for MockSecrets {
    type Handle = MockSecret;
    async fn get(&self, name: &str) -> flint_skill::SkillResult<Self::Handle> {
        if name == "missing" {
            Err(SkillError::from_host_error(
                HostInterface::Secrets,
                "NOT_FOUND".into(),
                format!("secret '{name}' not registered in vault"),
            ))
        } else {
            Ok(MockSecret)
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn database_query_decodes_rows() {
    let db = MockDb {
        rows: vec![
            r#"{"id":1,"name":"ada"}"#.to_string(),
            r#"{"id":2,"name":"grace"}"#.to_string(),
        ],
    };
    let rows = db.query("SELECT * FROM users", &[]).await.unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].get("name").and_then(|v| v.as_str()), Some("ada"));
    assert_eq!(rows[1].get("id").and_then(serde_json::Value::as_i64), Some(2));
}

#[tokio::test]
async fn llm_complete_round_trips_options() {
    let llm = MockLlm;
    let opts = LlmOptions::new()
        .with_model("gpt-4o-mini")
        .with_temperature(0.0)
        .with_max_tokens(64);
    let result = llm.complete("hi", &opts).await.unwrap();
    assert_eq!(result.text, "hello");
}

#[tokio::test]
async fn llm_embed_returns_vector() {
    let llm = MockLlm;
    let result = llm.embed("hello", None).await.unwrap();
    assert_eq!(result.vector.len(), 3);
}

#[test]
fn kv_round_trips_bytes() {
    let kv = MockKv {
        store: std::sync::Mutex::new(std::collections::HashMap::new()),
    };
    assert!(kv.get("absent").is_none());
    kv.set("present", b"value");
    assert_eq!(kv.get("present").as_deref(), Some(b"value".as_slice()));
}

#[test]
fn identity_claims_decode() {
    let id = MockIdentity;
    assert!(id.origin_jwt().is_none());
    let claims: Value = id.claims().unwrap();
    assert_eq!(claims["iss"], "flint:kiln");
    assert_eq!(claims["sub"], "did:web:example.com");
}

#[tokio::test]
async fn secrets_get_returns_handle_when_found() {
    let s = MockSecrets;
    let handle = s.get("api-key").await.unwrap();
    assert_eq!(handle.reveal().await.unwrap(), "super-secret-value");
}

#[tokio::test]
async fn secrets_get_surfaces_not_found() {
    let s = MockSecrets;
    let err = s.get("missing").await.unwrap_err();
    assert!(matches!(err, SkillError::Secrets { .. }));
    assert_eq!(err.code(), Some("NOT_FOUND"));
}

#[tokio::test]
async fn cedar_deny_round_trips_through_skill_error() {
    // Mirrors how a real adapter would surface a host Cedar denial.
    let err = SkillError::from_host_error(
        HostInterface::Secrets,
        "CEDAR_DENY".into(),
        "publisher has no reveal grant".into(),
    );
    assert_eq!(err.code(), Some("CEDAR_DENY"));
    assert_eq!(err.interface(), HostInterface::Secrets);
    let rendered = format!("{err}");
    assert!(rendered.contains("CEDAR_DENY"));
}
