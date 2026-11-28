use super::{attachment, identity, issue40_fixture::Issue40Fixture};
use fdb_domain::files::FileId;
use std::error::Error;
use tokio_postgres::error::SqlState;
use uuid::Uuid;

const INSERT: &str = "
    INSERT INTO flint_files.records(tenant_id, subject, file_id, revision, type_id, type_version, data)
    VALUES ($1, $2, $3, 1, $4, $5, $6)";
const READ: &str = "
    SELECT revision FROM flint_files.records
    WHERE tenant_id = $1 AND subject = $2 AND file_id = $3";
const UPDATE: &str = "
    UPDATE flint_files.records
    SET data = data || jsonb_build_object('expected_revision', revision, 'mutation_id', $4::text),
        revision = revision + 1
    WHERE tenant_id = $1 AND subject = $2 AND file_id = $3";

type CheckResult = Result<(), Box<dyn Error>>;

/// Exercise both independently so that a masked insert on one axis cannot hide the other.
pub(super) async fn check(
    fixture: &Issue40Fixture,
    tenant: &str,
    subject: &str,
    other_tenant: &str,
    other_subject: &str,
    original: &FileId,
) -> CheckResult {
    if tenant == other_tenant || subject == other_subject {
        return Err("FILE-3 requires distinct tenant and subject controls".into());
    }
    let owner = (tenant, subject);
    let mut failures = Vec::new();
    for (axis, caller) in [
        ("tenant", (other_tenant, subject)),
        ("subject", (tenant, other_subject)),
    ] {
        if let Err(err) = check_axis(fixture, owner, caller, original).await {
            failures.push(format!("{axis}: {err}"));
        }
    }
    if failures.is_empty() {
        return Ok(());
    }
    Err(format!(
        "FILE-3 direct SQL controls failed ({})",
        failures.join(", ")
    )
    .into())
}

async fn check_axis(
    fixture: &Issue40Fixture,
    owner: (&str, &str),
    caller: (&str, &str),
    original: &FileId,
) -> CheckResult {
    let (tenant, subject) = caller;
    let (original_tenant, original_subject) = owner;
    let mut client = fixture
        .pg
        .get()
        .await
        .map_err(|_| "fixture pool unavailable")?;
    let tx = client
        .transaction()
        .await
        .map_err(|_| "could not begin SQL control")?;
    tx.batch_execute("SET LOCAL ROLE authenticated")
        .await
        .map_err(|_| "could not assume authenticated role")?;
    let claims = identity(tenant, subject).claims_json;
    tx.execute(
        "SELECT set_config('request.jwt.claims', $1, true)",
        &[&claims],
    )
    .await
    .map_err(|_| "could not bind caller claims")?;
    let row = tx.query_one(
        "SELECT current_user = 'authenticated' AND row_security_active('flint_files.records'::regclass)",
        &[],
    ).await.map_err(|_| "could not inspect active RLS")?;
    if !row
        .try_get::<_, bool>(0)
        .map_err(|_| "invalid active RLS result")?
    {
        return Err("record RLS is inactive under the SQL caller".into());
    }

    check_own_row(&tx, tenant, subject).await?;
    if tx
        .query_opt(READ, &[&original_tenant, &original_subject, &original.0])
        .await
        .map_err(|_| "foreign select failed")?
        .is_some()
    {
        return Err("original record visible to foreign caller".into());
    }
    let mutation = Uuid::new_v4().to_string();
    if tx
        .execute(
            UPDATE,
            &[&original_tenant, &original_subject, &original.0, &mutation],
        )
        .await
        .map_err(|_| "foreign update failed")?
        != 0
    {
        return Err("foreign update affected the original row".into());
    }
    // Reuse the known-valid shape. The foreign pair has the same private type registered.
    let fresh_id = Uuid::new_v4().to_string();
    let mut record = attachment();
    record.mutation_id = Uuid::new_v4().to_string();
    let data = serde_json::to_value(&record).map_err(|_| "invalid fixture attachment")?;
    let result = tx
        .execute(
            INSERT,
            &[
                &original_tenant,
                &original_subject,
                &fresh_id,
                &record.type_id.0,
                &record.type_version,
                &data,
            ],
        )
        .await;
    tx.rollback()
        .await
        .map_err(|_| "could not roll back SQL control")?;
    let error = result.err().ok_or("foreign insert succeeded")?;
    let db = error
        .as_db_error()
        .ok_or("foreign insert failed without a PostgreSQL diagnostic")?;
    if *db.code() == SqlState::CHECK_VIOLATION {
        return Err(
            "record validation rejected the foreign insert (23514) before RLS could be observed"
                .into(),
        );
    }
    if *db.code() != SqlState::INSUFFICIENT_PRIVILEGE
        || !db.message().contains("row-level security policy")
        || !db.message().contains("table \"records\"")
    {
        return Err(format!(
            "foreign insert lacked a record RLS diagnostic (SQLSTATE {})",
            db.code().code()
        )
        .into());
    }
    Ok(())
}

async fn check_own_row(
    tx: &deadpool_postgres::Transaction<'_>,
    tenant: &str,
    subject: &str,
) -> CheckResult {
    let id = Uuid::new_v4().to_string();
    let mut record = attachment();
    record.mutation_id = Uuid::new_v4().to_string();
    let data = serde_json::to_value(&record).map_err(|_| "invalid fixture attachment")?;
    let inserted = tx
        .execute(
            INSERT,
            &[
                &tenant,
                &subject,
                &id,
                &record.type_id.0,
                &record.type_version,
                &data,
            ],
        )
        .await
        .map_err(|_| "own insert failed")?;
    let own = tx
        .query_opt(READ, &[&tenant, &subject, &id])
        .await
        .map_err(|_| "own select failed")?;
    let revision = own
        .ok_or("own record invisible")?
        .try_get::<_, i64>(0)
        .map_err(|_| "invalid own revision")?;
    if inserted != 1 || revision != 1 {
        return Err("own insert/select did not yield one initial revision".into());
    }
    let mutation = Uuid::new_v4().to_string();
    if tx
        .execute(UPDATE, &[&tenant, &subject, &id, &mutation])
        .await
        .map_err(|_| "own update failed")?
        != 1
    {
        return Err("own update did not affect exactly one row".into());
    }

    Ok(())
}
