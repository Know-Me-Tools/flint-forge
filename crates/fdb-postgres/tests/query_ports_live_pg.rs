//! PostgreSQL wire types, transaction-local identity and durable retry receipts through the public adapters.
#![allow(clippy::expect_used)]
use deadpool_postgres::{Config, Pool, Runtime};
use fdb_domain::VectorRpcRequest;
use fdb_ports::{BackendError, DatabaseBackend, IdempotencyContext, SqlExecutor};
use fdb_postgres::{PgBackend, PgRest, PgVectorRpc};
use fdb_query::QueryParam;
use forge_identity::RlsContext;
use serde_json::json;
use tokio_postgres::NoTls;

fn pool() -> Option<Pool> {
    let mut config = Config::new();
    config.url = Some(std::env::var("DATABASE_URL").ok()?);
    let pool = config
        .create_pool(Some(Runtime::Tokio1), NoTls)
        .expect("test pool");
    // Keep TEMP objects scoped to the fixture's sole session.
    pool.resize(1);
    Some(pool)
}
fn caller(role: &str, subject: &str) -> RlsContext {
    RlsContext {
        role: role.into(),
        claims_json: json!({"role":role,"sub":subject,"tenant_id":"adapter-test"}).to_string(),
        raw_bearer: "test-token".into(),
        keto_subject: subject.into(),
        vault_key_id: Some("test-key".into()),
    }
}

#[tokio::test]
async fn host_queries_apply_the_caller_scope_and_encode_postgres_types_as_json() {
    let Some(pool) = pool() else { return };
    let conn = pool.get().await.expect("connection");
    let role: String = conn
        .query_one("SELECT current_user", &[])
        .await
        .expect("login")
        .get(0);
    let backend = PgBackend::from_env().expect("backend");
    let who = caller(&role, "host-reader");
    let result = backend.query_json(&who,
        "SELECT $1::text AS label,$2::integer AS count,$3::jsonb AS metadata,$4::text AS missing,
                current_setting('request.jwt.claims')::jsonb->>'sub' AS subject,
                current_setting('app.vault_key_id') AS vault_key",
        &[r#""quoted value""#.into(),"18".into(),r#"{"ready":true}"#.into(),"null".into()]).await.expect("host query");
    assert_eq!(result.len(), 1);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&result[0]).expect("JSON row"),
        json!({"label":"quoted value","count":18,"metadata":{"ready":true},"missing":null,"subject":"host-reader","vault_key":"test-key"})
    );
    assert!(matches!(
        backend
            .query_json(&who, "SELECT $1::integer", &["invalid".into()])
            .await,
        Err(BackendError::Query(_))
    ));
    assert!(matches!(
        backend
            .acquire(&caller("unconfigured-test-role", "host-reader"))
            .await,
        Err(BackendError::Query(_))
    ));
    assert!(matches!(
        backend
            .acquire(&caller("forge_nonexistent_role", "host-reader"))
            .await,
        Err(BackendError::Query(_))
    ));
}

#[tokio::test]
async fn raw_sql_bindings_project_scalar_types_and_vector_similarity_in_order() {
    let Some(pool) = pool() else { return };
    let setup = pool.get().await.expect("connection");
    let role: String = setup
        .query_one("SELECT current_user", &[])
        .await
        .expect("login")
        .get(0);
    setup
        .batch_execute(
            "CREATE TEMP TABLE adapter_vectors (id int PRIMARY KEY, embedding vector(2));
        INSERT INTO adapter_vectors VALUES (1,'[0,1]'),(2,'[1,0]')",
        )
        .await
        .expect("session vectors");
    drop(setup);
    let who = caller(&role, "reader");
    let rest = PgRest::new(pool.clone());
    let rows = rest
        .execute_raw(
            "SELECT $1::text AS title, array_length($2::text[],1) AS n, $3::jsonb AS data,
                $4::bigint AS wide, $5::text AS missing, vector_dims($6::vector) AS dims,
                true AS flag, 3::smallint AS small, 0.5::real AS single, 0.75::float8 AS double",
            vec![
                QueryParam::Text("hello".into()),
                QueryParam::TextArray(vec!["a".into(), "b".into()]),
                QueryParam::Json(r#"{"enabled":true}"#.into()),
                QueryParam::BigInt(9999),
                QueryParam::Null,
                QueryParam::Vector(vec![1., 0.]),
            ],
            &who,
        )
        .await
        .expect("raw projection");
    assert_eq!(
        json!(rows),
        json!([{"title":"hello","n":2,"data":{"enabled":true},"wide":9999,
        "missing":null,"dims":2,"flag":true,"small":3,"single":0.5,"double":0.75}])
    );

    let vector = PgVectorRpc::new(pool);
    let mut query = VectorRpcRequest {
        table: "pg_temp.adapter_vectors".into(),
        column: "embedding".into(),
        embedding: vec![1., 0.],
        limit: 1,
        filter: None,
    };
    let nearest = vector
        .execute_similarity(&query, &who)
        .await
        .expect("similarity");
    assert_eq!(nearest.as_array().expect("results").len(), 1);
    assert_eq!(nearest[0]["id"], 2);
    assert_eq!(nearest[0]["distance"], 0.0);
    query.column.clear();
    assert!(matches!(
        vector.execute_similarity(&query, &who).await,
        Err(BackendError::Query(_))
    ));
    query.table.clear();
    assert!(matches!(
        vector.execute_similarity(&query, &who).await,
        Err(BackendError::Query(_))
    ));
}

#[tokio::test]
async fn retry_receipts_replay_the_committed_result_and_roll_back_failed_mutations() {
    let Some(pool) = pool() else { return };
    let setup = pool.get().await.expect("connection");
    let subject: String = setup
        .query_one("SELECT gen_random_uuid()::text", &[])
        .await
        .expect("unique owner")
        .get(0);
    setup
        .batch_execute(
            "CREATE TEMP TABLE adapter_counters (id int PRIMARY KEY, total int NOT NULL);
        INSERT INTO adapter_counters VALUES (7,0);
        GRANT SELECT, UPDATE ON adapter_counters TO authenticated",
        )
        .await
        .expect("session counter");
    drop(setup);
    let rest = PgRest::new(pool.clone());
    let who = caller("authenticated", &subject);
    let receipt = IdempotencyContext {
        mutation_id: "increment".into(),
        request_fingerprint: "a".repeat(64),
    };
    let increment = "UPDATE pg_temp.adapter_counters SET total=total+1 WHERE id=7 RETURNING total";
    for _ in 0..2 {
        let rows = rest
            .execute_idempotent(increment, vec![], &who, &receipt)
            .await
            .expect("increment or replay");
        assert_eq!(rows[0]["total"], 1);
    }
    let conflicting = IdempotencyContext {
        request_fingerprint: "b".repeat(64),
        ..receipt.clone()
    };
    assert!(matches!(
        rest.execute_idempotent(increment, vec![], &who, &conflicting)
            .await,
        Err(BackendError::IdempotencyConflict)
    ));
    let invalid = IdempotencyContext {
        mutation_id: "invalid".into(),
        request_fingerprint: "short".into(),
    };
    assert!(matches!(
        rest.execute_idempotent(increment, vec![], &who, &invalid)
            .await,
        Err(BackendError::Query(_))
    ));
    let retry = IdempotencyContext {
        mutation_id: "retry-after-error".into(),
        ..receipt
    };
    assert!(matches!(
        rest.execute_idempotent("SELECT 1/0", vec![], &who, &retry)
            .await,
        Err(BackendError::Query(_))
    ));
    let rows = rest
        .execute_idempotent(increment, vec![], &who, &retry)
        .await
        .expect("retry after rollback");
    assert_eq!(rows[0]["total"], 2);
    assert!(matches!(
        rest.execute_raw(
            "INSERT INTO pg_temp.adapter_counters VALUES (8,1)",
            vec![],
            &who
        )
        .await,
        Err(BackendError::Denied)
    ));
    let admin = pool.get().await.expect("cleanup connection");
    admin
        .execute(
            "DELETE FROM flint.mutation_receipts WHERE subject=$1",
            &[&subject],
        )
        .await
        .expect("remove test receipts");
}
