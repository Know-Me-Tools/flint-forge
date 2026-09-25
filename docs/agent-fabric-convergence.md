# Agent Fabric Convergence: Flint Forge planning notes

Date: 2026-09-25

Status: planning input; no domain schema or connector contract is ratified here

Source baseline: `dc313be3a044c65b05d845a9c34350bf5ca3ca3e`

## Role in the convergence

Flint Forge is the authoritative business-data and governed domain-effect plane. Quarry executes reads and mutations under Postgres RLS; Forge policy gates capabilities with Cedar; durable outboxes and CDC expose committed facts; Kiln runs signed WASM components through governed host interfaces.

The convergence plan's C05 keeps BossFang's workflow while delegating a complete run to UAR, and C06/C09 place runtime instances and team state in UAR; C01 must reconcile those statements with the accepted P1 contracts. Fabric transports committed facts. Gate authenticates and admits protected effects. Forge owns the final business transaction and its authoritative result. An agent, team, workflow, or Codex session may propose a command, but it cannot substitute for RLS, Cedar, or the transaction boundary.

## Current source facts

- `RlsContext` is built from a verified flint-gate JWT and is installed on each transaction; callers must commit side-effecting work or the pool recycle rolls it back (`forge-identity`, `fdb-ports`, `fdb-postgres`).
- REST mutations run a coarse Keto check followed by Cedar before executing under RLS (`crates/fdb-reflection/src/compilers/rest/mutations.rs`).
- `forge-policy` provides a fail-closed Cedar PEP shared by Quarry and Kiln-facing capability checks (`crates/forge-policy/src/lib.rs`, `crates/forge-policy/src/cedar.rs`).
- `ChangeEvent` currently describes operation, schema/table, current record, and old record. It does not itself carry a durable command id, authoritative result id, source occurrence, tenant, or publication cursor (`crates/fdb-domain/src/lib.rs`).
- The working default realtime path uses Postgres `LISTEN`/`NOTIFY`. The Fabric adapter intentionally returns unavailable because `WatchEntityType` is not present in the Fabric contract (`crates/fdb-realtime/src/lib.rs`). This is a concrete C11 dependency, not a working cross-host replication claim.
- Forge already has durable outbox patterns for webhooks and the Kiln background worker (`crates/ext-flint-hooks`, `crates/fke-server/src/kiln_bgw.rs`). Those rows are effect-specific; they do not establish a general offline domain-command/result protocol.
- The repository source contains substantial working surfaces beyond a scaffold, while the top-level `CLAUDE.md` still describes the repository as a scaffold. C01/D-FORGE should record that documentation drift before treating old status language as current evidence.

## Future domain-command contract

C11 should make one Forge transaction the authority for each business command. A caller should be able to submit a stable command identity, observe pending versus committed state, and query the authoritative result after a lost response. The transaction should apply the existing Gate identity, RLS, Keto where configured, and Cedar checks, then publish a committed fact through an outbox/CDC path. Exact schema, retry classification, and result shape remain C11 design work.

The contract must distinguish intent, transaction result, publication, and downstream projection. Replaying a command must not create a second mutation. Replaying a committed fact through Fabric may rebuild a projection but must not re-run the domain transaction. Deletes and revocations need explicit tombstone/visibility behavior so read models do not retain data that the authoritative source no longer discloses.

Forge should not become the general multi-agent task board. Its tables represent business state, effect intent/receipt, policy, artifacts, and domain projections. Team membership, agent scheduling, model turns, and workflow decisions remain with their accepted owners.

For Codex CLI, the model path is a normal UAR execution route. When it reaches Forge, every write must enter through the same authenticated, authorized, RLS-scoped command path as any other client. A local Codex process, skill manifest, or Rust component signature does not grant database or connector authority.

## Dependencies and next repository-scoped KBD child

Recommended next child: **C11 Forge authoritative command and committed-fact replication**.

It remains blocked on C01, C02, C04 and the D-FORGE, D-FRF, and D-GATE checkpoints. The child should first reconcile the existing mutation, transaction-commit, outbox, realtime, and Fabric-adapter surfaces, then claim exact files. It should reuse existing outbox/CDC and policy ports where their contracts fit rather than introduce a second event store or policy engine.

Acceptance must cover crashes before and after commit and publication, duplicate and out-of-order command delivery, lost command responses followed by authoritative result lookup, delete/tombstone propagation, tenant isolation, and pending-versus-committed client state through real Postgres and Fabric boundaries.

Do not include UAR team scheduling, BossFang workflow state, broad connector implementation, or personal CRDT synchronization in this child.

## Evidence to preserve in the child plan

- Baseline commit and current migration/protocol revisions.
- Exact transaction that owns command idempotency and result persistence.
- Existing RLS/Keto/Cedar order and authenticated identity source.
- Outbox commit relationship and publication cursor semantics.
- Fabric `WatchEntityType` dependency resolution or an explicitly supported alternative.
- Append-only migration and rollback impact for any new command/result/tombstone fields.
- A kickoff recheck of all recorded baselines, prerequisite receipts, and the single transaction/workflow-owner assignments.
