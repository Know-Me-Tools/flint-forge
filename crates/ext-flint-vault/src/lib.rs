//! flint_vault — Flint Forge sovereign secret store (Anvil suite).
//! ----------------------------------------------------------------------------
//! Stores secrets of ANY kind — database passwords, external-service API keys,
//! connection strings, tokens, certificates, and arbitrary secret parameters —
//! encrypted at rest in a normal Postgres table (so they ride along in
//! backups/WAL/replicas as ciphertext only). LLM provider keys are just one
//! consumer. Improves on the Supabase-Vault/pgsodium lineage in four ways:
//!
//!   1. Envelope encryption. The in-memory Data Encryption Key (DEK) is NOT a raw
//!      file — it is unwrapped at postmaster start from a Key Encryption Key (KEK)
//!      that lives in an external KMS (Azure Key Vault via managed identity, AWS
//!      KMS, GCP KMS, Vault Transit, ...). The KEK never enters the DB or process;
//!      revoking it renders every secret cryptographically dead (KMS kill-switch),
//!      and rotating it only rewraps the DEK — no data re-encryption.
//!   2. Typed categories (api_key | password | connection_string | token |
//!      certificate | secret_param), each with a per-category derived subkey so a
//!      category can be rotated independently.
//!   3. The DEK is never selectable from SQL; plaintext is never returned to SQL
//!      clients or WASM sandboxes. Secrets are resolved in-process for trusted
//!      consumers (flint_llm, flint_hooks, flint-gate) via SECURITY DEFINER paths
//!      granted only to a secret-reader role (brokered).
//!   4. Every privileged read is written to an append-only access log.
//!
//! Crypto: XChaCha20-Poly1305 (24-byte nonce, AEAD). The row `id` is bound in as
//! associated data, so a ciphertext copied to another row fails authentication.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use hkdf::Hkdf;
use pgrx::prelude::*;
use rand::RngCore;
use secrecy::{ExposeSecret, Secret};
use sha2::Sha256;
use std::io::Write;
use std::sync::OnceLock;
use zeroize::Zeroize;

pgrx::pg_module_magic!();

const KEY_LEN: usize = 32; // XChaCha20-Poly1305 key / DEK length
const NONCE_LEN: usize = 24; // XChaCha20-Poly1305 (extended) nonce
const HKDF_DOMAIN: &[u8] = b"flint_vault.v1.row-key";

// ---------------------------------------------------------------------------
// Data Encryption Key (DEK) — held in process memory, never exposed to SQL.
//
// Config (postgres server environment):
//   Production (KMS envelope):
//     FLINT_VAULT_UNWRAP_CMD       shell command; reads the wrapped DEK (base64)
//                                  on stdin, writes 32 raw plaintext DEK bytes to
//                                  stdout. Wrap a KMS CLI here, e.g. Azure:
//                                    az keyvault key unwrap --vault-name <v> \
//                                      --name <kek> --algorithm RSA-OAEP-256 ...
//     FLINT_VAULT_DEK_WRAPPED[_FILE]  the wrapped DEK (base64), inline or in a file.
//   Development (no KMS):
//     FLINT_VAULT_ROOT_KEY[_FILE]  base64 of a raw 32-byte DEK. Dev/test only.
// ---------------------------------------------------------------------------

static DEK: OnceLock<Secret<[u8; KEY_LEN]>> = OnceLock::new();

fn read_wrapped_dek() -> Vec<u8> {
    let b64 = std::env::var("FLINT_VAULT_DEK_WRAPPED")
        .ok()
        .or_else(|| {
            std::env::var("FLINT_VAULT_DEK_WRAPPED_FILE")
                .ok()
                .and_then(|p| std::fs::read_to_string(p).ok())
        })
        .unwrap_or_else(|| {
            error!(
                "flint_vault: FLINT_VAULT_UNWRAP_CMD is set but no wrapped DEK was \
                 provided (set FLINT_VAULT_DEK_WRAPPED or FLINT_VAULT_DEK_WRAPPED_FILE)"
            )
        });
    B64.decode(b64.trim())
        .unwrap_or_else(|e| error!("flint_vault: wrapped DEK is not valid base64: {e}"))
}

/// Run the external unwrap command (KEK stays in the KMS). Contract: wrapped DEK
/// (base64) on stdin → exactly 32 raw plaintext bytes on stdout.
fn run_unwrap(cmd: &str, wrapped_b64: &str) -> [u8; KEY_LEN] {
    use std::process::{Command, Stdio};
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| error!("flint_vault: failed to spawn unwrap command: {e}"));
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(wrapped_b64.as_bytes())
        .unwrap_or_else(|e| error!("flint_vault: failed to write to unwrap command: {e}"));
    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| error!("flint_vault: unwrap command did not complete: {e}"));
    if !out.status.success() {
        error!(
            "flint_vault: unwrap command failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let mut dek = out.stdout;
    if dek.len() != KEY_LEN {
        dek.zeroize();
        error!("flint_vault: unwrap command must output exactly {KEY_LEN} raw bytes");
    }
    let mut key = [0u8; KEY_LEN];
    key.copy_from_slice(&dek);
    dek.zeroize();
    key
}

fn load_dek() -> Secret<[u8; KEY_LEN]> {
    // Production path: KMS envelope.
    if let Ok(cmd) = std::env::var("FLINT_VAULT_UNWRAP_CMD") {
        let wrapped = String::from_utf8(read_wrapped_dek())
            .unwrap_or_else(|_| error!("flint_vault: wrapped DEK must be base64 text"));
        return Secret::new(run_unwrap(&cmd, wrapped.trim()));
    }
    // Development path: raw DEK from env/file.
    let raw_b64 = std::env::var("FLINT_VAULT_ROOT_KEY")
        .ok()
        .or_else(|| {
            std::env::var("FLINT_VAULT_ROOT_KEY_FILE")
                .ok()
                .and_then(|p| std::fs::read_to_string(p).ok())
        })
        .unwrap_or_else(|| {
            error!(
                "flint_vault: no key configured. For production set FLINT_VAULT_UNWRAP_CMD \
                 + FLINT_VAULT_DEK_WRAPPED; for dev set FLINT_VAULT_ROOT_KEY (base64 of 32 \
                 bytes). Generate a dev key with: openssl rand -base64 32"
            )
        });
    let mut decoded = B64
        .decode(raw_b64.trim())
        .unwrap_or_else(|e| error!("flint_vault: root key is not valid base64: {e}"));
    if decoded.len() != KEY_LEN {
        decoded.zeroize();
        error!("flint_vault: root key must decode to exactly {KEY_LEN} bytes");
    }
    let mut key = [0u8; KEY_LEN];
    key.copy_from_slice(&decoded);
    decoded.zeroize();
    Secret::new(key)
}

fn dek() -> &'static Secret<[u8; KEY_LEN]> {
    DEK.get_or_init(load_dek)
}

/// Per-(category, key_id) working key = HKDF-SHA256(DEK, info = category || key_id).
fn working_cipher(category: &str, key_id: &[u8; 16]) -> XChaCha20Poly1305 {
    let hk = Hkdf::<Sha256>::new(Some(HKDF_DOMAIN), dek().expose_secret());
    let mut info = Vec::with_capacity(category.len() + 16);
    info.extend_from_slice(category.as_bytes());
    info.extend_from_slice(key_id);
    let mut okm = [0u8; KEY_LEN];
    hk.expand(&info, &mut okm)
        .expect("hkdf expand of fixed length cannot fail");
    let cipher = XChaCha20Poly1305::new(Key::from_slice(&okm));
    okm.zeroize();
    cipher
}

#[inline]
fn uuid_bytes(u: &pgrx::Uuid) -> &[u8; 16] {
    u.as_bytes()
}

// ---------------------------------------------------------------------------
// Internal AEAD primitives. Callable from SQL but locked down (see lockdown SQL);
// they take/return ciphertext + metadata, never the key.
// ---------------------------------------------------------------------------

/// Encrypt `message` for `category`, binding it to row `id` via AEAD associated data.
#[pg_extern(immutable, parallel_safe)]
fn _vault_encrypt(
    message: &str,
    id: pgrx::Uuid,
    key_id: pgrx::Uuid,
    category: &str,
    nonce: Vec<u8>,
) -> String {
    if nonce.len() != NONCE_LEN {
        error!("flint_vault: nonce must be {NONCE_LEN} bytes");
    }
    let cipher = working_cipher(category, uuid_bytes(&key_id));
    let ct = cipher
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: message.as_bytes(),
                aad: uuid_bytes(&id),
            },
        )
        .unwrap_or_else(|_| error!("flint_vault: encryption failed"));
    B64.encode(ct)
}

/// Decrypt a value produced by `_vault_encrypt`. Fails if id/key_id/category/nonce
/// do not match what was used to encrypt (including a ciphertext moved between rows).
#[pg_extern(immutable, parallel_safe)]
fn _vault_decrypt(
    ciphertext: &str,
    id: pgrx::Uuid,
    key_id: pgrx::Uuid,
    category: &str,
    nonce: Vec<u8>,
) -> String {
    if nonce.len() != NONCE_LEN {
        error!("flint_vault: nonce must be {NONCE_LEN} bytes");
    }
    let raw = B64
        .decode(ciphertext)
        .unwrap_or_else(|e| error!("flint_vault: stored secret is not valid base64: {e}"));
    let cipher = working_cipher(category, uuid_bytes(&key_id));
    let pt = cipher
        .decrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: &raw,
                aad: uuid_bytes(&id),
            },
        )
        .unwrap_or_else(|_| {
            error!("flint_vault: decryption/authentication failed (wrong key, tampered data, or mismatched row)")
        });
    String::from_utf8(pt).unwrap_or_else(|_| error!("flint_vault: decrypted secret is not valid UTF-8"))
}

/// Fresh 24-byte CSPRNG nonce.
#[pg_extern(volatile, parallel_safe)]
fn _vault_gen_nonce() -> Vec<u8> {
    let mut nonce = vec![0u8; NONCE_LEN];
    rand::rngs::OsRng.fill_bytes(&mut nonce);
    nonce
}

// ---------------------------------------------------------------------------
// _PG_init: at postmaster start, if any key config is present, eagerly load the
// DEK so a misconfiguration (bad KMS unwrap, wrong length) fails fast and loud
// rather than on first secret access. Keyless load (e.g. building the image with
// no KMS yet) is allowed; first real use will then error clearly.
// ---------------------------------------------------------------------------

#[pg_guard]
pub extern "C-unwind" fn _PG_init() {
    let configured = std::env::var_os("FLINT_VAULT_UNWRAP_CMD").is_some()
        || std::env::var_os("FLINT_VAULT_ROOT_KEY").is_some()
        || std::env::var_os("FLINT_VAULT_ROOT_KEY_FILE").is_some();
    if configured {
        let _ = dek();
    }
}

// ---------------------------------------------------------------------------
// Public API: typed schema, audit log, decrypt-on-read view, typed creators, and
// the brokered secret resolvers. Emitted after the primitives via `requires`.
// ---------------------------------------------------------------------------

extension_sql!(
    r#"
CREATE TYPE vault.secret_category AS ENUM (
    'api_key',            -- keys to any external service (LLM providers, Stripe, SendGrid, ...)
    'password',           -- database and service-account passwords
    'connection_string',  -- DSNs / connection URLs that embed credentials
    'token',              -- OAuth / refresh tokens, webhook signing secrets
    'certificate',        -- private keys / certs (PEM)
    'secret_param'        -- arbitrary secret configuration values
);

CREATE TABLE vault.secrets (
    id          uuid                  PRIMARY KEY DEFAULT gen_random_uuid(),
    category    vault.secret_category NOT NULL,
    name        text                  NOT NULL,
    description text                  NOT NULL DEFAULT '',
    provider    text,            -- for api_key: 'anthropic','stripe',...; the target system
                                 -- for password/connection_string; else NULL
    scope       text,            -- optional tenant/environment scope
    secret      text                  NOT NULL,  -- base64 ciphertext
    key_id      uuid                  NOT NULL DEFAULT '00000000-0000-0000-0000-000000000001',
    nonce       bytea                 NOT NULL,
    created_at  timestamptz           NOT NULL DEFAULT now(),
    updated_at  timestamptz           NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX vault_secrets_category_name_scope_uidx
    ON vault.secrets (category, name, COALESCE(scope, ''));
-- One active api_key per (provider, scope) — applies only to provider-keyed services.
CREATE UNIQUE INDEX vault_secrets_provider_key
    ON vault.secrets (provider, COALESCE(scope, ''))
    WHERE category = 'api_key' AND provider IS NOT NULL;
COMMENT ON TABLE vault.secrets IS
    'Encrypted-at-rest secrets of any kind — database passwords, external-service API '
    'keys, connection strings, tokens, certificates, and secret parameters. The secret '
    'column is ciphertext; backups/WAL/replicas carry ciphertext only.';

-- Append-only audit of every privileged read/write.
CREATE TABLE vault.access_log (
    id        bigserial   PRIMARY KEY,
    at        timestamptz NOT NULL DEFAULT now(),
    actor     text        NOT NULL DEFAULT current_user,
    action    text        NOT NULL,   -- 'create' | 'update' | 'get' | 'resolve' | 'reveal'
    secret_id uuid,
    allowed   boolean     NOT NULL DEFAULT true,
    detail    text        NOT NULL DEFAULT ''
);

-- Decrypt-on-read. Guard as tightly as the secrets themselves (revoked below).
CREATE VIEW vault.decrypted_secrets AS
    SELECT s.id, s.category, s.name, s.description, s.provider, s.scope,
           vault._vault_decrypt(s.secret, s.id, s.key_id, s.category::text, s.nonce)
               AS decrypted_secret,
           s.created_at, s.updated_at
    FROM vault.secrets s;

CREATE FUNCTION vault.create_secret(
    new_category    vault.secret_category,
    new_secret      text,
    new_name        text,
    new_description text DEFAULT '',
    new_provider    text DEFAULT NULL,
    new_scope       text DEFAULT NULL
) RETURNS uuid
LANGUAGE plpgsql SECURITY DEFINER SET search_path = vault, public AS $$
DECLARE
    rec_id uuid  := gen_random_uuid();
    kid    uuid  := '00000000-0000-0000-0000-000000000001';
    n      bytea := vault._vault_gen_nonce();
BEGIN
    INSERT INTO vault.secrets (id, category, name, description, provider, scope, secret, key_id, nonce)
    VALUES (rec_id, new_category, new_name, COALESCE(new_description, ''), new_provider, new_scope,
            vault._vault_encrypt(new_secret, rec_id, kid, new_category::text, n), kid, n);
    INSERT INTO vault.access_log (action, secret_id, detail)
        VALUES ('create', rec_id, new_category::text || ':' || new_name);
    RETURN rec_id;
END;
$$;

CREATE FUNCTION vault.update_secret(
    secret_id       uuid,
    new_secret      text DEFAULT NULL,
    new_name        text DEFAULT NULL,
    new_description text DEFAULT NULL
) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path = vault, public AS $$
DECLARE
    cat vault.secret_category;
    kid uuid;
    n   bytea := vault._vault_gen_nonce();
BEGIN
    SELECT category, key_id INTO cat, kid FROM vault.secrets WHERE id = secret_id;
    IF kid IS NULL THEN RAISE EXCEPTION 'flint_vault: secret % not found', secret_id; END IF;
    UPDATE vault.secrets SET
        name        = COALESCE(new_name, name),
        description = COALESCE(new_description, description),
        secret      = CASE WHEN new_secret IS NULL THEN secret
                           ELSE vault._vault_encrypt(new_secret, secret_id, kid, cat::text, n) END,
        nonce       = CASE WHEN new_secret IS NULL THEN nonce ELSE n END,
        updated_at  = now()
    WHERE id = secret_id;
    INSERT INTO vault.access_log (action, secret_id) VALUES ('update', secret_id);
END;
$$;

-- Brokered secret resolution. Both functions return plaintext ONLY to roles granted
-- EXECUTE (never PUBLIC), log every access, and are the in-process path used by
-- flint_llm, flint_hooks, flint-gate, and the DB itself. WASM edge components never
-- reach these — they call flint:llm or a host-mediated outbound and the host injects
-- the secret at the boundary (brokered), or use the gated flint:secrets reveal path.

-- get_secret(name, [scope]) -> plaintext for any secret, by name.
CREATE FUNCTION vault.get_secret(want_name text, want_scope text DEFAULT NULL)
RETURNS text
LANGUAGE plpgsql SECURITY DEFINER SET search_path = vault, public AS $$
DECLARE r vault.secrets; out text;
BEGIN
    SELECT * INTO r FROM vault.secrets
        WHERE name = want_name AND COALESCE(scope, '') = COALESCE(want_scope, '')
        LIMIT 1;
    IF r.id IS NULL THEN
        INSERT INTO vault.access_log (action, allowed, detail)
            VALUES ('get', false, 'no secret named ' || want_name);
        RAISE EXCEPTION 'flint_vault: no secret named %', want_name;
    END IF;
    out := vault._vault_decrypt(r.secret, r.id, r.key_id, r.category::text, r.nonce);
    INSERT INTO vault.access_log (action, secret_id, detail) VALUES ('get', r.id, want_name);
    RETURN out;
END;
$$;

-- resolve_api_key(provider, [scope]) -> plaintext api_key for a provider-keyed service
-- (LLM providers, Stripe, ...). Convenience over get_secret for the api_key category.
CREATE FUNCTION vault.resolve_api_key(want_provider text, want_scope text DEFAULT NULL)
RETURNS text
LANGUAGE plpgsql SECURITY DEFINER SET search_path = vault, public AS $$
DECLARE r vault.secrets; out text;
BEGIN
    SELECT * INTO r FROM vault.secrets
        WHERE category = 'api_key' AND provider = want_provider
          AND COALESCE(scope, '') = COALESCE(want_scope, '')
        LIMIT 1;
    IF r.id IS NULL THEN
        INSERT INTO vault.access_log (action, allowed, detail)
            VALUES ('resolve', false, 'no api_key for provider=' || want_provider);
        RAISE EXCEPTION 'flint_vault: no api_key for provider %', want_provider;
    END IF;
    out := vault._vault_decrypt(r.secret, r.id, r.key_id, 'api_key', r.nonce);
    INSERT INTO vault.access_log (action, secret_id, detail)
        VALUES ('resolve', r.id, 'provider=' || want_provider);
    RETURN out;
END;
$$;

-- Kiln-facing gated reveal path (§4.4 / flint:host/secrets.reveal). The Rust-side
-- Cedar check (kiln:capability:secrets + a per-secret grant) happens BEFORE this
-- function is ever called — see fke-runtime's `secrets::HostSecret::reveal`. This
-- function trusts that decision and only decrypts + audits; it does not re-evaluate
-- Cedar itself (Postgres has no Cedar evaluator). Distinct from get_secret/
-- resolve_api_key, which are internal-only and never reachable from a WASM
-- component even indirectly.
CREATE FUNCTION vault.reveal_for_kiln(want_name text, publisher_did text, want_scope text DEFAULT NULL)
RETURNS text
LANGUAGE plpgsql SECURITY DEFINER SET search_path = vault, public AS $$
DECLARE r vault.secrets; out text;
BEGIN
    SELECT * INTO r FROM vault.secrets
        WHERE name = want_name AND COALESCE(scope, '') = COALESCE(want_scope, '')
        LIMIT 1;
    IF r.id IS NULL THEN
        INSERT INTO vault.access_log (action, allowed, detail)
            VALUES ('reveal', false, 'kiln:' || publisher_did || ' no secret named ' || want_name);
        RAISE EXCEPTION 'flint_vault: no secret named %', want_name;
    END IF;
    out := vault._vault_decrypt(r.secret, r.id, r.key_id, r.category::text, r.nonce);
    INSERT INTO vault.access_log (action, secret_id, detail)
        VALUES ('reveal', r.id, 'kiln:' || publisher_did);
    RETURN out;
END;
$$;

-- ---- Lockdown: no plaintext, no primitives, no key to PUBLIC. ----
REVOKE ALL ON ALL TABLES    IN SCHEMA vault FROM PUBLIC;
REVOKE ALL ON ALL FUNCTIONS IN SCHEMA vault FROM PUBLIC;
REVOKE ALL ON vault.decrypted_secrets FROM PUBLIC;

-- Secret-reader role: the brokered read path. Granted to trusted in-process consumers
-- (flint_llm worker, flint_hooks, flint-gate's service role) — never to PUBLIC.
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'flint_secret_reader') THEN
        CREATE ROLE flint_secret_reader NOLOGIN;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'flint_llm_worker') THEN
        CREATE ROLE flint_llm_worker NOLOGIN;
    END IF;
END $$;
GRANT USAGE ON SCHEMA vault TO flint_secret_reader;
GRANT EXECUTE ON FUNCTION vault.get_secret(text, text)      TO flint_secret_reader;
GRANT EXECUTE ON FUNCTION vault.resolve_api_key(text, text) TO flint_secret_reader;
GRANT flint_secret_reader TO flint_llm_worker;  -- flint_llm is one consumer among many

-- Kiln worker role: the ONLY role that may call the gated reveal path. Deliberately
-- NOT granted flint_secret_reader membership — Kiln components never get get_secret/
-- resolve_api_key, only the audited, Cedar-fronted reveal_for_kiln.
--
-- NOTE: fke-server's Kiln invocation connections currently run as whichever role
-- the invocation's RlsContext carries (the caller's own role for direct HTTP calls,
-- or "kiln_publisher" for BGW-triggered calls per kiln_bgw.rs — a role that, as of
-- this migration, no `CREATE ROLE` statement anywhere in this repo actually creates).
-- `GRANT flint_kiln_worker TO <that role>;` still needs to land wherever that role
-- is eventually created for `vault.reveal_for_kiln` to be reachable end-to-end.
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'flint_kiln_worker') THEN
        CREATE ROLE flint_kiln_worker NOLOGIN;
    END IF;
END $$;
GRANT USAGE ON SCHEMA vault TO flint_kiln_worker;
GRANT EXECUTE ON FUNCTION vault.reveal_for_kiln(text, text, text) TO flint_kiln_worker;

-- Writes go through a trusted host (flint-gate/Tauri/Axum) running as this role;
-- statement logging MUST be disabled on that path (plaintext is a function argument).
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'vault_admin') THEN
        CREATE ROLE vault_admin NOLOGIN;
    END IF;
END $$;
GRANT USAGE ON SCHEMA vault TO vault_admin;
GRANT EXECUTE ON FUNCTION vault.create_secret(vault.secret_category, text, text, text, text, text) TO vault_admin;
GRANT EXECUTE ON FUNCTION vault.update_secret(uuid, text, text, text) TO vault_admin;
"#,
    name = "vault_api",
    requires = [_vault_encrypt, _vault_decrypt, _vault_gen_nonce],
);

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    fn secret_roundtrip_general() {
        // A database password (provider-less), retrieved by name.
        Spi::run(
            "SELECT vault.create_secret('password','pg-prod-pw-9!','main-postgres',
             'primary db','main-postgres',NULL)",
        )
        .unwrap();
        let pw: String = Spi::get_one("SELECT vault.get_secret('main-postgres')").unwrap().unwrap();
        assert_eq!(pw, "pg-prod-pw-9!");

        // An external-service API key, retrieved by provider.
        Spi::run(
            "SELECT vault.create_secret('api_key','sk-test-123','anthropic prod',
             'note','anthropic',NULL)",
        )
        .unwrap();
        let key: String =
            Spi::get_one("SELECT vault.resolve_api_key('anthropic')").unwrap().unwrap();
        assert_eq!(key, "sk-test-123");
    }

    #[pg_test]
    fn api_key_roundtrip_by_provider() {
        // Store an API key for the OpenAI provider and retrieve it by provider name.
        // This exercises the (category, provider, scope) unique index and the
        // resolve_api_key brokered read path independently of secret_roundtrip_general.
        Spi::run(
            "SELECT vault.create_secret('api_key','sk-openai-test-key-12345','openai prod key',
             'openai integration','openai',NULL)",
        )
        .unwrap();

        let resolved: String =
            Spi::get_one("SELECT vault.resolve_api_key('openai')").unwrap().unwrap();
        assert_eq!(resolved, "sk-openai-test-key-12345");

        // Confirm that resolving an unknown provider raises an exception (the
        // function inserts a denied access_log row and calls RAISE EXCEPTION).
        let miss = Spi::get_one::<String>(
            "SELECT vault.resolve_api_key('nonexistent-provider')",
        );
        assert!(miss.is_err(), "expected an error for unknown provider, got: {miss:?}");
    }
}

#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {}
    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![]
    }
}
