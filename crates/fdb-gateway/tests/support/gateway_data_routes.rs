use super::{json, Client, PgPool, StatusCode, Value};
use std::time::Duration;

pub(super) async fn install_extensions(pool: &PgPool) {
    // Match the public extension APIs used by the gateway. The deterministic
    // provider lives only here and makes the semantic SQL path reproducible.
    sqlx::raw_sql(
        r#"
        CREATE EXTENSION vector;
        CREATE EXTENSION "ext-flint-auth";
        CREATE EXTENSION "ext-flint-meta" CASCADE;
        CREATE EXTENSION "ext-flint-hooks" CASCADE;
        CREATE EXTENSION pg_graphql;
        GRANT USAGE ON SCHEMA graphql TO authenticated;
        COMMENT ON SCHEMA public IS '@graphql({"introspection":true})';
        CREATE SCHEMA llm;
        CREATE FUNCTION llm.embed(text,text) RETURNS vector LANGUAGE sql AS $$
            SELECT array_fill(0.1::real,ARRAY[1152])::vector
        $$;
        CREATE TABLE public.samples(id int PRIMARY KEY, embedding vector(2));
        ALTER TABLE public.samples ENABLE ROW LEVEL SECURITY;
        CREATE POLICY samples_read ON public.samples FOR SELECT TO authenticated USING(true);
        GRANT SELECT ON public.samples TO authenticated;
        INSERT INTO public.samples VALUES(1,'[1,0]'),(2,'[0,1]');
        CREATE TABLE public.notices(id text PRIMARY KEY, body text NOT NULL);
        ALTER TABLE public.notices ENABLE ROW LEVEL SECURITY;
        CREATE POLICY notices_read ON public.notices FOR SELECT TO authenticated USING(true);
        GRANT SELECT ON public.notices TO authenticated;
        INSERT INTO public.notices VALUES ('notice', 'Message from the database');
        SELECT flint_meta.full_refresh()
    "#,
    )
    .execute(pool)
    .await
    .expect("gateway extension and data fixture");
}

pub(super) async fn query_and_discover(client: &Client, pool: &PgPool) {
    let discovery = client
        .json("GET", "/mcp/v1/tools", &Value::Null, StatusCode::OK)
        .await;
    assert!(discovery["tools"]
        .as_array()
        .is_some_and(|tools| !tools.is_empty()));
    let query = client
        .json(
            "POST",
            "/graphql",
            &json!({"query":"query Info { __typename }","operationName":"Info","variables":{}}),
            StatusCode::OK,
        )
        .await;
    assert_eq!(query["data"]["__typename"], "Query", "{query}");
    let schema = client
        .json(
            "POST",
            "/graphql",
            &json!({"query":"{ __schema { queryType { name } types { name } } }","variables":{}}),
            StatusCode::OK,
        )
        .await;
    assert_eq!(schema["data"]["__schema"]["queryType"]["name"], "Query");
    assert_eq!(
        schema["data"]["__schema"]["subscriptionType"]["name"],
        "Subscription"
    );
    let vector =
        json!({"embedding":[1.0,0.0],"table":"public.samples","column":"embedding","limit":1});
    for token in [None, Some("invalid")] {
        assert_eq!(
            client
                .request("POST", "/rpc/vector", &vector, token)
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    let nearest = client
        .json("POST", "/rpc/vector", &vector, StatusCode::OK)
        .await;
    assert_eq!(nearest[0]["id"], 1);
    assert_eq!(nearest[0]["distance"], 0.0);
    let mut invalid = vector;
    invalid["column"] = json!("");
    client
        .json("POST", "/rpc/vector", &invalid, StatusCode::BAD_REQUEST)
        .await;
    // Observe the durable worker's terminal and retry transitions. This
    // database owns the queue and has no external webhook consumers.
    for retries in [0, 1, 2, 3, 4] {
        sqlx::query("INSERT INTO flint.webhook_outbox(webhook_id,payload,headers,target_type,retry_count) VALUES(gen_random_uuid(),'{}','{}','agui_run',$1)")
            .bind(retries).execute(pool).await.expect("queued fixture");
    }
    tokio::time::timeout(Duration::from_secs(15),async {
        loop {
            let states:Vec<(i32,String,bool)>=sqlx::query_as("SELECT retry_count,status,visible_at>now() FROM flint.webhook_outbox ORDER BY retry_count").fetch_all(pool).await.expect("outbox states");
            if states.iter().all(|(_,state,_)|state!="pending") {
                assert_eq!(states,vec![(1,"retrying".into(),true),(2,"retrying".into(),true),(3,"retrying".into(),true),(4,"retrying".into(),true),(5,"failed".into(),false)]);
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }).await.expect("outbox batch handled");
}

pub(super) async fn scrape_pool_metrics(client: &Client) {
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            let response = client.request("GET", "/metrics", &Value::Null, None).await;
            assert_eq!(response.status(), StatusCode::OK);
            let bytes = axum::body::to_bytes(response.into_body(), 4 * 1024 * 1024)
                .await
                .expect("metrics body");
            let samples = std::str::from_utf8(&bytes).expect("Prometheus text");
            let sample = |key: &str| {
                samples.lines().find_map(|line| {
                    line.strip_prefix(key)?
                        .strip_prefix(' ')?
                        .parse::<u32>()
                        .ok()
                })
            };
            if let (Some(open), Some(idle)) = (
                sample("sqlx_pool_connections_open"),
                sample("sqlx_pool_connections_idle"),
            ) {
                assert!(open > 0 && idle <= open);
                assert!(sample("flint_tables_rls_reportable").is_some());
                break;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    })
    .await
    .expect("pool gauges sampled");
}
