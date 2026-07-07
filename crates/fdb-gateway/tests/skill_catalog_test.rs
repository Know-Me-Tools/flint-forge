//! Skill catalog accuracy gate — p8-c007.
//!
//! Verifies that every slug documented in `skills/flint-ui/catalogs/components.md`
//! exists in the live `flint_a2ui.components` table, and vice versa.
//!
//! **Requires a live database.** Skips cleanly when `DATABASE_URL` is not set
//! so it never blocks the standard `cargo test --workspace` run.
//!
//! Run explicitly:
//! ```sh
//! DATABASE_URL=postgres://... cargo test -p fdb-gateway skill_catalog
//! ```
#![forbid(unsafe_code)]

/// Parse `### \`<slug>\`` headings from the catalog markdown.
/// Returns the slug portion (between the backticks).
fn parse_catalog_slugs(markdown: &str) -> Vec<String> {
    markdown
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            // Match: ### `<slug>` — ComponentName
            let after_heading = trimmed.strip_prefix("### `")?;
            let slug = after_heading.split('`').next()?;
            if slug.is_empty() {
                None
            } else {
                Some(slug.to_owned())
            }
        })
        .collect()
}

#[tokio::test]
async fn skill_catalog_slugs_match_db() {
    let Some(db_url) = std::env::var("DATABASE_URL").ok() else {
        eprintln!("DATABASE_URL not set — skipping skill_catalog_slugs_match_db");
        return;
    };

    let pool = sqlx::PgPool::connect(&db_url)
        .await
        .expect("connect to DATABASE_URL");

    // Fetch all base component slugs from the live DB.
    let db_slugs: Vec<String> = sqlx::query_scalar(
        "SELECT slug FROM flint_a2ui.components WHERE is_base = true ORDER BY slug",
    )
    .fetch_all(&pool)
    .await
    .expect("query flint_a2ui.components");

    // Parse slugs from the skill catalog markdown.
    let catalog_md = include_str!("../../../skills/flint-ui/catalogs/components.md");
    let catalog_slugs = parse_catalog_slugs(catalog_md);

    // Every DB slug must appear in the catalog.
    let mut missing_from_catalog: Vec<&str> = Vec::new();
    for slug in &db_slugs {
        if !catalog_slugs.iter().any(|s| s == slug) {
            missing_from_catalog.push(slug.as_str());
        }
    }

    // Every catalog slug must exist in the DB.
    let mut missing_from_db: Vec<&str> = Vec::new();
    for slug in &catalog_slugs {
        if !db_slugs.contains(slug) {
            missing_from_db.push(slug.as_str());
        }
    }

    if !missing_from_catalog.is_empty() {
        panic!(
            "DB slugs missing from skills/flint-ui/catalogs/components.md:\n  {}",
            missing_from_catalog.join(", ")
        );
    }
    if !missing_from_db.is_empty() {
        panic!(
            "Catalog slugs not found in flint_a2ui.components:\n  {}",
            missing_from_db.join(", ")
        );
    }

    assert_eq!(
        db_slugs.len(),
        catalog_slugs.len(),
        "Slug count mismatch: DB has {}, catalog has {}",
        db_slugs.len(),
        catalog_slugs.len()
    );

    println!(
        "skill_catalog_slugs_match_db: {} slugs verified ✓",
        db_slugs.len()
    );
}

// ─── Unit tests (no DB required) ────────────────────────────────────────────

#[test]
fn parse_catalog_slugs_extracts_correct_slugs() {
    let md = r#"
## LAYOUT (8)

### `container` — Container
Top-level layout container.

### `row` — Row
Horizontal flex container.

## INPUT (2)

### `text-input` — TextInput
Single-line text field.
"#;
    let slugs = parse_catalog_slugs(md);
    assert_eq!(slugs, vec!["container", "row", "text-input"]);
}

#[test]
fn parse_catalog_slugs_skips_non_heading_lines() {
    let md = r#"
Some prose with `backticks` in it.
### Not a slug heading
### `` — empty
### `data-grid` — DataGrid
"#;
    let slugs = parse_catalog_slugs(md);
    assert_eq!(slugs, vec!["data-grid"]);
}

#[test]
fn all_55_catalog_slugs_parse_correctly() {
    let catalog_md = include_str!("../../../skills/flint-ui/catalogs/components.md");
    let slugs = parse_catalog_slugs(catalog_md);
    assert_eq!(
        slugs.len(),
        55,
        "Expected 55 slugs in catalog, found {}:\n{:?}",
        slugs.len(),
        slugs
    );
    // Spot-check a few well-known slugs
    assert!(slugs.contains(&"data-grid".to_owned()));
    assert!(slugs.contains(&"button".to_owned()));
    assert!(slugs.contains(&"container".to_owned()));
    assert!(slugs.contains(&"loading-spinner".to_owned()));
    assert!(slugs.contains(&"flint-meta-schema".to_owned()));
}
