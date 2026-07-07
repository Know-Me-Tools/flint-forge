# p10-c006 — CHANGELOG + `v0.10.0` Release Tag

**Phase:** 10 — Production Launch
**Priority:** P2 — independent of all other changes; can run any time
**Depends on:** none (P0/P1 changes recommended but not required)

## Problem

No `CHANGELOG.md`, no `cliff.toml`, no git tags. Workspace package version is
`0.1.0`. Stakeholders and operators have no artefact to reference for what
changed in this release.

## Version decision

**`v0.10.0`** (not `v1.0.0`). The codebase is production-grade but the
A2UI, Kiln, and SDK public APIs are still evolving. `v0.10.0` signals
"production milestone" without the API-stability promise of `v1.0.0`.
Tag `v1.0.0` when these APIs stabilise.

## Solution

### `cliff.toml`

git-cliff configuration for conventional commit changelog:

```toml
[changelog]
header = "# Changelog\n\nAll notable changes to Flint Forge are documented here.\n\n"
body = """
{% if version %}## [{{ version | trim_start_matches(pat="v") }}] — {{ timestamp | date(format="%Y-%m-%d") }}
{% else %}## [Unreleased]
{% endif %}
{% for group, commits in commits | group_by(attribute="group") %}
### {{ group | upper_first }}
{% for commit in commits %}
- {% if commit.scope %}**{{ commit.scope }}**: {% endif %}{{ commit.message | upper_first }} ([`{{ commit.id | truncate(length=7, end="") }}`](https://github.com/prometheusags/flint-forge/commit/{{ commit.id }}))\
{% endfor %}
{% endfor %}
"""
trim = true

[git]
conventional_commits = true
filter_unconventional = true
split_commits = false
commit_parsers = [
  { message = "^feat", group = "Features" },
  { message = "^fix", group = "Bug Fixes" },
  { message = "^perf", group = "Performance" },
  { message = "^refactor", group = "Refactoring" },
  { message = "^chore\\(kbd\\)", skip = true },
  { message = "^chore", group = "Maintenance" },
  { message = "^docs", group = "Documentation" },
  { message = "^style", group = "Style" },
  { message = "^test", group = "Testing" },
]
tag_pattern = "v[0-9].*"
```

### Steps

1. Bump `[workspace.package] version = "0.10.0"` in `Cargo.toml`
2. Generate `CHANGELOG.md`: `git cliff -o CHANGELOG.md`
3. Commit: `git commit -am "chore(release): v0.10.0"`
4. Tag: `git tag -s v0.10.0 -m "Flint Forge v0.10.0 — production hardening complete"`
5. Build Docker images with `v0.10.0` tag via CI (trigger `docker.yml`)
6. Create GitHub Release:
   ```bash
   gh release create v0.10.0 \
     --title "Flint Forge v0.10.0" \
     --notes-file <(git cliff --latest) \
     --latest
   ```
7. Record Docker image digests in release body:
   ```bash
   docker inspect ghcr.io/prometheusags/flint-forge/fdb-gateway:v0.10.0 \
     --format '{{index .RepoDigests 0}}'
   ```
