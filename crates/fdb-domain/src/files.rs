//! Private, versioned file metadata. File identity is never a global CID.
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Caller-assigned identity within a tenant and subject.
#[repr(transparent)]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct FileId(pub String);

/// Extensible file type identity within a tenant and subject.
#[repr(transparent)]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct FileTypeId(pub String);

/// Deterministic match groups; populated groups combine with AND.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchCriteria {
    /// Exact MIME or type/* alternatives, matched case-insensitively.
    #[serde(default)]
    pub mime_types: Vec<String>,
    /// Literal, case-sensitive filename suffix alternatives.
    #[serde(default)]
    pub filename_suffixes: Vec<String>,
}

/// Immutable type definition, with a strictly validated JSON Schema profile.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileType {
    /// Owner-local stable type identifier.
    pub type_id: FileTypeId,
    /// Positive immutable definition version.
    pub version: i64,
    /// Explicit supported schema vocabulary version.
    pub schema_profile: String,
    /// file-schema-v1 schema; unsupported keywords are rejected.
    pub schema: Value,
    /// Explicit supported matching criteria.
    #[serde(rename = "match", default)]
    pub criteria: MatchCriteria,
}

/// Inputs to deterministic type discovery.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchInput {
    /// Observed MIME, without parameters.
    pub mime_type: String,
    /// Descriptive filename, not an executable filesystem path.
    pub filename: String,
}

/// Full replacement of a private file instance's mutable metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PutFile {
    /// Immutable IPFS content reference, supplied by the content producer.
    pub cid: String,
    /// Optional descriptive logical path; no filesystem effects.
    pub path: Option<String>,
    /// MIME supplied by the content producer.
    pub mime_type: String,
    /// Explicit type; match discovery is never an implicit assignment.
    pub type_id: FileTypeId,
    /// Immutable schema version used to validate attributes.
    pub type_version: i64,
    /// Additional attributes validated atomically against the type schema.
    pub attributes: Value,
    /// User tags, matched with all-of semantics.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Categories, matched with all-of semantics.
    #[serde(default)]
    pub categories: Vec<String>,
    /// Zero for create, or the last observed positive revision for update.
    pub expected_revision: i64,
    /// Stable retry key; reuse with different content is rejected.
    pub mutation_id: String,
}

/// Persisted record returned without owner identifiers.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileRecord {
    /// Owner-local file instance identity.
    pub file_id: FileId,
    /// Current positive revision.
    pub revision: i64,
    /// Full attachment data at this revision.
    #[serde(flatten)]
    pub data: PutFile,
}

/// Bounded keyset discovery. All populated constraints combine with AND.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileQuery {
    /// Exact content-reference constraint.
    pub cid: Option<String>,
    /// Exact type identity constraint.
    pub type_id: Option<FileTypeId>,
    /// Required tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Required categories.
    #[serde(default)]
    pub categories: Vec<String>,
    /// JSONB containment constraint; must be an object.
    pub attributes: Value,
    /// Maximum records, from 1 through 100.
    pub limit: i64,
    /// Exclusive file identity cursor; results order by file_id.
    pub after: Option<FileId>,
}
