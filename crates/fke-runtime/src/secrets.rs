//! `flint:host/secrets` — Cedar-gated, audited secret access.
//!
//! The WASM guest only ever holds a `Resource<SecretHandle>` — never the
//! plaintext value. `reveal()` is the sole path to plaintext:
//!
//! 1. `Capability::Secrets` must be granted for this invocation (checked at
//!    `get()` time, same interface-level gate every other interface uses).
//! 2. `reveal()` additionally requires a per-secret Cedar grant
//!    (`kiln:secret:reveal` scoped to the secret's name) — default-deny, and
//!    re-checked on every `reveal()` call, not cached from `get()`.
//! 3. Only then is `vault.reveal_for_kiln` called, which decrypts and writes
//!    an audit row to `vault.access_log` (`ext-flint-vault`) regardless of
//!    outcome.
//!
//! High-value secrets are meant to be brokered at the host boundary instead
//! (injected into outbound calls without ever entering WASM linear memory);
//! `reveal()` is the escape hatch for secrets a component genuinely must read.

use crate::host_bindings::flint::host::secrets::{Host, HostError, HostSecret};
use crate::KilnHostState;
use fke_domain::Capability;
use forge_policy::Decision;
use wasmtime::component::Resource;

/// Backing type for the WIT `secret` resource, stored in a component
/// instance's `ResourceTable`.
pub struct SecretHandle {
    pub name: String,
    pub publisher_did: String,
}

fn vault_error(context: &str, e: impl std::fmt::Display) -> HostError {
    HostError {
        code: "VAULT_ERROR".to_owned(),
        message: format!("{context}: {e}"),
    }
}

impl HostSecret for KilnHostState {
    async fn reveal(&mut self, self_: Resource<SecretHandle>) -> Result<String, HostError> {
        let handle = self.table.get(&self_).map_err(|e| HostError {
            code: "INTERNAL".to_owned(),
            message: format!("invalid secret handle: {e}"),
        })?;
        let name = handle.name.clone();
        let publisher_did = handle.publisher_did.clone();

        // Per-secret Cedar check — default-deny when there's no Pep or no
        // caller identity for this invocation, same posture as every other
        // Cedar gate in this crate.
        let allowed = match (&self.pep, &self.identity) {
            (Some(pep), Some(who)) => {
                pep.check(who, &forge_policy::kiln::secret_reveal_request(&name))
                    .await
                    == Decision::Allow
            }
            _ => false,
        };
        if !allowed {
            return Err(HostError {
                code: "CEDAR_DENY".to_owned(),
                message: format!("no Cedar grant to reveal secret {name}"),
            });
        }

        let database = self.database.as_ref().ok_or_else(|| HostError {
            code: "UNAVAILABLE".to_owned(),
            message: "no database backend configured for this Kiln runtime".to_owned(),
        })?;
        // Present because `allowed` above required `self.identity` to be `Some`.
        let rls = self
            .identity
            .as_ref()
            .expect("identity checked present above");

        let params = vec![
            serde_json::to_string(&name).expect("string JSON-encoding is infallible"),
            serde_json::to_string(&publisher_did).expect("string JSON-encoding is infallible"),
        ];
        let rows = database
            .query_json(
                rls,
                "SELECT vault.reveal_for_kiln($1, $2) AS result_value",
                &params,
            )
            .await
            .map_err(|e| vault_error("reveal_for_kiln", e))?;
        let row_json = rows
            .first()
            .ok_or_else(|| vault_error("reveal_for_kiln", "returned no row"))?;
        let row: serde_json::Value =
            serde_json::from_str(row_json).map_err(|e| vault_error("decode reveal row", e))?;
        row.get("result_value")
            .and_then(|v| v.as_str())
            .map(str::to_owned)
            .ok_or_else(|| vault_error("decode reveal row", "missing result_value"))
    }

    async fn drop(&mut self, rep: Resource<SecretHandle>) -> wasmtime::Result<()> {
        self.table.delete(rep).map(|_| ()).map_err(wasmtime::Error::from)
    }
}

impl Host for KilnHostState {
    /// Allocates an opaque handle — no plaintext, no Cedar check on the
    /// secret name itself yet (that happens per-call in `reveal()`). Still
    /// gated by the interface-level `Secrets` capability.
    async fn get(&mut self, name: String) -> Result<Resource<SecretHandle>, HostError> {
        if !self.granted.contains(&Capability::Secrets) {
            return Err(HostError {
                code: "CAPABILITY_DENIED".to_owned(),
                message: "Secrets capability not granted for this invocation".to_owned(),
            });
        }
        let publisher_did = self
            .identity
            .as_ref()
            .map(|rls| rls.keto_subject.clone())
            .unwrap_or_default();
        self.table
            .push(SecretHandle { name, publisher_did })
            .map_err(|e| HostError {
                code: "INTERNAL".to_owned(),
                message: format!("failed to allocate secret handle: {e}"),
            })
    }
}
