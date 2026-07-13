# p14-c004 Tasks — Staging JWT Rotation Automation

## Tasks

- [x] Create `scripts/rotate_staging_jwt.sh` — `openssl rand -hex 32`, writes `secrets/jwt_secret.txt`, calls `gh secret set STAGING_JWT_SECRET`, supports `--dry-run`
- [x] Make executable (`chmod +x`)
- [x] Validate: `bash -n scripts/rotate_staging_jwt.sh`
- [x] Update `scripts/README.md` with `rotate_staging_jwt.sh` section
- [x] Add rotation procedure to `docs/runbook.md` — `docs/runbook.md` §12 "Staging JWT Secret Rotation (p14-c004)"
- [x] `cargo test --workspace` passes (no Rust changes)

<!-- p16-c006 reconcile (2026-07-13): verified against scripts/rotate_staging_jwt.sh, scripts/README.md, docs/runbook.md §12, bash -n. All items confirmed done. -->
