# Vault KMS Architecture Contract

## §1 Architecture: Key Hierarchy

```
KMS (Azure Key Vault / AWS KMS)
  └── Key Encryption Key (KEK) — managed by KMS, never leaves KMS
         └── Data Encryption Key (DEK) — 32 bytes, generated once
                └── Per-category subkeys via HKDF-SHA256
                       └── Encrypted secret ciphertext (XChaCha20-Poly1305)
```

The DEK is:
- Generated once with `openssl rand -out dek.bin 32`
- Wrapped (encrypted) by the KMS KEK using RSA-OAEP-256
- Stored as base64 in a Kubernetes secret (`FLINT_VAULT_DEK_WRAPPED`)
- Unwrapped at pod startup via `FLINT_VAULT_UNWRAP_CMD` shell hook
- Held in-process memory only — NEVER written to Postgres, logs, or disk

## §2 Azure Key Vault Setup

Prerequisites:
- Azure Key Vault instance with a 4096-bit RSA key named `flint-vault-kek`
- Managed identity bound to the Postgres pod with `Key Unwrap` permission

Create the key:
```bash
az keyvault key create \
  --vault-name "$VAULT_NAME" \
  --name flint-vault-kek \
  --kty RSA \
  --size 4096 \
  --ops wrapKey unwrapKey
```

## §3 Wrapped DEK Format

- Algorithm: RSA-OAEP-256 (SHA-256 hash, MGF1-SHA256 mask)
- Input: 32 raw bytes (256-bit DEK)
- Output: RSA key size / 8 = 512 bytes → base64 encoded (~688 chars)
- Env var: `FLINT_VAULT_DEK_WRAPPED=<base64-string>`

## §4 FLINT_VAULT_UNWRAP_CMD

The unwrap command receives the base64-wrapped DEK on stdin and must print the
raw DEK bytes (exactly 32 bytes, NOT base64) to stdout.

Azure Key Vault example:
```bash
#!/bin/bash
# /opt/flint/unwrap-dek.sh
# Reads wrapped DEK (base64) on stdin, writes 32 raw bytes to stdout.
WRAPPED=$(cat)
az keyvault key unwrap \
  --vault-name "$AZURE_VAULT_NAME" \
  --name flint-vault-kek \
  --algorithm RSA-OAEP-256 \
  --value "$WRAPPED" \
  --query result \
  --output tsv | base64 -d
```

Set in pod environment:
```
FLINT_VAULT_UNWRAP_CMD=/opt/flint/unwrap-dek.sh
FLINT_VAULT_DEK_WRAPPED=<base64-wrapped-dek-from-vault-init.sh>
```

Note: the `run_unwrap` implementation in `ext-flint-vault/src/lib.rs` pipes the
wrapped DEK string to the command's stdin; the command must read from stdin (not
`$1`). The script above uses `cat` to read stdin.

## §5 Development / Test Path

For local development and CI, use a raw root key (no KMS):
```bash
export FLINT_VAULT_ROOT_KEY=$(openssl rand -base64 32)
```

When `FLINT_VAULT_ROOT_KEY` is set, the vault skips the KMS unwrap path and uses
the key directly. This mode MUST NOT be used in production.

## §6 Kubernetes Secret Template

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: flint-vault-secrets
  namespace: flint
type: Opaque
stringData:
  FLINT_VAULT_DEK_WRAPPED: "<output of vault-init.sh step 2>"
  FLINT_VAULT_UNWRAP_CMD: "/opt/flint/unwrap-dek.sh"
  AZURE_VAULT_NAME: "<your-vault-name>"
```

Mount as environment variables in the Postgres pod spec:
```yaml
envFrom:
  - secretRef:
      name: flint-vault-secrets
```

## §7 Security Constraints

- Plaintext DEK MUST NOT be stored in any persistent medium
- DEK MUST NOT appear in any log output — verify `FLINT_VAULT_UNWRAP_CMD` output
  is not captured by log aggregators
- HKDF subkeys are derived per-category at runtime — they are never stored
- Access log (`vault.access_log`) records only category + operation, never
  plaintext values or key material
- Only `vault_admin` role may call `vault.create_secret` and `vault.update_secret`
- `flint_secret_reader` role may call `vault.get_secret()` and
  `vault.resolve_api_key()` only
- Postgres statement logging (`log_min_duration_statement`) MUST be disabled or
  filtered on the path that calls `vault.create_secret` — the plaintext secret is
  a function argument and would appear in the Postgres log otherwise

## §8 Key Rotation

### DEK rotation (re-wrap only)

Revoking or rotating the KEK requires only re-wrapping the DEK — no ciphertext
re-encryption:

1. Generate a new KEK version in the KMS
2. Unwrap the current wrapped DEK using the old KEK version (KMS keeps old
   version active during rotation)
3. Re-wrap the plaintext DEK bytes with the new KEK version
4. Update `FLINT_VAULT_DEK_WRAPPED` in the Kubernetes secret
5. Restart Postgres pods to reload the DEK
6. Revoke the old KEK version in the KMS

### Full DEK rotation (re-encrypt all secrets)

Full DEK rotation (replacing the 32-byte DEK itself) requires re-encrypting all
rows in `vault.secrets`. There is no built-in migration path in this version;
treat this as an exceptional operational event and script it using the
`vault.decrypted_secrets` view (accessible only to `vault_admin`) followed by
`vault.update_secret` calls with the new DEK loaded.
