//! Integration test against the isolated repair database, never a live catalog.
use fdb_gateway::a2ui_embedder;
use sqlx::PgPool;
use std::{sync::Arc, time::Duration};

#[tokio::test]
async fn replicas_coordinate_backfill_and_recover_notifications() {
    let Ok(url) = std::env::var("A2UI_TEST_DATABASE_URL") else {
        return;
    };
    let pool = PgPool::connect(&url).await.unwrap();
    let database: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        database, "flint_a2ui_repair_test",
        "isolated database required"
    );
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
