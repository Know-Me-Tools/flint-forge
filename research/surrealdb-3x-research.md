# SurrealDB 3.x Research Report

**Research Date:** 2026-06-29  
**Version Focused:** SurrealDB 3.1.5 (latest 3.x patch as of June 2026)  
**Research Goal:** Evaluate SurrealDB as a potential second backend alongside an existing PostgreSQL-based platform

---

## 1. Architecture Overview

### Multi-Model Engine

SurrealDB is a Rust-built multi-model database that unifies multiple data paradigms in a single engine:

| Model | Native Support | Notes |
|-------|---------------|-------|
| Document | ✅ Native | Schemaless or schema-full tables |
| Graph | ✅ Native | Typed edges via `RELATE`, graph traversal in SurrealQL |
| Relational | ✅ Native | ACID transactions, table constraints |
| Time-series | ✅ Native | Temporal queries, `VERSION` clause |
| Geospatial | ✅ Native | Spatial predicates, within-radius queries |
| Key-Value | ✅ Native | Underlying storage abstraction |
| Vector | ✅ Native | HNSW + DiskANN indexes, similarity search |
| Full-text | ✅ Native | BM25 indexing, custom analyzers |

### Embedded vs Server Mode

SurrealDB separates **compute** (query layer) from **storage** (persistence layer), allowing the same API across deployment modes:

| Mode | Storage Engine | Use Case |
|------|---------------|----------|
| **In-memory** | SurrealMX | Testing, ephemeral workloads, embedded apps |
| **Embedded file** | RocksDB / SurrealKV | In-process persistence, edge devices, mobile |
| **Single-node server** | RocksDB (recommended) / SurrealKV (beta) | Small-to-medium production |
| **Distributed cluster** | SurrealDS / TiKV / FoundationDB | HA, horizontal scaling |
| **Browser** | IndexedDB via WASM | PWAs, offline-first web apps |
| **Cloud managed** | SurrealDB Cloud | Zero-ops managed service |

**Key architectural advantage:** You can prototype embedded in-memory, then deploy the same code to a distributed cluster without rewriting application code.

---

## 2. GraphQL Support

### Current State (v3.1)

SurrealDB now ships **native GraphQL** as a first-class query surface:

- **Schema generation:** Auto-generated from table/field definitions following Apollo conventions (singular `fetchX`, plural `listX`, `createX`/`updateX`/`deleteX` mutations)
- **GraphQL aliases:** `DEFINE FIELD ... GRAPHQL_ALIAS` decouples database identifiers from API consumer names
- **Deprecation annotations:** `GRAPHQL_DEPRECATED` on fields/tables
- **Cursor pagination:** `Connection` queries with `edges`, `pageInfo`, lazy-evaluated `totalCount`
- **Offset pagination:** `limit` + `start` still supported alongside cursor
- **Multi-model queries:** Full-text search, vector similarity, and time-series aggregation all queryable through GraphQL
- **Subscriptions:** GraphQL Subscriptions supported (added in 3.1)
- **Root-level field comments:** Document fields in generated schema

### Comparison with pg_graphql

| Aspect | SurrealDB GraphQL | pg_graphql (Postgres extension) |
|--------|-------------------|--------------------------------|
| Schema generation | ✅ Native, from SurrealQL definitions | ✅ From Postgres schema |
| Graph traversal | ✅ Native via `RELATE` edges | ❌ Requires custom resolvers |
| Vector search in GraphQL | ✅ Direct query | ❌ Not native |
| Real-time subscriptions | ✅ Built-in | ❌ Not supported |
| Cursor pagination | ✅ Native Connection type | ✅ Supported |
| Multi-model in one query | ✅ Document + graph + vector | ❌ Relational only |
| Maturity | 🟡 Evolving (3.1 breaking changes) | 🟢 Stable, simpler surface |

**Verdict:** SurrealDB's GraphQL layer is more powerful for multi-model workloads but is less mature. pg_graphql is simpler and more stable for purely relational data. If your platform needs graph traversal or vector search from GraphQL, SurrealDB has a genuine advantage.

---

## 3. Realtime Capabilities

### Live Queries

```sql
-- Subscribe to all changes on messages in a chat room
LIVE SELECT * FROM message WHERE room = room:general;

-- Subscribe to a specific record
LIVE SELECT * FROM user:jaime;

-- DIFF mode for patch-style updates
LIVE SELECT DIFF FROM person;
```

- Clients receive a UUID on subscription; `KILL <uuid>` to unsubscribe
- Notifications pushed over **WebSocket** (bi-directional JSON-RPC protocol)
- Notifications reflect **committed** work only — rolled-back transactions don't emit
- Supports `WHERE` filtering — only matching rows trigger updates
- Under heavy concurrency: best-effort ordering, don't assume total order across all writers

### Change Feeds

For replay/history use cases (not real-time subscriptions):

```sql
-- Show changes since a specific version
SHOW CHANGES FOR TABLE person SINCE 123456;
```

- Durable, replayable change streams
- Better for batch pipelines, audit trails, catching up lagged consumers

### WebSocket Support

- Native WebSocket server built into the database (port 8000 by default)
- JSON-RPC protocol over WebSocket for queries, live subscriptions, and notifications
- No separate message broker (Kafka, Redis Pub/Sub) needed
- HTTP REST API also available for simple CRUD

### Comparison with Postgres

| Feature | SurrealDB | PostgreSQL |
|---------|-----------|------------|
| Live push notifications | ✅ Native `LIVE SELECT` | ❌ Requires LISTEN/NOTIFY + app layer |
| WebSocket protocol | ✅ Built-in | ❌ Requires external proxy (e.g., Supabase Realtime) |
| Change data capture | ✅ Native change feeds | ✅ Logical replication (more complex) |
| Diff/patch updates | ✅ `LIVE SELECT DIFF` | ❌ Manual implementation |
| Filtering subscriptions | ✅ `WHERE` in `LIVE SELECT` | ⚠️ Limited via channel filters |

**Verdict:** SurrealDB's realtime is significantly simpler to use — no separate infrastructure needed. For a Postgres platform already running Supabase/PgBouncer/LISTEN-NOTIFY, this is a meaningful simplification.

---

## 4. Vector Search

### Native Capabilities

SurrealDB has built-in vector search with **two ANN index types** (as of 3.1):

| Index | Best For | Trade-offs |
|-------|----------|------------|
| **HNSW** | In-memory or memory-resident vectors | Fastest warm-cache latency; higher memory use |
| **DiskANN** (new in 3.1) | Larger-than-memory vectors | Slower than HNSW but can scale beyond RAM; good for billion-scale |

**Features:**
- `DEFINE INDEX ... ON ... FIELDS embedding HNSW/DISKANN DIMENSION 768`
- Distance metrics: Cosine, Euclidean, Inner Product
- Element types: F16, U8, I8 (added in 3.1), plus F32
- Exact kNN via `vector::similarity::cosine(embedding, $query)`
- Approximate kNN via `<|K, EF|>` operator
- Hybrid search: combine vector similarity + full-text BM25 in one query

### Comparison with pgvector

| Aspect | SurrealDB | pgvector (PostgreSQL) |
|--------|-----------|----------------------|
| ANN index types | HNSW, DiskANN | HNSW, IVFFlat |
| Dimension limits | Not documented as limiting | ~2000 dims (F32), ~4000 (halfvec) |
| Distance metrics | Cosine, Euclidean, Inner Product | Euclidean, Cosine, Inner Product |
| Quantization | F16, U8, I8 | halfvec (16-bit) |
| Hybrid search (vector + text) | ✅ Single query | ⚠️ Requires combining queries manually |
| Graph traversal + vector | ✅ Native in one query | ❌ Not possible |
| ACID with vectors | ✅ Same transaction | ✅ Same transaction |
| Maturity | 🟡 Newer, fewer tuning options | 🟢 Battle-tested, extensive tuning |
| Scale | Single-node to distributed | Single-node only |

**Key dimensionality concern:** pgvector's 2000-dim limit (F32) is a real constraint for modern embedding models (e.g., OpenAI `text-embedding-3-large` is 3072 dims). SurrealDB does not document a hard dimensional limit, which may be advantageous for high-dim models.

**Verdict:** For pure vector search at moderate scale with existing Postgres infrastructure, pgvector is fine. For multi-model queries ("find products similar to this vector AND connected to this user's purchases via graph edges"), SurrealDB is uniquely capable.

---

## 5. Security Model

### RBAC (System Users)

SurrealDB implements RBAC at three hierarchical levels:

| Level | Scope | Roles |
|-------|-------|-------|
| **Root** | All namespaces + databases | OWNER, EDITOR, VIEWER |
| **Namespace** | All databases in namespace | OWNER, EDITOR, VIEWER |
| **Database** | Single database | OWNER, EDITOR, VIEWER |

```sql
DEFINE USER john ON ROOT PASSWORD "..." ROLES OWNER;
DEFINE USER readonly_app ON DATABASE PASSWORD "..." ROLES VIEWER;
```

Passwords hashed with **Argon2id** (default). Passhash pre-hashing supported so server never sees plaintext.

### Record Users (Scope-Based Access)

SurrealDB's unique feature: **end-user authentication directly in the database**.

```sql
DEFINE SCOPE userAccount SESSION 3d
    SIGNUP (CREATE user SET username = $username, pass = crypto::argon2::generate($pass))
    SIGNIN (SELECT * FROM user WHERE username = $username AND crypto::argon2::compare(pass, $pass));
```

- Creates `/signup` and `/signin` HTTP endpoints automatically
- Returns JWT tokens
- Scoped users have **no database access** by default — must be granted via permissions

### Row-Level / Field-Level Permissions

```sql
DEFINE TABLE article SCHEMALESS
    PERMISSIONS
      FOR select WHERE $scope = "userAccount"
      FOR create, update, delete NONE;

DEFINE FIELD secret ON article
    PERMISSIONS FOR select WHERE $auth.id = owner;
```

- **Deny-by-default:** No permissions = no access
- Variables: `$auth`, `$scope`, `$session`, `$token` for context-aware rules
- Permissions are **evaluated per-row/per-field** at query time
- Graph traversal and reference traversals respect permissions (security fix in 3.1.5)

### Capability Gating (Security Hardening)

Most capabilities are **disabled by default** and must be explicitly allowed:

- Scripting (JavaScript functions)
- Network access (`allow_net`)
- File access (`file_allowlist` — changed in 3.1.5 to deny-by-default)
- Surrealism (WASM plugins)

### Comparison with PostgreSQL

| Feature | SurrealDB | PostgreSQL |
|---------|-----------|------------|
| RBAC | ✅ Built-in, 3 levels | ✅ Roles + inheritance |
| Row-level security | ✅ Native `PERMISSIONS` | ✅ RLS policies (policies) |
| Column-level security | ✅ Field permissions | ✅ Column-level grants |
| End-user auth in DB | ✅ `DEFINE SCOPE` + JWT | ❌ Not native (requires app layer) |
| JWT generation | ✅ Built-in | ❌ Requires external service |
| Password hashing | ✅ Argon2id native | ⚠️ App responsibility |
| Capability gating | ✅ Feature-level disable | ❌ Not applicable |
| Audit logging | ✅ Enterprise only | ⚠️ pgaudit extension |

**Verdict:** SurrealDB's scope-based end-user auth is genuinely novel — it can eliminate backend auth layers for some apps. For an existing Postgres platform with established auth (e.g., Keycloak, Auth0, app-layer JWT), this may not be a differentiator unless you're building a BaaS-style API.

---

## 6. Edge Functions / WASM (Surrealism)

### Surrealism Extension Framework (v3.0+)

SurrealDB's extension system is called **Surrealism**. It allows custom functionality compiled to **WebAssembly** and loaded at runtime:

**Build pipeline:**
1. Write Rust functions with `#[surrealism]` attribute
2. Compile with `surreal module build --out module.surli`
3. Define a bucket: `DEFINE BUCKET my_bucket BACKEND "file:/path"`
4. Load module: `DEFINE MODULE mod::my_funcs AS f"my_bucket:/module.surli"`
5. Call from SurrealQL: `SELECT mod::my_funcs::predict(data) FROM table`

**Key features:**
- **WASM sandboxing:** Fresh isolated instance per call, no filesystem, no raw network
- **Hot reload:** `DEFINE MODULE OVERWRITE` — zero-downtime upgrades
- **Transaction safety:** Module callbacks run in the same ACID transaction as the invoking query
- **Async support:** Plugins can use async Rust (`reqwest`, `sqlx`) with `.await`
- **FlatBuffers serialization:** For data crossing host/guest boundary (3.1)
- **Attached filesystem:** Read-only bundled filesystem available inside sandbox (3.1)

**Use cases:**
- Custom ML inference (call local GPU runtime)
- External API calls (with `allow_net` capability)
- Domain-specific functions (financial modeling, text processing)
- Custom analyzers for full-text search

### Comparison with Postgres Extensions

| Aspect | SurrealDB Surrealism | PostgreSQL Extensions |
|--------|----------------------|----------------------|
| Language | Rust (→ WASM) | C, Rust, Python, SQL, etc. |
| Sandboxing | ✅ Strong WASM isolation | ❌ No sandbox (trusted language) |
| Hot reload | ✅ `DEFINE MODULE OVERWRITE` | ❌ Requires restart |
| Transaction safety | ✅ Same ACID context | ✅ In same transaction |
| Ecosystem | 🟡 Rust ecosystem only | 🟢 Massive ecosystem (PostGIS, pgvector, etc.) |
| Performance | 🟡 WASM overhead | 🟢 Native code |
| Security | 🟡 Experimental (requires `allow_experimental`) | 🟢 Mature |

**Verdict:** Surrealism is promising for secure, isolated extensions but is experimental. PostgreSQL's extension ecosystem is far more mature. If you need PostGIS-level geospatial sophistication or hundreds of existing extensions, Postgres wins. For safe, sandboxed custom logic without risking the database process, Surrealism has a genuine architectural advantage.

---

## 7. Performance Characteristics

### Benchmarks (SurrealDB's crud-bench vs PostgreSQL)

These are SurrealDB's own benchmarks — treat with appropriate skepticism, but they show directional trends:

**Single-Node Server: Throughput (OPS, higher is better)**

| Operation | SurrealDB (RocksDB) | PostgreSQL | Notes |
|-----------|---------------------|------------|-------|
| Create | 155,097 | 204,923 | Postgres faster on writes |
| Read | 508,757 | 283,699 | SurrealDB significantly faster |
| Update | 146,278 | 164,156 | Comparable |
| Delete | 86,515 | 198,739 | Postgres faster |
| Scan (count all) | 24.88 | 16.03 | Postgres faster |
| Scan (limit 100) | 1,545 | 3,051 | Postgres faster |
| Scan (offset+limit) | 128 | 3,884 | **Postgres much faster** |

**Latency (p99, lower is better)**

| Operation | SurrealDB (RocksDB) | PostgreSQL |
|-----------|---------------------|------------|
| Create | 79.04 ms | 57.57 ms |
| Read | 15.36 ms | 130.94 ms |
| Update | 82.30 ms | 62.59 ms |
| Delete | 226.43 ms | 47.62 ms |
| Scan (offset+limit) | ~700-800 ms | ~23-40 ms |

### Key Observations

1. **Reads are SurrealDB's strength:** Document KV-store foundation shows
2. **Writes/updates are competitive:** Within ~20-30% of Postgres
3. **Offset pagination is a weakness:** Deep pagination (`LIMIT ... START`) is significantly slower in SurrealDB. This is a known issue being addressed (3.1 added K-way merge for `IN [...] ORDER BY ... LIMIT` and predicate prefiltering)
4. **Graph traversals:** Single-scan edge traversals added in 3.1 (`SELECT ->likes->person FROM person:...`) — previously required two scans
5. **Memory optimizations in 3.1:** Small-string-optimized `Strand` type (23 bytes inline), `VecMap`/`VecSet` collections, reduced per-transaction overhead

### Startup Time & Resource Usage

- **Single binary:** Zero external dependencies — one Rust binary
- **Embedded mode:** No network stack, minimal latency overhead
- **In-memory backend:** Uses optimistic lock coupling (readers lock-free, retry on conflict) — 3.1 improvement
- **RocksDB tuning in 3.1:** Lower default readahead (256 KiB vs 4 MiB) for NVMe throughput, blob-file separation for large values, tiered memory scaling
- **No published cold-start numbers:** As an in-process/embedded database, "startup" is essentially library initialization

---

## 8. Deployment Options

### Summary Matrix

| Deployment | Storage | Scaling | HA | Best For | Managed Option |
|------------|---------|---------|-----|----------|----------------|
| **SurrealDB Cloud** | SurrealDS (distributed) | Vertical + Horizontal | ✅ Fully managed | Production without ops | Yes (Free/Start/Dedicated) |
| **Single-node** | RocksDB | Vertical | ❌ (filesystem backups) | Dev, small-medium prod | Self-hosted (free) |
| **Multi-node cluster** | SurrealDS / TiKV / FDB | Horizontal | ✅ | Large-scale, HA | Enterprise / Cloud Dedicated |
| **Embedded** | SurrealMX / SurrealKV / RocksDB | Application-bound | Application-bound | Edge, mobile, offline | No |
| **Browser** | IndexedDB | Single tab | ❌ | PWAs, local-first | No |

### Rust Embedding

```rust
let db = Surreal::new::<RocksDb>("/path/to/db").await?;
// or
let db = Surreal::new::<SurrealKv>("surrealkv://./path").await?;
// or in-memory
let db = Surreal::new::<Mem>("memory").await?;
```

- The `surrealdb` crate embeds the full query engine
- Same SurrealQL, same transactions, same permissions — just in-process
- Useful for testing, CLI tools, edge devices, or offline-first desktop apps

### Cloud Pricing

| Plan | Price | Specs |
|------|-------|-------|
| Free | $0 | 1 GB storage, 0.25 vCPU, 1 GB RAM |
| Start | ~$0.021/hour | Up to 512 GB, 16 vCPU, 64 GB RAM |
| Dedicated | Custom | Up to 1 PB cluster, multi-node |

---

## 9. AI/ML Integration

### Built-in Features

| Feature | Status | Description |
|---------|--------|-------------|
| Vector search | ✅ Core | HNSW + DiskANN indexes, similarity queries |
| Hybrid retrieval | ✅ Core | Vector + BM25 full-text in one query |
| GraphRAG | ✅ Core | Vector seed → graph traversal (`->relation->`) |
| Live memory | ✅ Core | `LIVE SELECT` for agent coordination |
| Server-side embeddings | ✅ Core | `fn::embed()` function (when configured) |
| MCP tool | ✅ Core | Model Context Protocol native integration |
| JavaScript functions | ✅ Core | ES2020 embedded functions for custom logic |
| Surrealism plugins | ✅ Experimental | WASM plugins for ML inference, external APIs |

### Spectron (Separate Product — Early Preview)

Spectron is SurrealDB's **dedicated AI agent memory layer** (separate from core database, waitlist-only as of June 2026):

- **Tri-temporal memory:** Provenance tracking, bi-temporal facts, supersession
- **Entity extraction:** Automatic entity/attribute/relation extraction from text
- **Knowledge graphs:** Native graph with entity disambiguation
- **Hybrid retrieval:** 8 fused signals (dense embeddings, BM25, graph walks, keyword bridges, section embeddings, PageRank, geospatial, trace history)
- **Tiered queries:** Sub-ms typed queries → semantic cache → hybrid retrieval → deep sweep
- **Multi-agent shared memory:** ACID coordination across agents via live queries
- **MCP-native:** Model Context Protocol server built-in

**Important:** Spectron is a separate product on top of SurrealDB, not part of the core database. It is currently in early preview with a waitlist.

### Comparison with Postgres for AI Workloads

| Capability | SurrealDB | PostgreSQL + Extensions |
|------------|-----------|-------------------------|
| Vector search | ✅ Native | ✅ pgvector |
| Full-text + vector hybrid | ✅ Single query | ⚠️ Manual combination |
| Graph relationships | ✅ Native | ❌ Requires separate graph DB |
| Agent memory / temporal | ✅ Via Spectron | ❌ Not available |
| LLM function calling | ✅ Surrealism plugins | ⚠️ pgai / plpython |
| RAG pipeline | ✅ Native multi-model | ⚠️ Multiple tools needed |

---

## 10. Comparison with PostgreSQL: When to Use Which

### Strengths of SurrealDB

| Strength | Details |
|----------|---------|
| **Multi-model in one query** | Document + graph + vector + relational + full-text in a single SurrealQL statement |
| **Realtime subscriptions** | Native `LIVE SELECT` over WebSocket — no separate infrastructure |
| **Graph traversal** | `->edge->vertex` syntax is more intuitive than recursive CTEs or JOIN chains |
| **Embedded deployment** | Can run in-process, in browser WASM, on edge devices — same API everywhere |
| **End-user auth in DB** | `DEFINE SCOPE` eliminates backend auth boilerplate for some architectures |
| **Schema flexibility** | Start schemaless, incrementally enforce schema without migrations |
| **Horizontal scaling** | Native distributed mode (SurrealDS / TiKV) — designed for it from the start |
| **Vector + graph together** | GraphRAG patterns (vector search → graph expansion) in one query |
| **Unified observability** | OpenTelemetry pipeline (metrics, traces, logs) built in (3.1) |
| **AI-native design** | Built for agent memory, RAG, and multi-modal retrieval |

### Strengths of PostgreSQL

| Strength | Details |
|----------|---------|
| **Maturity & stability** | 30+ years of production battle-testing |
| **Ecosystem** | Thousands of extensions, ORMs, tools, monitoring solutions |
| **SQL standard compliance** | True SQL — not SQL-like. Better portability, more developers know it |
| **Complex analytics** | Window functions, CTEs, LATERAL JOINs, sophisticated query planner |
| **Write performance** | Faster INSERT/UPDATE in benchmarks; more efficient for high-write OLTP |
| **Offset pagination** | Deep pagination (`OFFSET ... LIMIT`) is much faster |
| **Cost-based optimizer** | Mature, statistics-driven query planner with excellent execution plans |
| **Managed offerings** | Every cloud provider has managed Postgres (RDS, Cloud SQL, Azure, etc.) |
| **Community & support** | Massive community, extensive documentation, enterprise support options |
| **License** | True open source (PostgreSQL license) — no BSL restrictions |

### Weaknesses of SurrealDB

| Weakness | Details |
|----------|---------|
| **Maturity** | Launched 2022, 3.x released early 2026. Smaller community, fewer edge cases tested |
| **Ecosystem** | Fewer ORMs, tools, monitoring integrations. Growing but not comparable |
| **Query language** | SurrealQL is SQL-like but not SQL. Learning curve for existing SQL devs |
| **Offset pagination** | Performance degrades significantly with deep offsets |
| **Graph deep traversals** | May not match Neo4j on very deep/complex graph algorithms |
| **Licensing** | BSL 1.1 — not OSI-approved open source. 4-year rolling conversion to Apache 2.0 |
| **Managed hosting** | Fewer options than Postgres. SurrealDB Cloud is the primary managed option |
| **Enterprise features** | Audit logging, slow query telemetry only in Enterprise Edition |
| **Surrealism maturity** | WASM plugin system is experimental (requires `allow_experimental`) |

### Weaknesses of PostgreSQL for Modern Workloads

| Weakness | Details |
|----------|---------|
| **Multi-model friction** | JSON, graph (Apache AGE), vector (pgvector) are extensions, not native. Stitching required |
| **Realtime complexity** | LISTEN/NOTIFY is primitive; real-time APIs require Supabase/Powerbase/etc. |
| **Graph queries** | Recursive CTEs are verbose and often slower than native graph traversal |
| **Schema migrations** | Adding columns to large tables is disruptive; schema evolution requires care |
| **Horizontal scaling** | Read replicas only; write scaling requires manual sharding (Citus, etc.) |
| **Vector dimension limits** | pgvector ~2000 dims (F32), ~4000 (halfvec) — limits modern embedding models |
| **AI-native features** | No built-in agent memory, graph RAG, or temporal fact tracking |

### Decision Matrix: When to Choose Which

| Scenario | Recommendation |
|----------|----------------|
| **Existing large Postgres investment** | Stick with Postgres. Add SurrealDB only for specific multi-model workloads |
| **Need graph + vector + relational in one query** | SurrealDB is uniquely suited |
| **Real-time app (chat, live dashboard, multiplayer)** | SurrealDB's `LIVE SELECT` eliminates infrastructure |
| **Embedded/edge/browser database** | SurrealDB — Postgres cannot run in browser or truly embedded |
| **AI agent system with memory** | SurrealDB + Spectron (when available) is purpose-built |
| **High-throughput OLTP (millions of writes/sec)** | Postgres — SurrealDB writes are competitive but not superior |
| **Complex analytical queries (window functions, CTEs)** | Postgres — SurrealQL analytical capabilities are less mature |
| **Regulated enterprise requiring proven track record** | Postgres — 30+ years of audit history |
| **Multi-tenant SaaS with row-level security** | Both capable; Postgres RLS is more mature, SurrealDB scopes are more flexible |
| **Need to avoid BSL/licensing concerns** | Postgres — true open source |
| **Team with deep SQL expertise, no time to learn** | Postgres — SurrealQL has a learning curve |

---

## Recommendations for a Postgres-Based Platform

### Conservative Approach: Add SurrealDB as a Specialty Backend

If your platform is already on Postgres, the safest path is to add SurrealDB as a **second backend for specific workloads** rather than replacing Postgres:

1. **Graph-heavy features:** Social networks, knowledge graphs, recommendation engines
2. **Real-time features:** Live dashboards, collaborative editing, chat
3. **AI/vector features:** Semantic search, RAG pipelines, agent memory (when Spectron matures)
4. **Edge/offline features:** Browser-based tools, mobile apps, IoT

### Implementation Pattern: Dual Backend

```
┌─────────────────┐     ┌─────────────────┐
│   Application   │────▶│   PostgreSQL    │ (primary: relational data, transactions)
│     Layer       │     │   (pgvector)    │ (vectors if needed)
└─────────────────┘     └─────────────────┘
         │
         └────────────────▶┌─────────────────┐
                           │   SurrealDB     │ (specialty: graph, real-time, edge)
                           │   (embedded or  │
                           │    standalone)   │
                           └─────────────────┘
```

### Risk Mitigation

1. **Abstract data access:** Build a DAL (Data Access Layer) that abstracts database operations so the underlying store can be swapped
2. **Integration tests:** Extensive integration tests for SurrealDB queries — query planner edge cases exist
3. **Migration plan:** Maintain ability to fall back to Postgres + pgvector + Neo4j if needed
4. **Monitor 3.1.5+ patches:** SurrealDB is iterating rapidly; stay on latest patch for security fixes
5. **License audit:** Ensure BSL 1.1 terms are acceptable (free for non-DBaaS use; converts to Apache 2.0 after 4 years)

### When to Re-evaluate in 12-18 Months

- SurrealDB 3.2+ may address offset pagination performance
- Spectron general availability may make agent memory a compelling differentiator
- Expanded managed hosting options may reduce operational concerns
- Community ecosystem growth (ORMs, tools, monitoring) may close the gap with Postgres

---

## Sources & References

- SurrealDB 3.1 Release Notes: https://surrealdb.com/releases/3.1
- SurrealDB 3.1 Blog: https://surrealdb.com/blog/surrealdb-3-1-stability-diskann-and-a-new-release-process
- SurrealDB vs Postgres Comparison: https://surrealdb.com/comparison/postgres
- SurrealDB Performance Benchmarks: https://surrealdb.com/blog/beginning-our-benchmarking-journey
- SurrealDB Documentation: https://surrealdb.com/docs/
- SurrealDB Licensing: https://surrealdb.com/license
- SurrealDB Pricing: https://surrealdb.com/pricing/spectron
- SurrealDB Security Docs: https://surrealdb.com/docs/learn/security/authentication/summary
- SurrealDB Extensions: https://surrealdb.com/docs/learn/extensions
- SurrealDB Deployment: https://surrealdb.com/docs/build/deployment
- Spectron Platform: https://surrealdb.com/platform/spectron
- SurrealDB GitHub (releases): https://github.com/surrealdb/surrealdb/releases

---

*Report generated by research specialist agent. Data sourced from official SurrealDB documentation, release notes, and published benchmarks as of 2026-06-29.*
