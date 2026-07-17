//! Shared helpers for `forge-identity`'s JWKS/audience integration tests
//! (p16-c005). Each test file in this directory compiles to its own process,
//! so the crate's process-global JWKS cache (`forge_identity::jwks`) and the
//! `FLINT_GATE_*` environment variables never leak across test files —
//! deliberately kept to one test per file for that isolation.
#![allow(dead_code)]

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use ring::rand::SystemRandom;
use ring::signature::{EcdsaKeyPair, KeyPair, ECDSA_P256_SHA256_FIXED_SIGNING};
use std::time::{SystemTime, UNIX_EPOCH};

/// A freshly generated ES256 (P-256) keypair, plus its JWK public-key
/// coordinates for building a test JWKS response.
pub struct TestKey {
    pub kid: String,
    pub encoding_key: EncodingKey,
    pub x_b64: String,
    pub y_b64: String,
}

/// Generate a new ES256 keypair with the given `kid`.
pub fn generate_es256_key(kid: &str) -> TestKey {
    let rng = SystemRandom::new();
    let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng)
        .expect("generate P-256 pkcs8 keypair");
    let keypair = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8.as_ref(), &rng)
        .expect("load generated pkcs8 keypair");

    // Uncompressed SEC1 point: 0x04 || X(32 bytes) || Y(32 bytes) for P-256.
    let public = keypair.public_key().as_ref();
    assert_eq!(public.len(), 65, "expected uncompressed P-256 point");
    let x_b64 = URL_SAFE_NO_PAD.encode(&public[1..33]);
    let y_b64 = URL_SAFE_NO_PAD.encode(&public[33..65]);

    TestKey {
        kid: kid.to_string(),
        encoding_key: EncodingKey::from_ec_der(pkcs8.as_ref()),
        x_b64,
        y_b64,
    }
}

/// The JWKS `keys` entry (JSON) for a [`TestKey`]'s public half.
pub fn jwk_json(key: &TestKey) -> serde_json::Value {
    serde_json::json!({
        "kty": "EC",
        "crv": "P-256",
        "kid": key.kid,
        "alg": "ES256",
        "x": key.x_b64,
        "y": key.y_b64,
    })
}

/// A `{"keys": [...]}` JWKS response body from a set of [`TestKey`]s.
pub fn jwks_response(keys: &[&TestKey]) -> serde_json::Value {
    serde_json::json!({ "keys": keys.iter().map(|k| jwk_json(k)).collect::<Vec<_>>() })
}

/// Mint an ES256-signed JWT for `key`, valid for one hour from now.
pub fn make_token(key: &TestKey, iss: &str, aud: Option<&str>, sub: &str) -> String {
    let mut header = Header::new(Algorithm::ES256);
    header.kid = Some(key.kid.clone());

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before epoch")
        .as_secs();

    let mut claims = serde_json::json!({
        "sub": sub,
        "iss": iss,
        "exp": now + 3600,
    });
    if let Some(aud) = aud {
        claims["aud"] = serde_json::json!(aud);
    }

    jsonwebtoken::encode(&header, &claims, &key.encoding_key).expect("encode test JWT")
}
