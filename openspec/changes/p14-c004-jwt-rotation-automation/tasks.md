# p14-c004 Tasks — Staging JWT Rotation Automation

## Tasks

- [ ] Create `scripts/rotate_staging_jwt.sh` — `openssl rand -hex 32`, writes `secrets/jwt_secret.txt`, calls `gh secret set STAGING_JWT_SECRET`, supports `--dry-run`
- [ ] Make executable (`chmod +x`)
- [ ] Validate: `bash -n scripts/rotate_staging_jwt.sh`
- [ ] Update `scripts/README.md` with `rotate_staging_jwt.sh` section
- [ ] Add rotation procedure to `docs/runbook.md`
- [ ] `cargo test --workspace` passes (no Rust changes)
