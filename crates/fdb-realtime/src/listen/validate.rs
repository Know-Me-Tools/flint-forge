/// Validate `entity_type` (`<schema>.<table>`): each dot-segment is a SQL-safe
/// identifier (ASCII alnum/underscore, non-empty, not digit-led, ≤63 bytes). This
/// is defense-in-depth before the value is interpolated into the Keto check URL.
pub(super) fn is_safe_entity(entity: &str) -> bool {
    let mut parts = entity.split('.');
    let (Some(schema), Some(table), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    is_safe_ident_segment(schema) && is_safe_ident_segment(table)
}

/// Validate `tenant`: empty is allowed (tables without a tenant column), otherwise
/// a slug/UUID-shaped token — ASCII alnum, `_` or `-`, ≤128 bytes. Rejects the URL
/// reserved chars (`&`, `=`, `#`, `/`, whitespace) that could corrupt the Keto query.
pub(super) fn is_safe_tenant(tenant: &str) -> bool {
    tenant.is_empty()
        || (tenant.len() <= 128
            && tenant
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-'))
}

/// One `<schema>`/`<table>` identifier segment.
fn is_safe_ident_segment(seg: &str) -> bool {
    if seg.is_empty() || seg.len() > 63 {
        return false;
    }
    let mut bytes = seg.bytes();
    let first = bytes.next().unwrap_or(b'0');
    (first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// Does an event's `<schema>.<table>` equal the subscription's `entity_type`?
/// Extracted for unit testing the fan-out filter without a broadcast channel.
pub(super) fn matches(spec_entity: &str, ev_schema: &str, ev_table: &str) -> bool {
    match spec_entity.split_once('.') {
        Some((schema, table)) => schema == ev_schema && table == ev_table,
        None => false,
    }
}
