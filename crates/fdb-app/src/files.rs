//! File metadata request validation and deterministic type discovery.
use fdb_domain::files::{FileId, FileQuery, FileType, MatchInput, PutFile};
use fdb_ports::files::{FileError, FileMetadataStore};
use forge_identity::RlsContext;

/// Versioned, strict subset of JSON Schema; no remote references or fetching.
pub const SCHEMA_PROFILE: &str = "file-schema-v1";

/// Validate an owner-local identifier.
/// # Errors
/// Returns invalid for an empty, oversized or non-portable identifier.
pub fn identifier(value: &str) -> Result<(), FileError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
    {
        return Err(FileError::Invalid);
    }
    Ok(())
}

/// Require a tenant-bound authenticated owner, never a global CID namespace.
/// # Errors
/// Returns denied for missing subject/tenant or a role other than authenticated.
pub fn owner(rls: &RlsContext) -> Result<(String, String), FileError> {
    if rls.role != "authenticated" {
        return Err(FileError::Denied);
    }
    let tenant = rls.tenant().ok_or(FileError::Denied)?.0;
    let subject = rls.subject().ok_or(FileError::Denied)?.0;
    if tenant.is_empty() || subject.is_empty() {
        return Err(FileError::Denied);
    }
    Ok((tenant, subject))
}

fn labels(values: &[String]) -> bool {
    values.len() <= 64 && values.iter().all(|s| !s.is_empty() && s.len() <= 128)
}

/// Validate bounded record inputs. PostgreSQL enforces schema semantics atomically.
/// # Errors
/// Returns invalid on unsupported or oversized input.
pub fn validate_put(id: &FileId, value: &PutFile) -> Result<(), FileError> {
    identifier(&id.0)?;
    identifier(&value.type_id.0)?;
    identifier(&value.mutation_id)?;
    if value.cid.is_empty()
        || value.cid.len() > 256
        || value.cid.chars().any(char::is_whitespace)
        || value.path.as_ref().is_some_and(|p| p.len() > 4096)
        || value.mime_type.is_empty()
        || value.mime_type.len() > 256
        || value.type_version < 1
        || value.expected_revision < 0
        || !value.attributes.is_object()
        || !labels(&value.tags)
        || !labels(&value.categories)
        || serde_json::to_vec(value)
            .map_err(|_| FileError::Invalid)?
            .len()
            > 65_536
    {
        return Err(FileError::Invalid);
    }
    Ok(())
}

/// Validate a containment query and its keyset cursor.
/// # Errors
/// Returns invalid for unsupported shapes or an unbounded query.
pub fn validate_query(query: &FileQuery) -> Result<(), FileError> {
    if !(1..=100).contains(&query.limit)
        || !query.attributes.is_object()
        || !labels(&query.tags)
        || !labels(&query.categories)
        || serde_json::to_vec(query)
            .map_err(|_| FileError::Invalid)?
            .len()
            > 65_536
    {
        return Err(FileError::Invalid);
    }
    if let Some(id) = &query.after {
        identifier(&id.0)?;
    }
    if let Some(id) = &query.type_id {
        identifier(&id.0)?;
    }
    Ok(())
}

// Same token alphabet as registered MIME criteria, but concrete inputs never
// contain a wildcard, parameters, whitespace, an empty part or another slash.
fn concrete_mime(value: &str) -> bool {
    let Some((kind, subtype)) = value.split_once('/') else {
        return false;
    };
    let token = |part: &str| {
        !part.is_empty()
            && part
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"!#$&^_.+-".contains(&byte))
    };
    token(kind) && token(subtype)
}

/// Select at most one immutable type version; no heuristic ranking or guessing.
/// # Errors
/// Returns ambiguous for multiple matches, or a storage/request error.
pub async fn match_type(
    store: &dyn FileMetadataStore,
    rls: &RlsContext,
    input: &MatchInput,
) -> Result<Option<FileType>, FileError> {
    owner(rls)?;
    if input.mime_type.len() > 256
        || input.filename.len() > 4096
        || !concrete_mime(&input.mime_type)
    {
        return Err(FileError::Invalid);
    }
    let mime = input.mime_type.to_ascii_lowercase();
    let types = store.types(rls).await?;
    let mut selected = None;
    for value in types {
        let criteria = &value.criteria;
        if criteria.mime_types.is_empty() && criteria.filename_suffixes.is_empty() {
            continue;
        }
        let mime_matches = criteria.mime_types.is_empty()
            || criteria.mime_types.iter().any(|rule| {
                let rule = rule.to_ascii_lowercase();
                rule == mime
                    || rule
                        .strip_suffix('*')
                        .is_some_and(|prefix| mime.starts_with(prefix))
            });
        let filename_matches = criteria.filename_suffixes.is_empty()
            || criteria
                .filename_suffixes
                .iter()
                .any(|suffix| input.filename.ends_with(suffix));
        if mime_matches && filename_matches {
            if selected.is_some() {
                return Err(FileError::Ambiguous);
            }
            selected = Some(value);
        }
    }
    Ok(selected)
}
