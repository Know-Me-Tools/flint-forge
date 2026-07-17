# p16-c004 — Vulnerability disclosure channel

**Phase:** p16-v1.0-release-closure
**Priority:** P1 — Tier 1, beta-blocker
**Scope:** `SECURITY.md` (new), `.github/` config
**Delivery model:** self-hosted OSS

---

## Problem

There is **no `SECURITY.md`** and no disclosure address anywhere in the
repository. A researcher or customer who finds a vulnerability today has no
private channel; the report arrives as a public GitHub issue, or not at all.

This is not theoretical for this codebase. It ships:
- `flint_vault` — XChaCha20-Poly1305 secret store with a KMS-wrapped DEK
- JWT/RLS context propagation on every pooled connection
- a WASM sandbox executing customer-supplied components
- four pgrx extensions running as `unsafe` in-process Postgres code

The project already runs `cargo audit` in CI (`ci.yml:29`) and patched
RUSTSEC-2026-0204 in v1.0.0. The supply-chain hygiene is real. The intake
channel is missing.

## Change

Author `SECURITY.md` following the **OpenSSF Coordinated Vulnerability
Disclosure guide** (https://openssf.org/resources/guides/). GitHub renders this
file natively in its private-vulnerability-reporting UI.

Minimum content:
- **Where to report** — a monitored private address, or GitHub's private
  vulnerability reporting (enable it in repo settings).
- **What to expect** — acknowledgement window and a status-update cadence.
  State real numbers you can meet; do not copy a template's "24 hours" if
  nobody is on call. Self-hosted OSS does not require an SLA — it requires
  honesty about what the maintainers will actually do.
- **Supported versions** — which releases receive security fixes. `CHANGELOG.md`
  claims SemVer adherence; this table is where that claim becomes concrete.
- **Disclosure policy** — coordinated disclosure timeline.

Also add `SUPPORT.md` (where non-security questions go) and `CONTRIBUTING.md`.
`CODE_OF_CONDUCT.md` is optional for a beta and may be deferred.

## Acceptance Criteria

1. `SECURITY.md` exists at repo root and names a working private reporting
   channel that a maintainer monitors.
2. It states a supported-versions policy consistent with the SemVer claim in
   `CHANGELOG.md`.
3. GitHub's "Report a vulnerability" button is enabled, or the file gives an
   equivalent private path.
4. `SUPPORT.md` and `CONTRIBUTING.md` exist.
5. No committed response-time commitment the maintainers cannot meet.

## Non-Goals

- SLA, DPA, ToS, on-call rotation, status page, incident comms. **Out of scope
  for self-hosted OSS** — the operator runs the software; there is no service to
  page anyone about. (Confirmed by delivery-model decision, 2026-07-09.)
- CVE numbering authority / SBOM. Defer.

## Verification Command

```bash
test -f SECURITY.md && test -f SUPPORT.md && test -f CONTRIBUTING.md && echo OK
gh api repos/:owner/:repo --jq '.security_and_analysis'
```

## Risk

**Low.** Documentation only, no code.

## Open Questions

- **Who receives a vulnerability report today?** (from `analysis.md`) This must
  be answered by a human before the file can name an address. A `SECURITY.md`
  pointing at an unmonitored inbox is worse than none.
