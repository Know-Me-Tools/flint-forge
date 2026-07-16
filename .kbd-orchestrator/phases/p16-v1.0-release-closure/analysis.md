# Analysis — p16-v1.0-release-closure

**Phase:** 16 — v1.0 Release Closure
**Analyzed:** 2026-07-09
**Analyst:** claude-code
**Question posed:** "What will it take to have this repository fully ready for
consumption by real customers?"

---

## Scope note (read first)

The question asked was broader than what `/kbd-analyze` produces. This skill
researches **build-vs-adopt library decisions** for gaps the assessment found.
It has no mandate over pricing, contracts, support staffing, or go-to-market.

Web research (Tier 4, 4 queries) returned **almost nothing actionable**. Queries
for "developer infrastructure beta checklist" and "Postgres BaaS production
readiness" surfaced SMB disaster-recovery listicles, RLS explainers, and
competitor-comparison SEO pages. Two sources are worth citing and both are
process standards, not product intelligence:

- **OpenSSF, "Guide to Implementing a Coordinated Vulnerability Disclosure
  Process for Open Source Projects"** — https://openssf.org/resources/guides/
- **Semantic Versioning 2.0.0** — https://semver.org/ (CHANGELOG.md already
  claims adherence)

**Confidence in the web tier: LOW.** I am not going to launder generic listicles
into a customer-readiness plan. The findings below are grounded in the
repository, where the evidence is specific and checkable. This is the honest
outcome of the research budget, not a shortfall I'm hiding.

**Blocking caveat:** the readiness question cannot be answered without knowing
*what is being sold to whom*. Self-hosted OSS, managed SaaS, and licensed
on-prem have disjoint readiness bars (see Open Questions).

---

## Finding 1 — The default config silently delivers zero realtime events

**Severity: CRITICAL. This is the most customer-visible defect found.**

`fdb-gateway/src/main.rs:601` selects the change source:

```rust
let use_listen = std::env::var("FLINT_CHANGE_SOURCE").as_deref() == Ok("listen");
let change_source = if use_listen { ListenChangeSource::new(..) }
                    else { FabricChangeSource::new(..) };   // ← default
```

`FabricChangeSource::watch()` (`fdb-realtime/src/lib.rs:116`) does this:

```rust
tracing::warn!(..., "OQ-FRF-1: WatchEntityType not yet available in FRF; returning empty stream");
let empty = futures::stream::empty().boxed();
Ok(empty)
```

It returns `Ok` — not an error. A subscriber connects successfully, the Keto
check passes, and then **no events ever arrive**. It fails open into silence.

`ListenChangeSource` (`fdb-realtime/src/listen.rs:215`) is a complete
LISTEN/NOTIFY implementation with the Keto check failing closed. It works.

**Where each deployment path lands:**

| Path | `FLINT_CHANGE_SOURCE` | Subscriptions |
|---|---|---|
| Helm (`deploy/helm/flint-forge/values.yaml:55`) | `"listen"` | ✅ work |
| `.env.example:59` | commented out | ❌ silent no-op |
| docker-compose | unset | ❌ silent no-op |
| bare `cargo run -p fdb-gateway` | unset | ❌ silent no-op |

So the Helm user is fine and everyone following the quickstart is not. Meanwhile
`README.md:40` advertises "**GraphQL Subscription** — `async-graphql` over
`graphql-transport-ws`" with no caveat, and `README.md:104` promises RLS
enforcement on "every query / subscription event."

**Verdict — BUILD, trivial, do it first.** Either invert the default to
`listen`, or make `FabricChangeSource::watch()` return
`Err(StreamError::Unavailable)` instead of an empty `Ok`. A feature that is
absent is a bug; a feature that *pretends to work* is an incident. No library
adoption is involved.

---

## Finding 2 — amd64 image is broken (carried from assessment)

Already documented in `assessment.md`. `pg_cron` fails to link on linux/amd64
(`cannot find -lintl`, `images/postgres18/Dockerfile:97`). arm64 succeeds, so
the multi-arch manifest is skipped and the published `flint-forge-pg:18` is
arm64-only. The `Postgres integration tests` CI job has **0 successes in 8 runs**.

**Verdict — BUILD, one-line fix.** Add `gettext`/`libgettextpo-dev` to the
`pgcron` builder stage. No adopt decision. Blocks everything downstream.

---

## Finding 3 — Customer-facing governance files are absent

Present: `LICENSE`, `CHANGELOG.md` (Keep a Changelog + SemVer, per its header).

**Missing, all four:** `SECURITY.md`, `CONTRIBUTING.md`, `SUPPORT.md`,
`CODE_OF_CONDUCT.md`.

`SECURITY.md` is the one that actually matters for a customer beta: it is where
a researcher or customer reports a vulnerability, and its absence means reports
arrive as public GitHub issues or not at all. `cargo audit` already runs in CI
(`ci.yml:29`) and a RUSTSEC advisory was fixed in v1.0.0, so the project *does*
take supply chain seriously — it just has no intake channel.

**Verdict — ADOPT a template, don't invent one.** OpenSSF's CVD guide is the
reference implementation; GitHub renders `SECURITY.md` natively in the
vulnerability-reporting UI. Cost: hours, not days. No code, no dependency.

---

## Finding 4 — Stale doc-comments overstate and understate the code

Two doc-comments disagree with their own implementations, in opposite directions:

- `fdb-reflection/src/compilers/rest/mod.rs:62` — "CRUD handlers remain
  `todo!()` stubs pending the query-builder landing." **False.** `handle_insert`,
  `handle_update`, `handle_delete` are all implemented in
  `compilers/rest/mutations.rs`. The comment understates the product.
- `README.md:40` advertises subscriptions unconditionally. Overstates it
  (Finding 1).

I initially mis-flagged 7 `todo!()` + 1 `unimplemented!()` as live stubs. They
are **all inside comments and doc-strings**; `grep` matched the prose. There are
**zero `todo!()` on any live path**, which is a real credit to the codebase and
consistent with CLAUDE.md's integration-first rule. Recording the correction
because the wrong version nearly reached this document.

**Verdict — BUILD (documentation).** Costs an hour. No adopt decision.

---

## Finding 5 — k6 is not a gate, and never was

`ci.yml:98`: `if: github.event_name == 'workflow_dispatch'`. The k6 job is
skipped on every push and requires a `STAGING_BASE_URL` secret that no
environment supplies. `perf/k6/{health,regression,components,mcp_tools}.js`
exist as scripts. Nothing measures them; nothing enforces a threshold.

Calling these "performance baselines" in p15's reflection was generous. There is
no performance evidence for this product at any load, on any hardware, ever.

**Verdict — DEFER, but stop claiming it.** Real load validation needs staging
infrastructure that does not exist (`reflect.json` debt item 3). The honest
interim move is to delete the "validated" language, not to fabricate numbers.

---

## What "ready for real customers" actually requires

Ordered by whether a customer hits it on day one. Nothing here needs a new
library — **every gap is build, config, or prose.**

### Tier 0 — Ship-blockers (a customer hits these immediately)

1. **amd64 image** (Finding 2). Cloud is x86_64. Today they cannot `docker pull`.
2. **Realtime default** (Finding 1). Subscriptions silently return nothing
   outside Helm.
3. **One green `Postgres integration tests` run.** Zero recorded successes means
   *no* integration claim in any doc is currently evidence-backed.

### Tier 1 — Beta-blockers (they hit these in week one)

4. **`SECURITY.md`** + a disclosure address (Finding 3).
5. **Backup / restore / PITR procedure.** `docs/` has no runbook for this. Anyone
   trusting a database with data will ask, and there is no answer on disk. This
   is the single largest *undocumented* risk and I could not evaluate it,
   because nothing exists to evaluate.
6. **Upgrade path.** SemVer is claimed in CHANGELOG.md; no migration guide
   exists between versions, and `sqlx migrate` ordering was only just repaired
   in p15-c002.
7. **Doc/reality reconciliation** (Finding 4).

### Tier 2 — Trust-builders

8. Support channel + response expectation (`SUPPORT.md`).
9. Load evidence, once staging exists (Finding 5).
10. `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`.

### Not assessable from this repository

Pricing, contracts/ToS/DPA, GDPR/SOC2 posture, on-call rotation, status page,
incident comms, customer identity. These are business artifacts. **A beta with
real customers needs several of them and none exist in-tree.** Flagging, not
estimating — I have no basis to.

---

## Build-vs-Adopt Summary

The honest headline: **this analysis surfaced no adopt candidates.** Every
identified gap is a defect, a config default, a missing markdown file, or absent
infrastructure. That is why `library-candidates.json` is empty — not because
research was skipped, but because the gaps are not library-shaped. Manufacturing
candidates to fill the schema would be dishonest.

| Gap | Verdict | Cost | Dependency |
|---|---|---|---|
| pg_cron amd64 link | BUILD | 1 line | none |
| Realtime default / fail-open | BUILD | ~10 lines | none |
| Green integration CI | BUILD | follows amd64 fix | pg_cron |
| SECURITY.md | ADOPT template (OpenSSF/GitHub) | hours | none |
| Backup/PITR runbook | BUILD | days | needs an owner decision |
| Upgrade/migration guide | BUILD | days | SemVer policy |
| Doc reconciliation | BUILD | hours | none |
| Load validation | DEFER | — | staging infra |

---

## Open Questions (blocking Spec)

1. **What is being sold, and to whom?** Self-hosted OSS, managed SaaS, or
   licensed on-prem? The readiness bar differs enormously — a managed offering
   needs status page, on-call, and DPA; self-hosted OSS needs none of them and
   needs a far better upgrade guide. **Nothing downstream can be specced without
   this answer.**
2. **Does the beta include realtime subscriptions?** If yes, Finding 1 is Tier 0
   and the FRF dependency (OQ-FRF-1, open since p3) needs a resolution date. If
   no, the feature must be removed from README before a customer reads it.
3. **Is amd64 required?** (carried from assessment) If every target is arm64, fix
   the manifest rather than the linker. README and Helm imply no such limit.
4. **Who receives a vulnerability report today?** There is no address anywhere in
   the repo.
5. **Does any backup/restore procedure exist outside this repository?** If it
   exists in ops runbooks elsewhere, Tier-1 item 5 collapses to a doc link. If
   not, it is genuine engineering work.
6. **Re-tag v1.0.0 or ship v1.0.1?** (carried from assessment, still unanswered)

---

## Recommendation

**Do not run `/kbd-spec` yet.** Open Question 1 gates the entire phase: the
Tier-1/Tier-2 lists change composition depending on the delivery model, and
speccing against a guess would bake in the wrong readiness bar.

Sequencing that holds regardless of the answer:

1. Fix `pg_cron` amd64 (unblocks CI, image, and every integration claim).
2. Fix the realtime default or make it fail loudly.
3. Get one green integration run.
4. Then re-open the readiness question with Q1 answered.

Steps 1–3 are unambiguous, cheap, and independent of the business model. They
are also, notably, the same three things p15 believed it had already done.
