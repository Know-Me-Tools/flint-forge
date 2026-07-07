# Plan — p12-v1-release

**Phase:** 12 — v1.0.0 Release
**Authored:** 2026-07-06
**Change backend:** OpenSpec
**Changes:** 2 total (1 deferred, 1 actionable)
**Seeded from:** `assessment.md`

---

## Change Status

| Change | Priority | Status | Reason |
|---|---|---|---|
| p12-c001-k6-measure | P0 | **DEFERRED** | Live staging stack required; goals.md fallback applied |
| p12-c002-v1-tag | P0 | **EXECUTE** | All prerequisites met; ready immediately |

---

## Ordering

With c001 deferred, there is exactly one actionable change:

```
p12-c002-v1-tag   ← execute immediately
p12-c001-k6-measure  ← execute as post-release operator action when staging is available
```

c001 and c002 are independent — c001 can be executed before or after c002 with
no ordering constraint. Tagging v1.0.0 does not depend on measured baselines.

---

## p12-c002 — v1.0.0 Release Tag (full scope)

**7 file changes + git operations + GitHub Release**

### 1. `.github/workflows/docker.yml` — add version tag trigger

Add `tags: ['v[0-9]*']` to the `on.push` block so `docker.yml` runs on version
tag pushes as well as `main` branch pushes:

```yaml
on:
  push:
    branches: [main]
    tags: ['v[0-9]*']
```

Add `${{ github.ref_name }}` tagged image to both job `tags:` blocks:

```yaml
# gateway job
tags: |
  ghcr.io/${{ github.repository_owner }}/flint-gateway:latest
  ghcr.io/${{ github.repository_owner }}/flint-gateway:${{ github.sha }}
  ghcr.io/${{ github.repository_owner }}/flint-gateway:${{ github.ref_name }}
```

Same pattern for `kiln` job with `flint-kiln`.

**Note:** On branch pushes `github.ref_name` is the branch name (`main`); on
tag pushes it is the tag name (`v1.0.0`). The `latest` + SHA tags are unchanged.

### 2. `Cargo.toml` — `version = "0.10.0"` → `"1.0.0"`

Single-line edit under `[workspace.package]`.

### 3. `CHANGELOG.md` — regenerate via `git cliff`

```bash
git cliff --tag v1.0.0 -o CHANGELOG.md
```

Expected `v1.0.0` section (from verified preview):
- `fix(security)`: crossbeam-epoch update (RUSTSEC-2026-0204)
- `feat(api)`: A2UI + Kiln ABI freeze
- `feat(ops)`: Dockerfile entrypoint secrets wiring
- `feat(perf,ops)`: k6 annotation + staging token rotation

### 4. Gate

```bash
cargo clippy --workspace -- -D warnings   # must be clean
cargo test --workspace                    # 457 tests
```

### 5. Commit + tag + push

```bash
git add -A
git commit -m "chore(release): v1.0.0

First stable API release. All primary surfaces documented and versioned..."
git tag -a v1.0.0 -m "Flint Forge v1.0.0 — first stable API release"
git push origin main v1.0.0
```

### 6. GitHub Release

```bash
git cliff --latest > /tmp/v1-notes.md
gh release create v1.0.0 \
  --title "Flint Forge v1.0.0 — First Stable API Release" \
  --notes-file /tmp/v1-notes.md \
  --latest
```

Release notes will include a note that k6 baselines are TBD pending a staging run.

---

## Build / Quality Gates

```bash
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo test --workspace
cargo audit
```

---

## MVP Gate Checklist

- [ ] `docker.yml` triggers on `v*` tags
- [ ] `[workspace.package] version = "1.0.0"` in `Cargo.toml`
- [ ] `git tag v1.0.0` pushed to origin
- [ ] GitHub Release `v1.0.0` created (with TBD k6 note)
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` clean
