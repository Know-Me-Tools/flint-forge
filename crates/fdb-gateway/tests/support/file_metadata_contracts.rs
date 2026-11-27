use super::{
    attachment, definition, http_json, json, router, Arc, Body, BodyExt, Deny, Error, Extension,
    FileApiState, FileError, FileId, FileMetadataStore, FileType, FileTypeId, MatchInput, Permit,
    PgFileMetadata, Request, RlsContext, ServiceExt, StatusCode, Value,
};

pub(super) async fn check_sql_validation(
    pg_pool: &deadpool_postgres::Pool,
    store: &PgFileMetadata,
    owner: &RlsContext,
    id: &FileId,
    typ: &FileType,
) -> Result<(), Box<dyn Error>> {
    // Direct generic SQL cannot bypass validation or forge successful receipts.
    let client = pg_pool.get().await?;
    client
        .batch_execute("BEGIN; SET LOCAL ROLE authenticated")
        .await?;
    client
        .execute(
            "SELECT set_config('request.jwt.claims',$1,true)",
            &[&owner.claims_json],
        )
        .await?;
    let bad=client.execute("UPDATE flint_files.records SET revision=revision+1,data=jsonb_set(jsonb_set(data,'{expected_revision}',to_jsonb(revision)),'{attributes,amount}','\"invalid\"') WHERE file_id='one'",&[]).await;
    assert_eq!(
        bad.as_ref().err().and_then(tokio_postgres::Error::code),
        Some(&tokio_postgres::error::SqlState::CHECK_VIOLATION),
    );
    client.batch_execute("ROLLBACK").await?;
    assert_eq!(store.get(owner, id).await?.revision, 3);
    client
        .batch_execute("BEGIN; SET LOCAL ROLE authenticated")
        .await?;
    client
        .execute(
            "SELECT set_config('request.jwt.claims',$1,true)",
            &[&owner.claims_json],
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
            &[&owner.claims_json],
        )
        .await?;
    let mut numeric_type = serde_json::to_value(typ)?;
    numeric_type["type_id"] = json!(123);
    let bad_type = client.execute(
        "INSERT INTO flint_files.types(tenant_id,subject,type_id,version,definition) VALUES($1,$2,'123',1,$3)",
        &[&owner.tenant().ok_or("missing test tenant")?.0, &owner.keto_subject, &numeric_type],
    ).await;
    assert_eq!(
        bad_type
            .as_ref()
            .err()
            .and_then(tokio_postgres::Error::code),
        Some(&tokio_postgres::error::SqlState::CHECK_VIOLATION),
    );
    client.batch_execute("ROLLBACK").await?;
    let listed = store.types(owner).await?;
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].type_id, typ.type_id);
    let selected = fdb_app::files::match_type(
        store,
        owner,
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

    Ok(())
}

pub(super) async fn check_http_writes(
    store: Arc<PgFileMetadata>,
    independent: &PgFileMetadata,
    owner: &RlsContext,
) -> Result<(), Box<dyn Error>> {
    // Real Axum->app->port->Postgres request; JWT verification remains a separate boundary.
    let api = router(FileApiState {
        store: Some(store.clone()),
        pep: Arc::new(Permit),
        keto: None,
    })
    .layer(Extension(owner.clone()));
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
    let mut http_type = definition();
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
        .types(owner)
        .await?
        .iter()
        .any(|value| value.type_id == http_type.type_id));
    let mut http_record = attachment();
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
    let persisted = independent.get(owner, &FileId("http-one".into())).await?;
    assert_eq!(persisted.data.type_id, http_type.type_id);
    assert_eq!(persisted.data.type_version, 1);
    assert_eq!(persisted.data.attributes, http_record.attributes);

    check_matching(&api, &mut http_type).await?;
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
        independent.get(owner, &FileId("null-byte".into())).await,
        Err(FileError::NotFound)
    ));

    Ok(())
}

async fn check_matching(
    api: &axum::Router,
    http_type: &mut FileType,
) -> Result<(), Box<dyn Error>> {
    let (status, matched) = http_json(
        api,
        "POST",
        "/files/v1/types/match",
        &json!({"mime_type":"IMAGE/PNG","filename":"one.png"}),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(matched["data"]["type_id"], "http-image");
    let (status, unmatched) = http_json(
        api,
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
            api,
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
        api,
        "PUT",
        "/files/v1/types",
        &serde_json::to_value(&*http_type)?,
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    let (status, ambiguity) = http_json(
        api,
        "POST",
        "/files/v1/types/match",
        &json!({"mime_type":"image/png","filename":"one.png"}),
    )
    .await?;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(ambiguity["error"]["code"], "ambiguous_type");

    Ok(())
}

pub(super) async fn check_denied_and_disabled(
    store: Arc<PgFileMetadata>,
    independent: &PgFileMetadata,
    owner: RlsContext,
    id: &FileId,
) -> Result<(), Box<dyn Error>> {
    let denied = router(FileApiState {
        store: Some(store),
        pep: Arc::new(Deny),
        keto: None,
    })
    .layer(Extension(owner.clone()));
    let response = denied
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/files/v1/records/one")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&attachment())?))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(independent.get(&owner, id).await?.revision, 3);
    let disabled = router(FileApiState {
        store: None,
        pep: Arc::new(Permit),
        keto: None,
    })
    .layer(Extension(owner));
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
