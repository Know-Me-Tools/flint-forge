//! Disposable PostgreSQL and signed-identity fixture for the issue 40 acceptance cases.
#[path = "../../../forge-identity/tests/common/mod.rs"]
mod jwks;
#[path = "../../src/policy_source.rs"]
mod policy_source;
#[path = "../../src/rls_layer.rs"]
mod rls_layer;

use axum::{middleware, routing::get, Json, Router};
use deadpool_postgres::Pool;
use fdb_gateway::files::{router, FileApiState};
use fdb_postgres::PgFileMetadata;
use forge_policy::{files::record_grant, CedarPolicyEngine, PolicySource};
use jsonwebtoken::{Algorithm, Header};
use ring::digest::{digest, SHA256};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    env,
    error::Error,
    fs,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::{net::TcpListener, task::JoinHandle};

type FixtureResult<T> = Result<T, Box<dyn Error>>;
const MANIFEST: &str = include_str!("../../../../scripts/issue40-fixture/manifest.json");
const ISSUER: &str = "https://gate.know-me.tools";
const AUDIENCE: &str = "flint-core";

#[derive(Deserialize)]
struct Manifest {
    fixture_id: String,
    host: String,
    port: u16,
    database: String,
    baseline_sha256: String,
}

impl Manifest {
    fn inputs() -> FixtureResult<(Self, String)> {
        for (name, expected) in [
            ("FILE_METADATA_TEST_ALLOW_WRITES", "isolated-disposable"),
            ("FLINT_AUTHZ_MODE", "rls"),
            ("FLINT_FILE_METADATA_ENABLED", "true"),
        ] {
            if env::var(name).as_deref() != Ok(expected) {
                return Err(format!("missing fixture setting {name}").into());
            }
        }
        let path =
            env::var_os("FILE_METADATA_TEST_FIXTURE_MANIFEST").ok_or("missing fixture manifest")?;
        let provided: Value = serde_json::from_slice(&fs::read(path)?)?;
        if provided != serde_json::from_str::<Value>(MANIFEST)? {
            return Err("fixture manifest differs from the checked-in packet".into());
        }
        let manifest: Self = serde_json::from_value(provided)?;
        if manifest.host != "127.0.0.1" || !manifest.database.ends_with("_file_metadata_test") {
            return Err("fixture manifest must identify a disposable loopback database".into());
        }
        let url = env::var("FILE_METADATA_TEST_DATABASE_URL")
            .map_err(|_| "missing fixture connection")?;
        let target = format!("@{}:{}/{}", manifest.host, manifest.port, manifest.database);
        // The launcher generates a fresh hex password. Accept exactly its URL form so
        // both connection drivers receive the same host and cannot parse extra options.
        let password = url
            .strip_prefix("postgresql://postgres:")
            .and_then(|rest| rest.strip_suffix(&target))
            .ok_or("connection differs from the approved fixture target")?;
        if password.is_empty()
            || !password.bytes().all(|b| b.is_ascii_hexdigit())
            || env::var_os("PGOPTIONS").is_some()
        {
            return Err("unexpected fixture connection options".into());
        }
        Ok((manifest, url))
    }

    async fn verify_connection(&self, client: &deadpool_postgres::Object) -> FixtureResult<()> {
        let row = client.query_one(
            "SELECT current_database()::text, current_setting('forge.fixture_id', true),
                    current_user::text, inet_server_port(), current_setting('server_version_num')::integer",
            &[],
        ).await?;
        let port: Option<i32> = row.try_get(3)?;
        if row.try_get::<_, String>(0)? != self.database
            || row.try_get::<_, Option<String>>(1)?.as_deref() != Some(&self.fixture_id)
            || row.try_get::<_, String>(2)? != "postgres"
            || port != Some(5432)
            || row.try_get::<_, i32>(4)? / 10_000 != 18
        {
            return Err("connected PostgreSQL server differs from the fixture manifest".into());
        }
        Ok(())
    }
}

type EnabledPolicies = BTreeMap<String, String>;

fn policy_digest(policies: &EnabledPolicies) -> String {
    // BTreeMap's UTF-8 byte order matches the manifest's PostgreSQL COLLATE "C".
    let canonical = policies
        .iter()
        .map(|(id, text)| format!("{id}\u{1f}{text}"))
        .collect::<Vec<_>>()
        .join("\u{1e}");
    let bytes = digest(&SHA256, canonical.as_bytes());
    let mut hex = String::with_capacity(bytes.as_ref().len() * 2);
    for b in bytes.as_ref() {
        use std::fmt::Write;
        write!(hex, "{b:02x}").expect("writing into a String is infallible");
    }
    hex
}

async fn enabled(tx: &deadpool_postgres::Transaction<'_>) -> FixtureResult<EnabledPolicies> {
    let rows = tx
        .query(
            "SELECT id, policy_text FROM flint_meta.cedar_policies WHERE enabled",
            &[],
        )
        .await?;
    rows.iter()
        .map(|row| Ok((row.try_get(0)?, row.try_get(1)?)))
        .collect()
}

// A case runs in its own process. The guard also serializes local JWKS configuration
// if both named cases are explicitly selected by a developer in the same process.
static CASE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub(super) struct Issue40Fixture {
    pub(super) api: Router,
    pub(super) independent: PgFileMetadata,
    pub(super) pg: Pool,
    key: jwks::TestKey,
    server: JoinHandle<()>,
    installed: Vec<String>,
    manifest: Manifest,
    _case: tokio::sync::MutexGuard<'static, ()>,
}

impl Issue40Fixture {
    pub(super) async fn new(grants: &[(&str, &str)]) -> FixtureResult<Self> {
        let case = CASE.lock().await;
        let (manifest, url) = Manifest::inputs()?;
        let pg = super::pool(&url).map_err(|_| "fixture pool configuration failed")?;
        let mut client = pg
            .get()
            .await
            .map_err(|_| "fixture PostgreSQL connection failed")?;
        manifest.verify_connection(&client).await?;
        let privileged = sqlx::PgPool::connect(&url)
            .await
            .map_err(|_| "fixture policy connection failed")?;
        let source = Arc::new(policy_source::DbPolicySource::new(privileged));

        let tx = client.transaction().await?;
        tx.batch_execute("LOCK TABLE flint_meta.cedar_policies IN SHARE ROW EXCLUSIVE MODE")
            .await?;
        let mut expected = enabled(&tx).await?;
        if policy_digest(&expected) != manifest.baseline_sha256 {
            return Err("enabled policy set differs from the pinned pre-grant baseline".into());
        }
        let mut installed = Vec::with_capacity(grants.len());
        for (index, &(subject, tenant)) in grants.iter().enumerate() {
            let text = record_grant(subject, tenant).ok_or("invalid fixture grant")?;
            let id = format!("issue40-grant-{index}");
            tx.execute(
                "INSERT INTO flint_meta.cedar_policies(id, name, policy_text, enabled)
                 VALUES ($1, 'Disposable file record grant', $2, true)",
                &[&id, &text],
            )
            .await
            .map_err(|_| "could not install fixture grant")?;
            expected.insert(id.clone(), text);
            installed.push(id);
        }
        let enabled_sha256 = policy_digest(&expected);
        if enabled(&tx).await? != expected {
            return Err("installed policy set differs from the rendered grants".into());
        }
        tx.commit().await?;
        let loaded = source
            .load()
            .await?
            .into_iter()
            .map(|entry| (entry.id, entry.text))
            .collect::<EnabledPolicies>();
        if loaded != expected {
            return Err("policy source loaded a different enabled set".into());
        }
        let engine = CedarPolicyEngine::new(source).await;
        engine.reload().await?;
        eprintln!(
            "issue40_policy_set baseline_sha256={} enabled_sha256={enabled_sha256}",
            manifest.baseline_sha256
        );

        let key = jwks::generate_es256_key("synthetic-issue40");
        let public = jwks::jwks_response(&[&key]);
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let jwks_url = format!("http://{}/jwks", listener.local_addr()?);
        let local_jwks = Router::new().route("/jwks", get(move || async move { Json(public) }));
        let server = tokio::spawn(async move {
            axum::serve(listener, local_jwks)
                .await
                .expect("synthetic loopback JWKS server failed");
        });
        env::set_var("FLINT_GATE_JWKS_URL", jwks_url);
        env::set_var("FLINT_GATE_ISSUER", ISSUER);
        env::set_var("FLINT_GATE_AUDIENCE", AUDIENCE);
        env::set_var("FLINT_GATE_MODE", "production");
        env::set_var("FLINT_GATE_JWKS_TTL_SECS", "0");
        let api = router(FileApiState {
            store: Some(Arc::new(PgFileMetadata::new(pg.clone()))),
            pep: Arc::new(engine),
            keto: None,
        })
        .layer(middleware::from_fn(rls_layer::require_rls));
        let independent = PgFileMetadata::new(
            super::pool(&url).map_err(|_| "independent pool configuration failed")?,
        );
        Ok(Self {
            api,
            independent,
            pg,
            key,
            server,
            installed,
            manifest,
            _case: case,
        })
    }

    pub(super) fn bearer(
        &self,
        subject: &str,
        tenant: &str,
        actor: Option<&str>,
    ) -> FixtureResult<String> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let mut claims = json!({
            "sub": subject, "tenant_id": tenant, "role": "authenticated", "flint_kind": "agent",
            "iss": ISSUER, "aud": AUDIENCE, "scope": "", "iat": now, "exp": now + 300,
            "jti": uuid::Uuid::new_v4().to_string(),
        });
        if let Some(actor) = actor {
            claims["act"] = json!({"sub": actor});
        }
        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some(self.key.kid.clone());
        jsonwebtoken::encode(&header, &claims, &self.key.encoding_key)
            .map_err(|_| "failed to sign synthetic fixture token".into())
    }

    pub(super) async fn cleanup(self) -> FixtureResult<()> {
        let mut client = self
            .pg
            .get()
            .await
            .map_err(|_| "fixture cleanup connection failed")?;
        self.manifest.verify_connection(&client).await?;
        let tx = client.transaction().await?;
        tx.batch_execute("LOCK TABLE flint_meta.cedar_policies IN SHARE ROW EXCLUSIVE MODE")
            .await?;
        let removed = tx
            .execute(
                "DELETE FROM flint_meta.cedar_policies WHERE id = ANY($1)",
                &[&self.installed],
            )
            .await?;
        if usize::try_from(removed)? != self.installed.len()
            || policy_digest(&enabled(&tx).await?) != self.manifest.baseline_sha256
        {
            return Err(
                "fixture cleanup did not restore the pinned enabled policy baseline".into(),
            );
        }
        tx.commit().await?;
        Ok(())
    }
}

impl Drop for Issue40Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}
