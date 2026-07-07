# p10-c006 Tasks — CHANGELOG + `v0.10.0` Release Tag

## Tasks

- [ ] Bump `[workspace.package] version` from `0.1.0` to `0.10.0` in `Cargo.toml`
- [ ] Create `cliff.toml` at workspace root with conventional-commit configuration
- [ ] Install git-cliff: `cargo install git-cliff --locked`
- [ ] Generate `CHANGELOG.md`: `git cliff -o CHANGELOG.md`
- [ ] Review generated CHANGELOG.md; edit any entries that need clarification
- [ ] Commit version bump + CHANGELOG: `git commit -am "chore(release): v0.10.0"`
- [ ] Create signed tag: `git tag -s v0.10.0 -m "Flint Forge v0.10.0 — production hardening complete"`
- [ ] Push tag: `git push origin v0.10.0`
- [ ] Trigger `docker.yml` workflow to publish `fdb-gateway:v0.10.0` and `fke-server:v0.10.0`
- [ ] Create GitHub Release: `gh release create v0.10.0 --title "Flint Forge v0.10.0" --notes-file <(git cliff --latest) --latest`
- [ ] Add Docker image digests to release body
- [ ] `cargo test --workspace` passes
