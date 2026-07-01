# Execution Plan — p3-graphql-hybrid-engine

## Backend
`openspec` — all changes are tracked as OpenSpec structures under `openspec/changes/`

## Dispatch Contract

Changes are executed in dependency order by Claude Code directly. The executor
reads each `openspec/changes/<change-id>/tasks.md` and applies the tasks to
the Rust workspace.

## Execution Order

1. **p3-c005-pg-graphql-pg18** + **p3-c008-extended-guc-propagation** — parallel, no blockers
2. **p3-c001-graphql-passthrough** — after p3-c005 OQ-3 resolved
3. **p3-c007-graphql-compiler** — after p3-c001
4. **p3-c004-graphql-transport-ws** — after p3-c007
5. **p3-c002-subscriptions** — after p3-c004 + OQ-FRF-1 resolved
6. **p3-c003-introspection-merge** — after p3-c001 + p3-c007 (can parallel with 5)
7. **p3-c006-keto-sync** — after OQ-8 resolved (can parallel with 5-6)
8. **p3-c009-predicate-pushdown** — P2 deferral, after p3-c002 production-verified

## QA Gate
- Documentation-only changes (p3-c005): skip QA
- All other changes: `cargo check --workspace` + `cargo clippy --workspace -- -D warnings` + `cargo test --workspace`
- Security-critical changes (p3-c002, p3-c006): spawn `security-reviewer` agent after implementation

## Phase Complete Criteria
- 8 P0 changes `qa_passed` in `progress.json`
- p3-c009 may remain `pending` (P2)
- `cargo test --workspace` passes
- `/graphql` POST and GET routes respond correctly

## Created
2026-06-30
