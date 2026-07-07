# p12-c002 — v1.0.0 Release Tag

**Phase:** 12 — v1.0.0 Release  **Priority:** P0  **Depends on:** none

## What this delivers

The `v1.0.0` tag, the updated `CHANGELOG.md`, and the GitHub Release marking
Flint Forge's first stable API release. Also adds version-tag support to
`docker.yml` so `ghcr.io/.../flint-gateway:v1.0.0` and
`ghcr.io/.../flint-kiln:v1.0.0` are published automatically.

## Changes

### 1. `docker.yml` — add version tag trigger

Add `tags: ['v[0-9]*']` to the `on.push` trigger and append the ref-name
tag to each image's `tags:` block:

```yaml
on:
  push:
    branches: [main]
    tags: ['v[0-9]*']          # ← new
```

For each build step, add `ghcr.io/${{ github.repository_owner }}/flint-gateway:${{ github.ref_name }}` to the `tags:` multiline string. `github.ref_name` equals the tag name (e.g., `v1.0.0`) when the trigger is a tag push.

**Note:** `github.ref_name` is only the tag name on tag pushes; on branch pushes
it is the branch name. The existing `latest` and `${{ github.sha }}` tags are
unaffected — they continue to be pushed on every main branch push.

### 2. `Cargo.toml` — version bump

```toml
[workspace.package]
version = "1.0.0"   # was 0.10.0
```

### 3. `CHANGELOG.md` — regenerate for v1.0.0

```bash
git cliff --tag v1.0.0 -o CHANGELOG.md
```

Review the generated section; the `v1.0.0` entry should include:
- `feat(api)`: A2UI + Kiln ABI freeze
- `feat(ops)`: Dockerfile entrypoint secrets wiring
- `feat(perf,ops)`: k6 baseline annotation + staging token rotation
- `fix(security)`: crossbeam-epoch update

### 4. Commit + tag + push

```bash
git add docker.yml Cargo.toml CHANGELOG.md Cargo.lock
git commit -m "chore(release): v1.0.0

First stable API release. All primary surfaces documented and versioned:
- A2UI HTTP API (docs/api/a2ui.md, #[non_exhaustive] enums, FLINT_A2UI_API_VERSION=1)
- Kiln WIT ABI (docs/api/kiln-abi.md, @since annotations, FLINT_KILN_ABI_VERSION=1)
- SDKs at 1.0.0 (@flint/react, flint_genui)
- Dockerfile entrypoints wire secrets from /run/secrets/
- mint_smoke_token.sh replaces static STAGING_SMOKE_TOKEN

Note: k6 performance baselines are TBD pending a live staging run."

git tag -a v1.0.0 -m "Flint Forge v1.0.0 — first stable API release"
git push origin main v1.0.0
```

### 5. GitHub Release

```bash
git cliff --latest 2>/dev/null > /tmp/v1-notes.md
gh release create v1.0.0 \
  --title "Flint Forge v1.0.0 — First Stable API Release" \
  --notes-file /tmp/v1-notes.md \
  --latest
```

After the `docker.yml` CI run completes for the `v1.0.0` tag, extract and
append the image digests to the release body:

```bash
# Run after CI completes — extracts digests from the ghcr.io registry
gh release edit v1.0.0 --notes-file <(
  cat /tmp/v1-notes.md
  echo ""
  echo "## Docker Images"
  echo ""
  echo "| Image | Tag | Digest |"
  echo "|---|---|---|"
  for img in "flint-gateway" "flint-kiln"; do
    digest=$(docker manifest inspect \
      ghcr.io/${{ github.repository_owner }}/${img}:v1.0.0 2>/dev/null | \
      python3 -c "import json,sys; m=json.load(sys.stdin); print(m.get('config',{}).get('digest','pending'))" 2>/dev/null || echo "see CI run")
    echo "| ghcr.io/.../flint-forge/${img} | v1.0.0 | \`${digest}\` |"
  done
)
```
