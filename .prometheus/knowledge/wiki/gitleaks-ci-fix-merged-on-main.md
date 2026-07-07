---
type: Reference
id: gitleaks-ci-fix-merged-on-main
title: Gitleaks CI Fix Merged on Main
tags:
- gitleaks
- ci
- secret-scanning
- okf
- llm-wiki
- prometheus-skill-pack
links:
- okf-v0-1-llm-wiki-adoption-phase-completion
sources:
- stdin
timestamp: 2026-07-03T14:28:04.850778+00:00
created_at: 2026-07-03T14:28:04.850778+00:00
updated_at: 2026-07-03T14:28:04.850778+00:00
revision: 0
---

## Context

- Phase: `phase-okf-llm-wiki-adoption`
- Project path: `/Users/gqadonis/Projects/prometheus/prometheus-skill-pack`
- Captured at: `2026-07-03T14:25:36Z`
- Source identifier: `manual:phase-okf-llm-wiki-adoption`
- This update follows the broader [OKF v0.1 LLM Wiki Adoption Phase Completion](/okf-v0-1-llm-wiki-adoption-phase-completion.md) work.

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
- Ensure `pk lint` enforces OKF v0.1 conformance with permissive consumption semantics.

## Gitleaks Resolution

- Gitleaks is green on `main`.
- PR `#22` merged to `main` at commit `a77f50e`.
- The post-merge run's **Secret scanning (gitleaks)** job completed successfully.
- The previously permanently failing secret scanning check now performs a real full-history scan.
- Local `main` was synced.
- Branch `ci/gitleaks-cli-no-license` was deleted locally and remotely.

## Implementation Details

- The licensed gitleaks action was fully removed.
- The only remaining mention of the removed action is an explanatory comment.
- The workflow now installs the pinned gitleaks CLI version `v8.30.1`.
- The job runs:

```sh
gitleaks git .
```

## Validation

- Repository confirmed clean:
  - `0` findings across the working tree.
  - Full history scan covers `225` commits.
- The check should now fail only when a genuinely introduced secret is detected.

## CI Status on `main`

Green checks:

- hooks-integrity ✅
- gitleaks ✅
- Rust CLI ✅
- AgentSkills compliance ✅
- sycophancy e2e ✅
- skill-collision ✅

Pre-existing red checks, unrelated to the gitleaks work:

- **Check Formatting**
  - `123` unformatted files.
  - Mostly generated `site/.docusaurus/*` build artifacts and docs.
  - Likely remediation: run `prettier --write` and add `site/.docusaurus/` to `.prettierignore`.
- **forge-rs (fmt + clippy + test)**
  - Pre-existing Rust CI failure.
  - Requires inspection of the actual fmt/clippy/test error output.

## Follow-Up Options

- No remaining gitleaks work is outstanding.
- Optional follow-up PRs could address:
  - Formatting failure.
  - `forge-rs` fmt/clippy/test failure.

# Citations

1. stdin