# p10-c006 Tasks — CHANGELOG + `v0.10.0` Release Tag

## Tasks

- [x] Bump `[workspace.package] version` from `0.1.0` to `0.10.0` in `Cargo.toml` — landed in commit `581543e` (HEAD has since moved to `1.0.0` via a later release)
- [x] Create `cliff.toml` at workspace root with conventional-commit configuration
- [x] Install git-cliff: `cargo install git-cliff --locked` — procedural; evidenced by the generated CHANGELOG.md existing
- [x] Generate `CHANGELOG.md`: `git cliff -o CHANGELOG.md` — `## [0.10.0] — 2026-07-07` section present
- [x] Review generated CHANGELOG.md; edit any entries that need clarification — content reads as curated (grouped by Bug Fixes/Docs/Features/etc.), not a raw dump
- [x] Commit version bump + CHANGELOG: `git commit -am "chore(release): v0.10.0"` — commit `581543e` "chore(release): v0.10.0 — production hardening complete"
- [ ] Create signed tag: `git tag -s v0.10.0 -m "Flint Forge v0.10.0 — production hardening complete"` — OPEN: tag `v0.10.0` exists but is NOT actually GPG-signed (`git tag -v v0.10.0` → "error: no signature found"); it's a plain annotated tag, not the signed tag this task specifies
- [x] Push tag: `git push origin v0.10.0` — confirmed present on `origin` via `git ls-remote --tags`
- [ ] Trigger `docker.yml` workflow to publish `fdb-gateway:v0.10.0` and `fke-server:v0.10.0` — OPEN: no `docker.yml` run exists for the `v0.10.0` tag (checked all historical runs) — the tag-push trigger for `docker.yml` was only added later by p12-c002, so pushing `v0.10.0` never fired a build. (The later `v1.0.0` tag DID trigger it, but that run failed on a lowercase-repository-name error.)
- [x] Create GitHub Release: `gh release create v0.10.0 --title "Flint Forge v0.10.0" --notes-file <(git cliff --latest) --latest` — confirmed published (non-draft), populated body
- [ ] Add Docker image digests to release body — OPEN: release body contains no `sha256:`/digest references at all
- [ ] `cargo test --workspace` passes — OPEN/unverifiable: same finding as p10-c003 — CI at commit `581543e` failed at the `fmt` step before reaching `test`, so this was never actually demonstrated green at ship time for this commit

**Status note (p16-c006 reconcile):** version/CHANGELOG/cliff.toml/tag-push/GitHub-release genuinely shipped, but the tag is not GPG-signed, the Docker publish step never ran for `v0.10.0`, and no digests were added to the release body — these remain open debt.
