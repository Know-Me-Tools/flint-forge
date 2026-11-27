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

#[tokio::test]
#[ignore = "Requires explicit disposable PostgreSQL fixture and FILE_METADATA_TEST_ALLOW_WRITES=isolated-disposable"]
async fn persisted_validation_isolation_replay_concurrency_discovery() -> Result<(), Box<dyn Error>>
{
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
    let pg_pool = pool(&url)?;
    let store = Arc::new(PgFileMetadata::new(pg_pool.clone()));
    let suffix = uuid::Uuid::new_v4().to_string();
    let a = identity(&format!("tenant-{suffix}"), "alice");
    let b = identity(&format!("other-{suffix}"), "alice");
    let c = identity(&format!("tenant-{suffix}"), "bob");
    let typ = definition();
    store.put_type(&a, &typ).await?;
    store.put_type(&a, &typ).await?;
    let mut conflicting = typ.clone();
    conflicting.schema["title"] = json!("changed");
    assert!(matches!(
        store.put_type(&a, &conflicting).await,
        Err(FileError::Conflict)
    ));
    let mut unsupported = typ.clone();
    unsupported.version = 2;
    unsupported.schema["minimum"] = json!(0);
    assert!(matches!(
        store.put_type(&a, &unsupported).await,
        Err(FileError::Invalid)
    ));
    let id = FileId("one".into());
    let initial = attachment();
    let created = store.put(&a, &id, &initial).await?;
    assert_eq!(created.revision, 1);
    // Independent pool proves a committed PostgreSQL row, not session-local state.
    let independent = PgFileMetadata::new(pool(&url)?);
    assert_eq!(independent.get(&a, &id).await?.revision, 1);
    assert!(matches!(store.get(&b, &id).await, Err(FileError::NotFound)));
    assert!(matches!(store.get(&c, &id).await, Err(FileError::NotFound)));
    assert!(store.types(&b).await?.is_empty());
    assert!(store.types(&c).await?.is_empty());
    let mut invalid = initial.clone();
    invalid.expected_revision = 1;
    invalid.mutation_id = "bad-value".into();
    invalid.attributes["amount"] = json!("not a number");
    assert!(matches!(
        store.put(&a, &id, &invalid).await,
        Err(FileError::Invalid)
    ));
    assert_eq!(store.get(&a, &id).await?.revision, 1);
    invalid.attributes["amount"] = json!(11);
    assert_eq!(store.put(&a, &id, &invalid).await?.revision, 2); // failed attempt created no receipt
    let mut left = invalid.clone();
    left.expected_revision = 2;
    left.mutation_id = "concurrent-left".into();
    let mut right = left.clone();
    right.mutation_id = "concurrent-right".into();
    let (l, r) = tokio::join!(store.put(&a, &id, &left), store.put(&a, &id, &right));
    assert!(l.is_ok() != r.is_ok());
    assert!(matches!(l, Err(FileError::Conflict)) || matches!(r, Err(FileError::Conflict)));
    assert_eq!(store.get(&a, &id).await?.revision, 3);
    assert_eq!(store.put(&a, &id, &initial).await?.revision, 1); // replay is historical
    let mut reused = initial.clone();
    reused.attributes["amount"] = json!(999);
    assert!(matches!(
        store.put(&a, &id, &reused).await,
        Err(FileError::Conflict)
    ));
    let second = FileId("two".into());
    let mut same = initial.clone();
    same.mutation_id = "create-two".into();
    let (l, r) = tokio::join!(store.put(&a, &second, &same), store.put(&a, &second, &same));
    assert_eq!(l?.revision, 1);
    assert_eq!(r?.revision, 1);
    let query = FileQuery {
        cid: Some(initial.cid.clone()),
        type_id: Some(typ.type_id.clone()),
        tags: vec!["paid".into()],
        categories: vec!["finance".into()],
        attributes: json!({"currency":"USD"}),
        limit: 100,
        after: None,
    };
    assert_eq!(store.query(&a, &query).await?.len(), 2);
    assert!(store.query(&b, &query).await?.is_empty());
    let mut paged = query.clone();
    paged.after = Some(id.clone());
    assert_eq!(store.query(&a, &paged).await?.len(), 1);
    let mut mismatch = query.clone();
    mismatch.attributes = json!({"currency":"EUR"});
    assert!(store.query(&a, &mismatch).await?.is_empty());
    // Exact CID reuse by another owner is legal and never reveals Alice's attributes.
    store.put_type(&b, &typ).await?;
    store.put(&b, &id, &initial).await?;
    assert_eq!(store.get(&a, &id).await?.revision, 3);
    assert_eq!(store.get(&b, &id).await?.revision, 1);

    // Direct generic SQL cannot bypass validation or forge successful receipts.
    let client = pg_pool.get().await?;
    client
        .batch_execute("BEGIN; SET LOCAL ROLE authenticated")
        .await?;
    client
        .execute(
            "SELECT set_config('request.jwt.claims',$1,true)",
            &[&a.claims_json],
        )
        .await?;
    let bad=client.execute("UPDATE flint_files.records SET revision=revision+1,data=jsonb_set(jsonb_set(data,'{expected_revision}',to_jsonb(revision)),'{attributes,amount}','\"invalid\"') WHERE file_id='one'",&[]).await;
    assert_eq!(
        bad.as_ref().err().and_then(tokio_postgres::Error::code),
        Some(&tokio_postgres::error::SqlState::CHECK_VIOLATION),
    );
    client.batch_execute("ROLLBACK").await?;
    assert_eq!(store.get(&a, &id).await?.revision, 3);
    client
        .batch_execute("BEGIN; SET LOCAL ROLE authenticated")
        .await?;
    client
        .execute(
            "SELECT set_config('request.jwt.claims',$1,true)",
            &[&a.claims_json],
        )
        .await?;
    let forged=client.execute("INSERT INTO flint_files.receipts(tenant_id,subject,mutation_id,request,response) SELECT tenant_id,subject,'forged','{}','{}' FROM flint_files.records LIMIT 1",&[]).await;
    assert_eq!(
        forged.as_ref().err().and_then(tokio_postgres::Error::code),
        Some(&tokio_postgres::error::SqlState::CHECK_VIOLATION),
    );
    client.batch_execute("ROLLBACK").await?;

    // A numeric JSON type_id must not poison an immutable, readable registry.
    client
        .batch_execute("BEGIN; SET LOCAL ROLE authenticated")
        .await?;
    client
        .execute(
            "SELECT set_config('request.jwt.claims',$1,true)",
            &[&a.claims_json],
        )
        .await?;
    let mut numeric_type = serde_json::to_value(&typ)?;
    numeric_type["type_id"] = json!(123);
    let bad_type = client.execute(
        "INSERT INTO flint_files.types(tenant_id,subject,type_id,version,definition) VALUES($1,$2,'123',1,$3)",
        &[&a.tenant().ok_or("missing test tenant")?.0, &a.keto_subject, &numeric_type],
    ).await;
    assert_eq!(
        bad_type
            .as_ref()
            .err()
            .and_then(tokio_postgres::Error::code),
        Some(&tokio_postgres::error::SqlState::CHECK_VIOLATION),
    );
    client.batch_execute("ROLLBACK").await?;
    let listed = store.types(&a).await?;
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].type_id, typ.type_id);
    let selected = fdb_app::files::match_type(
        store.as_ref(),
        &a,
        &MatchInput {
            mime_type: "Application/PDF".into(),
            filename: "one.pdf".into(),
        },
    )
    .await?
    .ok_or("expected intact type match after rejected insert")?;
    assert_eq!(selected.type_id, typ.type_id);

    // Pin the specific PostgreSQL U+0000 rejection, not a blanket error class.
    let null_json = serde_json::to_string(&json!({"value": "\u{0000}"}))?;
    let null_value = client
        .query_one("SELECT $1::text::jsonb", &[&null_json])
        .await;
    assert_eq!(
        null_value
            .as_ref()
            .err()
            .and_then(tokio_postgres::Error::code)
            .map(tokio_postgres::error::SqlState::code),
        Some("22P05"),
    );

    // Real Axum->app->port->Postgres request; JWT verification remains a separate boundary.
    let api = router(FileApiState {
        store: Some(store.clone()),
        pep: Arc::new(Permit),
        keto: None,
    })
    .layer(Extension(a.clone()));
    let response = api
        .clone()
        .oneshot(
            Request::builder()
                .uri("/files/v1/records/one")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(&response.into_body().collect().await?.to_bytes())?;
    assert_eq!(body["data"]["revision"], 3);
    // Successful mutations traverse actual HTTP/app/policy/adapter/Postgres wiring.
    let mut http_type = typ.clone();
    http_type.type_id = FileTypeId("http-image".into());
    http_type.criteria.mime_types = vec!["image/*".into()];
    http_type.criteria.filename_suffixes = vec![".png".into()];
    let (status, body) = http_json(
        &api,
        "PUT",
        "/files/v1/types",
        &serde_json::to_value(&http_type)?,
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"], serde_json::to_value(&http_type)?);
    assert!(independent
        .types(&a)
        .await?
        .iter()
        .any(|value| value.type_id == http_type.type_id));
    let mut http_record = initial.clone();
    http_record.type_id = http_type.type_id.clone();
    http_record.mime_type = "image/png".into();
    http_record.path = Some("images/one.png".into());
    http_record.mutation_id = "http-create".into();
    let (status, body) = http_json(
        &api,
        "PUT",
        "/files/v1/records/http-one",
        &serde_json::to_value(&http_record)?,
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["file_id"], "http-one");
    assert_eq!(body["data"]["revision"], 1);
    assert_eq!(body["data"]["type_id"], "http-image");
    assert_eq!(body["data"]["type_version"], 1);
    assert_eq!(body["data"]["cid"], http_record.cid);
    assert_eq!(body["data"]["attributes"], http_record.attributes);
    let persisted = independent.get(&a, &FileId("http-one".into())).await?;
    assert_eq!(persisted.data.type_id, http_type.type_id);
    assert_eq!(persisted.data.type_version, 1);
    assert_eq!(persisted.data.attributes, http_record.attributes);

    let (status, matched) = http_json(
        &api,
        "POST",
        "/files/v1/types/match",
        &json!({"mime_type":"IMAGE/PNG","filename":"one.png"}),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(matched["data"]["type_id"], "http-image");
    let (status, unmatched) = http_json(
        &api,
        "POST",
        "/files/v1/types/match",
        &json!({"mime_type":"image/jpeg","filename":"one.jpg"}),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert!(unmatched["data"].is_null());
    for malformed in [
        "image",
        "image/",
        "/png",
        "image/png/extra",
        "image/png;charset=utf-8",
        "image/*",
        "*/png",
        " image/png",
        "image/png ",
        "image/p ng",
    ] {
        let (status, error) = http_json(
            &api,
            "POST",
            "/files/v1/types/match",
            &json!({"mime_type":malformed,"filename":"one.png"}),
        )
        .await?;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(error["error"]["code"], "invalid_metadata");
    }
    http_type.version = 2;
    let (status, _) = http_json(
        &api,
        "PUT",
        "/files/v1/types",
        &serde_json::to_value(&http_type)?,
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    let (status, ambiguity) = http_json(
        &api,
        "POST",
        "/files/v1/types/match",
        &json!({"mime_type":"image/png","filename":"one.png"}),
    )
    .await?;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(ambiguity["error"]["code"], "ambiguous_type");

    let mut null_record = http_record.clone();
    null_record.mutation_id = "http-null-byte".into();
    null_record.tags = vec!["\u{0000}".into()];
    let (status, error) = http_json(
        &api,
        "PUT",
        "/files/v1/records/null-byte",
        &serde_json::to_value(&null_record)?,
    )
    .await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error["error"]["code"], "invalid_metadata");
    assert!(matches!(
        independent.get(&a, &FileId("null-byte".into())).await,
        Err(FileError::NotFound)
    ));

    let denied = router(FileApiState {
        store: Some(store),
        pep: Arc::new(Deny),
        keto: None,
    })
    .layer(Extension(a.clone()));
    let response = denied
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/files/v1/records/one")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&initial)?))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(independent.get(&a, &id).await?.revision, 3);
    let disabled = router(FileApiState {
        store: None,
        pep: Arc::new(Permit),
        keto: None,
    })
    .layer(Extension(a));
    let response = disabled
        .oneshot(
            Request::builder()
                .uri("/files/v1/records/one")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    Ok(())
}

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
