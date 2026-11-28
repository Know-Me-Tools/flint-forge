//! Application-specific rules, wire messages, and publication in a request transaction.
use std::sync::Arc;

use fdb_reflection::compilers::a2ui::{
    A2uiAssembler, A2uiPublisher, AssemblerError, AssemblyContext,
};
use serde_json::{json, Value};
use tokio::sync::mpsc;
use uuid::Uuid;

struct Publisher(mpsc::UnboundedSender<(String, Vec<u8>)>);

#[async_trait::async_trait]
impl A2uiPublisher for Publisher {
    async fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), AssemblerError> {
        self.0
            .send((topic.into(), payload.to_vec()))
            .expect("receiver alive");
        Ok(())
    }
}

#[tokio::test]
async fn assembles_and_publishes_the_first_matching_rule() -> Result<(), Box<dyn std::error::Error>>
{
    let Ok(url) = std::env::var("DATABASE_URL") else {
        return Ok(());
    };
    let pool = sqlx::PgPool::connect(&url).await?;
    let mut tx = pool.begin().await?;
    let app_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let slug = format!("card-{app_id}");
    sqlx::query("INSERT INTO flint_a2ui.applications (id, slug, name, owner_id) VALUES ($1, $2, 'Orders', $3)")
        .bind(app_id)
        .bind(app_id.to_string())
        .bind(owner_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO flint_a2ui.components (slug, category, primitive_type, schema, is_base)
                 VALUES ($1, 'data-display', 'OrderCard', '{}', true)",
    )
    .bind(&slug)
    .execute(&mut *tx)
    .await?;

    for (event_type, priority, filter, config) in [
        (
            "order",
            1,
            json!({"order.status": "shipped"}),
            json!({"component": "unused"}),
        ),
        (
            "order",
            2,
            json!({"order.status": "paid", "order.hold": null}),
            json!({"component_slug": slug, "props": {"label": "Paid"}}),
        ),
        ("order", 3, json!({}), json!({"component": "unused"})),
        ("incomplete", 1, json!({}), json!({})),
        (
            "unknown_component",
            1,
            json!({}),
            json!({"component": "no-such-rule-component"}),
        ),
    ] {
        sqlx::query(
            "INSERT INTO flint_a2ui.assembly_rules
                     (application_id, event_type, priority, event_filter, assembly_config)
                     VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(app_id)
        .bind(event_type)
        .bind(priority)
        .bind(filter)
        .bind(config)
        .execute(&mut *tx)
        .await?;
    }
    let (sender, mut receiver) = mpsc::unbounded_channel();
    let assembler = A2uiAssembler::new(pool).with_publisher(Arc::new(Publisher(sender)));
    let surface_id = Uuid::new_v4();
    let actions = json!([{"name": "receipt"}]);
    let mut ctx = AssemblyContext {
        event_type: "order".into(),
        event_payload: json!({"catalog_id": "orders-v1", "order": {"status": "paid"}, "actions": actions}),
        application_id: Some(app_id),
        jwt_claims: json!({"sub":owner_id}),
        surface_id: Some(surface_id),
    };
    sqlx::query("SELECT set_config('role', 'authenticated', true),
                        set_config('request.jwt.claims', $1, true), set_config('app.jwt_claims', $1, true)")
        .bind(ctx.jwt_claims.to_string()).execute(&mut *tx).await?;
    let surface = assembler.assemble_on(&mut tx, &ctx).await?;
    let wire = surface.to_json();
    assert_eq!(wire["surfaceId"], surface_id.to_string());
    assert_eq!(wire["catalogId"], "orders-v1");
    let messages = wire["messages"].as_array().expect("message array");
    assert_eq!(
        messages
            .iter()
            .map(|m| m["op"].as_str().expect("operation"))
            .collect::<Vec<_>>(),
        [
            "createSurface",
            "updateComponents",
            "updateDataModel",
            "updateActions"
        ]
    );
    assert_eq!(
        messages[1]["components"][0],
        json!({"id":"main","component":"OrderCard","props":{"label":"Paid"}})
    );
    assert_eq!(messages[2]["value"], ctx.event_payload);
    assert_eq!(messages[3]["actions"], actions);
    let (topic, bytes) = receiver.try_recv()?;
    assert_eq!(topic, "a2ui.surfaces");
    assert_eq!(
        serde_json::from_slice::<Value>(&bytes)?,
        serde_json::to_value(&surface)?
    );

    for (event, reason) in [
        ("incomplete", "component_slug"),
        ("unknown_component", "component not found"),
    ] {
        ctx.event_type = event.into();
        assert!(matches!(assembler.assemble_on(&mut tx, &ctx).await,
            Err(AssemblerError::InvalidConfig(message)) if message.contains(reason)));
    }
    ctx.event_type = "no-rule".into();
    assert!(matches!(assembler.assemble_on(&mut tx, &ctx).await,
        Err(AssemblerError::MissingField(field)) if field == "data_source"));
    ctx.event_payload = json!({"data_source": {}});
    assert!(matches!(assembler.assemble_on(&mut tx, &ctx).await,
        Err(AssemblerError::MissingField(field)) if field == "data_source.table"));
    assert!(matches!(
        receiver.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
    tx.rollback().await?;
    Ok(())
}
