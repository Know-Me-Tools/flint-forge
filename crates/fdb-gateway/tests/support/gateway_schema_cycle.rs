use super::{json, Client, PgPool, StatusCode, Value};

pub(super) async fn provision(service: &Client, pool: &PgPool, namespace: &str) {
    // The caller constructs the namespace solely from a fresh UUID.
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "CREATE SCHEMA {namespace}; GRANT USAGE, CREATE ON SCHEMA {namespace} TO flint_provisioner"
    )))
    .execute(pool)
    .await
    .expect("prepare owned schema");
    let spec = json!({"namespace":namespace,"tables":[{"name":"notes","tenantScoped":true,"columns":[{"name":"id","type":"text","nullable":false,"primaryKey":true}]}]});
    let plan = service
        .json("POST", "/schema/v1/plan", &spec, StatusCode::OK)
        .await;
    let hash = json!({"planHash":plan["planHash"]});
    assert_eq!(plan["noop"], false);
    let repeat = service
        .json("POST", "/schema/v1/plan", &spec, StatusCode::OK)
        .await;
    assert_eq!(repeat["planId"], plan["planId"]);
    for already_applied in [false, true] {
        let result = service
            .json("POST", "/schema/v1/apply", &hash, StatusCode::OK)
            .await;
        assert_eq!(result["alreadyApplied"], already_applied);
        assert_eq!(result["applied"], true);
    }
    let ddl = service
        .json(
            "GET",
            &format!("/schema/v1/tables/{namespace}/notes/ddl"),
            &Value::Null,
            StatusCode::OK,
        )
        .await;
    assert_eq!(ddl["rlsForced"], true);
    assert!(ddl["ddl"].as_str().expect("DDL").contains("CREATE TABLE"));
    let status = service
        .json("GET", "/schema/v1/status", &Value::Null, StatusCode::OK)
        .await;
    assert_eq!(status["enabled"], true);
    assert_eq!(status["namespaces"], json!([namespace]));
    let mut disallowed = spec.clone();
    for ns in ["public", "unlisted_schema"] {
        disallowed["namespace"] = json!(ns);
        service
            .json(
                "POST",
                "/schema/v1/plan",
                &disallowed,
                StatusCode::FORBIDDEN,
            )
            .await;
    }
    let unknown = service
        .json(
            "GET",
            &format!("/schema/v1/tables/{namespace}/missing/ddl"),
            &Value::Null,
            StatusCode::NOT_FOUND,
        )
        .await;
    assert_eq!(unknown["error"], "unknown table");
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "DROP SCHEMA {namespace} CASCADE"
    )))
    .execute(pool)
    .await
    .expect("remove owned schema");
    sqlx::query("DELETE FROM flint_schema.provision_ledger WHERE namespace=$1")
        .bind(namespace)
        .execute(pool)
        .await
        .expect("remove fixture ledger");
}
