# p12-c002 Tasks — v1.0.0 Release Tag

## Tasks

- [x] Add `tags: ['v[0-9]*']` to `on.push` in `.github/workflows/docker.yml`
- [x] Add `ghcr.io/${{ github.repository_owner }}/flint-gateway:${{ github.ref_name }}` to gateway job `tags:` block — p16-c006 reconcile note: ships as `${{ steps.repo.outputs.owner }}` (a lower-cased repo-owner step), not the raw `github.repository_owner` — a necessary fix since ghcr.io requires lowercase names, landed AFTER the v1.0.0 tag (see the CI-failure gap below)
- [x] Add `ghcr.io/${{ github.repository_owner }}/flint-kiln:${{ github.ref_name }}` to kiln job `tags:` block — same lowercasing note as above
- [x] Bump `[workspace.package] version` from `0.10.0` to `1.0.0` in `Cargo.toml`
- [x] Run `git cliff --tag v1.0.0 -o CHANGELOG.md`; review generated v1.0.0 section
- [x] `cargo test --workspace` passes
- [x] `cargo clippy --workspace -- -D warnings` clean
- [x] `git add -A && git commit -m "chore(release): v1.0.0"` — commit `310e7f6`
- [x] `git tag -a v1.0.0 -m "Flint Forge v1.0.0 — first stable API release"`
- [x] `git push origin main v1.0.0`
- [x] Wait for `docker.yml` CI to complete for the `v1.0.0` tag push — completed, but see the still-open gap below
- [x] `git cliff --latest > /tmp/v1-notes.md` — ephemeral scratch file, not independently verifiable, but its downstream output (the GitHub Release body) is verified below
- [x] `gh release create v1.0.0 --title "Flint Forge v1.0.0 — First Stable API Release" --notes-file /tmp/v1-notes.md --latest` — confirmed published, non-draft, 2026-07-07T12:03:23Z

## Still-open debt (p16-c006 reconcile, 2026-07-13)

- [ ] The `docker.yml` CI run triggered by the `v1.0.0` tag push actually **FAILED** (run `28864604465`, both `fdb-gateway` and `fke-server` jobs: `invalid tag "ghcr.io/.../flint-kiln:latest": repository name must be lowercase`). The lowercasing fix landed in a *later* commit (`df58380 fix(ci): lowercase ghcr.io repository owner in Docker workflow`), which is not part of the `v1.0.0` tag. **No `ghcr.io/.../flint-gateway:v1.0.0` or `flint-kiln:v1.0.0` image was ever actually published** — contradicting this change's headline deliverable. Recommend re-tagging (e.g. `v1.0.1`) or manually publishing the `v1.0.0` images from the fixed workflow if that tag's images are still needed.

<!-- p16-c006 reconcile (2026-07-13): verified against .github/workflows/docker.yml, Cargo.toml, CHANGELOG.md, git log/tag, `gh run view`/`gh release view`. One genuine, real-world-impacting gap found (v1.0.0 Docker images never published) and tracked above rather than rubber-stamped. -->
