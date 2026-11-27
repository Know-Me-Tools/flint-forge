# Private file metadata grants

One enabled grant permits a single verified Kratos subject to insert and update private file metadata in one approved tenant through `PUT /files/v1/records/{file_id}`. Provision a pair only from an authorized active `ipfs-console/file-metadata-owner` grant whose tenant agrees with `rights.console_tenant(subject)`. The Cedar grant is a separate operator-controlled entitlement. The file route picks the action from the expected revision. PostgreSQL checks both owner fields under forced RLS. The target gateway mode is `FLINT_AUTHZ_MODE=rls`. [Issue #40](https://github.com/Know-Me-Tools/flint-forge/issues/40)

## Trusted inputs

The console backend resolves exactly one tenant for the verified subject and keeps the exchanged token server-side. Gate trusts the tenant supplied by its confidential console client. Forge verifies the token signature, issuer, audience and expiration. The file-record Cedar entry point derives its `consoleTenant` attribute from the following verified claims.

| Claim | Required value |
| --- | --- |
| `iss` and `aud` | `https://gate.know-me.tools` and the scalar string `flint-core` |
| `sub` | Nonempty original subject, matching the exact Cedar User principal |
| `tenant_id` | Nonempty string matching the tenant in the approved grant |
| `role` | `authenticated`, also the effective gateway role |
| `act.sub` and `flint_kind` | `ipfs-console-backend` and `agent` |

The verified tenant reaches Cedar only on the dedicated record mutation path and only for the expected delegation. A type-registration or generic table request receives no console attribute from this path. The record grant carries only the two actions on the exact record resource. The presence of authenticated role, profile data or arbitrary request context grants no access under this policy. [Accepted membership policy](https://github.com/Prometheus-AGS/flint-infra/blob/06a4bcf611dfae527acaf1aa85d8454a5b454dba/docs/design/identity/0002-rights-and-authorization-store.md) · [Gate exchange contract](https://github.com/Prometheus-AGS/flint-infra/blob/06a4bcf611dfae527acaf1aa85d8454a5b454dba/docs/design/identity/0003-ipfs-console-token-exchange.md) · [Gate source change](https://github.com/Know-Me-Tools/flint-gate/pull/23)

## Materializing a policy row

`forge_policy::files::record_grant(subject, tenant)` renders the versioned template using Cedar string literals and returns `None` for an empty input. The offline example accepts the subject and tenant on separate input lines and writes the policy to standard output. Keep any rendered policy containing real identifiers in the operator's protected policy store. This example uses synthetic inputs.

```sh
printf '%s\n' '00000000-0000-4000-8000-000000000001' 'fixture-tenant' |
  cargo run --quiet --locked -p forge-policy --example render_console_grant
```

Use the same renderer for the approved isolated fixture and the eventual operator provisioning path. Store the rendered text verbatim in `flint_meta.cedar_policies.policy_text`, with a stable opaque row ID, a descriptive non-identifying name, and the approved enabled state. Bind database parameters for all values. Each grant is one static policy. The gateway loads the enabled rows through `DbPolicySource` on startup. A running evaluator retains its prior set until a successful reload or restart. Have the operator retire the corresponding Cedar row on expiry, revocation or tenant reassignment and account for every running evaluator. The console credential has a 300-second lifetime. A stale enabled row can match a subsequently issued credential for the same pair. Rights-store changes do not automatically update Forge policy rows. Publishing or activating policy and enabling the console require separate authorization.

## Fixture policy baseline

The source baseline at Forge `ce1efd189bcb520242acd530848e3b920d1036cd` creates the policy table in migration 0005 and seeds one enabled policy, `a2ui-rls-mutations-v1`, in migration 0019. Its resource list contains only A2UI catalog tables. SHA-256 of `migrations/0019_a2ui_cedar_mutation_capability.sql` is `11935b726548eb8c15aa4ee9edc8b6033cd93c236e39b173bb47f509c9c3dfc2`. Migration 0021 defines owner-scoped RLS and grants no Cedar capability.

At the approved verification boundary, compare the actual enabled row IDs and exact policy texts loaded from the isolated database against this baseline plus the explicitly rendered synthetic grants. Capture a digest of the compared set and the source revisions. Additional enabled policies require independent overlap review for records and types. Fixture evidence applies to the policy set exercised. A live installation can contain separately provisioned policies.
