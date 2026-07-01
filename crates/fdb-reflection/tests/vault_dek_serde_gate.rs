//! Security gate: `test_vault_dek_not_in_compiled_state`.
//!
//! Non-negotiable P2-carried gate (RFC-FORGE §2.4 / model.rs SECURITY note):
//! plaintext DEK / master-key material MUST NEVER appear in the compiled schema
//! state that is held in memory and (for its serializable core) emitted as JSON.
//!
//! `CompiledState` itself is not `Serialize` (it holds an Axum `Router` and an
//! async-graphql `Schema`). Its serde-serializable core is `DatabaseModel`,
//! which carries `tables[*].vault_key: Option<EncryptedDek>`. `EncryptedDek` is
//! ciphertext-only by contract, so serializing a populated model must expose
//! only the ciphertext bytes under `vault_key` — never a plaintext-key field.

use fdb_reflection::model::{Column, DatabaseModel, EncryptedDek, Table};

/// Field-name fragments that would indicate plaintext key material leaked into
/// the serialized representation. `vault_key` is intentionally NOT here — it is
/// the allowed ciphertext field.
const FORBIDDEN_KEY_FRAGMENTS: &[&str] = &[
    "\"dek\"",
    "plaintext",
    "master_key",
    "masterkey",
    "private_key",
    "secret_key",
    "unwrapped",
    "cleartext",
];

/// A model with a populated (ciphertext) vault key on a table.
fn model_with_vault_key() -> DatabaseModel {
    DatabaseModel {
        tables: vec![Table {
            schema: "public".into(),
            name: "secrets".into(),
            columns: vec![Column {
                name: "value".into(),
                pg_type: "text".into(),
                nullable: false,
                default: None,
            }],
            pk: vec!["id".into()],
            fk: vec![],
            rls_enabled: true,
            // Ciphertext bytes standing in for a KMS-wrapped DEK.
            vault_key: Some(EncryptedDek(vec![0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02])),
        }],
        functions: vec![],
        views: vec![],
        version: 7,
    }
}

#[test]
fn test_vault_dek_not_in_compiled_state() {
    let model = model_with_vault_key();

    // 1. Serde JSON must not carry any plaintext-key field name.
    let json = serde_json::to_string(&model).expect("model serializes");
    let lower = json.to_ascii_lowercase();
    for frag in FORBIDDEN_KEY_FRAGMENTS {
        assert!(
            !lower.contains(frag),
            "serialized model leaked forbidden key fragment `{frag}`: {json}"
        );
    }

    // 2. The only key-related field present is the ciphertext `vault_key`.
    assert!(
        json.contains("vault_key"),
        "populated model should still carry the ciphertext vault_key field"
    );

    // 3. `Debug` render must not leak plaintext-key field names either.
    let dbg = format!("{model:?}").to_ascii_lowercase();
    for frag in FORBIDDEN_KEY_FRAGMENTS {
        assert!(
            !dbg.contains(frag),
            "Debug render leaked forbidden key fragment `{frag}`"
        );
    }
}

/// A model with NO vault key must serialize cleanly too (regression guard: the
/// `Option::None` case must not synthesize any key field).
#[test]
fn test_model_without_vault_key_has_no_key_fields() {
    let mut model = model_with_vault_key();
    model.tables[0].vault_key = None;

    let json = serde_json::to_string(&model).expect("serializes");
    let lower = json.to_ascii_lowercase();
    for frag in FORBIDDEN_KEY_FRAGMENTS {
        assert!(!lower.contains(frag), "leaked `{frag}` with no vault key set");
    }
}
