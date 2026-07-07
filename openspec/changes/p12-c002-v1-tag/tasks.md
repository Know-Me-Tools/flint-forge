# p12-c002 Tasks — v1.0.0 Release Tag

## Tasks

- [ ] Add `tags: ['v[0-9]*']` to `on.push` in `.github/workflows/docker.yml`
- [ ] Add `ghcr.io/${{ github.repository_owner }}/flint-gateway:${{ github.ref_name }}` to gateway job `tags:` block
- [ ] Add `ghcr.io/${{ github.repository_owner }}/flint-kiln:${{ github.ref_name }}` to kiln job `tags:` block
- [ ] Bump `[workspace.package] version` from `0.10.0` to `1.0.0` in `Cargo.toml`
- [ ] Run `git cliff --tag v1.0.0 -o CHANGELOG.md`; review generated v1.0.0 section
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` clean
- [ ] `git add -A && git commit -m "chore(release): v1.0.0"`
- [ ] `git tag -a v1.0.0 -m "Flint Forge v1.0.0 — first stable API release"`
- [ ] `git push origin main v1.0.0`
- [ ] Wait for `docker.yml` CI to complete for the `v1.0.0` tag push
- [ ] `git cliff --latest > /tmp/v1-notes.md`
- [ ] `gh release create v1.0.0 --title "Flint Forge v1.0.0 — First Stable API Release" --notes-file /tmp/v1-notes.md --latest`
