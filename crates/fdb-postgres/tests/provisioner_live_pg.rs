//! Live-Postgres integration tests for `PgProvisioner` (p17-c003, FFS-001 §8
//! Phase 2 gate).
//!
//! DATABASE_URL-gated: runs when `DATABASE_URL` is set (and migration 0015's
//! artifacts — `flint_schema.provision_ledger` + the `flint_provisioner`
//! role — are present), skips cleanly otherwise.
//!
//! The three assertions that carry the gate:
//! 1. **D7 regression**: after `apply`, the table is visible on a FRESH
//!    connection — i.e. the transaction really committed, it was not
//!    silently rolled back by deadpool recycling.
//! 2. **Rollback**: a failing statement mid-plan leaves no partial table and
//!    a `failed` ledger row carrying a SQLSTATE (and never the statement).
//! 3. **Privilege containment**: `CREATE` in a namespace without an operator
//!    `GRANT CREATE … TO flint_provisioner` fails at the Postgres layer
//!    (SQLSTATE 42501), regardless of what the app layer would allow.
//!
//! The tests connect with `DATABASE_URL` (an admin/owner role in CI); the
//! adapter's `SET LOCAL ROLE flint_provisioner` still runs the DDL as the
//! dedicated role, which is exactly the production shape.

#![allow(clippy::expect_used)]

use fdb_domain::provision::{ColumnSpec, ColumnType, SchemaSpec, TableSpec};
use fdb_domain::provision::{Namespace, PlanHash, PlanId, PlannedRecord, ValidatedPlan};
use fdb_ports::SchemaProvisioner;
use fdb_postgres::PgProvisioner;
use tokio_postgres::NoTls;

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok().filter(|s| !s.is_empty())
}

async fn admin(url: &str) -> tokio_postgres::Client {
    let (client, conn) = tokio_postgres::connect(url, NoTls)
        .await
        .expect("connect admin");
    tokio::spawn(async move {
        let _ = conn.await;
    });
    client
}

/// Migration-0015 artifacts present? (CI images built before 0015, or a
/// scratch DB that never ran migrations, should skip rather than fail.)
async fn prereqs_present(client: &tokio_postgres::Client) -> bool {
    let ledger: bool = client
        .query_one(
            "SELECT to_regclass('flint_schema.provision_ledger') IS NOT NULL",
            &[],
        )
        .await
        .expect("regclass query")
        .get(0);
    let role: bool = client
        .query_one(
            "SELECT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'flint_provisioner')",
            &[],
        )
        .await
        .expect("role query")
        .get(0);
    ledger && role
}

fn minimal_spec(ns: &str, table: &str) -> SchemaSpec {
    SchemaSpec {
        namespace: Namespace(ns.into()),
        tables: vec![TableSpec {
            name: table.into(),
            comment: None,
            tenant_scoped: false,
            acknowledge_unscoped: true,
            columns: vec![ColumnSpec {
                name: "id".into(),
                column_type: ColumnType::Text,
                nullable: false,
                primary_key: true,
                default: None,
            }],
            indexes: vec![],
            api_exposed: true,
        }],
    }
}

fn planned(
    ns: &str,
    table: &str,
    plan_id: &str,
    hash: &str,
    ddl: &str,
) -> (PlannedRecord, ValidatedPlan) {
    let record = PlannedRecord {
        plan_id: PlanId(plan_id.into()),
        hash: PlanHash(hash.into()),
        namespace: Namespace(ns.into()),
        spec: minimal_spec(ns, table),
        ddl: ddl.into(),
    };
    let validated = ValidatedPlan {
        plan_id: PlanId(plan_id.into()),
        hash: PlanHash(hash.into()),
        namespace: Namespace(ns.into()),
        ddl: ddl.into(),
    };
    (record, validated)
}

async fn reset_fixture(client: &tokio_postgres::Client, ns: &str, plan_ids: &[&str]) {
    client
        .batch_execute(&format!("DROP SCHEMA IF EXISTS {ns} CASCADE"))
        .await
        .expect("drop fixture schema");
    for id in plan_ids {
        client
            .execute(
                "DELETE FROM flint_schema.provision_ledger WHERE plan_id = $1",
                &[id],
            )
            .await
            .expect("clear fixture ledger row");
    }
}

/// Scenario 2 of the gate: a failing statement mid-plan leaves no partial
/// table and a `failed` ledger row carrying a SQLSTATE (never the statement).
async fn assert_failed_apply_rolls_back(
    provisioner: &PgProvisioner,
    fresh: &tokio_postgres::Client,
    ns: &str,
) {
    let (record, validated) = planned(
        ns,
        "t_fail",
        "pln_it_fail",
        "sha256:it-fail",
        &format!("CREATE TABLE {ns}.t_fail (id text PRIMARY KEY);\nSELECT 1/0;"),
    );
    provisioner
        .persist_planned(&record)
        .await
        .expect("persist planned");
    let err = provisioner
        .apply(&validated, "it-subject", None)
        .await
        .expect_err("division by zero must fail the apply");
    let msg = format!("{err}");
    assert!(
        msg.contains("SQLSTATE 22012"),
        "error must carry SQLSTATE only, got: {msg}"
    );
    assert!(
        !msg.contains("CREATE TABLE"),
        "error must never echo the statement"
    );

    let partial: bool = fresh
        .query_one(
            &format!("SELECT to_regclass('{ns}.t_fail') IS NOT NULL"),
            &[],
        )
        .await
        .expect("partial query")
        .get(0);
    assert!(!partial, "failed apply must leave no partial table");

    let (status, error_code): (String, Option<String>) = {
        let row = fresh
            .query_one(
                "SELECT status, error_code FROM flint_schema.provision_ledger WHERE plan_id = 'pln_it_fail'",
                &[],
            )
            .await
            .expect("failed ledger row");
        (row.get(0), row.get(1))
    };
    assert_eq!(status, "failed");
    assert_eq!(error_code.as_deref(), Some("22012"));
}

#[tokio::test]
async fn apply_commits_visible_on_fresh_connection_and_rolls_back_on_failure() {
    let Some(url) = database_url() else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };
    let client = admin(&url).await;
    if !prereqs_present(&client).await {
        eprintln!("skipping: migration 0015 artifacts absent");
        return;
    }

    let ns = "p17c003_it";
    reset_fixture(&client, ns, &["pln_it_ok", "pln_it_fail"]).await;
    client
        .batch_execute(&format!(
            "CREATE SCHEMA {ns}; GRANT USAGE, CREATE ON SCHEMA {ns} TO flint_provisioner;"
        ))
        .await
        .expect("create granted fixture schema");

    let provisioner = PgProvisioner::from_url(&url).expect("pool");

    // 1. D7: apply commits — table visible on a FRESH connection.
    let (record, validated) = planned(
        ns,
        "t_ok",
        "pln_it_ok",
        "sha256:it-ok",
        &format!("CREATE TABLE {ns}.t_ok (id text PRIMARY KEY);"),
    );
    provisioner
        .persist_planned(&record)
        .await
        .expect("persist planned");
    let applied = provisioner
        .apply(&validated, "it-subject", Some(1))
        .await
        .expect("apply must succeed");
    assert!(!applied.already_applied);

    let fresh = admin(&url).await;
    let visible: bool = fresh
        .query_one(&format!("SELECT to_regclass('{ns}.t_ok') IS NOT NULL"), &[])
        .await
        .expect("fresh visibility query")
        .get(0);
    assert!(
        visible,
        "D7 regression: table must exist on a fresh connection after apply"
    );

    let (status, applied_by): (String, Option<String>) = {
        let row = fresh
            .query_one(
                "SELECT status, applied_by FROM flint_schema.provision_ledger WHERE plan_id = 'pln_it_ok'",
                &[],
            )
            .await
            .expect("ledger row");
        (row.get(0), row.get(1))
    };
    assert_eq!(status, "applied");
    assert_eq!(applied_by.as_deref(), Some("it-subject"));

    // Replay by hash: second apply of the same hash is a no-op.
    let replay = provisioner
        .apply(&validated, "it-subject", Some(1))
        .await
        .expect("replay apply");
    assert!(
        replay.already_applied,
        "same-hash replay must report alreadyApplied"
    );

    assert_committed_catalog(&provisioner, &record).await;
    assert_failed_apply_rolls_back(&provisioner, &fresh, ns).await;

    reset_fixture(&client, ns, &["pln_it_ok", "pln_it_fail"]).await;
}

async fn assert_committed_catalog(provisioner: &PgProvisioner, record: &PlannedRecord) {
    let ns = &record.namespace;
    assert!(provisioner.schema_exists(ns).await.expect("namespace"));
    let tables = provisioner.introspect_namespace(ns).await.expect("catalog");
    assert_eq!(tables.len(), 1);
    assert_eq!(tables[0].name, "t_ok");
    assert_eq!(tables[0].primary_key, ["id"]);
    assert_eq!(tables[0].columns[0].sql_type, "text");
    let details = provisioner
        .table_ddl_info(ns, "t_ok")
        .await
        .expect("DDL info")
        .expect("table");
    assert_eq!(details.columns[0].name, "id");
    assert!(!details.columns[0].nullable);
    assert!(details.columns[0].pk_ordinal.is_some());
    assert!(!details.rls_enabled);
    assert!(provisioner
        .table_ddl_info(ns, "absent")
        .await
        .expect("missing table")
        .is_none());
    assert!(provisioner.persist_planned(record).await.is_err());
    let stored = provisioner
        .load_planned(&record.hash)
        .await
        .expect("lookup")
        .expect("plan");
    assert_eq!(stored.plan_id, record.plan_id);
    assert_eq!(stored.status, "applied");
    assert_eq!(stored.spec.namespace, *ns);
    assert!(provisioner
        .load_planned(&PlanHash("sha256:unknown-fixture".into()))
        .await
        .expect("absent lookup")
        .is_none());
    provisioner
        .record_version_after(&record.plan_id, 2)
        .await
        .expect("version receipt");
    assert!(provisioner
        .record_version_after(&PlanId("absent-fixture".into()), 3)
        .await
        .is_err());
    let latest = provisioner
        .last_apply()
        .await
        .expect("ledger summary")
        .expect("apply history");
    assert!(matches!(latest.status.as_str(), "applied" | "failed"));
    assert!(latest.at_epoch > 0);
}

/// Audit invariant: apply WITHOUT a persisted planned row must refuse and
/// roll the DDL back — committed DDL with no ledger record would be an
/// unauditable change (Base Rule #18).
#[tokio::test]
async fn apply_without_planned_row_is_refused_and_rolled_back() {
    let Some(url) = database_url() else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };
    let client = admin(&url).await;
    if !prereqs_present(&client).await {
        eprintln!("skipping: migration 0015 artifacts absent");
        return;
    }

    let ns = "p17c003_orphan";
    reset_fixture(&client, ns, &["pln_it_orphan_never_persisted"]).await;
    client
        .batch_execute(&format!(
            "CREATE SCHEMA {ns}; GRANT USAGE, CREATE ON SCHEMA {ns} TO flint_provisioner;"
        ))
        .await
        .expect("create granted fixture schema");

    let provisioner = PgProvisioner::from_url(&url).expect("pool");
    let (_, orphan) = planned(
        ns,
        "t_orphan",
        "pln_it_orphan_never_persisted",
        "sha256:it-orphan",
        &format!("CREATE TABLE {ns}.t_orphan (id text PRIMARY KEY);"),
    );
    let err = provisioner
        .apply(&orphan, "it-subject", None)
        .await
        .expect_err("apply without a planned ledger row must refuse");
    assert!(
        format!("{err}").contains("persist_planned"),
        "error must name the missing precondition: {err}"
    );
    let orphaned: bool = client
        .query_one(
            &format!("SELECT to_regclass('{ns}.t_orphan') IS NOT NULL"),
            &[],
        )
        .await
        .expect("orphan query")
        .get(0);
    assert!(!orphaned, "DDL must not survive without its audit record");

    // If the DDL itself fails before the missing row can be detected, the
    // caller needs both the SQLSTATE and the failure to record the outcome.
    let failed_orphan = ValidatedPlan {
        ddl: "SELECT 1/0".into(),
        ..orphan
    };
    let error = provisioner
        .apply(&failed_orphan, "it-subject", None)
        .await
        .expect_err("both the DDL and its ledger transition fail")
        .to_string();
    assert!(error.contains("SQLSTATE 22012"));
    assert!(error.contains("failed-ledger transition also did not persist"));

    reset_fixture(&client, ns, &["pln_it_orphan_never_persisted"]).await;
}

#[tokio::test]
async fn provisioner_cannot_create_outside_granted_namespaces() {
    let Some(url) = database_url() else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };
    let client = admin(&url).await;
    if !prereqs_present(&client).await {
        eprintln!("skipping: migration 0015 artifacts absent");
        return;
    }

    let ns = "p17c003_ungranted";
    reset_fixture(&client, ns, &["pln_it_deny"]).await;
    // Schema exists but flint_provisioner gets NO grant.
    client
        .batch_execute(&format!("CREATE SCHEMA {ns};"))
        .await
        .expect("create ungranted schema");

    let provisioner = PgProvisioner::from_url(&url).expect("pool");
    let (record, validated) = planned(
        ns,
        "t_deny",
        "pln_it_deny",
        "sha256:it-deny",
        &format!("CREATE TABLE {ns}.t_deny (id text PRIMARY KEY);"),
    );
    provisioner
        .persist_planned(&record)
        .await
        .expect("persist planned");
    let err = provisioner
        .apply(&validated, "it-subject", None)
        .await
        .expect_err("CREATE without a namespace grant must be refused by Postgres");
    assert!(
        format!("{err}").contains("SQLSTATE 42501"),
        "containment must be enforced at the Postgres privilege layer: {err}"
    );

    reset_fixture(&client, ns, &["pln_it_deny"]).await;
}

#[tokio::test]
async fn ordinary_role_gets_only_sqlstate_when_it_cannot_access_the_ledger() {
    let Some(url) = database_url() else { return };
    let admin = admin(&url).await;
    if !prereqs_present(&admin).await {
        return;
    }

    // Apply the restricted role at connection startup, including to the
    // ledger pre-check that runs before any DDL transaction is opened.
    let restricted_url = if url.starts_with("postgres://") || url.starts_with("postgresql://") {
        let separator = if url.contains('?') { '&' } else { '?' };
        format!("{url}{separator}options=-crole%3Dauthenticated")
    } else {
        format!("{url} options=-crole=authenticated")
    };
    let provisioner = PgProvisioner::from_url(&restricted_url).expect("restricted pool");
    let (record, plan) = planned(
        "ordinary_role",
        "example",
        "fixture_role_denied",
        "sha256:fixture-denied",
        "SELECT 1",
    );
    for result in [
        provisioner.persist_planned(&record).await,
        provisioner.load_planned(&record.hash).await.map(|_| ()),
        provisioner
            .apply(&plan, "test-operator", None)
            .await
            .map(|_| ()),
        provisioner.record_version_after(&record.plan_id, 1).await,
        provisioner.last_apply().await.map(|_| ()),
    ] {
        let error = result
            .expect_err("ordinary role has no ledger grant")
            .to_string();
        assert!(error.contains("SQLSTATE 42501"), "{error}");
        assert!(
            !error.contains("provision_ledger"),
            "database details escaped: {error}"
        );
    }
}
