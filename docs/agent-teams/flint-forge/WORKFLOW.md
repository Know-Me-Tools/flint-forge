# Operating the Flint Forge specialist pool

These definitions await a bounded work order. They do not authorize application
changes, deployments, external messages, new services or background agent processes.
Use the selected harness's native subagents; do not implement a scheduler around
the JSON team state. An agent definition, skill, model and execution harness are
different things.

## Choose a small working group

| Task | Implementer | Add when needed | Independent reviewer |
|---|---|---|---|
| SQL/index/pool/RLS/migration | forge-postgres18 | forge-security for authority changes | forge-reviewer |
| REST/RPC/GraphQL behavior | forge-quarry-api | PostgreSQL or metadata owner | forge-reviewer |
| Catalog/reflection/schema management | forge-metadata | PostgreSQL and security owners | forge-reviewer |
| Realtime/hook/outbox/reconnect | forge-realtime | Quarry and security owners | forge-reviewer |
| WASM edge function/signature/WIT | forge-kiln | Security or external runtime owner | forge-reviewer |
| SQL model jobs/embeddings | forge-ember | PostgreSQL or realtime owner | forge-reviewer |
| React/Flutter/CLI contract | forge-sdk | Quarry or metadata owner | forge-reviewer |
| Image/CI/recovery/observability | forge-release | PostgreSQL owner | forge-reviewer |

Use `forge-lead` when the change crosses boundaries. For a small isolated edit, one
implementer plus a review pass is sufficient. Keep at most four active agents as a
coordination default; the harness's enforced limit may be lower. One local Rust
compiler/build writer avoids contention with existing UAR work.

## Work order and ownership

Record outcome and acceptance cases, repo/worktree/HEAD, canonical KBD identity,
baseline versions, exact write paths, producers/reviewer, chosen harness/model,
database/test namespace and evidence output. The manifest's `owns` entries are
upper bounds; narrow them per task. Paths outside the manifest require an explicit
ownership assignment, never an inferred catch-all. Existing active owners prevail.

The common guide output is retained as discovery evidence; the eleven-role manifest
is authoritative. Role-level `dependsOn` is deliberately empty: dormant specialists
must not block every change. Use actual task dependencies in agent-team-manage:
accepted contract → selected producers → boundary verification → independent review
→ integration. A reviewer must receive all selected producer evidence before approval.

Maintain the optional local ledger at `.agent-team/flint-forge/state.json` through
the agent-team-manage CLI and revision checks. It has no tasks initially and is
Git-ignored. Keep durable task receipts with the owning KBD phase. Do not let this
local mirror mark canonical KBD work complete. The existing p17 waypoint is stale
relative to project/reflection records; reconcile that before choosing a next phase.

Use a separate assigned worktree for implementation where required by the owning
repository. The current project-local definitions do not move or merge any active
branch. Preserve Forge database-role/GraphQL worktrees, the UAR Working Agent task,
and the cross-project convergence initiative. Never reset, stash or rebase another
writer's work as part of team setup.

## Contracts and verification

Read the selected skills through native discovery or directly from
`.agents/skills/<id>/SKILL.md`. Compass MCP may be unavailable in a harness even
though configured for another one; use the project Compass CLI or source inspection,
recording the fallback. Check graph freshness before trusting call relationships.

Agree a versioned API/schema behavior matrix and a PostgreSQL fixture before coding.
Producers write their crate-local tests; the reviewer owns shared acceptance assets
and findings. Follow AGENTS.md: complete and connect the phase, compile-check coherent
slices, then run the phase-boundary integration suite. Failed checks require repair,
not suppression. Source-only, skipped, blocked and executed checks remain distinct.

Minimum relevant acceptance examples: pooled two-tenant isolation after error/rollback;
role/owner/BYPASSRLS and definer behavior; GraphQL mutation commit and role-scoped
introspection; REST embedding/RPC/header/error parity; metadata without privileged
leaks; denied delivery after revocation; replay/unknown effect reconciliation;
WASM signature/capability/cancellation; restore and extension compatibility; and
real SDK requests. Select cases that apply to the assigned slice.

## Cross-harness handoff and optional memory

Use agent-team-handoff, retaining exact commits/patches, accepted contract, outputs,
tests, unresolved findings, policy profile and next command. A new harness must
revalidate permissions and available tools. A summary is not a portable native
session and does not carry approvals. Explicitly transfer the task owner; two
harnesses must not concurrently own the same change.

Karpathy logs and surreal-memory are optional evidence stores. Publish only through
an operator-configured authorized endpoint and retain scope/provenance/outcome.
Never infer a service URL, copy credentials, share an embedded DB directory between
processes or claim exactly-once memory writes. Link genuine KBD boundary receipts;
do not synthesize task completion or make an agent log the authorization authority.
