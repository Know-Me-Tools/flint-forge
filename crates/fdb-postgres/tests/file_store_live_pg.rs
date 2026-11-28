//! Exercise file metadata over the deployed RLS policies in a dedicated, short-lived database.
#![allow(clippy::expect_used)]

use deadpool_postgres::{Config, Pool, Runtime};
use fdb_domain::files::{FileId, FileQuery, FileType, FileTypeId, MatchCriteria, PutFile};
use fdb_ports::files::{FileError, FileMetadataStore};
use fdb_postgres::PgFileMetadata;
use forge_identity::RlsContext;
use serde_json::json;
use tokio_postgres::NoTls;

fn identity(tenant: &str, subject: &str) -> RlsContext {
    RlsContext {
        role: "authenticated".into(),
        claims_json: json!({"sub":subject,"tenant_id":tenant}).to_string(),
        raw_bearer: String::new(),
        keto_subject: subject.into(),
        vault_key_id: None,
    }
}

fn document_type() -> FileType {
    FileType {
        type_id: FileTypeId("document".into()),
        version: 1,
        schema_profile: "file-schema-v1".into(),
        schema: json!({"type":"object","properties":{"pages":{"type":"integer"}},"required":["pages"]}),
        criteria: MatchCriteria::default(),
    }
}

fn attachment() -> PutFile {
    PutFile {
        cid: "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3odne45kwf5f6fp4gfo6t5tpu".into(),
        path: Some("doc.pdf".into()),
        mime_type: "application/pdf".into(),
        type_id: FileTypeId("document".into()),
        type_version: 1,
        attributes: json!({"pages":1}),
        tags: vec!["draft".into()],
        categories: vec!["docs".into()],
        expected_revision: 0,
        mutation_id: "create".into(),
    }
}

async fn assert_receipt_lifecycle(
    store: &PgFileMetadata,
    owner: &RlsContext,
    id: &FileId,
    initial: &PutFile,
) {
    assert_eq!(
        store
            .put(owner, id, initial)
            .await
            .expect("create record")
            .revision,
        1
    );
    let mut next = initial.clone();
    next.mutation_id = "update".into();
    next.expected_revision = 1;
    next.attributes = json!({"pages":"invalid"});
    assert!(matches!(
        store.put(owner, id, &next).await,
        Err(FileError::Invalid)
    ));
    assert_eq!(
        store
            .get(owner, id)
            .await
            .expect("old revision after rollback")
            .revision,
        1
    );
    next.attributes = json!({"pages":2});
    assert_eq!(
        store
            .put(owner, id, &next)
            .await
            .expect("correct update")
            .revision,
        2
    );
    assert_eq!(
        store
            .put(owner, id, initial)
            .await
            .expect("historical receipt")
            .revision,
        1
    );
    assert_eq!(
        store
            .get(owner, id)
            .await
            .expect("latest revision")
            .revision,
        2
    );
    next.attributes = json!({"pages":3});
    assert!(matches!(
        store.put(owner, id, &next).await,
        Err(FileError::Conflict)
    ));
    next.mutation_id = "stale".into();
    assert!(matches!(
        store.put(owner, id, &next).await,
        Err(FileError::Conflict)
    ));
    next.expected_revision = -1;
    assert!(matches!(
        store.put(owner, id, &next).await,
        Err(FileError::Invalid)
    ));
}

async fn exercise_store(pool: Pool) {
    let setup = pool.get().await.expect("setup connection");
    setup
        .batch_execute(include_str!("../../../migrations/0021_file_metadata.sql"))
        .await
        .expect("metadata migration");
    setup
        .batch_execute(include_str!(
            "../../../migrations/0022_file_record_insert_validation.sql"
        ))
        .await
        .expect("record validation migration");
    drop(setup);
    let store = PgFileMetadata::new(pool);
    let owner = identity("tenant-one", "alice");
    let mut typ = document_type();
    store
        .put_type(&owner, &typ)
        .await
        .expect("create definition");
    store
        .put_type(&owner, &typ)
        .await
        .expect("identical definition");
    assert_eq!(
        store.types(&owner).await.expect("owner types")[0].type_id,
        typ.type_id
    );
    typ.schema["title"] = json!("different");
    assert!(matches!(
        store.put_type(&owner, &typ).await,
        Err(FileError::Conflict)
    ));

    let id = FileId("report".into());
    let initial = attachment();
    assert_receipt_lifecycle(&store, &owner, &id, &initial).await;

    let mut query = FileQuery {
        cid: Some(initial.cid),
        type_id: Some(typ.type_id),
        tags: vec!["draft".into()],
        categories: vec!["docs".into()],
        attributes: json!({"pages":2}),
        limit: 1,
        after: None,
    };
    assert_eq!(
        store.query(&owner, &query).await.expect("search")[0].file_id,
        id
    );
    for other in [
        identity("tenant-two", "alice"),
        identity("tenant-one", "bob"),
    ] {
        assert!(matches!(
            store.get(&other, &id).await,
            Err(FileError::NotFound)
        ));
        assert!(store
            .query(&other, &query)
            .await
            .expect("private search")
            .is_empty());
        assert!(store.types(&other).await.expect("private types").is_empty());
    }
    query.after = Some(id);
    assert!(store
        .query(&owner, &query)
        .await
        .expect("next page")
        .is_empty());
    query.limit = 0;
    assert!(matches!(
        store.query(&owner, &query).await,
        Err(FileError::Invalid)
    ));
    let mut anon = owner;
    anon.role = "anon".into();
    assert!(matches!(store.types(&anon).await, Err(FileError::Denied)));
    assert!(matches!(
        store.types(&identity("", "alice")).await,
        Err(FileError::Denied)
    ));
}

#[tokio::test]
async fn publishes_types_and_persists_private_revisioned_files() {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        return;
    };
    let mut config = Config::new();
    config.url = Some(url);
    let admin_pool = config
        .create_pool(Some(Runtime::Tokio1), NoTls)
        .expect("admin pool");
    let admin = admin_pool.get().await.expect("admin connection");
    let database: String = admin
        .query_one(
            "SELECT 'forge_' || replace(gen_random_uuid()::text,'-','') || '_file_metadata_test'",
            &[],
        )
        .await
        .expect("unique database name")
        .get(0);
    admin
        .batch_execute(&format!("CREATE DATABASE {database} TEMPLATE template0"))
        .await
        .expect("create isolated database");
    config.dbname = Some(database.clone());
    let isolated = config
        .create_pool(Some(Runtime::Tokio1), NoTls)
        .expect("isolated pool");
    let outcome = tokio::spawn(exercise_store(isolated)).await;
    admin
        .batch_execute(&format!("DROP DATABASE {database} WITH (FORCE)"))
        .await
        .expect("remove isolated database");
    outcome.expect("file metadata contract");
}

#[tokio::test]
async fn closed_pool_preserves_the_file_ports_unavailable_error() {
    let mut config = Config::new();
    config.dbname = Some("closed_pool".into());
    let pool = config
        .create_pool(Some(Runtime::Tokio1), NoTls)
        .expect("pool");
    pool.close();
    let store = PgFileMetadata::new(pool);
    let owner = identity("acme", "alice");
    let id = FileId("report".into());
    let query = FileQuery {
        cid: None,
        type_id: None,
        tags: vec![],
        categories: vec![],
        attributes: json!({}),
        limit: 1,
        after: None,
    };
    assert!(matches!(
        store.put_type(&owner, &document_type()).await,
        Err(FileError::Unavailable)
    ));
    assert!(matches!(
        store.types(&owner).await,
        Err(FileError::Unavailable)
    ));
    assert!(matches!(
        store.put(&owner, &id, &attachment()).await,
        Err(FileError::Unavailable)
    ));
    assert!(matches!(
        store.get(&owner, &id).await,
        Err(FileError::Unavailable)
    ));
    assert!(matches!(
        store.query(&owner, &query).await,
        Err(FileError::Unavailable)
    ));
}
