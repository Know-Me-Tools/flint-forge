# Security Policy

Flint Forge is self-hosted software: you run it, you operate it, and there is
no vendor on call for you. This document exists so a researcher or operator
who finds a vulnerability has a clear, private path to report it — and so we
are honest about what response you can actually expect.

## Reporting a Vulnerability

**Do not open a public GitHub issue for a security vulnerability.**

Report privately using GitHub's [private vulnerability
reporting](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing/privately-reporting-a-security-vulnerability)
feature, enabled on this repository:

1. Go to the [Security tab](https://github.com/Know-Me-Tools/flint-forge/security).
2. Click **Report a vulnerability**.
3. Describe the issue, affected version(s), and reproduction steps.

This routes the report to the maintainers privately, outside the public issue
tracker, and lets us collaborate on a fix in a private fork before disclosure.

## What to Expect

This project does not have a dedicated security team or a 24/7 on-call
rotation. What we commit to:

- **Acknowledgement:** we will acknowledge a new report within **5 business
  days**.
- **Status updates:** if a report requires investigation, we will provide a
  status update at least every **2 weeks** until it is resolved or declined.
- **No fixed resolution SLA.** Severity and complexity vary too much to
  promise a fix-by date up front. High-severity issues (anything touching
  `flint_vault` secret handling, JWT/RLS context propagation, or the WASM
  sandbox) are treated as the highest priority work in the project.

We would rather state modest numbers we actually meet than copy a template
promising same-day triage nobody is staffed to deliver.

## Supported Versions

| Version | Supported |
| ------- | --------- |
| 1.0.x   | ✅ |
| < 1.0   | ❌ (pre-release; upgrade to 1.0.x) |

Flint Forge follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
(see `CHANGELOG.md`). Security fixes are backported to the latest minor
release of the current major version only. There is no long-term-support
branch at this time — as a self-hosted project, operators are expected to
track releases on the current major version.

## Disclosure Policy

We follow **coordinated disclosure**:

1. You report privately (see above).
2. We confirm, investigate, and develop a fix.
3. We coordinate a disclosure date with you — by default, **90 days** from the
   initial report, or sooner if a fix ships first. We will ask for more time
   if a fix is unusually complex; we will not ask for less time to hide a
   longer-than-expected fix.
4. We publish a GitHub Security Advisory and, where applicable, request a CVE
   through GitHub's advisory database.

Credit is given to reporters in the advisory unless you ask to remain
anonymous.

## Scope

This project ships several security-sensitive components. Reports touching
any of the following are especially high priority:

- `flint_vault` — XChaCha20-Poly1305 secret store, KMS-wrapped DEK
- `flint_auth` / `fdb-auth` — JWT verification and Postgres RLS context
  (`SET LOCAL ROLE`, `request.jwt.claims`) propagation
- `fke-runtime` — the Wasmtime WASM component sandbox executing
  externally-supplied components
- The four `ext-flint-*` pgrx extensions, which run as `unsafe`,
  in-process Postgres code

Out of scope: vulnerabilities requiring physical access to a deployment,
social engineering of a specific operator, or issues in third-party
dependencies that should be reported upstream (though we're glad to help
route those — please still tell us).

## Supply Chain

CI runs `cargo audit` on every build (`.github/workflows/ci.yml`) and
dependency advisories are tracked and patched as part of the normal release
process (see `CHANGELOG.md` for examples, e.g. RUSTSEC-2026-0204).
