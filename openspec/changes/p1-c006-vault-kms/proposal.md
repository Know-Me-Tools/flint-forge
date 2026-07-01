# p1-c006 — flint_vault: Azure Key Vault KMS unwrap documentation + example

## Why

`ext-flint-vault` already implements the shell-based KMS unwrap path (`FLINT_VAULT_UNWRAP_CMD`). What's missing is the Azure Key Vault managed identity wiring example, the wrapped DEK format specification, and the Kubernetes secret-injection pattern needed to make the production KMS path operational.

## What

- Write `docs/contracts/vault-kms.md`:
  - Explain the envelope encryption architecture (KEK in Azure Key Vault, wrapped DEK in env/file)
  - Provide a concrete `FLINT_VAULT_UNWRAP_CMD` example using `az keyvault key unwrap`
  - Document the wrapped DEK format: RSA-OAEP-256 wrapped 32-byte key, base64-encoded
  - Provide Kubernetes `Secret` + `Pod` spec for injecting `FLINT_VAULT_DEK_WRAPPED` and managed identity annotation
  - Provide a `openssl rand -base64 32` + `az keyvault key wrap` example for initial DEK creation
- Write `docs/operations/vault-init.sh` — shell script template for DEK creation and wrapping
- Add a pgrx integration test that exercises the dev path (`FLINT_VAULT_ROOT_KEY`): create a secret, retrieve it, verify roundtrip
  - NOTE: The code-level implementation in `ext-flint-vault/src/lib.rs` is complete; this adds test coverage and documentation only

## Contract

`docs/contracts/vault-kms.md` exists with a working Azure example. `cargo pgrx test -p flint_vault` passes the `secret_roundtrip_general` test (already exists) plus a new `api_key_roundtrip` test. The `vault-init.sh` script is executable and documented.

## Out of scope

GCP KMS, AWS KMS, HashiCorp Vault Transit paths — documented as "same shell-command pattern, substitute CLI."

## Constraints

- Never log the plaintext DEK or any secret value in tests or documentation examples
- Test must use `FLINT_VAULT_ROOT_KEY` env var (dev path) — NOT the shell unwrap path in CI (no KMS in CI)
- File size ≤ 500 lines per file

## Reference

- `crates/ext-flint-vault/src/lib.rs` (complete implementation — read the `load_dek()` and `run_unwrap()` functions)
- Azure documentation: `az keyvault key wrap/unwrap`
- Kubernetes workload identity: azure.workload.identity/client-id annotation
