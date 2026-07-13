//! Pinned Sigstore Fulcio certificate chain verification.
//!
//! Cryptographically verifies a Fulcio-issued leaf certificate against a
//! pinned intermediate CA, and that intermediate against a pinned root CA —
//! not a string match on the issuer field (p16-c002; that was trivially
//! bypassable by any self-signed cert whose issuer DN happened to contain
//! "fulcio"). Both pinned certs were fetched from sigstore/root-signing (see
//! their doc comments for provenance) and are re-validated against each
//! other by [`tests::pinned_intermediate_chains_to_pinned_root`].

use fke_ports::SignError;
use x509_cert::{der::DecodePem, der::Encode, Certificate};

/// Sigstore Fulcio root CA (self-signed, P-384). Fetched from
/// `https://raw.githubusercontent.com/sigstore/root-signing/main/targets/fulcio_v1.crt.pem`
/// (2026-07-13). Subject/issuer: `O=sigstore.dev, CN=sigstore`; valid
/// 2021-10-07 → 2031-10-05. Verified with `openssl verify` against the
/// pinned intermediate below before embedding.
pub(crate) const FULCIO_ROOT_PEM: &str = "-----BEGIN CERTIFICATE-----
MIIB9zCCAXygAwIBAgIUALZNAPFdxHPwjeDloDwyYChAO/4wCgYIKoZIzj0EAwMw
KjEVMBMGA1UEChMMc2lnc3RvcmUuZGV2MREwDwYDVQQDEwhzaWdzdG9yZTAeFw0y
MTEwMDcxMzU2NTlaFw0zMTEwMDUxMzU2NThaMCoxFTATBgNVBAoTDHNpZ3N0b3Jl
LmRldjERMA8GA1UEAxMIc2lnc3RvcmUwdjAQBgcqhkjOPQIBBgUrgQQAIgNiAAT7
XeFT4rb3PQGwS4IajtLk3/OlnpgangaBclYpsYBr5i+4ynB07ceb3LP0OIOZdxex
X69c5iVuyJRQ+Hz05yi+UF3uBWAlHpiS5sh0+H2GHE7SXrk1EC5m1Tr19L9gg92j
YzBhMA4GA1UdDwEB/wQEAwIBBjAPBgNVHRMBAf8EBTADAQH/MB0GA1UdDgQWBBRY
wB5fkUWlZql6zJChkyLQKsXF+jAfBgNVHSMEGDAWgBRYwB5fkUWlZql6zJChkyLQ
KsXF+jAKBggqhkjOPQQDAwNpADBmAjEAj1nHeXZp+13NWBNa+EDsDP8G1WWg1tCM
WP/WHPqpaVo0jhsweNFZgSs0eE7wYI4qAjEA2WB9ot98sIkoF3vZYdd3/VtWB5b9
TNMea7Ix/stJ5TfcLLeABLE4BNJOsQ4vnBHJ
-----END CERTIFICATE-----";

/// Sigstore Fulcio intermediate CA (P-384), issued by the root above. Fetched
/// from
/// `https://raw.githubusercontent.com/sigstore/root-signing/main/targets/fulcio_intermediate_v1.crt.pem`
/// (2026-07-13). Subject: `O=sigstore.dev, CN=sigstore-intermediate`; issuer:
/// `O=sigstore.dev, CN=sigstore`; valid 2022-04-13 → 2031-10-05. This is the
/// cert Fulcio's live CA actually signs leaf certificates with.
pub(crate) const FULCIO_INTERMEDIATE_PEM: &str = "-----BEGIN CERTIFICATE-----
MIICGjCCAaGgAwIBAgIUALnViVfnU0brJasmRkHrn/UnfaQwCgYIKoZIzj0EAwMw
KjEVMBMGA1UEChMMc2lnc3RvcmUuZGV2MREwDwYDVQQDEwhzaWdzdG9yZTAeFw0y
MjA0MTMyMDA2MTVaFw0zMTEwMDUxMzU2NThaMDcxFTATBgNVBAoTDHNpZ3N0b3Jl
LmRldjEeMBwGA1UEAxMVc2lnc3RvcmUtaW50ZXJtZWRpYXRlMHYwEAYHKoZIzj0C
AQYFK4EEACIDYgAE8RVS/ysH+NOvuDZyPIZtilgUF9NlarYpAd9HP1vBBH1U5CV7
7LSS7s0ZiH4nE7Hv7ptS6LvvR/STk798LVgMzLlJ4HeIfF3tHSaexLcYpSASr1kS
0N/RgBJz/9jWCiXno3sweTAOBgNVHQ8BAf8EBAMCAQYwEwYDVR0lBAwwCgYIKwYB
BQUHAwMwEgYDVR0TAQH/BAgwBgEB/wIBADAdBgNVHQ4EFgQU39Ppz1YkEZb5qNjp
KFWixi4YZD8wHwYDVR0jBBgwFoAUWMAeX5FFpWapesyQoZMi0CrFxfowCgYIKoZI
zj0EAwMDZwAwZAIwPCsQK4DYiZYDPIaDi5HFKnfxXx6ASSVmERfsynYBiX2X6SJR
nZU84/9DZdnFvvxmAjBOt6QpBlc4J/0DxvkTCqpclvziL6BCCPnjdlIB3Pu3BxsP
mygUY7Ii2zbdCdliiow=
-----END CERTIFICATE-----";

/// Cryptographically verify `leaf` was signed by the pinned Fulcio
/// intermediate, and the intermediate by the pinned root. Also checks the
/// intermediate's and root's own validity windows. Returns the parsed
/// intermediate certificate on success so callers (e.g. SCT verification,
/// which needs the intermediate's `SubjectPublicKeyInfo`) don't have to
/// re-parse the pinned PEM.
pub(crate) fn verify_chain_to_pinned_root(leaf: &Certificate) -> Result<Certificate, SignError> {
    let intermediate =
        Certificate::from_pem(FULCIO_INTERMEDIATE_PEM).map_err(|_| SignError::Invalid)?;
    let root = Certificate::from_pem(FULCIO_ROOT_PEM).map_err(|_| SignError::Invalid)?;

    check_cert_validity(&intermediate)?;
    check_cert_validity(&root)?;

    verify_signed_by(leaf, &intermediate)?;
    verify_signed_by(&intermediate, &root)?;
    Ok(intermediate)
}

/// Verify `cert`'s signature was produced by `issuer`'s P-384 key over
/// `cert`'s DER-encoded TBS (to-be-signed) bytes — the actual cryptographic
/// check a chain-of-trust requires, not a string comparison.
pub(crate) fn verify_signed_by(cert: &Certificate, issuer: &Certificate) -> Result<(), SignError> {
    use p384::ecdsa::{signature::Verifier as _, DerSignature, VerifyingKey};

    let issuer_key_bytes = issuer
        .tbs_certificate
        .subject_public_key_info
        .subject_public_key
        .raw_bytes();
    let issuer_key =
        VerifyingKey::from_sec1_bytes(issuer_key_bytes).map_err(|_| SignError::Invalid)?;

    let tbs_der = cert
        .tbs_certificate
        .to_der()
        .map_err(|_| SignError::Invalid)?;
    let sig_bytes = cert.signature.as_bytes().ok_or(SignError::Invalid)?;
    let signature = DerSignature::try_from(sig_bytes).map_err(|_| SignError::Invalid)?;

    issuer_key
        .verify(&tbs_der, &signature)
        .map_err(|_| SignError::Invalid)
}

/// Verify the certificate `notBefore`/`notAfter` window against system time.
pub(crate) fn check_cert_validity(cert: &Certificate) -> Result<(), SignError> {
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| SignError::Invalid)?
        .as_secs();

    let nb = cert
        .tbs_certificate
        .validity
        .not_before
        .to_unix_duration()
        .as_secs();
    let na = cert
        .tbs_certificate
        .validity
        .not_after
        .to_unix_duration()
        .as_secs();

    if now_secs < nb || now_secs > na {
        Err(SignError::Invalid)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// p16-c002 regression: the pinned intermediate must genuinely,
    /// cryptographically chain to the pinned root (externally confirmed once
    /// via `openssl verify` before embedding — this test guards against a
    /// future edit silently breaking that relationship). Also confirms
    /// `verify_signed_by` rejects the reverse (root "signed by" intermediate
    /// is not a valid chain direction).
    #[test]
    fn pinned_intermediate_chains_to_pinned_root() {
        let intermediate = Certificate::from_pem(FULCIO_INTERMEDIATE_PEM).expect("intermediate");
        let root = Certificate::from_pem(FULCIO_ROOT_PEM).expect("root");

        assert!(
            verify_signed_by(&intermediate, &root).is_ok(),
            "pinned intermediate must chain to pinned root"
        );
        assert!(
            verify_signed_by(&root, &intermediate).is_err(),
            "root is not signed by the intermediate — wrong direction must fail"
        );
    }
}
