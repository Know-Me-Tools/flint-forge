# p11-c006 Tasks — Staging Token Rotation

## Tasks

- [x] Create `scripts/mint_smoke_token.sh` — self-signed HS256 JWT; reads `$JWT_SECRET` env var, then `secrets/jwt_secret.txt`, then `/run/secrets/jwt_secret`; `exp = now + 3600`
- [x] Make `scripts/mint_smoke_token.sh` executable (`chmod +x`)
- [x] Validate script syntax: `bash -n scripts/mint_smoke_token.sh`
- [x] Test locally (dry run): `JWT_SECRET=test123 ./scripts/mint_smoke_token.sh | cut -d. -f2 | base64 -d 2>/dev/null | python3 -m json.tool` — verify claims shape — produced valid `{"sub":"smoke","role":"authenticated","exp":...,"iat":...}`
- [x] Update `.github/workflows/deploy.yml`: add `Mint smoke token` step using `STAGING_JWT_SECRET` secret; replace static `STAGING_SMOKE_TOKEN` usage with `$SMOKE_TOKEN` — `.github/workflows/deploy.yml:104-117`
- [x] Add `STAGING_JWT_SECRET` to secrets documentation table in `docs/runbook.md §9.1` — `docs/runbook.md:767`
- [x] Remove `STAGING_SMOKE_TOKEN` from the secrets table (replaced by `STAGING_JWT_SECRET`)
- [x] Add `docs/runbook.md §11` covering `mint_smoke_token.sh` usage and the `STAGING_JWT_SECRET` secret — `docs/runbook.md:985-1131`
- [x] Update `scripts/README.md` to document `mint_smoke_token.sh` — `scripts/README.md:183`
- [x] `cargo test --workspace` passes (no Rust changes)

**Minor out-of-scope note (p16-c006 reconcile, not one of this change's own tasks):** `docs/performance.md:77` and `perf/k6/README.md:49` still reference the old `STAGING_SMOKE_TOKEN` secret name in a sentence about the k6 regression CI job — stale but unrelated to this checklist.
