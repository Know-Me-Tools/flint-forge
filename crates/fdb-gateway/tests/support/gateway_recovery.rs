//! Recovery after a catalog or provisioning ledger becomes temporarily unavailable.
//! The mounted gateway fixture owns the database for the duration of these changes.
use super::{json, Client, PgPool, StatusCode, Uuid, Value};

pub(super) async fn provisioning(service: &Client, pool: &PgPool, spec: &Value, hash: &Value) {
    const PLAN: &str = "/schema/v1/plan";
    const APPLY: &str = "/schema/v1/apply";
    sqlx::query("ALTER TABLE flint_schema.provision_ledger RENAME TO offline_ledger")
        .execute(pool)
        .await
        .expect("take fixture ledger offline");
    for (method, route, input, message) in [
        ("POST", PLAN, spec, "plan store lookup failed"),
        ("POST", APPLY, hash, "plan store lookup failed"),
        (
            "GET",
            "/schema/v1/status",
            &Value::Null,
            "ledger query failed",
        ),
    ] {
        let body = service
            .json(method, route, input, StatusCode::INTERNAL_SERVER_ERROR)
            .await;
        assert_eq!(body, json!({"error":message}), "{route}");
    }
    sqlx::query("ALTER TABLE flint_schema.offline_ledger RENAME TO provision_ledger")
        .execute(pool)
        .await
        .expect("restore fixture ledger");
    let existing = service.json("POST", PLAN, spec, StatusCode::OK).await;
    assert_eq!(existing["planHash"], hash["planHash"]);

    let mut new_spec = spec.clone();
    new_spec["tables"][0]["name"] = json!("journal");
    // Reject new plans while permitting reads of the existing plan. Removing
    // the constraint must allow exactly one durable record on retry.
    sqlx::query("ALTER TABLE flint_schema.provision_ledger ADD CONSTRAINT fixture_writes_paused CHECK (false) NOT VALID")
        .execute(pool).await.expect("pause fixture ledger writes");
    let blocked = service
        .json("POST", PLAN, &new_spec, StatusCode::INTERNAL_SERVER_ERROR)
        .await;
    assert_eq!(blocked, json!({"error":"plan persistence failed"}));
    sqlx::query("ALTER TABLE flint_schema.provision_ledger DROP CONSTRAINT fixture_writes_paused")
        .execute(pool)
        .await
        .expect("resume fixture ledger writes");
    let planned = service.json("POST", PLAN, &new_spec, StatusCode::OK).await;
    let records: i64 =
        sqlx::query_scalar("SELECT count(*) FROM flint_schema.provision_ledger WHERE plan_hash=$1")
            .bind(planned["planHash"].as_str().expect("plan hash"))
            .fetch_one(pool)
            .await
            .expect("ledger count");
    assert_eq!(records, 1);

    // An operator created a table with a different type after the plan was reviewed.
    let namespace = spec["namespace"].as_str().expect("fixture namespace");
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE TABLE {namespace}.journal (id integer PRIMARY KEY)"
    )))
    .execute(pool)
    .await
    .expect("independent schema change");
    let stale = service
        .json(
            "POST",
            APPLY,
            &json!({"planHash":planned["planHash"]}),
            StatusCode::CONFLICT,
        )
        .await;
    assert!(stale["error"]
        .as_str()
        .is_some_and(|message| message.contains("column `id` differs")));
    service
        .json("POST", PLAN, &new_spec, StatusCode::BAD_REQUEST)
        .await;
}

pub(super) async fn catalog(client: &Client, pool: &PgPool, application: Uuid) {
    const ROUTE: &str = "/a2ui/v1/surfaces/assemble";
    let request = json!({"application_id":application,"event_type":"catalog.preview"});
    sqlx::query("ALTER TABLE flint_a2ui.assembly_rules RENAME TO offline_rules")
        .execute(pool)
        .await
        .expect("take fixture rules offline");
    let unavailable = client
        .json("POST", ROUTE, &request, StatusCode::INTERNAL_SERVER_ERROR)
        .await;
    assert_eq!(unavailable, json!({"error":"internal server error"}));
    sqlx::query("ALTER TABLE flint_a2ui.offline_rules RENAME TO assembly_rules")
        .execute(pool)
        .await
        .expect("restore fixture rules");
    let recovered = client.json("POST", ROUTE, &request, StatusCode::OK).await;
    assert!(recovered["messages"]
        .as_array()
        .is_some_and(|messages| !messages.is_empty()));

    sqlx::query("INSERT INTO flint_a2ui.assembly_rules(application_id,event_type,assembly_config) VALUES ($1,'incomplete.preview','{}')")
        .bind(application).execute(pool).await.expect("incomplete rule");
    let invalid = client
        .json(
            "POST",
            ROUTE,
            &json!({"application_id":application,"event_type":"incomplete.preview"}),
            StatusCode::BAD_REQUEST,
        )
        .await;
    assert_eq!(
        invalid,
        json!({"error":"invalid config","message":"assembly_config must contain component_slug or component"})
    );
}
