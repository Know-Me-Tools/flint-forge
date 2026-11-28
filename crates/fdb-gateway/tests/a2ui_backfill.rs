//! Integration test against the isolated repair database, never a live catalog.
use fdb_gateway::a2ui_embedder;
#[path = "support/gateway_fixture_database.rs"]
mod fixture;
use sqlx::PgPool;
use std::{sync::Arc, time::Duration};

#[tokio::test]
async fn replicas_coordinate_backfill_and_recover_notifications() {
    std::env::set_var("FLINT_A2UI_EMBED_MODEL", "tongyi-embedding-vision-plus");
    std::env::set_var("FLINT_A2UI_EMBED_DIMENSIONS", "1152");
    if let Ok(url) = std::env::var("A2UI_TEST_DATABASE_URL") {
        let pool = PgPool::connect(&url).await.expect("repair database");
        let db: String = sqlx::query_scalar("SELECT current_database()")
            .fetch_one(&pool)
            .await
            .expect("database name");
        assert_eq!(
            db, "flint_a2ui_repair_test",
            "isolated repair database required"
        );
        check_replica_recovery(pool).await;
        return;
    }
    let Ok(url) = std::env::var("DATABASE_URL") else {
        return;
    };
    // The mock provider and its counter must never replace objects in the CI
    // database. Every invocation owns a new database and drops it on failure.
    fixture::with_database(&url, |options| async move {
        let pool = PgPool::connect_with(options).await.expect("embedding database");
        let mut transaction = pool.begin().await.expect("begin fixture installation");
        for sql in [
            include_str!("../../../migrations/0002_flint_a2ui.sql"),
            include_str!("../../../migrations/0017_a2ui_embedding_invalidation.sql"),
            include_str!("../../../migrations/0018_a2ui_dashscope_dimensions.sql"),
            "CREATE SCHEMA llm; INSERT INTO flint_a2ui.components(slug,category,primitive_type,schema,description) VALUES ('sample','input','Text','{}','Example text'),('second','input','Text','{}','Second example');",
        ] {
            sqlx::raw_sql(sql).execute(&mut *transaction).await.expect("install embedding schema");
        }
        transaction.commit().await.expect("commit fixture installation");
        check_provider_recovery(&pool).await;
        check_replica_recovery(pool).await;
    }).await;
}

async fn check_provider_recovery(pool: &PgPool) {
    sqlx::raw_sql("CREATE FUNCTION llm.embed(input text,model text DEFAULT 'default') RETURNS vector LANGUAGE sql AS $$ SELECT '[1]'::vector $$")
        .execute(pool).await.expect("incompatible provider fixture");
    let error = a2ui_embedder::backfill_missing(pool)
        .await
        .expect_err("invalid vector rejected");
    assert!(error.to_string().contains("embedding dimension mismatch"));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM flint_a2ui.embeddings")
        .fetch_one(pool)
        .await
        .expect("persisted count");
    assert_eq!(count, 0);
    assert!(a2ui_embedder::query_embedding(pool, "Search")
        .await
        .is_none());

    // Sequence values survive a failed transaction, as requests to an external
    // provider do. The first two attempts simulate temporary rate limiting.
    sqlx::raw_sql("CREATE SEQUENCE llm.attempts;
        CREATE OR REPLACE FUNCTION llm.embed(input text, model text DEFAULT 'default') RETURNS vector
        LANGUAGE plpgsql AS $$ BEGIN
          IF nextval('llm.attempts') < 3 THEN RAISE EXCEPTION 'gateway error 429'; END IF;
          RETURN array_fill(0.1::real,ARRAY[1152])::vector;
        END $$")
        .execute(pool).await.expect("transient provider fixture");
    a2ui_embedder::backfill_missing(pool)
        .await
        .expect("retry accepted vectors");
    let (attempts, embeddings): (i64, i64) = sqlx::query_as(
        "SELECT last_value, (SELECT count(*) FROM flint_a2ui.embeddings) FROM llm.attempts",
    )
    .fetch_one(pool)
    .await
    .expect("retry accounting");
    assert_eq!(embeddings, 2);
    assert_eq!(attempts, embeddings + 2);
}

async fn check_replica_recovery(pool: PgPool) {
    sqlx::raw_sql(
        "CREATE TABLE IF NOT EXISTS public.embedding_calls(id bigserial PRIMARY KEY);
        CREATE OR REPLACE FUNCTION llm.embed(input text,model text DEFAULT 'default') RETURNS vector
        LANGUAGE plpgsql VOLATILE AS $$ BEGIN
          INSERT INTO public.embedding_calls DEFAULT VALUES;
          RETURN array_fill(0.1::real,ARRAY[1152])::vector;
        END $$;
        TRUNCATE public.embedding_calls;
        DELETE FROM flint_a2ui.embeddings;",
    )
    .execute(&pool)
    .await
    .unwrap();
    let expected: i64 = sqlx::query_scalar("SELECT count(*) FROM flint_a2ui.components")
        .fetch_one(&pool)
        .await
        .unwrap();
    let (a, b) = tokio::join!(
        a2ui_embedder::backfill_missing(&pool),
        a2ui_embedder::backfill_missing(&pool)
    );
    a.unwrap();
    b.unwrap();
    let calls: i64 = sqlx::query_scalar("SELECT count(*) FROM public.embedding_calls")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        calls, expected,
        "replicas must not duplicate backfill requests"
    );
    let invalid: i64=sqlx::query_scalar("SELECT count(*) FROM flint_a2ui.embeddings WHERE vector_dims(embedding)<>1152 OR model<>'tongyi-embedding-vision-plus'").fetch_one(&pool).await.unwrap();
    assert_eq!(invalid, 0);
    let shared = Arc::new(pool.clone());
    let listener_a = a2ui_embedder::spawn(shared.clone());
    let listener_b = a2ui_embedder::spawn(shared);
    let component: uuid::Uuid =
        sqlx::query_scalar("SELECT id FROM flint_a2ui.components ORDER BY id LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap();
    // A no-op update exercises the durable invalidation trigger without changing data.
    sqlx::query("UPDATE flint_a2ui.components SET description=description WHERE id=$1")
        .bind(component)
        .execute(&pool)
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(35), async {
        loop {
            let found: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM flint_a2ui.embeddings WHERE component_id=$1)",
            )
            .bind(component)
            .fetch_one(&pool)
            .await
            .unwrap();
            if found {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .unwrap();
    listener_a.abort();
    listener_b.abort();
    let _ = tokio::join!(listener_a, listener_b);
    let calls: i64 = sqlx::query_scalar("SELECT count(*) FROM public.embedding_calls")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        calls,
        expected + 1,
        "notifications must not duplicate paid embedding requests"
    );
    sqlx::raw_sql("DELETE FROM flint_a2ui.embeddings; DROP FUNCTION llm.embed(text,text); DROP TABLE public.embedding_calls;").execute(&pool).await.unwrap();
}
