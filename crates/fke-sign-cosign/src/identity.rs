//! Fulcio OIDC identity extraction and operator-configurable allowlisting.
//!
//! Fulcio embeds the verified OIDC issuer and subject identity into
//! Sigstore-specific X.509v3 extensions (and the standard SAN extension) on
//! every leaf certificate it issues — see the [Fulcio OID directory]. Without
//! this, any identity Fulcio was willing to issue a certificate for (via any
//! OIDC provider it federates with) is accepted as long as the cert
//! cryptographically chains and its SCT verifies; there is no way to
//! restrict Kiln to, say, a specific GitHub org/repo or email domain.
//!
//! This module extracts that identity — issuer from extension
//! `1.3.6.1.4.1.57264.1.8` (V2, DER-encoded UTF8String), falling back to the
//! deprecated `1.3.6.1.4.1.57264.1.1` (V1, raw string bytes) — and subject
//! from the SAN extension's `rfc822Name` (email-based OIDC identities) or
//! `uniformResourceIdentifier` (CI/CD workload identities, e.g. a GitHub
//! Actions workflow ref). This mirrors what `cosign --certificate-identity`
//! / `--certificate-oidc-issuer` match against.
//!
//! If `FLINT_COSIGN_IDENTITY_ALLOWLIST` is unset, any identity is accepted —
//! matching prior behavior. Configure the env var to restrict Kiln to
//! specific signers.
//!
//! [Fulcio OID directory]: https://github.com/sigstore/fulcio/blob/main/docs/oid-info.md

use const_oid::ObjectIdentifier;
use fke_ports::SignError;
use x509_cert::der::{asn1::Utf8StringRef, Decode};
use x509_cert::ext::pkix::{name::GeneralName, SubjectAltName};
use x509_cert::Certificate;

/// Fulcio "Issuer (deprecated)" extension: raw string bytes, no DER wrapping.
const OID_ISSUER_V1: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.6.1.4.1.57264.1.1");
/// Fulcio "Issuer (V2)" extension: DER-encoded UTF8String.
const OID_ISSUER_V2: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.6.1.4.1.57264.1.8");

/// Comma-separated `issuer|subject` allowlist entries. `subject` may end in
/// `*` for a prefix match (e.g. a GitHub org/repo prefix). Example:
/// `FLINT_COSIGN_IDENTITY_ALLOWLIST="https://token.actions.githubusercontent.com|https://github.com/my-org/*,https://accounts.google.com|releases@my-domain.com"`
const ALLOWLIST_ENV: &str = "FLINT_COSIGN_IDENTITY_ALLOWLIST";

/// The (issuer, subject) identity Fulcio embedded in a leaf certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SignerIdentity {
    pub(crate) issuer: String,
    pub(crate) subject: String,
}

struct AllowedEntry {
    issuer: String,
    subject: String,
    subject_is_prefix: bool,
}

impl AllowedEntry {
    fn matches(&self, identity: &SignerIdentity) -> bool {
        if self.issuer != identity.issuer {
            return false;
        }
        if self.subject_is_prefix {
            identity.subject.starts_with(&self.subject)
        } else {
            self.subject == identity.subject
        }
    }
}

/// Extract `leaf`'s embedded identity and, if `FLINT_COSIGN_IDENTITY_ALLOWLIST`
/// is configured, reject it unless it matches an entry. Unconfigured means
/// no restriction (prior behavior).
pub(crate) fn verify_identity_allowlist(leaf: &Certificate) -> Result<(), SignError> {
    let Some(allowlist) = load_allowlist() else {
        return Ok(());
    };

    let identity = extract_identity(leaf)?;
    if allowlist.iter().any(|entry| entry.matches(&identity)) {
        Ok(())
    } else {
        Err(SignError::Invalid)
    }
}

fn load_allowlist() -> Option<Vec<AllowedEntry>> {
    let raw = std::env::var(ALLOWLIST_ENV).ok()?;
    if raw.trim().is_empty() {
        return None;
    }
    Some(
        raw.split(',')
            .filter_map(|entry| {
                let (issuer, subject) = entry.trim().split_once('|')?;
                let (subject, subject_is_prefix) = match subject.strip_suffix('*') {
                    Some(prefix) => (prefix, true),
                    None => (subject, false),
                };
                Some(AllowedEntry {
                    issuer: issuer.trim().to_owned(),
                    subject: subject.trim().to_owned(),
                    subject_is_prefix,
                })
            })
            .collect(),
    )
}

/// Extract the OIDC issuer (preferring the V2 extension) and SAN-based
/// subject identity embedded in `leaf` by Fulcio.
pub(crate) fn extract_identity(leaf: &Certificate) -> Result<SignerIdentity, SignError> {
    let issuer = extract_issuer(leaf).ok_or(SignError::Invalid)?;
    let subject = extract_subject(leaf).ok_or(SignError::Invalid)?;
    Ok(SignerIdentity { issuer, subject })
}

fn extract_issuer(leaf: &Certificate) -> Option<String> {
    extract_der_utf8_extension(leaf, OID_ISSUER_V2)
        .or_else(|| extract_raw_string_extension(leaf, OID_ISSUER_V1))
}

fn extract_subject(leaf: &Certificate) -> Option<String> {
    let (_, san) = leaf.tbs_certificate.get::<SubjectAltName>().ok()??;
    san.0.into_iter().find_map(|name| match name {
        GeneralName::Rfc822Name(email) => Some(email.as_str().to_owned()),
        GeneralName::UniformResourceIdentifier(uri) => Some(uri.as_str().to_owned()),
        _ => None,
    })
}

fn find_extension_bytes(leaf: &Certificate, oid: ObjectIdentifier) -> Option<&[u8]> {
    leaf.tbs_certificate
        .extensions
        .as_deref()?
        .iter()
        .find(|ext| ext.extn_id == oid)
        .map(|ext| ext.extn_value.as_bytes())
}

/// `1.3.6.1.4.1.57264.1.1`-style: `extnValue` is the raw UTF-8 bytes
/// directly, with no nested DER string tag, per Fulcio's OID doc.
fn extract_raw_string_extension(leaf: &Certificate, oid: ObjectIdentifier) -> Option<String> {
    let bytes = find_extension_bytes(leaf, oid)?;
    std::str::from_utf8(bytes).ok().map(str::to_owned)
}

/// `1.3.6.1.4.1.57264.1.8`+-style: `extnValue` is a DER-encoded UTF8String.
fn extract_der_utf8_extension(leaf: &Certificate, oid: ObjectIdentifier) -> Option<String> {
    let bytes = find_extension_bytes(leaf, oid)?;
    Utf8StringRef::from_der(bytes).ok().map(|s| s.as_str().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sct::tests::REAL_FULCIO_LEAF_PEM;
    use x509_cert::der::DecodePem;

    fn real_leaf() -> Certificate {
        Certificate::from_pem(REAL_FULCIO_LEAF_PEM).expect("real leaf")
    }

    /// Extraction against a real, live Fulcio-issued certificate: issuer via
    /// the V2 extension, subject via the URI SAN (a workload-identity
    /// federation URI in this fixture, structurally identical to how a
    /// GitHub Actions workflow ref would appear).
    #[test]
    fn extracts_real_issuer_and_subject() {
        let identity = extract_identity(&real_leaf()).expect("identity");
        assert_eq!(identity.issuer, "https://issuer.enforce.dev");
        assert_eq!(
            identity.subject,
            "https://issuer.enforce.dev/598a0e86be2fee60d1c2fb8d858552f59d9c5111/cb21e2e2a7b411f1"
        );
    }

    /// V1 raw-string issuer extraction path, isolated from the V2 extension,
    /// against the same real certificate (it carries both).
    #[test]
    fn extracts_v1_raw_string_issuer_directly() {
        let issuer = extract_raw_string_extension(&real_leaf(), OID_ISSUER_V1);
        assert_eq!(issuer.as_deref(), Some("https://issuer.enforce.dev"));
    }

    /// `cargo test` runs tests in parallel within one process, and
    /// `std::env::var` is process-global — every test below that touches
    /// `ALLOWLIST_ENV` must hold this lock for its entire set/read/unset
    /// sequence or they race and flip each other's results.
    static ALLOWLIST_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Set `ALLOWLIST_ENV`, run `verify_identity_allowlist` against the real
    /// fixture leaf, then unset it — all while holding the shared lock.
    fn check_with_allowlist(value: Option<&str>) -> Result<(), SignError> {
        let _guard = ALLOWLIST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match value {
            Some(v) => std::env::set_var(ALLOWLIST_ENV, v),
            None => std::env::remove_var(ALLOWLIST_ENV),
        }
        let result = verify_identity_allowlist(&real_leaf());
        std::env::remove_var(ALLOWLIST_ENV);
        result
    }

    /// No allowlist configured — any identity is accepted (prior behavior).
    #[test]
    fn unconfigured_allowlist_accepts_any_identity() {
        assert!(check_with_allowlist(None).is_ok());
    }

    /// Exact issuer+subject match is accepted.
    #[test]
    fn exact_match_is_accepted() {
        let result = check_with_allowlist(Some(
            "https://issuer.enforce.dev|https://issuer.enforce.dev/598a0e86be2fee60d1c2fb8d858552f59d9c5111/cb21e2e2a7b411f1",
        ));
        assert!(result.is_ok());
    }

    /// A prefix (`*`) subject pattern matches.
    #[test]
    fn prefix_match_is_accepted() {
        let result = check_with_allowlist(Some(
            "https://issuer.enforce.dev|https://issuer.enforce.dev/598a0e86be2fee60d1c2fb8d858552f59d9c5111/*",
        ));
        assert!(result.is_ok());
    }

    /// A configured allowlist rejects an identity that isn't on it.
    #[test]
    fn non_matching_identity_is_rejected() {
        let result = check_with_allowlist(Some(
            "https://token.actions.githubusercontent.com|https://github.com/some-other-org/*",
        ));
        assert!(matches!(result, Err(SignError::Invalid)));
    }

    /// Matching subject but wrong issuer is rejected — both must match.
    #[test]
    fn subject_match_with_wrong_issuer_is_rejected() {
        let result = check_with_allowlist(Some(
            "https://not-the-real-issuer.example|https://issuer.enforce.dev/598a0e86be2fee60d1c2fb8d858552f59d9c5111/cb21e2e2a7b411f1",
        ));
        assert!(matches!(result, Err(SignError::Invalid)));
    }
}
