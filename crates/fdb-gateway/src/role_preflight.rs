//! Checks that the connecting role owns every extension the gateway's own migrations depend on.
//!
//! See `docs/contracts/database-roles.md` for the full contract this enforces. In short: migration
//! `0003_a2ui_triggers.sql` runs `CREATE TRIGGER ... ON flint_meta.cache_tables`, and `CREATE
//! TRIGGER` requires ownership of the target table — no `GRANT` substitutes, not even `GRANT ALL`.
//! `cache_tables` belongs to the `ext-flint-meta` pgrx extension, so the connecting role must be the
//! role that ran `CREATE EXTENSION` for it. Any other role fails migration 3 with a bare `permission
//! denied for schema flint_meta` that points nowhere near the actual cause.

use sqlx::PgPool;

/// Extensions the gateway's own migrations touch and therefore must own. Kept in sync by hand with
/// `01-extensions.sql` / `02-grants.sql` rather than queried dynamically: a dynamic "every extension
/// in this database" check would also flag extensions the gateway never modifies (e.g. `pg_graphql`,
/// `pgcrypto`), which the connecting role has no need to own.
const REQUIRED_EXTENSIONS: &[&str] = &["ext-flint-meta", "ext-flint-auth", "flint_vault", "flint_llm"];

/// The first required extension not owned by the current connection's role, if any, as
/// `(extension_name, owning_role_name)`.
///
/// Returns `Ok(None)` when every required extension is either absent (nothing to own) or owned by
/// the connecting role. Absence is not this check's concern — a missing extension fails later, in
/// the migration itself, with its own clear error.
pub(crate) async fn mismatched_extension_owner(
    pool: &PgPool,
) -> Result<Option<(String, String)>, sqlx::Error> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        r"
        SELECT e.extname, r.rolname
        FROM pg_extension e
        JOIN pg_roles r ON r.oid = e.extowner
        WHERE e.extname = ANY($1)
          AND r.rolname <> current_user
        ",
    )
    .bind(REQUIRED_EXTENSIONS)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().next())
}

/// Checks role ownership and returns the ready-to-propagate startup error if it fails, or `Ok(())`
/// if the connecting role owns every required extension. Kept here rather than inline at the call
/// site so `bootstrap.rs` — already the workspace's largest composition root — carries only the
/// check, not the message.
pub(crate) async fn check(pool: &PgPool) -> anyhow::Result<()> {
    if let Some((extname, owner)) = mismatched_extension_owner(pool).await? {
        anyhow::bail!(
            "DATABASE_URL connects as a role that does not own the \"{extname}\" extension \
             (owned by \"{owner}\"). fdb-gateway's own migrations create objects inside \
             extension-owned schemas (see migrations/0003_a2ui_triggers.sql) and that requires \
             table ownership, which no GRANT confers. Connect as \"{owner}\" — the role that ran \
             CREATE EXTENSION, typically POSTGRES_USER on this image's first boot — or grant a \
             separate, least-privilege role for your own application schema in addition to this \
             one. See docs/contracts/database-roles.md."
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `REQUIRED_EXTENSIONS` names must exist verbatim (including the `ext-` hyphenated ones), or
    /// the `= ANY($1)` comparison silently matches nothing and this check becomes a no-op that never
    /// fires — worse than absent, because it looks like protection.
    #[test]
    fn required_extensions_match_the_hyphenated_pgrx_names() {
        assert!(REQUIRED_EXTENSIONS.contains(&"ext-flint-meta"));
        assert!(REQUIRED_EXTENSIONS.contains(&"ext-flint-auth"));
        assert_eq!(REQUIRED_EXTENSIONS.len(), 4);
    }
}
