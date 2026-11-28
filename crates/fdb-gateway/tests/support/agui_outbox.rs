use super::*;
use sqlx::postgres::PgPoolOptions;

#[path = "gateway_fixture_database.rs"]
mod fixture;

#[tokio::test]
async fn undeliverable_hooks_back_off_then_stop_retrying() {
    let Ok(admin_url) = std::env::var("DATABASE_URL") else {
        return;
    };
    fixture::with_database(&admin_url, |options| async move {
        let pool = PgPoolOptions::new().connect_with(options).await.unwrap();
        sqlx::query("CREATE EXTENSION \"ext-flint-hooks\" CASCADE")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO flint.webhook_outbox (webhook_id, payload, headers, target_type, retry_count)
             SELECT gen_random_uuid(), '{}'::jsonb, '{}'::jsonb, 'agui_run', n
             FROM generate_series(0, 4) AS n",
        )
        .execute(&pool)
        .await
        .unwrap();
        let agui = AgUiState::new(4);
        process_batch(&pool, &agui).await.unwrap();
        let attempts: Vec<(String, i32, i32)> = sqlx::query_as(
            "SELECT status, retry_count,
              round(EXTRACT(EPOCH FROM (visible_at - updated_at)))::int
             FROM flint.webhook_outbox ORDER BY retry_count",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(attempts.len(), 5);
        let schedule = [(1, 30), (2, 60), (3, 120), (4, 300)];
        for ((status, count, delay), expected) in attempts[..4].iter().zip(schedule) {
            assert_eq!(status, "retrying");
            assert_eq!((*count, *delay), expected);
        }
        assert_eq!(attempts[4].0, "failed");
        assert_eq!(attempts[4].1, 5);
        process_batch(&pool, &agui).await.unwrap();
        let sum: i64 = sqlx::query_scalar("SELECT SUM(retry_count) FROM flint.webhook_outbox")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(sum, 15);
        pool.close().await;
    }).await;
}
