# Phase reflection for private file metadata authorization

KBD closed this phase with a PASS on 2026-11-27. Forge #40 met all five adopted acceptance scenarios in the approved PostgreSQL 18.7 fixture. Both named checks ran once after the repair and passed. An entitled console owner could create and revise private file metadata, and another owner could neither read nor change it. The implementation and redacted handoff are on the local branch `codex/issue-40-record-permissions`. Publication remains pending. All Keto work stays deferred. [Task 5 handoff](issue40-run-repair/README.md)

## Goal achievement

| Scenario | Result | Verified behavior |
| --- | --- | --- |
| FILE-1 Owner save | PASS | The owner received HTTP 200 on create and update. Independent reads matched revisions 1 and 2. Invalid attributes returned 422 and left no row. |
| FILE-2 Missing grant | PASS | Valid insert and update requests without a record grant returned 403. Independent reads confirmed the expected state. |
| FILE-3 Tenant and subject isolation | PASS | Under each foreign SQL identity, the caller could write its own row. Reads and updates of the original owner’s row affected zero rows, and foreign inserts received a records-table RLS violation (42501). Foreign HTTP updates returned 409. |
| FILE-4 Record-only grant | PASS | Type registration returned 403 and the private type list was unchanged. |
| FILE-5 Binding scope | PASS | A claim outside the granted tenant, a missing actor and a wrong actor each received 403. |

The generated closure packet copies its implementation flag from the adoption snapshot, which predates execution. The reconciled runtime ledger records one completed change. [Runtime ledger](progress.json)

All four implementation tasks reconciled across the runtime, the phase ledger and the native change backend. Scenario coverage was 5/5 (100%) for the tested Forge configuration. The fixture represented Gate with a local signing key. The live console exchange and production deployment require separate work. [Contract](adopted-contract.json) · [Scenario evidence](boundary-results.json)

## Delta and root cause

The first run passed owner save and failed one isolation criterion. Both foreign inserts were rejected with SQLSTATE 23514. Migration 0021’s before-insert validator looked up a private type that the foreign caller could not see. The validator raised its error before PostgreSQL could evaluate the records-table ownership policy. [Original receipts](issue40-run/README.md)

## Corrective actions and delivery

The approved repair in `8a59888` moved insert validation after the RLS check, within the same SQL statement. Update validation retained its timing. An invalid owner insert served as a rollback control. The repair preserved the existing Cedar grant revision `c74c101` and introduced no new privileges. Both disposable databases were removed. The initial results are pinned at `a058aea` and the passing results at `0c8240f`. [Repair receipts](issue40-run-repair/README.md)

## Recalled lessons

The earlier lesson about preserving exact sources and failure evidence applied here. Keeping the failed packet made it possible to distinguish validation error 23514 from the required RLS denial 42501 and to compare the repair against the original oracle. [Prior context](prior-context.md)

## Quality and coordination

Assessment and planning reviews added the independent database checks and caller-owned positive controls. The policy received a separate source review. The execute handoff was reconstructed during reflection from the signed-off task ledger and saved receipts. The redacted Task 5 handoff is local. Its delivery to an external consumer and live policy parity are unverified. [Execution record](execution.md) · [Execute handoff](handoffs/execute.handoff.json)

The contract had zero amendments, one resolved fixture-approval dependency, zero unexercised scenarios and one authorized repair out of one allowed. Across both runs there were four named test executions, three passes and the original failure. Reflection reused those results. A separate B4 static-QA findings count and a post-closure defect observation window were unavailable. [Accuracy record](accuracy.json)

## Lessons Learned

- For a row-isolation requirement, pair the denied foreign write with a successful same-owner write and verify which database policy produced the denial.
- When changing validation timing, exercise an invalid owner write and independently confirm rollback.

## Next Phase Seed

WebUI Task 5 can consume the pinned identity binding, reviewed Forge grant and redacted evidence. Enabling the console bridge or deploying the policy requires separate authorization. Keto stays deferred. No successor phase was requested.

## Codify as Skill?

The migration and its regression check capture this case. No new skill is proposed.
