//! RFC 6962 Signed Certificate Timestamp (SCT) verification.
//!
//! Fulcio embeds one or more SCTs in every leaf certificate as proof the
//! precertificate was submitted to a public Certificate Transparency log
//! before the final certificate was issued (X.509v3 extension OID
//! `1.3.6.1.4.1.11129.2.4.2`). Without checking this, a compromised or
//! misbehaving Fulcio instance's certs would still be accepted as long as
//! they cryptographically chain to the pinned root — CT logging is the
//! independent check that Fulcio itself is behaving honestly.
//!
//! Verification follows [RFC 6962 §3.2]: the CT log's signature covers a
//! `TimestampedEntry` built from the SCT's own version/timestamp/extensions,
//! the SHA-256 hash of the *issuing* (intermediate) CA's
//! `SubjectPublicKeyInfo`, and the DER-encoded "precertificate" TBSCertificate
//! — which for an already-issued cert is reconstructed by taking the leaf's
//! own TBSCertificate and removing the SCT-list extension itself (the only
//! difference between what was submitted to the log and the final cert).
//!
//! This exact reconstruction was validated byte-for-byte against a real,
//! live Sigstore-issued certificate fetched from `rekor.sigstore.dev` before
//! being written here (see the `tests` module).
//!
//! [RFC 6962 §3.2]: https://datatracker.ietf.org/doc/html/rfc6962#section-3.2

use const_oid::AssociatedOid;
use fke_ports::SignError;
use p256::ecdsa::{signature::Verifier as _, DerSignature, VerifyingKey};
use p256::elliptic_curve::pkcs8::DecodePublicKey;
use sha2::{Digest, Sha256};
use x509_cert::der::Encode;
use x509_cert::ext::pkix::sct::{
    HashAlgorithm, SignatureAlgorithm, SignedCertificateTimestamp, SignedCertificateTimestampList,
};
use x509_cert::Certificate;

/// Sigstore's 2022-generation CT log public key (P-256). Fetched from
/// `https://raw.githubusercontent.com/sigstore/root-signing/main/targets/ctfe_2022.pub`
/// (2026-07-13); cross-checked against `trusted_root.json` in the same repo,
/// where `SHA-256(DER(this key))` equals the `keyId` published for CT log
/// `https://ctfe.sigstore.dev/2022`
/// (`3T0wasbHETJjGR4cmWc3AqJKXrjePK3/h4pygC8p7o4=`) — the log Fulcio has used
/// for embedded SCTs since 2022-04-13, matching the pinned intermediate CA's
/// validity start in `chain.rs`.
///
/// Only this current-generation log key is pinned. An older 2021-2022
/// generation (`ctfe.pub` / test log) predates the currently active Fulcio
/// intermediate and is intentionally not accepted here.
const CTFE_2022_PUB_PEM: &str = "-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEiPSlFi0CmFTfEjCUqF9HuCEcYXNK
AaYalIJmBZ8yyezPjTqhxrKBpMnaocVtLJBI1eM3uXnQzQGAJdJ4gs9Fyw==
-----END PUBLIC KEY-----";

/// Verify at least one SCT embedded in `leaf` against the pinned CT log key.
/// `intermediate` must be the CA cert that actually signed `leaf` (the
/// caller already established this via [`crate::chain::verify_chain_to_pinned_root`]) —
/// its `SubjectPublicKeyInfo` hash is part of what the log signed over.
pub(crate) fn verify_embedded_scts(
    leaf: &Certificate,
    intermediate: &Certificate,
) -> Result<(), SignError> {
    let (_, sct_list) = leaf
        .tbs_certificate
        .get::<SignedCertificateTimestampList>()
        .map_err(|_| SignError::Invalid)?
        .ok_or(SignError::Invalid)?;

    let serialized_scts = sct_list
        .parse_timestamps()
        .map_err(|_| SignError::Invalid)?;
    if serialized_scts.is_empty() {
        return Err(SignError::Invalid);
    }

    let log_key = ct_log_verifying_key()?;
    let log_id = ct_log_id(&log_key)?;
    let precert_tbs_der = reconstruct_precert_tbs(leaf)?;
    let issuer_key_hash = spki_sha256(intermediate)?;

    for serialized in &serialized_scts {
        let Ok(sct) = serialized.parse_timestamp() else {
            continue;
        };
        if sct.log_id.key_id != log_id {
            continue; // signed by a log we don't pin — try the next SCT
        }
        if sct.signature.algorithm.hash != HashAlgorithm::Sha256
            || sct.signature.algorithm.signature != SignatureAlgorithm::Ecdsa
        {
            continue; // Sigstore only ever issues sha256+ecdsa SCTs
        }
        let Ok(signature) = DerSignature::try_from(sct.signature.signature.as_slice()) else {
            continue;
        };
        let Ok(signed_bytes) = build_timestamped_entry(&sct, &issuer_key_hash, &precert_tbs_der)
        else {
            continue;
        };
        if log_key.verify(&signed_bytes, &signature).is_ok() {
            return Ok(());
        }
    }

    Err(SignError::Invalid)
}

/// Parse the pinned CT log's raw `SubjectPublicKeyInfo` PEM into a P-256
/// verifying key.
fn ct_log_verifying_key() -> Result<VerifyingKey, SignError> {
    let public_key =
        p256::PublicKey::from_public_key_pem(CTFE_2022_PUB_PEM).map_err(|_| SignError::Invalid)?;
    Ok(VerifyingKey::from(public_key))
}

/// `LogID` per RFC 6962 §3.2: `SHA-256(DER(SubjectPublicKeyInfo))` of the
/// log's own public key — used to match an embedded SCT to this pinned key.
fn ct_log_id(log_key: &VerifyingKey) -> Result<[u8; 32], SignError> {
    let public_key = p256::PublicKey::from(log_key);
    let spki_der = spki::SubjectPublicKeyInfoOwned::from_key(public_key)
        .map_err(|_| SignError::Invalid)?
        .to_der()
        .map_err(|_| SignError::Invalid)?;
    Ok(Sha256::digest(spki_der).into())
}

/// Reconstruct the DER-encoded "precertificate" TBSCertificate that was
/// originally submitted to the CT log: `leaf`'s own TBSCertificate with the
/// SCT-list extension removed (RFC 6962 §3.2 — "it is also possible to
/// reconstruct this TBSCertificate from the final certificate by extracting
/// the TBSCertificate from it and deleting the SCT extension").
fn reconstruct_precert_tbs(leaf: &Certificate) -> Result<Vec<u8>, SignError> {
    let mut tbs = leaf.tbs_certificate.clone();
    if let Some(extensions) = tbs.extensions.as_mut() {
        extensions.retain(|ext| ext.extn_id != SignedCertificateTimestampList::OID);
    }
    tbs.to_der().map_err(|_| SignError::Invalid)
}

/// SHA-256 of the DER-encoded `SubjectPublicKeyInfo` of `cert` — the
/// `issuer_key_hash` field of RFC 6962's `PreCert` structure.
fn spki_sha256(cert: &Certificate) -> Result<[u8; 32], SignError> {
    let spki_der = cert
        .tbs_certificate
        .subject_public_key_info
        .to_der()
        .map_err(|_| SignError::Invalid)?;
    Ok(Sha256::digest(spki_der).into())
}

/// Build the exact byte string an SCT's signature covers for a
/// `precert_entry` (RFC 6962 §3.2): the `digitally-signed` struct's fields,
/// concatenated in TLS presentation-language order with no extra framing.
fn build_timestamped_entry(
    sct: &SignedCertificateTimestamp,
    issuer_key_hash: &[u8; 32],
    precert_tbs_der: &[u8],
) -> Result<Vec<u8>, SignError> {
    let extensions = sct.extensions.as_slice();
    // Both are TLS `opaque<..>` fields with a protocol-defined max length
    // (2^24-1 and 2^16-1 respectively) — an oversized value is a malformed
    // input to reject, not a truncation to silently swallow.
    let tbs_len = u32::try_from(precert_tbs_der.len()).map_err(|_| SignError::Invalid)?;
    let ext_len = u16::try_from(extensions.len()).map_err(|_| SignError::Invalid)?;

    let mut buf = Vec::with_capacity(44 + precert_tbs_der.len() + extensions.len());
    buf.push(0x00); // sct_version = v1
    buf.push(0x00); // signature_type = certificate_timestamp
    buf.extend_from_slice(&sct.timestamp.to_be_bytes()); // uint64 timestamp
    buf.extend_from_slice(&1u16.to_be_bytes()); // entry_type = precert_entry(1)
    buf.extend_from_slice(issuer_key_hash); // PreCert.issuer_key_hash[32]
    buf.extend_from_slice(&tbs_len.to_be_bytes()[1..]); // opaque<1..2^24-1>: 3-byte length
    buf.extend_from_slice(precert_tbs_der);
    buf.extend_from_slice(&ext_len.to_be_bytes()); // CtExtensions<0..2^16-1>
    buf.extend_from_slice(extensions);
    Ok(buf)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use x509_cert::der::DecodePem;

    /// A real, live Fulcio-issued leaf certificate fetched from
    /// `rekor.sigstore.dev` (log entry
    /// `108e9186e8c5677a7b6d3b01a291e57ce3d5187f5387fbc50e736d63c2bd82ea05a683eeeae15d90`,
    /// fetched 2026-07-13). Chains to the pinned intermediate (AKI matches
    /// the intermediate's SKI, confirmed via `openssl verify`) and carries a
    /// real embedded SCT signed by the pinned 2022 CT log, plus real Fulcio
    /// OIDC identity extensions (used by `identity.rs`'s tests too). Its
    /// 10-minute validity window (2026-06-28 21:36–21:46 UTC) has long since
    /// expired, which is irrelevant here — SCT/identity extraction don't
    /// check the leaf's own validity window (that's `check_cert_validity`,
    /// tested separately).
    pub(crate) const REAL_FULCIO_LEAF_PEM: &str = "-----BEGIN CERTIFICATE-----
MIIDVTCCAtugAwIBAgIUWNIUpBTnHflZZoSe3niBG4EGATcwCgYIKoZIzj0EAwMw
NzEVMBMGA1UEChMMc2lnc3RvcmUuZGV2MR4wHAYDVQQDExVzaWdzdG9yZS1pbnRl
cm1lZGlhdGUwHhcNMjYwNjI4MjEzNjE2WhcNMjYwNjI4MjE0NjE2WjAAMFkwEwYH
KoZIzj0CAQYIKoZIzj0DAQcDQgAE8eBTvOmvq7I2CQ+3CqpacS5kmLij2NRlm81U
CQ5gAQvNApVXyRj/ans80UiVQxnvaD9HewEYjC+a2hRSG93jm6OCAfowggH2MA4G
A1UdDwEB/wQEAwIHgDATBgNVHSUEDDAKBggrBgEFBQcDAzAdBgNVHQ4EFgQUoWwz
AhfbyC/F6DvleKWNX9Vr+KMwHwYDVR0jBBgwFoAU39Ppz1YkEZb5qNjpKFWixi4Y
ZD8wYgYDVR0RAQH/BFgwVoZUaHR0cHM6Ly9pc3N1ZXIuZW5mb3JjZS5kZXYvNTk4
YTBlODZiZTJmZWU2MGQxYzJmYjhkODU4NTUyZjU5ZDljNTExMS9jYjIxZTJlMmE3
YjQxMWYxMCgGCisGAQQBg78wAQEEGmh0dHBzOi8vaXNzdWVyLmVuZm9yY2UuZGV2
MCoGCisGAQQBg78wAQgEHAwaaHR0cHM6Ly9pc3N1ZXIuZW5mb3JjZS5kZXYwSQYK
KwYBBAGDvzABGAQ7DDk1OThhMGU4NmJlMmZlZTYwZDFjMmZiOGQ4NTg1NTJmNTlk
OWM1MTExL2NiMjFlMmUyYTdiNDExZjEwgYkGCisGAQQB1nkCBAIEewR5AHcAdQDd
PTBqxscRMmMZHhyZZzcCokpeuN48rf+HinKALynujgAAAZ8QKWVBAAAEAwBGMEQC
ICMTJM4veXSnsPHcrTVTSRA5LLFvws+lodtsi8l/NO2hAiB6xRY4Kd8WvEyXvMcG
09+EPDn7c19fhxVVrpBLNUWGXDAKBggqhkjOPQQDAwNoADBlAjEAxHT30oj4knmi
Py8/Ha0pSwHnrm11osS+0ABlImoKJ43GIml89qp6J94YBbgJHBRaAjBnND0to8VG
seOwFXP5/K1Gdp4E7kevoJJhP3MuZQ+zsL9EPYJLL9Cw5rCT6I1Dn2A=
-----END CERTIFICATE-----";

    fn fixtures() -> (Certificate, Certificate) {
        let leaf = Certificate::from_pem(REAL_FULCIO_LEAF_PEM).expect("real leaf");
        let intermediate =
            Certificate::from_pem(crate::chain::FULCIO_INTERMEDIATE_PEM).expect("intermediate");
        (leaf, intermediate)
    }

    /// End-to-end proof against a real, live Sigstore-issued certificate —
    /// not just internal self-consistency. Confirms the exact byte
    /// reconstruction matches what Sigstore's production CT log actually
    /// signed.
    #[test]
    fn verifies_real_production_sct() {
        let (leaf, intermediate) = fixtures();
        assert!(
            verify_embedded_scts(&leaf, &intermediate).is_ok(),
            "must verify the real embedded SCT against the pinned 2022 CT log key"
        );
    }

    /// A cert with no SCT-list extension at all must be rejected. The pinned
    /// intermediate CA cert itself has no SCT extension (CA certs aren't
    /// logged the way leaf certs are), so it doubles as this fixture.
    #[test]
    fn rejects_cert_with_no_sct_extension() {
        let (_, intermediate) = fixtures();
        assert!(matches!(
            verify_embedded_scts(&intermediate, &intermediate),
            Err(SignError::Invalid)
        ));
    }

    /// Flipping a single byte of the real SCT signature must invalidate it.
    #[test]
    fn rejects_tampered_sct_signature() {
        let (leaf, intermediate) = fixtures();
        let mut tbs = leaf.tbs_certificate.clone();
        let exts = tbs.extensions.as_mut().expect("extensions");
        let sct_ext = exts
            .iter_mut()
            .find(|e| e.extn_id == SignedCertificateTimestampList::OID)
            .expect("sct extension present");
        let mut bytes = sct_ext.extn_value.as_bytes().to_vec();
        // Flip a byte deep inside the signature portion (well past the fixed
        // header: outer len(2) + inner len(2) + version(1) + log_id(32) +
        // timestamp(8) + ext_len(2) + hash/sig algo(2) + sig_len(2)).
        let tamper_at = bytes.len() - 10;
        bytes[tamper_at] ^= 0xFF;
        sct_ext.extn_value = x509_cert::der::asn1::OctetString::new(bytes).expect("octet string");
        let tampered = Certificate {
            tbs_certificate: tbs,
            signature_algorithm: leaf.signature_algorithm.clone(),
            signature: leaf.signature.clone(),
        };
        assert!(matches!(
            verify_embedded_scts(&tampered, &intermediate),
            Err(SignError::Invalid)
        ));
    }
}
