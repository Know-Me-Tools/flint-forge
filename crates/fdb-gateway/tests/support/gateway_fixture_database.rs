//! Run against a new database on the configured integration server. Provider
//! stubs, role grants and background consumers stay inside that database.
use futures::FutureExt;
use sqlx::{postgres::PgConnectOptions, PgPool};
use std::{future::Future, panic::AssertUnwindSafe};

pub async fn with_database<Fut: Future<Output = ()>>(
    admin_url: &str,
    run: impl FnOnce(PgConnectOptions) -> Fut,
) {
    let admin = PgPool::connect(admin_url)
        .await
        .expect("integration server");
    let name = format!("forge_test_{}", uuid::Uuid::new_v4().simple());
    // Both DDL identifiers consist only of the fixed prefix and generated UUID hex.
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE DATABASE {name} TEMPLATE template0"
    )))
    .execute(&admin)
    .await
    .expect("create owned test database");
    let options = admin.connect_options().as_ref().clone().database(&name);
    let result = AssertUnwindSafe(run(options)).catch_unwind().await;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "DROP DATABASE {name} WITH (FORCE)"
    )))
    .execute(&admin)
    .await
    .expect("drop owned test database");
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}
