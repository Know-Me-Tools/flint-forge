//! Shared server state and the manifest signature-verification gate (p16-c002).

use std::sync::Arc;

use fke_registry::PgComponentStore;
use fke_runtime::EdgeRuntime;

/// Shared server state.
#[derive(Clone)]
pub(crate) struct KilnState {
    pub(crate) runtime: Arc<EdgeRuntime>,
    pub(crate) store: Arc<PgComponentStore>,
    pub(crate) registry: Arc<fke_registry::PgRegistry>,
    /// `did:prometheus:`-scheme verifier (in-memory/HTTP key resolution, no
    /// per-call network cost after the first). Constructed once and shared —
    /// `VerifierDid` holds a TTL key cache that must persist across calls.
    pub(crate) verifier_did: Arc<fke_sign_did::VerifierDid>,
    /// Cosign/Sigstore verifier (Rekor transparency-log lookup keyed by
    /// content digest). Constructed once for a shared `reqwest::Client`.
    pub(crate) verifier_cosign: Arc<fke_sign_cosign::VerifierCosign>,
}

/// Dispatch to the verifier matching `manifest.publisher_did`'s scheme, and
/// reject outright when a `did:prometheus:` manifest carries no signature
/// (`VerifierCosign` doesn't need one — it looks the signature up from Rekor
/// by content digest, so only the DID path can fail this check before
/// dispatch).
///
/// p16-c002: this is the supply-chain trust gate. It must run (a) at
/// register, so an unsigned/invalid upload is rejected before it is ever
/// stored, and (b) at invoke on every cold cache-load (see `invoke_impl`) —
/// the same lifecycle as the WASM bytes cache itself, so a component is
/// re-verified independently of whatever checks ran at register, without
/// paying a full verification cost (a Rekor HTTP round-trip, for Cosign) on
/// every single request.
pub(crate) async fn verify_manifest_signature(
    state: &KilnState,
    manifest: &fke_domain::FunctionManifest,
    artifact: &[u8],
) -> Result<(), fke_ports::SignError> {
    use base64::Engine as _;
    use fke_ports::SignatureVerifier;

    if manifest.publisher_did.starts_with("did:prometheus:") {
        let sig_b64 = manifest
            .signature_b64
            .as_deref()
            .ok_or(fke_ports::SignError::Unsigned)?;
        let sig_bytes = base64::engine::general_purpose::STANDARD
            .decode(sig_b64)
            .map_err(|_| fke_ports::SignError::Invalid)?;
        state
            .verifier_did
            .verify(manifest, &sig_bytes, artifact)
            .await
    } else {
        // Cosign path: the verifier fetches its own signature material from
        // Rekor by content digest, so the (possibly absent) signature_b64 is
        // irrelevant here — pass an empty slice.
        state.verifier_cosign.verify(manifest, &[], artifact).await
    }
}
