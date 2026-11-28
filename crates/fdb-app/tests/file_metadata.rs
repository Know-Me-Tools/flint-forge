//! File metadata request validation and immutable type selection.
use async_trait::async_trait;
use fdb_app::files::{identifier, match_type, owner, validate_put, validate_query, SCHEMA_PROFILE};
use fdb_domain::files::{
    FileId, FileQuery, FileRecord, FileType, FileTypeId, MatchCriteria, MatchInput, PutFile,
};
use fdb_ports::files::{FileError, FileMetadataStore};
use forge_identity::RlsContext;
use serde_json::{json, Value};

struct Catalog(Vec<FileType>);

#[async_trait]
impl FileMetadataStore for Catalog {
    async fn types(&self, _: &RlsContext) -> Result<Vec<FileType>, FileError> {
        Ok(self.0.clone())
    }

    async fn put_type(&self, _: &RlsContext, _: &FileType) -> Result<FileType, FileError> {
        panic!("discovery must be read-only")
    }
    async fn put(&self, _: &RlsContext, _: &FileId, _: &PutFile) -> Result<FileRecord, FileError> {
        panic!("discovery must be read-only")
    }
    async fn get(&self, _: &RlsContext, _: &FileId) -> Result<FileRecord, FileError> {
        panic!("discovery only consults types")
    }
    async fn query(&self, _: &RlsContext, _: &FileQuery) -> Result<Vec<FileRecord>, FileError> {
        panic!("discovery only consults types")
    }
}

fn who() -> RlsContext {
    RlsContext {
        role: "authenticated".into(),
        claims_json: json!({"sub":"reader","tenant_id":"workspace","role":"authenticated"})
            .to_string(),
        raw_bearer: String::new(),
        keto_subject: "reader".into(),
        vault_key_id: None,
    }
}

fn definition(id: &str, mimes: &[&str], suffixes: &[&str]) -> FileType {
    FileType {
        type_id: FileTypeId(id.into()),
        version: 1,
        schema_profile: SCHEMA_PROFILE.into(),
        schema: json!({"type":"object"}),
        criteria: MatchCriteria {
            mime_types: mimes.iter().map(ToString::to_string).collect(),
            filename_suffixes: suffixes.iter().map(ToString::to_string).collect(),
        },
    }
}

#[tokio::test]
async fn match_combines_populated_criteria_and_requires_one_visible_type() {
    let who = who();
    assert_eq!(
        owner(&who).expect("authenticated owner"),
        ("workspace".into(), "reader".into())
    );
    let mut catalog = Catalog(vec![
        definition("manual", &[], &[]),
        definition("image", &["IMAGE/*"], &[".png"]),
    ]);
    let mut input = MatchInput {
        mime_type: "image/PNG".into(),
        filename: "logo.png".into(),
    };
    assert_eq!(
        match_type(&catalog, &who, &input)
            .await
            .expect("match")
            .expect("image")
            .type_id
            .0,
        "image"
    );
    input.filename = "logo.PNG".into();
    assert!(match_type(&catalog, &who, &input)
        .await
        .expect("no match")
        .is_none());
    input.filename = "logo.png".into();
    catalog.0.push(definition("png", &["image/png"], &[]));
    assert!(matches!(
        match_type(&catalog, &who, &input).await,
        Err(FileError::Ambiguous)
    ));
    for mime in ["image", "image/*", "/png", "image/png;v=1"] {
        input.mime_type = mime.into();
        assert!(matches!(
            match_type(&catalog, &who, &input).await,
            Err(FileError::Invalid)
        ));
    }
    for claims in [
        json!({"sub":"reader","role":"authenticated"}),
        json!({"sub":"","tenant_id":"space","role":"authenticated"}),
    ] {
        assert!(matches!(
            owner(&RlsContext {
                claims_json: claims.to_string(),
                ..who.clone()
            }),
            Err(FileError::Denied)
        ));
    }
    assert!(matches!(
        owner(&RlsContext {
            role: "anon".into(),
            ..who
        }),
        Err(FileError::Denied)
    ));
}

#[test]
fn bounded_record_and_discovery_requests_validate_their_shared_identifiers() {
    let id = FileId("invoice-1.0".into());
    assert!(identifier(&"a".repeat(128)).is_ok());
    for invalid in ["", "user/file", &"a".repeat(129)] {
        assert!(matches!(identifier(invalid), Err(FileError::Invalid)));
    }
    let record = json!({"cid":"sample-cid", "mime_type":"text/plain","type_id":"receipt", "type_version":1, "attributes":{}, "expected_revision":0, "mutation_id":"save","path":"folder/receipt","tags":["new"], "categories":["docs"]});
    assert!(validate_put(
        &id,
        &serde_json::from_value(record.clone()).expect("fixture")
    )
    .is_ok());
    for (key, value) in [
        ("cid", json!("two words")),
        ("path", json!("p".repeat(4097))),
        ("mime_type", json!("")),
        ("type_version", json!(0)),
        ("expected_revision", json!(-1)),
        ("attributes", json!([])),
        ("tags", json!([""])),
        ("categories", json!(vec!["document"; 65])),
        ("attributes", json!({"memo":"x".repeat(66_000)})),
    ] {
        let mut data = record.clone();
        data[key] = value;
        assert!(
            matches!(
                validate_put(&id, &serde_json::from_value(data).expect("typed fixture")),
                Err(FileError::Invalid)
            ),
            "{key}"
        );
    }

    let query = json!({"attributes":{},"limit":100,"type_id":"receipt","after":"a"});
    assert!(validate_query(&serde_json::from_value(query.clone()).expect("fixture")).is_ok());
    for (key, value) in [
        ("limit", json!(0)),
        ("limit", json!(101)),
        ("after", json!("bad id")),
        ("type_id", json!("")),
        ("attributes", Value::Null),
        ("tags", json!([""])),
        ("categories", json!([""])),
        ("attributes", json!({"memo":"x".repeat(66_000)})),
    ] {
        let mut data = query.clone();
        data[key] = value;
        assert!(
            matches!(
                validate_query(&serde_json::from_value(data).expect("typed fixture")),
                Err(FileError::Invalid)
            ),
            "{key}"
        );
    }
}
