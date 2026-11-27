# Execution of Forge issue 40

Execute started on 2026-11-27 with the native KBD backend. The adopted contract at revision 1 passed the readiness check. The Codex driver opened T1 using kbd-apply. The current gpt-6.2-astra route owns source pinning and KBD state. A read-only worker traced the identity handoff. The ordered T2, T3 and T4 assignments remain in plan.md.

## T1 source result

The local branch `codex/issue-40-file-api-baseline` retains commit `ce3b507712a9527a5080610baa0530ba4294c3d4` on base `ce1efd189bcb520242acd530848e3b920d1036cd`. Its 20 file API and documentation paths matched the prior isolated snapshot manifest byte for byte. Seven further paths capture the pooled-connection implementation, callers and tests used by that source. All 27 Git blobs were compared to the working tree. An alternate index kept the operator’s checkout on main and preserved its pending edits. The commit exists locally. Its purpose is reproducible source inspection. [File list and Git identities](file-api-source.json)

The existing recall lesson about preserving exact source before validation informed the choice to retain a Git tree. No production boundary checks ran.

## Earlier T1 checkpoint (superseded by the merged identity handoff)

The identity owner is tracking the membership policy and server-side Gate issuance in [flint-infra issue 22](https://github.com/Prometheus-AGS/flint-infra/issues/22). The issue is open. Its criteria require a verified Kratos subject to resolve to exactly one authorized tenant and fail closed for missing, expired, revoked or ambiguous membership. Task 5 expects the console audience `ipfs-console` on input and Gate-issued `flint-core` on output, with preserved `sub`, a policy-derived `tenant_id` and `role=authenticated`. These are the requested contract fields. They do not by themselves establish record-write eligibility.

The identity owner's github-issues-clearance assessment marks the authoritative policy missing. Its local ADR 0002 proposes a store and leaves the binding undecided. The WebUI Task 5 source trace identifies deployed Gate source revision `b3019caac22d0dfbff867653486237e28fae4ab0`. The pinned Forge Cedar evaluator uses the verified subject as principal with empty context. Selecting a subject or authenticated-role permit before scope is known could give the capability in an unintended tenant.

| Evidence | Snapshot |
| --- | --- |
| flint-infra owner assessment | `/Users/gqadonis/Projects/prometheus/flint-core-infra/.kbd-orchestrator/phases/github-issues-clearance/assessment.md`, SHA-256 `b1802ca59e891a817a8e0bb9279b96973ec6e0784a77ea383916cbfd799b40e3` |
| Console Task 5 authority trace | `/Users/gqadonis/.codex/worktrees/agentic-ui-frontend/kubo/openspec/changes/agentic-ipfs-november-integration/identity-bridge-assessment.md`, SHA-256 `35bbed2e126e3c0979f3b1b40b3c9a9c8255c97f3afc992f938a8526501ea6e6` |

At the earlier checkpoint, T1 was held until the identity owner provides the approved policy revision, tenant-selection and record-write eligibility semantics, and the corresponding Gate issuance/configuration references. Check the handoff against these claims and the saved source pin. Then select the record-only policy under T2. Policy review, synthetic verification evidence and the final Task 5 handoff are outstanding. The console bridge stays disabled, Keto stays deferred, and verification and deployment remain separately authorized.

## T1 completed after the identity handoff

Flint Infra merged ADR 0002, ADR 0003 and the rights store at `06a4bcf611dfae527acaf1aa85d8454a5b454dba`. The registry seeds the application's `file-metadata-owner` role. The server-side console client resolves the verified subject through `rights.console_tenant`; only one distinct non-null authorized tenant permits exchange. Gate trusts that client's selected tenant. The file policy must scope an explicit Forge grant to the approved owner pair. A signed role alone cannot distinguish entitled writers.

Gate PR #23 merged at `5c1ba332a859c63b4041a8db5da3d81d968a5ac5`. Infrastructure reports 21 of 21 isolated issuer/rights cases matched against Gate `1a4cd4297d0f952b0bfe232824070b6f97ac2a69`. Its output contract is a short-lived Gate-signed token with preserved subject, resolved tenant, authenticated role, Flint Core audience and the console backend as actor. Those results establish identity issuance in the isolated rig. Forge authorization needs its own evidence.

[Accepted membership policy](https://github.com/Prometheus-AGS/flint-infra/blob/06a4bcf611dfae527acaf1aa85d8454a5b454dba/docs/design/identity/0002-rights-and-authorization-store.md) · [Gate implementation](https://github.com/Know-Me-Tools/flint-gate/pull/23) · [Upstream evidence](https://github.com/Prometheus-AGS/flint-infra/blob/06a4bcf611dfae527acaf1aa85d8454a5b454dba/docs/design/identity/evidence/issue-22/summary.md)

## T2 reviewed source

The local commit `c74c101826b59d8db50d142ea2f339aecf602f69` adds the Cedar record grant and the gateway context derived from the verified console bearer. A distinct source reviewer traced the file mutation entry point, the pinned identity contract, Cedar string escaping and enabled-policy loading. They found no blocking source findings. Grants are explicit per subject and tenant and permit insert and update on file records. Type registration requires separate authority.

## T3 approval packet and Task 5 status

The [fixture manifest](../../../scripts/issue40-fixture/manifest.json) and [approval instructions](../../../scripts/issue40-fixture/README.md) bind verification to two disposable local PostgreSQL instances, one per named check. The issuer uses an ephemeral loopback signing fixture representing the accepted Gate claim shape. HTTP requests exercise signature verification, the file route, the database-backed Cedar loader and the PostgreSQL adapter. SQL controls vary tenant and subject independently under the authenticated role. The identity issuer supports one tenant per subject at a time, so the ordinary HTTP cross-tenant control uses distinct entitled subjects. An additional synthetic out-of-scope tenant claim checks Forge grant scoping.

The Forge phase has run zero boundary commands. FILE-1 through FILE-5 are NOT_EXERCISED. T3 awaits explicit approval to provision, write and clean up under the manifest. Receipts from an authorized run belong under this phase's `issue40-run/` directory. One potential diagnostic matters for FILE-3. Migration 0021 validates a record's private type before insert. If it rejects a foreign insert with SQLSTATE 23514, the harness reports that the required records-table RLS violation was unobserved. A validation error cannot satisfy that criterion.

WebUI PR #13 Task 5 can use the upstream identity handoff and reviewed policy source for planning. Authorization acceptance and bridge activation await Forge evidence. Keto remains deferred.

The local branch `codex/issue-40-record-permissions` pins the verification kit and its manifest at `7bd87d9c2b22c81e186849eaa90f4c498549fdc7`. Static checks matched the manifest, migration hashes, command definitions and source bytes. Receipt parsing was checked with synthetic log text. The approved boundary has zero runs.

## Approved run and Task 5 handoff

The operator approved the pinned fixture, one execution of each named check and cleanup in chat on 2026-11-27. One packet executed two named tests, with one pass and one failure. FILE-3 failed the records-table RLS insert diagnostic on both ownership axes. The packet stopped and the local fixtures were removed. Forge #40 acceptance remains blocked. The other four scenarios passed. [Redacted Task 5 results and cleanup evidence](issue40-run/README.md)

## Approved FILE-3 repair and verification

The operator authorized the repair, synthetic writes and one fresh execution of each named check. Local commit `8a59888c9d5c2595f35ed8d6e3126d66dcc99220` introduces migration 0022 and pins it in the disposable fixture. Both tests passed once and all five scenarios satisfied the contract. Direct SQL produced the records-table RLS violation on both ownership axes, and the owner validation control confirmed rollback. Both fixtures were removed. [Updated Task 5 handoff and receipts](issue40-run-repair/README.md)
