# p1-c006 — Tasks

- [ ] Write `docs/contracts/vault-kms.md`:
  - [ ] §1 Architecture: KEK → wrapped DEK → in-process plaintext DEK (never to SQL)
  - [ ] §2 Azure Key Vault: managed identity annotation, `az keyvault key wrap` command, env var format
  - [ ] §3 Wrapped DEK format: RSA-OAEP-256, base64, expected size
  - [ ] §4 FLINT_VAULT_UNWRAP_CMD: exact example with `az keyvault key unwrap --vault-name ... --name ... --algorithm RSA-OAEP-256 --value $1` pattern
  - [ ] §5 Dev/test path: `FLINT_VAULT_ROOT_KEY=$(openssl rand -base64 32)`
  - [ ] §6 Kubernetes secret injection template
- [ ] Write `docs/operations/vault-init.sh`:
  - [ ] Step 1: Generate 32-byte DEK: `openssl rand -bin 32 > dek.bin`
  - [ ] Step 2: Wrap DEK with Azure KV: `az keyvault key wrap --vault-name $VAULT --name $KEY --algorithm RSA-OAEP-256 --value $(base64 dek.bin)`
  - [ ] Step 3: Store wrapped DEK in Kubernetes secret
  - [ ] Cleanup: `shred -u dek.bin`
- [ ] Add pgrx `#[pg_test]` for `api_key_roundtrip`: create api_key secret for 'openai', resolve by provider, verify value
- [ ] Verify existing `secret_roundtrip_general` test passes: `cargo pgrx test -p flint_vault --features pg18`
- [ ] GATE: both vault tests pass; docs/contracts/vault-kms.md exists and reviewed

## Notes

- The `ext-flint-vault` Rust implementation is already complete (70% of this change is documentation)
- Use `FLINT_VAULT_ROOT_KEY` env var in test setup: `std::env::set_var("FLINT_VAULT_ROOT_KEY", base64_of_test_key)`
- Verify `secret_roundtrip_general` test still passes after any additions (it creates both a password and an api_key)
