//! Authored, intentionally ignored acceptance against an already-provisioned disposable DB.
//! No DDL, server startup, credentials discovery or default DATABASE_URL fallback.
use axum::{
    body::Body,
    http::{Request, StatusCode},
    Extension,
};
use fdb_domain::files::{
    FileId, FileQuery, FileType, FileTypeId, MatchCriteria, MatchInput, PutFile,
};
use fdb_gateway::files::{router, FileApiState};
use fdb_ports::files::{FileError, FileMetadataStore};
use fdb_postgres::PgFileMetadata;
use forge_identity::RlsContext;
use forge_policy::{Decision, Pep};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::{error::Error, str::FromStr, sync::Arc};
use tower::ServiceExt;

struct Permit;
#[async_trait::async_trait]
impl Pep for Permit {
    async fn check(&self, _who: &RlsContext, request: &forge_policy::Request) -> Decision {
        // Synthetic test-only policy, scoped to the exact production resources.
        match (request.resource.as_str(), request.action.as_str()) {
            ("flint_files.types", "insert") | ("flint_files.records", "insert" | "update") => {
                Decision::Allow
            }
            _ => Decision::Deny,
        }
    }
}
struct Deny;
#[async_trait::async_trait]
impl Pep for Deny {
    async fn check(&self, _who: &RlsContext, _request: &forge_policy::Request) -> Decision {
        Decision::Deny
    }
}
fn identity(tenant: &str, subject: &str) -> RlsContext {
    RlsContext {
        role: "authenticated".into(),
        claims_json: json!({"sub":subject,"tenant_id":tenant,"role":"authenticated"}).to_string(),
        raw_bearer: String::new(),
        keto_subject: subject.into(),
        vault_key_id: None,
    }
}
fn pool(url: &str) -> Result<deadpool_postgres::Pool, Box<dyn Error>> {
    let mut config = deadpool_postgres::Config::new();
    config.url = Some(url.into());
    Ok(config.create_pool(
        Some(deadpool_postgres::Runtime::Tokio1),
        tokio_postgres::NoTls,
    )?)
}
fn definition() -> FileType {
    FileType {
        type_id: FileTypeId("invoice".into()),
        version: 1,
        schema_profile: "file-schema-v1".into(),
        schema: json!({"type":"object","properties":{"amount":{"type":"integer"},"currency":{"type":"string","enum":["USD","EUR"]}},"required":["amount","currency"],"additionalProperties":false}),
        criteria: MatchCriteria {
            mime_types: vec!["application/pdf".into()],
            filename_suffixes: vec![".pdf".into()],
        },
    }
}
fn attachment() -> PutFile {
    PutFile {
        cid: "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3odne45kwf5f6fp4gfo6t5tpu".into(),
        path: Some("invoices/one.pdf".into()),
        mime_type: "application/pdf".into(),
        type_id: FileTypeId("invoice".into()),
        type_version: 1,
        attributes: json!({"amount":10,"currency":"USD"}),
        tags: vec!["paid".into()],
        categories: vec!["finance".into()],
        expected_revision: 0,
        mutation_id: "create-one".into(),
    }
}

async fn http_json(
    api: &axum::Router,
    method: &str,
    path: &str,
    body: &Value,
) -> Result<(StatusCode, Value), Box<dyn Error>> {
    let response = api
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(body)?))?,
        )
        .await?;
    let status = response.status();
    let value = serde_json::from_slice(&response.into_body().collect().await?.to_bytes())?;
    Ok((status, value))
}

async fn check_immutable_type(
    store: &PgFileMetadata,
    owner: &RlsContext,
    typ: &FileType,
) -> Result<(), Box<dyn Error>> {
    store.put_type(owner, typ).await?;
    store.put_type(owner, typ).await?;
    let mut conflicting = typ.clone();
    conflicting.schema["title"] = json!("changed");
    assert!(matches!(
        store.put_type(owner, &conflicting).await,
        Err(FileError::Conflict)
    ));
    let mut unsupported = typ.clone();
    unsupported.version = 2;
    unsupported.schema["minimum"] = json!(0);
    assert!(matches!(
        store.put_type(owner, &unsupported).await,
        Err(FileError::Invalid)
    ));
    Ok(())
}

fn disposable_write_fixture_url() -> Result<String, Box<dyn Error>> {
    if std::env::var("FILE_METADATA_TEST_ALLOW_WRITES").as_deref() != Ok("isolated-disposable") {
        return Err("explicit disposable-fixture write opt-in required".into());
    }
    let url = std::env::var("FILE_METADATA_TEST_DATABASE_URL")?;
    let config = tokio_postgres::Config::from_str(&url)?;
    if !config
        .get_dbname()
        .is_some_and(|name| name.ends_with("_file_metadata_test"))
    {
        return Err("database name must end with _file_metadata_test".into());
    }
    Ok(url)
}

#[tokio::test]
#[ignore = "Requires explicit disposable PostgreSQL fixture and FILE_METADATA_TEST_ALLOW_WRITES=isolated-disposable"]
async fn persisted_validation_isolation_replay_concurrency_discovery() -> Result<(), Box<dyn Error>>
{
    let url = disposable_write_fixture_url()?;
    let pg_pool = pool(&url)?;
    let store = Arc::new(PgFileMetadata::new(pg_pool.clone()));
    let suffix = uuid::Uuid::new_v4().to_string();
    let owner = identity(&format!("tenant-{suffix}"), "alice");
    let other_tenant = identity(&format!("other-{suffix}"), "alice");
    let other_subject = identity(&format!("tenant-{suffix}"), "bob");
    let typ = definition();
    check_immutable_type(&store, &owner, &typ).await?;
    let id = FileId("one".into());
    let initial = attachment();
    let created = store.put(&owner, &id, &initial).await?;
    assert_eq!(created.revision, 1);
    // Independent pool proves a committed PostgreSQL row, not session-local state.
    let independent = PgFileMetadata::new(pool(&url)?);
    assert_eq!(independent.get(&owner, &id).await?.revision, 1);
    assert!(matches!(
        store.get(&other_tenant, &id).await,
        Err(FileError::NotFound)
    ));
    assert!(matches!(
        store.get(&other_subject, &id).await,
        Err(FileError::NotFound)
    ));
    assert!(store.types(&other_tenant).await?.is_empty());
    assert!(store.types(&other_subject).await?.is_empty());
    let mut invalid = initial.clone();
    invalid.expected_revision = 1;
    invalid.mutation_id = "bad-value".into();
    invalid.attributes["amount"] = json!("not a number");
    assert!(matches!(
        store.put(&owner, &id, &invalid).await,
        Err(FileError::Invalid)
    ));
    assert_eq!(store.get(&owner, &id).await?.revision, 1);
    invalid.attributes["amount"] = json!(11);
    assert_eq!(store.put(&owner, &id, &invalid).await?.revision, 2); // failed attempt created no receipt
    let mut left = invalid.clone();
    left.expected_revision = 2;
    left.mutation_id = "concurrent-left".into();
    let mut right = left.clone();
    right.mutation_id = "concurrent-right".into();
    let (left_result, right_result) = tokio::join!(
        store.put(&owner, &id, &left),
        store.put(&owner, &id, &right)
    );
    assert!(left_result.is_ok() != right_result.is_ok());
    assert!(
        matches!(left_result, Err(FileError::Conflict))
            || matches!(right_result, Err(FileError::Conflict))
    );
    assert_eq!(store.get(&owner, &id).await?.revision, 3);
    assert_eq!(store.put(&owner, &id, &initial).await?.revision, 1); // replay is historical
    let mut reused = initial.clone();
    reused.attributes["amount"] = json!(999);
    assert!(matches!(
        store.put(&owner, &id, &reused).await,
        Err(FileError::Conflict)
    ));
    let second = FileId("two".into());
    let mut same = initial.clone();
    same.mutation_id = "create-two".into();
    let (left_result, right_result) = tokio::join!(
        store.put(&owner, &second, &same),
        store.put(&owner, &second, &same)
    );
    assert_eq!(left_result?.revision, 1);
    assert_eq!(right_result?.revision, 1);
    let query = FileQuery {
        cid: Some(initial.cid.clone()),
        type_id: Some(typ.type_id.clone()),
        tags: vec!["paid".into()],
        categories: vec!["finance".into()],
        attributes: json!({"currency":"USD"}),
        limit: 100,
        after: None,
    };
    assert_eq!(store.query(&owner, &query).await?.len(), 2);
    assert!(store.query(&other_tenant, &query).await?.is_empty());
    let mut paged = query.clone();
    paged.after = Some(id.clone());
    assert_eq!(store.query(&owner, &paged).await?.len(), 1);
    let mut mismatch = query.clone();
    mismatch.attributes = json!({"currency":"EUR"});
    assert!(store.query(&owner, &mismatch).await?.is_empty());
    // Exact CID reuse by another owner is legal and never reveals Alice's attributes.
    store.put_type(&other_tenant, &typ).await?;
    store.put(&other_tenant, &id, &initial).await?;
    assert_eq!(store.get(&owner, &id).await?.revision, 3);
    assert_eq!(store.get(&other_tenant, &id).await?.revision, 1);

    file_metadata_contracts::check_sql_validation(&pg_pool, &store, &owner, &id, &typ).await?;
    file_metadata_contracts::check_http_writes(store.clone(), &independent, &owner).await?;
    file_metadata_contracts::check_denied_and_disabled(store, &independent, owner, &id).await
}

#[path = "support/file_metadata_contracts.rs"]
mod file_metadata_contracts;

#[path = "support/issue40_fixture.rs"]
mod issue40_fixture;
#[path = "support/issue40_isolation.rs"]
mod issue40_isolation;
#[path = "support/issue40_scenarios.rs"]
mod issue40_scenarios;

#[tokio::test]
#[ignore = "Requires approved issue 40 disposable database and fixture manifest"]
async fn issue40_owner_save() -> Result<(), Box<dyn Error>> {
    issue40_scenarios::owner_save().await
}

#[tokio::test]
#[ignore = "Requires approved issue 40 disposable database and fixture manifest"]
async fn issue40_authorization_controls() -> Result<(), Box<dyn Error>> {
    issue40_scenarios::authorization_controls().await
}
