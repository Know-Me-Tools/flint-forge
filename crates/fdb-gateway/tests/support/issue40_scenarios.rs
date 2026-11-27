//! HTTP and independent persistence oracles for the synthetic Forge #40 packet.
use super::{attachment, definition, identity, issue40_fixture::Issue40Fixture, issue40_isolation};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use fdb_domain::files::{FileId, FileRecord, FileTypeId, PutFile};
use fdb_ports::files::{FileError, FileMetadataStore};
use forge_identity::RlsContext;
use http_body_util::BodyExt;
use serde::Serialize;
use serde_json::{json, Value};
use std::error::Error;
use tower::ServiceExt;
use uuid::Uuid;

type TestResult<T> = Result<T, Box<dyn Error>>;
const ACTOR: Option<&str> = Some("ipfs-console-backend");

fn fresh() -> String {
    Uuid::new_v4().to_string()
}

fn mutation(revision: i64, amount: i64) -> PutFile {
    let mut data = attachment();
    data.expected_revision = revision;
    data.mutation_id = fresh();
    data.attributes["amount"] = json!(amount);
    data
}

async fn put(
    fixture: &Issue40Fixture,
    bearer: &str,
    path: &str,
    body: &impl Serialize,
    expected: StatusCode,
) -> TestResult<Value> {
    let request = Request::put(path)
        .header("authorization", format!("Bearer {bearer}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(body)?))?;
    let response = fixture.api.clone().oneshot(request).await?;
    assert_eq!(response.status(), expected);
    Ok(serde_json::from_slice(
        &response.into_body().collect().await?.to_bytes(),
    )?)
}

fn path(id: &FileId) -> String {
    format!("/files/v1/records/{}", id.0)
}

async fn missing(f: &Issue40Fixture, ctx: &RlsContext, id: &FileId) -> TestResult<()> {
    match f.independent.get(ctx, id).await {
        Err(FileError::NotFound) => Ok(()),
        _ => Err("independent read did not return metadata_not_found".into()),
    }
}

async fn assert_row(f: &Issue40Fixture, ctx: &RlsContext, expected: &FileRecord) -> TestResult<()> {
    let stored = f.independent.get(ctx, &expected.file_id).await?;
    assert_eq!(
        serde_json::to_value(stored)?,
        serde_json::to_value(expected)?
    );
    Ok(())
}

async fn save_twice(
    f: &Issue40Fixture,
    ctx: &RlsContext,
    bearer: &str,
    id: &FileId,
) -> TestResult<FileRecord> {
    missing(f, ctx, id).await?;
    let mut expected = FileRecord {
        file_id: id.clone(),
        revision: 0,
        data: attachment(),
    };
    for (revision, amount) in [(1, 10), (2, 21)] {
        expected.data = mutation(revision - 1, amount);
        expected.revision = revision;
        let response = put(f, bearer, &path(id), &expected.data, StatusCode::OK).await?;
        assert_eq!(response["data"], serde_json::to_value(&expected)?);
        assert_row(f, ctx, &expected).await?;
    }
    Ok(expected)
}

pub(super) async fn owner_save() -> TestResult<()> {
    let subject = fresh();
    let tenant = fresh();
    let ctx = identity(&tenant, &subject);
    let f = Issue40Fixture::new(&[(&subject, &tenant)]).await?;
    let result = async {
        f.independent.put_type(&ctx, &definition()).await?;
        let bearer = f.bearer(&subject, &tenant, ACTOR)?;
        save_twice(&f, &ctx, &bearer, &FileId(fresh())).await?;
        eprintln!("issue40_evidence FILE-1 owner insert=200 revision=1 update=200 revision=2 independent_full_row_matches=true");
        let id = FileId(fresh());
        let mut invalid = mutation(0, 10);
        invalid.attributes["amount"] = json!("invalid");
        let response = put(&f, &bearer, &path(&id), &invalid, StatusCode::UNPROCESSABLE_ENTITY).await?;
        assert_eq!(response["error"]["code"], "invalid_metadata");
        missing(&f, &ctx, &id).await?;
        eprintln!("issue40_evidence FILE-1 invalid_attributes=422 independent_row_absent=true");
        Ok(())
    }.await;
    f.cleanup().await?;
    result
}

async fn denied_record(f: &Issue40Fixture, bearer: &str, ctx: &RlsContext) -> TestResult<()> {
    let id = FileId(fresh());
    let url = path(&id);
    let create = mutation(0, 11);
    missing(f, ctx, &id).await?;
    let denied = put(f, bearer, &url, &create, StatusCode::FORBIDDEN).await?;
    assert_eq!(denied["error"]["code"], "metadata_denied");
    missing(f, ctx, &id).await?;
    let initial = FileRecord {
        file_id: id.clone(),
        revision: 1,
        data: create,
    };
    f.independent.put(ctx, &id, &initial.data).await?;
    assert_row(f, ctx, &initial).await?;
    let denied = put(f, bearer, &url, &mutation(1, 20), StatusCode::FORBIDDEN).await?;
    assert_eq!(denied["error"]["code"], "metadata_denied");
    assert_row(f, ctx, &initial).await
}

pub(super) async fn authorization_controls() -> TestResult<()> {
    let tenant = fresh();
    let other_tenant = fresh();
    let subject = fresh();
    let neighbor = fresh();
    let remote = fresh();
    let unbound = fresh();
    let owner = identity(&tenant, &subject);
    // The accepted upstream resolver issues only one tenant per subject at a time.
    // Use independent single-tenant subjects over HTTP and vary each field alone in SQL.
    let f = Issue40Fixture::new(&[
        (&subject, &tenant),
        (&neighbor, &tenant),
        (&remote, &other_tenant),
    ])
    .await?;
    let result = async {
        for (sub, tid) in [(&subject, &tenant), (&subject, &other_tenant), (&neighbor, &tenant),
                           (&remote, &other_tenant), (&unbound, &tenant)] {
            f.independent.put_type(&identity(tid, sub), &definition()).await?;
        }
        let token = f.bearer(&subject, &tenant, ACTOR)?;
        let original = save_twice(&f, &owner, &token, &FileId(fresh())).await?;
        denied_record(&f, &f.bearer(&unbound, &tenant, ACTOR)?, &identity(&tenant, &unbound)).await?;
        eprintln!("issue40_evidence FILE-2 insert=403 update=403 code=metadata_denied independent_state_matches=true");
        // A synthetic out-of-scope claim tests the Forge boundary independently of issuance.
        denied_record(&f, &f.bearer(&subject, &other_tenant, ACTOR)?, &identity(&other_tenant, &subject)).await?;
        for actor in [None, Some("another-client")] {
            denied_record(&f, &f.bearer(&subject, &tenant, actor)?, &owner).await?;
        }
        eprintln!("issue40_evidence FILE-5 off_tenant=403 missing_actor=403 wrong_actor=403 independent_state_matches=true");
        let mut new_type = definition();
        new_type.type_id = FileTypeId(fresh());
        let before = f.independent.types(&owner).await?;
        assert!(!before.iter().any(|t| t.type_id == new_type.type_id));
        let denied = put(&f, &token, "/files/v1/types", &new_type, StatusCode::FORBIDDEN).await?;
        assert_eq!(denied["error"]["code"], "metadata_denied");
        assert_eq!(serde_json::to_value(f.independent.types(&owner).await?)?, serde_json::to_value(before)?);
        eprintln!("issue40_evidence FILE-4 type_registration=403 code=metadata_denied independent_type_set_unchanged=true");
        for (sub, tid) in [(&neighbor, &tenant), (&remote, &other_tenant)] {
            let foreign = identity(tid, sub);
            let token = f.bearer(sub, tid, ACTOR)?;
            let control = save_twice(&f, &foreign, &token, &FileId(fresh())).await?;
            missing(&f, &foreign, &original.file_id).await?;
            assert_row(&f, &owner, &original).await?;
            let conflict = put(&f, &token, &path(&original.file_id), &mutation(original.revision, 31), StatusCode::CONFLICT).await?;
            assert_eq!(conflict["error"]["code"], "metadata_conflict");
            assert_row(&f, &owner, &original).await?;
            assert_row(&f, &foreign, &control).await?;
            missing(&f, &foreign, &original.file_id).await?;
        }
        eprintln!("issue40_evidence FILE-3 http_owner_controls=200 foreign_update=409 independent_state_matches=true");
        issue40_isolation::check(&f, &tenant, &subject, &other_tenant, &neighbor, &original.file_id).await?;
        eprintln!("issue40_evidence FILE-3 sql_axes=2 foreign_select=0 foreign_update=0 foreign_insert=rls_denied");
        Ok(())
    }.await;
    f.cleanup().await?;
    result
}
