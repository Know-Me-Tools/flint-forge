# p14-c004 — Staging JWT Rotation Automation

**Phase:** 14 — v1.1.0  **Priority:** P2  **Depends on:** none

## Problem

`scripts/rotate_secrets.sh` regenerates `secrets/jwt_secret.txt` locally but
the GitHub Actions `STAGING_JWT_SECRET` repository secret must be updated
manually via `gh secret set`. This is error-prone and easily forgotten.

## Solution

Create `scripts/rotate_staging_jwt.sh`:

```bash
#!/usr/bin/env bash
# scripts/rotate_staging_jwt.sh — Rotate the staging JWT signing key end-to-end.
#
# 1. Generates a new random signing key
# 2. Writes it to secrets/jwt_secret.txt
# 3. Updates the GitHub Actions STAGING_JWT_SECRET repository secret
# 4. Prints the new key for verification
#
# Usage:
#   ./scripts/rotate_staging_jwt.sh              # rotate + update gh secret
#   ./scripts/rotate_staging_jwt.sh --dry-run    # preview without writing
set -euo pipefail

# ... (implementation: openssl rand, gh secret set STAGING_JWT_SECRET)
```

### Runbook update

Add the rotation procedure to `docs/runbook.md §11` or a new §12.
