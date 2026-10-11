---
name: forge-delivery
description: "Plan, coordinate, release, or independently review Flint Forge changes across its database, API, runtime and client boundaries."
license: MIT
metadata:
  author: prometheus
  version: "1.0.0"
---

# forge-delivery

Read AGENTS.md and CLAUDE.md, then canonical KBD project/progress/reflection records. A waypoint can be stale: at team creation p17's waypoint said execute_ready but project.json and reflection.md marked completion. Reconcile evidence; never restart work or hand-edit generated KBD projections.

Choose one implementer for an isolated task, adding the responsible specialist and independent reviewer only when needed. The 11-role roster is a pool, not an instruction to spawn 11 workers. Default to at most four active agents, including the lead, and one local Rust build writer. Native harness limits still apply. The lead coordinates and integrates; a child role cannot create another orchestration loop.

Before editing, reserve exact repository/worktree/HEAD, KBD task, role, paths and acceptance criteria in the team's local ledger using agent-team-manage. Ownership globs are coordination, not sandbox enforcement. Existing active writers take precedence. Read other projects; changes there need their own ownership and checkout. Do not edit the UAR Working Agent task's UAR/The Boss worktrees or Forge's database-role/GraphQL worktrees without a coordinated handoff.

Follow Forge's integration-first process: implement and connect the phase, compile-check coherent slices, then run the required phase-boundary integration suite. Read Cargo.toml, rust-toolchain.toml and extension manifests for versions; older prose is not authoritative. No repeated full builds, no ignored failures. For this agent-definition-only task, manifest/native-load/ownership validation is appropriate; do not compile Rust merely to validate Markdown.

Before handoff, record exact revision and dirty patch scope, accepted contracts, tests actually run, failures, task status, unresolved outcomes and next command. Native session continuation and a summary handoff are different. Use agent-team-handoff; preserve KBD authority. Shared memory is optional evidence, not an authorization or locking service. Use only a configured authorized surreal-memory endpoint; never let processes share one embedded DB directory. Karpathy progress receipts are references to real boundaries, never synthetic completions.

Independent review reads broadly but writes only findings and explicitly assigned test assets. It checks integration, tenant-negative cases, behavior changes and evidence; the producer fixes defects. SRE/release work includes backup/restore/PITR rehearsal, upgrade/downgrade feasibility, metrics without identity payloads, extension matrix, capacity and operator documentation. A completed change is not authorization to deploy.

Read docs/agent-teams/flint-forge/RESEARCH.md for sources and boundaries, and WORKFLOW.md for task routing.
