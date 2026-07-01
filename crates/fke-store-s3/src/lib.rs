//! ComponentStore adapter: s3. Content-addressed.
#![forbid(unsafe_code)]

use async_trait::async_trait;
use fke_domain::ContentId;
use fke_ports::{ComponentStore, StoreError};

pub struct StoreS3;

#[async_trait]
impl ComponentStore for StoreS3 {
    async fn put(&self, _bytes: &[u8]) -> Result<ContentId, StoreError> {
        todo!()
    }
    async fn get(&self, _id: &ContentId) -> Result<Vec<u8>, StoreError> {
        todo!()
    }
    async fn exists(&self, _id: &ContentId) -> Result<bool, StoreError> {
        todo!()
    }
}
