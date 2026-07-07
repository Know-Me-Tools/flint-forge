---
type: Reference
id: okf-v0-1-llm-wiki-adoption-phase-completion
title: OKF v0.1 LLM Wiki Adoption Phase Completion
tags:
- okf
- llm-wiki
- pk
- liter-llm
- skills
- ci
- deployment
sources:
- stdin
- manual:phase-okf-llm-wiki-adoption
timestamp: 2026-07-03T14:05:29.744150+00:00
created_at: 2026-07-03T14:05:29.744150+00:00
updated_at: 2026-07-03T14:05:29.744150+00:00
revision: 0
---

## Phase Goals

- Make `pk` wiki entries conform to OKF v0.1:
  - Required `type` frontmatter.
  - Recommended `title`, `description`, `resource`, `tags`, and `timestamp` metadata.
  - Unknown-key tolerance for permissive consumption.
- Maintain reserved `index.md` and `log.md` at the wiki root per OKF sections 6–7, updated on every ingest.
- Move cross-links from frontmatter `links` arrays to bundle-relative markdown body links.
- Adopt a `# Citations` section convention per OKF sections 5 and 8.
- Expose Karpathy LLM Wiki operations as first-class repo skills:
  - `ingest`
  - `query`
  - `lint`
- Add a wiki schema document.
- Ensure `pk lint` enforces OKF v0.1 conformance while preserving permissive consumption semantics.

## Merge and Sync Status

- PR `#21` merged to `main` as squash commit `e966e75` at 14:03 UTC.
- Local `main` was pulled and is up to date with origin.
- Working tree is clean.
- Feature branch was deleted locally and remotely.
- `tools/liter-llm` pointer on `main` is `d8223de3`, corresponding to `liter-llm` v1.9.2.
- `tools/liter-llm` working checkout matches the recorded pointer.

## Runtime Services

All 6 services are healthy and running current code:

| Service | Port | Version / Commit | Notes |
|---|---:|---|---|
| `surrealdb-native` | `28000` | — | Healthy |
| `surreal-memory` | `23001` | `b2ed891` | `/health` reliable |
| `pk-cherry` | `8942` | `feb2170` | Healthy |
| `forge-mcp` | `8943` | — | Healthy |
| `surface-bridge` | `7890` | — | Healthy |
| `sovereign-sync` | `7892` | — | Healthy |

## Rebuilt and Re-signed Binaries

Binaries were rebuilt from correct synced commits and re-signed:

- `liter-llm` v1.9.2
- `pk` / `pk-cherry` at `feb2170`
- `surreal-memory-server` at `b2ed891`
- `sycophancy-correction` at `bc348ff`

## Skills and Hook Installation

- 122 skills were reinstalled across all 14 AI-tool platforms.
- Marketplace symlinks were rebuilt.
- Plugin was registered globally.
- Hooks were verified as installed, written, and active on supported tools:
  - Claude Code
  - OpenCode

## Changes Included in PR #21

- Updated `liter-llm` pointer to v1.9.2.
- Fixed OpenCode `kbd-close` load bug.
- Extended `install-binaries.sh` to support:
  - `pk`
  - `surreal`
  - `sycophancy`
  - binary re-signing
  - dual-directory installation
- Normalized `hooks.json`.

## Remaining Pre-existing CI Failures

The following failures remain open on `main` and were pre-existing, not introduced by this work:

- `gitleaks` license failure.
- Prettier debt failure.

These should be handled in a separate focused PR if CI cleanup is desired.

# Citations

1. stdin
2. manual:phase-okf-llm-wiki-adoption