//! Verify both public pool constructors with process-isolated operator settings.
#![allow(clippy::expect_used)]

use fdb_domain::provision::Namespace;
use fdb_ports::SchemaProvisioner;
use fdb_postgres::{error::PgError, PgBackend, PgProvisioner};
use std::process::Command;

#[tokio::test]
async fn database_settings_select_the_operator_configured_pool() {
    // Run each case in a child so concurrent tests always retain their own environment.
    const CASE: &str = "FORGE_TEST_DATABASE_CONFIG_CASE";
    match std::env::var(CASE).as_deref() {
        Ok("unset" | "invalid") => {
            assert!(matches!(PgBackend::from_env(), Err(PgError::Config(_))));
            assert!(matches!(PgProvisioner::from_env(), Err(PgError::Config(_))));
            return;
        }
        Ok("configured") => {
            let provisioner = PgProvisioner::from_env().expect("configured provisioner");
            assert!(provisioner
                .schema_exists(&Namespace("pg_catalog".into()))
                .await
                .expect("catalog lookup"));
            return;
        }
        _ => {}
    }

    let executable = std::env::current_exe().expect("test executable");
    let configured = std::env::var("DATABASE_URL").ok().filter(|s| !s.is_empty());
    let mut cases = vec![("unset", None), ("invalid", Some("unrecognized setting"))];
    if let Some(url) = configured.as_deref() {
        cases.push(("configured", Some(url)));
    }
    for (name, address) in cases {
        let mut child = Command::new(&executable);
        child
            .args([
                "--exact",
                "database_settings_select_the_operator_configured_pool",
            ])
            .env(CASE, name);
        for key in ["DATABASE_URL", "PROVISIONER_DATABASE_URL"] {
            if let Some(address) = address {
                child.env(key, address);
            } else {
                child.env_remove(key);
            }
        }
        assert!(
            child.status().expect("isolated test process").success(),
            "setting case {name}"
        );
    }
}
