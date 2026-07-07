---
type: Reference
id: cowork-cli-integration-analysis-for-prometheus-skill-pack
title: cowork CLI Integration Analysis for prometheus-skill-pack
tags:
- cowork-cli
- prometheus-skill-pack
- skill-management
- platform-support
- github-releases
- phase-tracking
links:
- gitleaks-ci-fix-merged-on-main
sources:
- stdin
- manual:cowork-integration
timestamp: 2026-07-03T21:53:15.829561+00:00
created_at: 2026-07-03T21:53:15.829561+00:00
updated_at: 2026-07-03T21:53:15.829561+00:00
revision: 0
---

## Context

Phase `cowork-integration` investigated integrating the forked `cowork` CLI (`git@github.com:GQAdonis/cowork-skills.git`) into `prometheus-skill-pack` as the standard skill installation and management utility.

Work is planned in a dedicated worktree outside the skill-pack directory to avoid polluting the main tree:

- KBD root: `/Users/gqadonis/Projects/prometheus/prometheus-skill-pack/.claude/worktrees/charming-diffie-309eef`
- Captured: `2026-07-03T21:51:00Z`
- Phase status: `plan_ready`
- Next command: `/kbd-plan cowork-integration`

## Phase Goals

- Assess the forked `cowork` architecture and produce an integration plan for making it a standard CLI in `prometheus-skill-pack`.
- Add explicit skill installation support for:
  - Zed
  - Kimi Code CLI
  - Kimi Desktop
  - MiniMax Desktop
  - MMX CLI evaluation
- Make `cowork` aware of prometheus-skill-pack management so it can:
  - update the pack
  - update toolchains
  - repair broken installations
- Model Claude Code plugin and marketplace mechanics in detail.
- Extend support to installing and managing Codex plugins and OpenCode plugins.
- Integrate the updated `cowork` CLI into the skill-pack install pipeline and document it as the primary skill-management utility.

## Analysis Outcome

`kbd-analyze` completed for `cowork-integration`.

- All 5 open questions resolved.
- All 6 build-vs-adopt decisions uncontested.
- 12 changes identified across 4 implementation waves and ready to spec.

## Platform Resolution

| Platform | Verdict | Required Action |
|---|---|---|
| Zed | Confirmed directory-only skill support | Add new `agents.rs` entry targeting `~/.config/zed/skills/` |
| Kimi Code CLI | Confirmed skill directory plus TOML MCP config | Add new agent entry and TOML config writer for `~/.kimi-code/skills/` |
| Kimi Desktop | Confirmed macOS-specific deep path | Add new agent entry with macOS guard |
| MiniMax Desktop | Shares `~/.minimax/skills/` with CLI | Update detection only |
| MMX CLI | Out of scope; no skill system | Document and close |

## Decisions

All 6 build-vs-adopt decisions were resolved without escalation.

- Largest score gap: `90/5`, favoring fork-and-extend.
- Tightest score gap: `85/15`, favoring direct OpenCode JSON writes.
- No contested stacks remain.

### Binary Distribution

Decision: distribute `cowork` via GitHub Releases with pre-built binaries and a `cargo build` fallback.

Rationale:

- Matches the existing release-artifact precedent used around gitleaks work; see [Gitleaks CI Fix Merged on Main](/gitleaks-ci-fix-merged-on-main.md).
- `crates.io` publication is blocked by an upstream naming conflict.
- Current tool build artifacts already total approximately `17 GB`; adding source builds for `cowork` would make disk-management pressure worse.

### Plugin and Config Writers

Decisions:

- Use direct JSON/TOML writers for OpenCode and Codex plugin/config management.
- Use a shell-out approach for the pack-management subcommand.

## Parallel Work

The `dsg` execution track should start after `cowork` `change-001`.

- Scope: 5 OpenSpec changes.
- Blocking status: non-blocking.
- Urgency: high.

## Produced Artifacts

- `.kbd-orchestrator/phases/cowork-integration/analysis.md` — full narrative analysis
- `.kbd-orchestrator/phases/cowork-integration/library-candidates.json` — machine-readable contract for planning
- `.kbd-orchestrator/phases/cowork-integration/decision-log.md` — decision audit trail

## Current Position

```text
Position: cowork-integration
Status: plan_ready
Last: Analysis complete — all 5 OQs resolved, 6 uncontested decisions
Next: /kbd-plan cowork-integration
```

# Citations

1. stdin
2. manual:cowork-integration