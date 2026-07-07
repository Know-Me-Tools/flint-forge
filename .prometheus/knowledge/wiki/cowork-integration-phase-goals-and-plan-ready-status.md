---
type: Reference
id: cowork-integration-phase-goals-and-plan-ready-status
title: cowork-integration Phase Goals and Plan-Ready Status
tags:
- cowork-cli
- prometheus-skill-pack
- skill-management
- platform-support
- phase-tracking
- plugin-management
links:
- cowork-cli-integration-analysis-for-prometheus-skill-pack
sources:
- stdin
- manual:cowork-integration
timestamp: 2026-07-03T22:00:29.186053+00:00
created_at: 2026-07-03T22:00:29.186053+00:00
updated_at: 2026-07-03T22:00:29.186053+00:00
revision: 0
---

## Context

Phase `cowork-integration` is focused on integrating the forked `cowork` CLI codebase into `prometheus-skill-pack` as the standard skill installation and management CLI.

- Forked repository: `git@github.com:GQAdonis/cowork-skills.git`
- KBD root/worktree: `/Users/gqadonis/Projects/prometheus/prometheus-skill-pack/.claude/worktrees/charming-diffie-309eef`
- Captured at: `2026-07-03T21:56:25Z`
- Phase status: `plan_ready`
- Current position: `cowork-integration`
- Next step: `/kbd-plan cowork-integration` after synthesis of research-agent outputs

Work is intentionally being done in a dedicated worktree outside the main skill-pack directory to support clean investigation without polluting the primary tree.

## Goals

- **G-01:** Investigate the forked `cowork` codebase and produce an architecture assessment with a clear integration plan for adopting it as a standard CLI in `prometheus-skill-pack`.
- **G-02:** Add explicit target-platform support so skills can be installed for:
  - Zed
  - Kimi Code CLI
  - MMX CLI
  - Kimi Desktop
  - MiniMax Desktop
- **G-03:** Make `cowork` aware of how `prometheus-skill-pack` is managed so it can:
  - update the pack
  - update toolchains
  - repair broken installations
- **G-04:** Model Claude Code plugin and marketplace mechanics in full detail, then extend `cowork` to support installing and managing:
  - Codex plugins
  - OpenCode plugins
- **G-05:** Integrate the updated `cowork` CLI into the skill-pack install pipeline and document it as the primary skill-management utility.

## Research Status

Two research agents were launched and results were pending at capture time:

1. Submodule strategy and `dsg` full-scope investigation.
2. `agentskills.io` compliance requirements for `dsg`.

The phase is ready for planning once agent findings are synthesized. See related prior context in [cowork CLI Integration Analysis for prometheus-skill-pack](/cowork-cli-integration-analysis-for-prometheus-skill-pack.md).

# Citations

1. stdin
2. manual:cowork-integration