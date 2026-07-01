//! name@version → (digest, manifest, cwasm cache). SurrealDB- or Postgres-backed.
#![forbid(unsafe_code)]

use async_trait::async_trait;
use fke_domain::FunctionManifest;
use fke_ports::{ComponentRegistry, StoreError};

pub struct Registry;

#[async_trait]
impl ComponentRegistry for Registry {
    async fn resolve(&self, _name: &str, _version: &str) -> Result<FunctionManifest, StoreError> {
        todo!()
    }
}
