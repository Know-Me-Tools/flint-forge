/// Integration gate tests for p5-c005: application model, role hierarchy,
/// and permission-filtered component resolution.
///
/// Requires a live Postgres 18 with migrations 0001-0006 applied and seeds run.
/// Set DATABASE_URL before running:
///     DATABASE_URL=... cargo test --test a2ui_application_model_test
/// Tests skip gracefully when DATABASE_URL is unset.
///
/// # Idempotency
///
/// These tests write fixed slugs and user ids into shared tables, so they must
/// leave the database exactly as they found it or a second run fails on a
/// duplicate key. Three separate defects broke that:
///
/// 1. The `components` and `role_assignments` inserts had no `ON CONFLICT`
///    clause, unlike the `applications`/`roles` inserts beside them, so a
///    re-run hit `23505` immediately.
/// 2. Cleanup ran only on the success path — any failing assertion left the
///    fixtures behind, so one failure poisoned every subsequent run.
/// 3. **The cleanup never worked even on success.** It passed several
///    statements separated by `;` to a single `sqlx::query(...).bind(...)`,
///    which uses Postgres's extended protocol — that rejects multi-statement
///    SQL with `ERROR: there is no parameter $1`, and the error was discarded
///    by `let _ =`. Verified against a live Postgres 18: zero rows deleted.
///
/// The fix is `purge_fixture`, called both before and after each test, running
/// one statement per call. Cleaning up *first* is what makes a run recover from
/// a previously-poisoned database instead of requiring manual intervention.
use sqlx::PgPool;
use uuid::Uuid;

async fn connect() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPool::connect(&url).await.ok()
}

/// Delete every row this file's fixtures create, in foreign-key order.
///
/// `role_assignments` → `roles` → `components` → `applications`: the reverse of
/// creation order. Doing it in any other order raises
/// `violates foreign key constraint`, which is exactly what happens when you
/// try to clear these by hand.
///
/// Idempotent and safe on an already-clean database, so it can run at the start
/// of a test (to recover from an earlier failed run) as well as at the end.
/// Each statement is issued separately — see the module docs for why bundling
/// them silently did nothing.
async fn purge_fixture(pool: &PgPool, app_slug: &str) {
    let app_ids: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM flint_a2ui.applications WHERE slug = $1")
            .bind(app_slug)
            .fetch_all(pool)
            .await
            .expect("look up fixture application");

    for app_id in app_ids {
        sqlx::query("DELETE FROM flint_a2ui.role_assignments WHERE application_id = $1")
            .bind(app_id)
            .execute(pool)
            .await
            .expect("purge role_assignments");
        sqlx::query("DELETE FROM flint_a2ui.roles WHERE application_id = $1")
            .bind(app_id)
            .execute(pool)
            .await
            .expect("purge roles");
        sqlx::query("DELETE FROM flint_a2ui.components WHERE application_id = $1")
            .bind(app_id)
            .execute(pool)
            .await
            .expect("purge components");
        sqlx::query("DELETE FROM flint_a2ui.applications WHERE id = $1")
            .bind(app_id)
            .execute(pool)
            .await
            .expect("purge application");
    }
}

#[tokio::test]
async fn test_resolve_components_returns_base_for_anonymous_user() {
    let Some(pool) = connect().await else { return };

    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT slug FROM flint_a2ui.resolve_components(NULL, '{}'::jsonb)")
            .fetch_all(&pool)
            .await
            .expect("resolve_components query failed");

    let slugs: Vec<_> = rows.into_iter().map(|r| r.0).collect();
    assert!(
        slugs.contains(&"data-grid".to_string()),
        "anonymous user should see base component 'data-grid'"
    );
    assert!(
        slugs.contains(&"button".to_string()),
        "anonymous user should see base component 'button'"
    );
}

#[tokio::test]
async fn test_resolve_components_app_specific_requires_role() {
    let Some(pool) = connect().await else { return };

    // Clear anything a previous (possibly failed) run left behind, so this test
    // does not inherit a poisoned database.
    purge_fixture(&pool, "p5c005-test-app").await;

    let app_id: Uuid = sqlx::query_scalar(
        "INSERT INTO flint_a2ui.applications (slug, name)
         VALUES ('p5c005-test-app', 'p5-c005 test app')
         ON CONFLICT (slug) DO UPDATE SET name = EXCLUDED.name
         RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("insert test application failed");

    // Create an app-specific component. `ON CONFLICT` mirrors the applications
    // and roles inserts around it; without it a re-run failed on the
    // `components_slug_key` unique constraint.
    sqlx::query(
        "INSERT INTO flint_a2ui.components
             (slug, category, primitive_type, schema, is_base, application_id, description)
         VALUES ('p5c005-custom-widget', 'feedback', 'CustomWidget',
                 '{\"type\":\"object\"}'::jsonb, false, $1, 'app-specific widget')
         ON CONFLICT (slug) DO UPDATE SET application_id = EXCLUDED.application_id",
    )
    .bind(app_id)
    .execute(&pool)
    .await
    .expect("insert app-specific component failed");

    // User WITHOUT a role assignment should NOT see the app-specific component.
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT slug FROM flint_a2ui.resolve_components($1, '{\"flint\":{\"user_id\":\"unauthorized-user\"}}'::jsonb)",
    )
    .bind(app_id)
    .fetch_all(&pool)
    .await
    .expect("resolve_components query failed");

    let slugs: Vec<_> = rows.into_iter().map(|r| r.0).collect();
    assert!(
        slugs.contains(&"data-grid".to_string()),
        "unauthorized user should still see base components"
    );
    assert!(
        !slugs.contains(&"p5c005-custom-widget".to_string()),
        "unauthorized user must NOT see app-specific components"
    );

    // Assign a role to the user.
    let role_id: Uuid = sqlx::query_scalar(
        "INSERT INTO flint_a2ui.roles (application_id, slug, name)
         VALUES ($1, 'viewer', 'Viewer')
         ON CONFLICT (application_id, slug) DO UPDATE SET name = EXCLUDED.name
         RETURNING id",
    )
    .bind(app_id)
    .fetch_one(&pool)
    .await
    .expect("insert role failed");

    sqlx::query(
        "INSERT INTO flint_a2ui.role_assignments (application_id, role_id, user_id)
         VALUES ($1, $2, 'authorized-user')
         ON CONFLICT (application_id, role_id, user_id) DO NOTHING",
    )
    .bind(app_id)
    .bind(role_id)
    .execute(&pool)
    .await
    .expect("insert role assignment failed");

    // Authorized user SHOULD see the app-specific component.
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT slug FROM flint_a2ui.resolve_components($1, '{\"flint\":{\"user_id\":\"authorized-user\"}}'::jsonb)",
    )
    .bind(app_id)
    .fetch_all(&pool)
    .await
    .expect("resolve_components query failed");

    let slugs: Vec<_> = rows.into_iter().map(|r| r.0).collect();
    assert!(
        slugs.contains(&"p5c005-custom-widget".to_string()),
        "authorized user should see app-specific component"
    );

    purge_fixture(&pool, "p5c005-test-app").await;
}

#[tokio::test]
async fn test_role_hierarchy_inheritance() {
    let Some(pool) = connect().await else { return };

    purge_fixture(&pool, "p5c005-hierarchy-app").await;

    let app_id: Uuid = sqlx::query_scalar(
        "INSERT INTO flint_a2ui.applications (slug, name)
         VALUES ('p5c005-hierarchy-app', 'p5-c005 hierarchy app')
         ON CONFLICT (slug) DO UPDATE SET name = EXCLUDED.name
         RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("insert test application failed");

    let parent_role_id: Uuid = sqlx::query_scalar(
        "INSERT INTO flint_a2ui.roles (application_id, slug, name)
         VALUES ($1, 'admin', 'Admin')
         ON CONFLICT (application_id, slug) DO UPDATE SET name = EXCLUDED.name
         RETURNING id",
    )
    .bind(app_id)
    .fetch_one(&pool)
    .await
    .expect("insert parent role failed");

    let _child_role_id: Uuid = sqlx::query_scalar(
        "INSERT INTO flint_a2ui.roles (application_id, slug, name, parent_role_id)
         VALUES ($1, 'editor', 'Editor', $2)
         ON CONFLICT (application_id, slug) DO UPDATE SET name = EXCLUDED.name
         RETURNING id",
    )
    .bind(app_id)
    .bind(parent_role_id)
    .fetch_one(&pool)
    .await
    .expect("insert child role failed");

    // Assign user to the parent role. resolve_application_roles returns the
    // assigned role plus all descendant roles, so the user should be resolved
    // as both 'admin' and 'editor'.
    sqlx::query(
        "INSERT INTO flint_a2ui.role_assignments (application_id, role_id, user_id)
         VALUES ($1, $2, 'hierarchy-user')
         ON CONFLICT (application_id, role_id, user_id) DO NOTHING",
    )
    .bind(app_id)
    .bind(parent_role_id)
    .execute(&pool)
    .await
    .expect("insert role assignment failed");

    let roles: Vec<(String,)> = sqlx::query_as(
        "SELECT slug FROM flint_a2ui.resolve_application_roles($1, '{\"flint\":{\"user_id\":\"hierarchy-user\"}}'::jsonb)",
    )
    .bind(app_id)
    .fetch_all(&pool)
    .await
    .expect("resolve_application_roles query failed");

    let slugs: Vec<_> = roles.into_iter().map(|r| r.0).collect();
    assert!(
        slugs.contains(&"admin".to_string()),
        "user should have direct assigned role 'admin'"
    );
    assert!(
        slugs.contains(&"editor".to_string()),
        "user should inherit descendant role 'editor'"
    );

    purge_fixture(&pool, "p5c005-hierarchy-app").await;
}
