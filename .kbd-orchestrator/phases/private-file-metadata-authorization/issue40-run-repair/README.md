# Forge #40 repair verification for WebUI Task 5

On 2026-11-27 both named checks passed on PostgreSQL 18.7, each executed once. All five Forge acceptance scenarios passed in the approved disposable RLS fixture. The repair is local commit `8a59888c9d5c2595f35ed8d6e3126d66dcc99220` on `codex/issue-40-record-permissions`. [First run and original failure](../issue40-run/README.md)

Migration 0022 applies the existing record validator after insert and before update. PostgreSQL now evaluates the records-table ownership policy before the validator reads a private type. The validator still rejects invalid owner attributes within the statement. [Migration](../../../../migrations/0022_file_record_insert_validation.sql) · [PostgreSQL 18 policy ordering](https://www.postgresql.org/docs/18/sql-createpolicy.html)

| Named check | Observations |
| --- | --- |
| issue40_owner_save | Owner insert and update returned 200 and persisted the expected data at revisions 1 and 2. Invalid attributes returned 422, and an independent read found no row. [Receipt](issue40_owner_save.json) · [Log](issue40_owner_save.log) |
| issue40_authorization_controls | Missing grant, wrong tenant, missing or wrong actor, and type registration received 403. Foreign HTTP updates received 409 and independent reads confirmed intact state. In both direct SQL ownership checks the caller could insert, read and update its own row. Foreign reads and updates affected zero rows. Both foreign inserts raised a PostgreSQL records-table RLS violation (42501). [Receipt](issue40_authorization_controls.json) · [Log](issue40_authorization_controls.log) |

Each check ran against a fresh local database with the four pinned migrations. The two receipts contain the exact commands and policy digests. The runner cleaned up both containers and their memory-backed databases. Independent checks found zero matching containers and confirmed that loopback port 55440 was free. The fixture used a synthetic signed identity. Verification applies to this Forge configuration. Deployment and the live console exchange are outside this run. Keto remains deferred. [Run manifest](manifest.json)
