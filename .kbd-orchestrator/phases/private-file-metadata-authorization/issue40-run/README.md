# Forge #40 verification handoff for WebUI PR #13, Task 5

On 2026-11-27 the operator authorized the two named synthetic checks under the pinned manifest. One packet ran, each case executed once, and the packet failed the FILE-3 acceptance criterion. The console bridge awaits that evidence. The run used Forge in RLS mode with an ephemeral local JWKS representing Gate. Keto remains deferred.

| Check | Exit | Observed result |
| --- | --- | --- |
| issue40_owner_save | 0 | One test passed. The owner created revision 1 and updated it to revision 2. An independent connection confirmed both complete rows. |
| issue40_authorization_controls | 101 | One test failed. The HTTP controls passed. In both direct SQL ownership checks, record validation rejected the foreign insert before the records-table RLS rejection could be observed. |

| Scenario | Result | Evidence |
| --- | --- | --- |
| FILE-1 Owner save | PASS | Both HTTP writes returned 200 and each independent read matched. |
| FILE-2 Missing grant | PASS | Insert and update returned 403 with metadata_denied. Independent reads confirmed the absent and unchanged rows. |
| FILE-3 Tenant and subject isolation | FAIL | Granted callers could create and update their own records. Foreign HTTP updates returned 409 with both namespaces intact. Each authenticated SQL caller passed its own-row controls and could neither select nor update the other owner's row. Both foreign insert probes reached the record validator and returned SQLSTATE 23514. The contract requires an RLS-specific 42501 on the records table. |
| FILE-4 Record-only grant | PASS | Type registration returned 403 and the independent type list was unchanged. |
| FILE-5 Binding scope | PASS | An ungranted tenant, a missing actor and a wrong actor each received 403. Independent reads confirmed the expected state. |

The fixture used a fresh database for each command on 127.0.0.1 port 55440. The pinned image pull, manifest checks, all three migrations and the pre-grant policy digest check succeeded. Both per-case receipts contain the effective enabled-policy digest. The runner removed both temporary containers and their memory-backed databases. A follow-up Docker query found zero matching containers, and the port had no listener.

The controls JSON field `ran_named_case_once` is false because the runner sets it only for a passing named test. The controls log independently reports one named case executed and one failure. The raw runner receipts are preserved.

- [Owner-save receipt](issue40_owner_save.json) and [log](issue40_owner_save.log)
- [Authorization-controls receipt](issue40_authorization_controls.json) and [log](issue40_authorization_controls.log)
- [Fixture manifest](manifest.json) and [adopted contract](../adopted-contract.json)

Local verification kit revision `7bd87d9c2b22c81e186849eaa90f4c498549fdc7`. Reviewed Forge policy revision `c74c101826b59d8db50d142ea2f339aecf602f69`. The receipts also pin the file API, infrastructure identity handoff and tested Gate revision. The verified identity shape uses the Gate issuer, flint-core audience, authenticated role, agent kind, console backend actor, subject and tenant. The synthetic fixtures exercise the Forge boundary. The live console exchange, deployment and live policy parity were outside the approved run.

During the offline compile Cargo added the existing serde dependency of fdb-postgres to Cargo.lock. Dependency versions stayed the same. The local evidence commit includes the resolved lockfile. All other files introduced or modified by the pinned verification kit matched their Git blobs after the run.
