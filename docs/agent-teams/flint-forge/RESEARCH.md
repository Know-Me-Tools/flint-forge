# Flint Forge development team: research and design

Research date: 2026-09-24, America/Chicago. This is a team-definition task, not
authorization to implement the convergence roadmap or to claim Supabase parity.
Firecrawl web searches, PostgreSQL 18/PostgREST Context7 documentation, installed
harness help/source, the Forge source tree and its Compass graph informed this team.
[Search provenance](evidence/web-sources.json) includes screened candidates as well
as adopted sources. Native exporter receipts record their own contract snapshots.

## Recommendation

Use an eleven-role specialist pool. Most changes need a lead, one implementer and
an independent reviewer; add another specialist only for a separate boundary.
Default to no more than four concurrent agents and one local Rust build writer.
No runtime is started by these definitions. The harness owns spawning and execution;
the team ledger records coordination and does not enforce filesystem or Cedar policy.

The Supabase architecture separates PostgreSQL, REST, metadata, identity, realtime,
storage and functions. That separation is useful for assigning expertise, while
Forge's Rust modules and the surrounding Flint projects determine ownership here.
[Supabase architecture](https://supabase.com/docs/guides/getting-started/architecture)

## Roles and concrete boundaries

| Native agent ID | Responsibility | Primary write ownership |
|---|---|---|
| `forge-lead` | Architecture, task decomposition, shared interfaces and integration | Workspace manifests; shared domain/port crates; decision records |
| `forge-postgres18` | PostgreSQL 18 SQL, plans/indexes, migrations, pooled transactions, auth/vault extensions | `fdb-postgres`, `ext-flint-auth`, `ext-flint-vault`, migrations |
| `forge-security` | Verified identity, role mapping, Keto/Cedar integration and security review | `forge-identity`, `forge-policy`, `fdb-auth`; security findings |
| `forge-quarry-api` | PostgREST/RPC compatibility and hybrid GraphQL behavior | `fdb-query`, `fdb-app`, `fdb-gateway`; compatibility matrix |
| `forge-metadata` | postgres-meta comparison, catalog reflection and generated contracts | `ext-flint-meta`, `fdb-reflection`; metadata records |
| `forge-realtime` | LISTEN/Fabric adapters, hooks, outbox and replay | `fdb-realtime`, `ext-flint-hooks`; event contracts |
| `forge-kiln` | WASM host, signed artifacts, component registry, stores and WIT | Kiln runtime/server/registry/store/signing crates; WIT |
| `forge-ember` | In-database model work, provider budgets and governed skills | `ext-flint-llm`, `flint-skill`; Ember records |
| `forge-sdk` | React/Flutter consumers and CLI contracts | `packages`, `forge-cli`, existing Flint UI skill |
| `forge-release` | PostgreSQL/extension packaging, observability, recovery and operator docs | Docker/deploy/observability, scripts and CI workflows |
| `forge-reviewer` | Independent conformance, isolation, integration and code-quality review | Root tests/performance assets and review findings; no production fixes |

These are future assignment boundaries, not permission to displace an active owner.
The manifest holds the exact globs. Crate-local tests remain with that crate's author;
the reviewer owns shared acceptance assets and independently judges supplied evidence.
Gateway integration is intentionally owned by Quarry even when a subsystem expert
specifies the contract. SQL migrations have one database owner. This avoids a
security reviewer, realtime expert and metadata expert all editing the same files.

The generic creator guide proposed implementation/security/docs/review roles. We
refined that baseline because Forge has independently assignable database, compiler,
runtime and transport subsystems. A dedicated full-time designer, marketing agent,
or C-level twin would not improve this coding team's current deliverables. Add one
for a concrete product assignment rather than making it a standing execution cost.

## What the call graph changes about the team

The verified graph covers **218 files in 30 crates**, with **3,213 nodes and 7,338
relationships**, including all five workspace-excluded Anvil extensions. The source
revision, graph digest and 41 representative call edges are recorded in
[call-graph evidence](evidence/call-graph.json). Actual Compass CLI results are saved
for [Quarry entrypoint](evidence/quarry-main-callees.json) and
[Cedar evaluation](evidence/cedar-callees.json).

| Source evidence | Team consequence |
|---|---|
| `fdb_gateway::main` calls `fdb_gateway::bootstrap::run` | One Quarry owner integrates gateway behavior; specialists do not independently rewrite composition. |
| `fke_server::main` calls `EdgeRuntime::new` and component adapters | Kiln needs a runtime/capability specialist, separate from SQL/API work. |
| Cedar evaluation calls `build_entity_uid` and `RlsContext::subject` | Security must review identity-to-policy translation alongside database isolation. |
| Identity verification calls JWKS lookup/refetch helpers | Token rotation, issuer/audience validation and cache behavior need explicit security acceptance. |
| Vault decryption calls `working_cipher` | Database-extension expertise and independent secret-handling review are both needed. |
| `PgGraphQl::execute` source acquires scoped context, calls `graphql.resolve` and commits | API and PostgreSQL owners must jointly prove mutations commit and preserve RLS; trait dispatch is not fully established by the structural graph. |

Graph edges are extractor evidence, not complete runtime reachability. In particular,
`ext-flint-meta` includes SQL through a macro, so a Rust-only graph cannot establish
its catalog-query behavior. The metadata role must read extension SQL. Inline tests
also remain in the graph. No application integration test was run for this definition task.

## Compatibility expertise and acceptance

**PostgreSQL 18 and RLS.** The database expert handles query planning, pool budgets,
transaction scope, migrations and extension version compatibility. The security
expert reviews the resulting access controls independently. Grants, `USING`,
`WITH CHECK`, table ownership, `FORCE ROW LEVEL SECURITY`, `BYPASSRLS`, views and
definer functions are separate controls. `FORCE` does not remove superuser or
`BYPASSRLS` exemptions. Pool reuse must not transfer tenant context between requests.
[PostgreSQL 18 RLS](https://www.postgresql.org/docs/18/ddl-rowsecurity.html),
[CREATE POLICY](https://www.postgresql.org/docs/18/sql-createpolicy.html),
[function security](https://www.postgresql.org/docs/18/sql-createfunction.html)

**PostgREST APIs.** Compatible JSON alone is insufficient. Pin a compatibility
version, then check filters/nulls, embedding and foreign-key disambiguation, RPC
resolution, profile/Prefer/range/count headers, writes, errors and schema-cache
changes. Research used version 14 documentation; selecting a production parity
baseline remains part of an actual implementation work order. Supported, partial
and deliberately unsupported features must be distinguished.
[PostgREST API](https://docs.postgrest.org/en/v14/references/api.html),
[resource embedding](https://docs.postgrest.org/en/v14/references/api/resource_embedding.html),
[errors](https://docs.postgrest.org/en/v14/references/errors.html)

**GraphQL.** Forge's query/mutation path uses `pg_graphql`; subscriptions use the
separate async-graphql surface. Role-sensitive introspection and combined schemas
need contract tests, not a generic Apollo resolver rewrite. PostgreSQL privileges
and RLS control access; introspection configuration is version-sensitive, with
1.6+ documented as disabled by default. Compare role-specific schemas and verify
mutation commit, errors, variables, subscription revocation and client operations.
[pg_graphql security](https://supabase.com/docs/guides/graphql/security),
[extension configuration](https://supabase.com/docs/guides/database/extensions/pg_graphql)

**postgres-meta.** Metadata is a privileged management plane, distinct from ordinary
row APIs. Compare listing/managing roles, schemas, tables, columns, constraints,
functions, indexes, extensions and policies against Forge's actual operations.
Require catalog fidelity for quoted identifiers, composite keys, generated columns
and overloads; verify reflection invalidation and provisioning drift. This team
does not assume every postgres-meta endpoint is already implemented.
[postgres-meta source](https://github.com/supabase/postgres-meta),
[PostgreSQL catalogs](https://www.postgresql.org/docs/18/catalogs.html)

**Realtime.** Current Forge source makes `ListenChangeSource` the rollout default;
Fabric is an alternative, not a mandatory dependency. Subscription admission is
insufficient evidence of delivery authorization. Test tenant visibility, deletes,
revocation, reconnect, replay gaps and duplicate effects for each transport. Supabase
Realtime is a useful behavioral comparison, not Forge's transport implementation.
[Supabase Postgres Changes](https://supabase.com/docs/guides/realtime/postgres-changes)

## Ecosystem relationship map

This maps local source observations plus the convergence planning documents. Target
contracts below are not claims that every mode is implemented today.

| Project | Relevant authority and handoff |
|---|---|
| Flint Forge | Authoritative domain transactions, database/API behavior, reflection, extensions and edge-function execution. It remains useful without agents. |
| Flint Gate | Identity/access-plane integration; Forge still verifies its own accepted identity/audience and enforces local policy. |
| Flint Realtime Fabric | Authorized transport, stream/resume contracts and sync integration; Forge retains domain-write and row-disclosure authority. |
| UAR | Agent execution harness and governed tools; shared WASM contracts need explicit version compatibility, not duplicate loops. |
| BossFang/librefang | Channels, orchestration and explicit native/delegated execution selection; not a replacement database authority. |
| The Boss | Controller/studio for local or remote services. Managed-process control differs from attaching to an external instance. |
| KnowMe app/system | Personal/decentralized and offline consumers; local personal replication and enterprise-authoritative writes have different rules. |
| surreal-memory-server | Optional agent knowledge/handoff substrate, not the domain database, team scheduler or authorization service. |
| Full/mini skill packs | Upstream agent-team tooling; this task consumes the installed skill without altering either pack or its deployment. |

Supabase Auth and object storage should not be assumed to live wholly inside Forge.
Gate is an existing identity boundary; Kiln component stores are not automatically
a Supabase Storage-compatible service. A future parity request must assign such
gaps explicitly instead of silently expanding this team's implementation scope.

Local architecture sources:
[operational modes](../../../../worktrees/agent-fabric-convergence/librefang/docs/plans/agent-fabric-convergence/operational-modes.md),
[ownership/dependencies](../../../../worktrees/agent-fabric-convergence/librefang/docs/plans/agent-fabric-convergence/dependencies.md),
[Forge specification](../../FLINT-FORGE-SPEC.md).
The shared planning checkout is a local reference, not a portable repository dependency.

## Skill selection and provenance

The [Supabase-maintained PostgreSQL skill](https://github.com/supabase/agent-skills)
is available through [skills.sh](https://www.skills.sh/supabase/agent-skills/supabase-postgres-best-practices).
An already-installed MIT-labelled version **1.1.1** was reused as instruction files
and references only; no external package or executable was installed. The local
snapshot is not claimed to be the newest upstream version. File hashes and source
locations are captured in the installation/provenance receipts.

That skill is valuable for indexes, plans, pools and locking, but its security examples
need Forge-specific interpretation. We inspected the RLS references: session-level
`SET` must not replace transaction-local identity; correlated expressions are not
necessarily evaluated once; a definer function's effective privileges and RLS behavior
depend on its owner and configuration. Unmeasured speedup numbers are not acceptance
criteria. Primary PostgreSQL documentation and tests govern these decisions.

Seven project-authored, AgentSkills-format supplements bind that knowledge to Forge:

| Skill | Main users |
|---|---|
| `forge-delivery` | Every role: ownership, KBD, test economy, handoff and evidence |
| `forge-postgres18-security` | Database, security, API, realtime, Ember, release and review |
| `forge-api-compat` | Quarry, metadata, SDK and review |
| `forge-metadata-contracts` | Metadata and review |
| `forge-fabric-events` | Lead, security, realtime and release |
| `forge-wasm-ember` | Kiln and Ember, with explicitly separate execution semantics |
| `forge-client-contracts` | SDK/CLI consumers |

The installed community `graphql-inspector-diff` and `graphql-inspector-validate`
skills were inspected but not copied or bound: provenance and some example flags/
classification guidance need validation against the actual tool release. The team
instead links the [upstream GraphQL Inspector project](https://github.com/graphql-hive/graphql-inspector)
for optional schema and operation checks. The broad Supabase product skill is also
not bound: it can direct work toward Supabase-hosted services and commands that are
not Forge's architecture. No trustworthy dedicated postgres-meta or PostgREST skill
was established in the bounded search; the new focused skills fill those gaps with
primary sources rather than asserting nonexistent community coverage.

Additional candidates were screened in follow-up skills.sh searches:
[postgresql-table-design](https://www.skills.sh/wshobson/agents/postgresql-table-design)
offers community schema/index guidance, but was not adopted without full instruction
review and adds overlap with the selected database skill;
[neon-postgres](https://www.skills.sh/neondatabase/agent-skills/neon-postgres)
focuses on Neon's hosted/serverless workflow, so it is not a default skill for Forge's
sovereign deployment. Neither candidate was installed. A PostgREST-specific search
returned unrelated and insufficiently established candidates rather than a verified
replacement for primary API documentation.

The authored entrypoints use the required `name`/`description`, supported license and
string metadata fields from the [AgentSkills specification](https://agentskills.io/specification).
No scripts or hooks are added to the skill payloads.

## Models and harness support

Codex hard roles bind `gpt-6-astra`; SDK/release roles bind `gpt-5.6-sol`, using
configured/host-offered model identities. Strength categories are task-policy
annotations, not benchmarks. Selection receipts preserve declared availability and
unknown prices; there was no live inference or provider billing probe. No monetary
ceiling or cheapest-provider claim is made. Use task-level low-tier routing only after
discovering a suitable model; never downgrade an RLS or security review silently.

Claude inherits the configured session model. OpenCode binds its configured
`kimi-for-coding/k3`. Kimi's observed default is `kimi-code/k3`; its supported
invocation/global setting, not role model frontmatter, controls selection. MiniMax,
Copilot and service exports inherit their configured defaults. Before costly or
restricted work, use agent-team-models with the selected deployment's actual discovery
and liter-llm catalog, explicit aliases, prices and capability requirements.

See [native setup and evidence levels](README.md) for local versions and installation
locations. Claude and Kimi plugin/marketplace alternatives are staged, but the project
agent copies are the selected installation path. DeepSeek persona/team profiles are
experimental exports; UAR and BossFang payloads are registration proposals, not live
agents. Source-verified export is never reported as native execution certification.

## Before any implementation task

Reconcile KBD and active-owner evidence, pin the task's compatibility versions and
database fixture, select only the necessary roles, and reserve one writer per path.
Then prove the complete vertical slice: database transaction → API → client/event
delivery → authorization → recovery. The independent reviewer should demand both
positive and negative evidence, particularly for cross-tenant and privileged metadata
cases. Preserve unavailable or unperformed checks as explicit gaps.
