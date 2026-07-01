//! SignatureVerifier adapter: Ed25519 / did:prometheus VC (sovereign default).
#![forbid(unsafe_code)]

use async_trait::async_trait;
use fke_domain::FunctionManifest;
use fke_ports::{SignError, SignatureVerifier};

pub struct VerifierDid;

#[async_trait]
impl SignatureVerifier for VerifierDid {
    async fn verify(
        &self,
        _m: &FunctionManifest,
        _sig: &[u8],
        _art: &[u8],
    ) -> Result<(), SignError> {
        todo!()
    }
}
