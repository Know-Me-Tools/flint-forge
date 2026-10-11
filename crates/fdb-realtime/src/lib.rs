//! Realtime change-stream adapters for Flint Quarry.
//!
//! `ListenChangeSource` is the rollout default. `FabricChangeSource` consumes
//! Fabric's versioned, authorized and resumable `WatchEntityType` stream.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod fabric;
pub mod listen;

pub use fabric::{FabricChangeSource, FabricError, FrfConfig};
pub use listen::{ListenChangeSource, ListenConfig, ListenError};

use fdb_ports::StreamError;

/// Configuration for the optional Keto HTTP admission check.
#[derive(Debug, Clone)]
pub struct KetoConfig {
    /// Base URL of the Keto read API, e.g. `http://keto:4466`.
    pub base_url: String,
}

/// Check whether `subject` has `view` on one tenant-scoped entity type.
///
/// The subject is sensitive and is never included in logs or returned errors.
/// Transport and malformed-response failures fail closed as unavailable.
pub(crate) async fn keto_check_via_http(
    client: &reqwest::Client,
    keto_base_url: &str,
    namespace: &str,
    object: &str,
    subject: &str,
) -> Result<(), StreamError> {
    let response = client
        .get(format!("{keto_base_url}/relation-tuples/check"))
        .query(&[
            ("namespace", namespace),
            ("object", object),
            ("relation", "view"),
            ("subject_id", subject),
        ])
        .send()
        .await
        .map_err(|_| StreamError::Unavailable)?;

    match response.status() {
        status if status.is_success() => {
            let body: serde_json::Value = response
                .json()
                .await
                .map_err(|_| StreamError::Unavailable)?;
            if body.get("allowed").and_then(serde_json::Value::as_bool) == Some(true) {
                Ok(())
            } else {
                Err(StreamError::Denied)
            }
        }
        reqwest::StatusCode::FORBIDDEN => Err(StreamError::Denied),
        _ => Err(StreamError::Unavailable),
    }
}
