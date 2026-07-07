# p8-c007 — Claude Code Skill Gate Tests

**Phase:** 8 — SDK Completeness
**Priority:** P2
**Depends on:** p8-c001 (export accuracy requires correct slug map)

## What this change delivers

- Automated slug accuracy gate: verifies `skills/flint-ui/catalogs/components.md` slugs match the live DB
- Updated `SKILL.md` with verified `claude plugin install` path
- `cargo test` integration test for slug accuracy (guarded by `DATABASE_URL`)

## Design

### Slug accuracy test (Rust integration test)

```rust
// crates/fdb-gateway/tests/skill_catalog_test.rs
#[tokio::test]
async fn skill_catalog_slugs_match_db() {
    let Some(db_url) = std::env::var("DATABASE_URL").ok() else { return; };
    let pool = PgPool::connect(&db_url).await.expect("pool");

    let db_slugs: Vec<String> = sqlx::query_scalar(
        "SELECT slug FROM flint_a2ui.components WHERE is_base = true ORDER BY slug"
    )
    .fetch_all(&pool).await.expect("query");

    let catalog_md = include_str!("../../../skills/flint-ui/catalogs/components.md");
    let catalog_slugs: Vec<&str> = catalog_md
        .lines()
        .filter_map(|l| l.strip_prefix("### `").and_then(|l| l.split('`').next()))
        .collect();

    for slug in &db_slugs {
        assert!(catalog_slugs.contains(&slug.as_str()),
            "DB slug '{slug}' missing from skills/flint-ui/catalogs/components.md");
    }
}
```

### `SKILL.md` install documentation

Add to `skills/flint-ui/SKILL.md`:
```markdown
## Installation

\`\`\`bash
claude plugin install flint-ui@prometheus-ags/flint-forge
\`\`\`

Or from local checkout:
\`\`\`bash
claude plugin install ./skills/flint-ui
\`\`\`
```
