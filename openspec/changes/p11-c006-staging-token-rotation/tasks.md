# p11-c006 Tasks — Staging Token Rotation

## Tasks

- [ ] Create `scripts/mint_smoke_token.sh` — self-signed HS256 JWT; reads `$JWT_SECRET` env var, then `secrets/jwt_secret.txt`, then `/run/secrets/jwt_secret`; `exp = now + 3600`
- [ ] Make `scripts/mint_smoke_token.sh` executable (`chmod +x`)
- [ ] Validate script syntax: `bash -n scripts/mint_smoke_token.sh`
- [ ] Test locally (dry run): `JWT_SECRET=test123 ./scripts/mint_smoke_token.sh | cut -d. -f2 | base64 -d 2>/dev/null | python3 -m json.tool` — verify claims shape
- [ ] Update `.github/workflows/deploy.yml`: add `Mint smoke token` step using `STAGING_JWT_SECRET` secret; replace static `STAGING_SMOKE_TOKEN` usage with `$SMOKE_TOKEN`
- [ ] Add `STAGING_JWT_SECRET` to secrets documentation table in `docs/runbook.md §9.1`
- [ ] Remove `STAGING_SMOKE_TOKEN` from the secrets table (replaced by `STAGING_JWT_SECRET`)
- [ ] Add `docs/runbook.md §11` covering `mint_smoke_token.sh` usage and the `STAGING_JWT_SECRET` secret
- [ ] Update `scripts/README.md` to document `mint_smoke_token.sh`
- [ ] `cargo test --workspace` passes (no Rust changes)
