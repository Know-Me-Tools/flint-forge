# p3-c012 — forge-policy Cedar Policy Engine

## Change ID
`p3-c012-forge-policy-cedar`

## Phase
`p3-auth-rls-keto`

## Goal Mapping
**G1** — Cedar policy evaluation crate. `PolicyEngine::evaluate(principal,
action, resource, context)` returns allow/deny; policy bundles loaded from
`flint_meta.cedar_policies` table.

## Problem
`forge-policy/src/lib.rs` defines the `Pep` trait, `Decision`, and `Request`
types only. No `cedar-policy` dependency, no concrete engine, no policy
loading. `flint_meta.cedar_policies` table does NOT exist
(OQ-cedar-table — pre-flight confirmed absent in `crates/ext-flint-meta/sql/flint_meta.sql`).

## Open Question Resolution
- **OQ-cedar:** pin `cedar-policy = "4"` (current `4.11.2`, confirmed via
  `cargo search`). Stay on the public 4.x API; avoid `cedar-policy-core`
  internals.
- **OQ-cedar-table:** resolved — table is absent; this change adds it.

## Scope
- Add `flint_meta.cedar_policies` table via new SQL migration under
  `crates/ext-flint-meta/sql/` (or migration file following established
  convention). Columns: `id`, `name`, `policy_text`, `enabled`, timestamps.
  Privileged pool only — never RLS-filtered.
- Add `cedar-policy = "4"` to `[workspace.dependencies]` and to
  `forge-policy/Cargo.toml`.
- Implement `CedarPolicyEngine` struct implementing the existing `Pep` trait.
- `PolicyLoader`: loads enabled policies from `flint_meta.cedar_policies`
  using the **privileged pool** (not RLS pool). Cache `Schema + PolicySet`,
  hot-reload on outbox event (reuse ArcSwap pattern if ergonomic).
- `evaluate()` semantics: policy load failure, schema compile failure, or
  evaluation error → `Decision::Deny` (fail closed). Tracing spans on the
  port boundary; never log policy bodies or principal identifiers.
- Integration into `fdb-app` mutation use-cases: inject `Arc<dyn Pep>`
  alongside `Arc<dyn KetoCheck>` from c011.

## Out of Scope
- CRUD handler bodies (c013/c014) — Cedar call site lands there but body
  logic is out of scope here.
- Cedar policy authoring UI / admin endpoints.

## Acceptance Criteria
- [ ] `flint_meta.cedar_policies` migration applied; table visible in pgrx run
- [ ] `cedar-policy = "4"` in workspace deps; `cargo check` green
- [ ] `CedarPolicyEngine` implements `Pep`; unit tests cover allow + deny + load-failure-deny
- [ ] Policy load uses privileged pool only
- [ ] No policy body or principal identifier logged
- [ ] `cargo check --workspace` + clippy + `cargo test -p forge-policy` green
