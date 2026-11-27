# Goals

- Source of scope and acceptance criteria is [Forge issue #40](https://github.com/Know-Me-Tools/flint-forge/issues/40). The consumer is Task 5 in the IPFS console integration linked there.
- Add or identify a reviewed, versioned Cedar policy for insert and update on `flint_files.records`. Document eligible principals, permit conditions, and consumption of the approved Flint identity binding. Preserve existing tenant and subject isolation in PostgreSQL RLS.
- Scope this phase to Forge’s existing `rls` mode, with Cedar authorizing mutations and PostgreSQL enforcing owner isolation. The operator deferred all Keto work on 2026-11-27. Keep `flint_files.types` registration and grants outside this phase.
- At the separately approved verification boundary, use isolated synthetic data to demonstrate owner create and update and denial for callers without the grant, callers from a different tenant, and callers with a different owner in the same tenant.
- Hand Task 5 the policy source revision, expected verified claims, and approved verification evidence. Omit credentials, tokens, and live identity data. Keep the console bridge disabled pending both identity and Forge handoffs. Verification and deployment require separate approval.
