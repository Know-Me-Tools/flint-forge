# Supabase Complete Architecture & Feature Research
## Research Date: 2025-06-29

> **Research Scope:** This document covers Supabase's architecture and feature set as of 2024-2025, based on official documentation, community sources, and technical analysis.

---

## 1. Supabase Auth (GoTrue)

### What Supabase Offers
- **JWT-based authentication API** built on GoTrue (Go language, MIT license)
- Full user management: sign-up, sign-in, sign-out, password reset, email confirmations
- Multiple auth methods: email/password, magic links, one-time passwords (OTP), social login, SSO (single sign-on), phone auth
- Multi-factor authentication (MFA) via TOTP
- Session management with automatic refresh via proxy layer
- Server-side rendering (SSR) support with `createServerClient` / `createBrowserClient` factories

### Architecture & Technology Choices
- **GoTrue** is a Go-based JWT API server that stores user data in the `auth.users` table within the Postgres database
- Deep Postgres integration: auth data lives in the same database as application data, enabling foreign key relationships
- Integrates seamlessly with Row Level Security (RLS) policies for authorization
- Password hashing uses **bcrypt**
- Supports both symmetric and asymmetric JWT signing keys (asymmetric enables local validation without network requests)
- Supports `proxy.ts` pattern for Next.js and modern framework integration

### Supported Providers (2024-2025)
- Apple, Azure, Bitbucket, Discord, Facebook, Figma, GitHub, GitLab, Google, Kakao, Keycloak, LinkedIn, Notion, Slack, Spotify, Twitch, Twitter/X, WorkOS, Zoom, Fly
- Phone auth via Twilio
- SSO for enterprise

### Known Limitations / Gaps
- **No native passkey support** (as of 2025)
- **Auth UI library was archived** in October 2025 (maintenance mode since February 2024); replacement uses shadcn-based blocks
- No built-in RBAC/ABAC — roles and permissions must be implemented manually via RLS or custom tables
- No native organization/team-level auth concepts
- C# library for GoTrue is under development
- Tokens lack access scopes, making integration with secure microservices challenging

### Resource Overhead & Performance
- Supports **50,000 MAU (Monthly Active Users)** on the free tier
- Pro plan includes **2 million auth users**
- Minimal latency overhead — JWT validation is fast; asymmetric keys enable local validation
- Typical implementation time: 30 minutes to 2 hours for basic email/password auth

---

## 2. Supabase Database (Postgres)

### What Supabase Offers
- **Full PostgreSQL database** — not an abstraction, not a fork. You get complete SQL access.
- Pre-installed with **50+ extensions** including:
  - `pgvector` — vector embeddings for AI
  - `PostGIS` — geospatial data
  - `pg_cron` — scheduled jobs
  - `pg_net` — asynchronous HTTP/HTTPS requests (webhooks)
  - `pg_graphql` — GraphQL support inside Postgres
  - `pg_stat_statements` — query performance monitoring
  - `vault` — secrets management
- Full ACID compliance, complex joins, CTEs, stored procedures, triggers, views, materialized views, partial indexes
- JSONB support for document-like data
- Full-text search via `tsvector`
- Database branching (added in late 2024) for testing schema changes in isolation
- Connection pooling via **Supavisor** (replaced PgBouncer in 2024-2025)
- Read replicas for query load distribution (physical replicas, GA)
- Daily backups; point-in-time recovery (PITR) on paid plans
- Database webhooks — send row changes to external HTTP endpoints

### Architecture & Technology Choices
- Core philosophy: everything is built around Postgres
- Supavisor is an **Elixir-based**, cloud-native, multi-tenant connection pooler
- Uses PostgreSQL's native logical replication for Realtime features
- Schema migrations via CLI (`supabase db diff`)
- Type-safe client generation from database schema

### Known Limitations / Gaps
- Free tier limited to **500MB database storage**; projects auto-pause after 7 days of inactivity
- Backups do NOT include Storage objects (only database metadata)
- Advanced Postgres features require raw SQL knowledge
- No built-in analytics dashboard — requires external tools
- No native queue system (requires custom implementation with `pg_cron` or external service)
- Backup retention on Team plan is only 14 days (not configurable)
- Branching is a paid/managed feature

### Resource Overhead & Performance
- Pro plan: **8GB database space** at $25/month
- Query performance: typically **20-50ms** for indexed queries
- Connection pooling handles high concurrency efficiently
- Read replicas available for scaling read-heavy workloads
- Poorly optimized queries will slow down regardless of platform
- For high-write tables (1,000+ inserts/sec), Realtime CDC can spike CPU

---

## 3. Supabase Realtime

### What Supabase Offers
- Three distinct real-time modes:
  1. **Postgres Changes** — listen to database INSERT, UPDATE, DELETE events via logical replication
  2. **Broadcast** — send ephemeral, low-latency messages between clients (cursors, typing indicators, game state)
  3. **Presence** — track and synchronize shared state (online users, room occupancy) using CRDTs
- Authorization for Broadcast and Presence via RLS policies (added August 2024)
- **Broadcast from Database** (April 2025) — trigger broadcasts from database changes with custom SQL logic

### Architecture & Technology Choices
- Built on **Elixir/Phoenix** — the same technology stack powering Discord and WhatsApp
- Uses **PostgreSQL logical replication** (WAL — Write-Ahead Log) for Postgres Changes
- WebSocket-based communication
- For Postgres Changes: polls database via logical replication, converts changes to JSON, broadcasts to authorized clients
- For Broadcast/Presence: server-mediated direct messaging, no database impact
- Authorization caching: RLS policies checked on channel subscription, cached in memory for the connection duration
- `realtime.topic()` function for policy-based channel access control

### Known Limitations / Gaps
- **No message delivery guarantee** — the server does not guarantee every message will be delivered
- **Postgres Changes latency: 50-200ms** (WAL processing overhead)
- Subscribing to high-write tables (1,000+ inserts/sec) can overload Realtime and spike database CPU
- **Stale Presence bug** after tab visibility changes — requires re-tracking on visibilitychange
- **Leaked channels** are the #1 cause of hitting connection limits — must call `supabase.removeChannel(channel)` on unmount
- Pro plan handles up to **500 concurrent connections**; above that requires Team plan or self-hosting
- Broadcast and Presence authorization is Public Beta (as of late 2024)
- Postgres Changes already respects RLS, but Broadcast/Presence authorization is newer

### Resource Overhead & Performance
- Broadcast latency: **under 50ms** (direct server-mediated)
- Presence latency: **under 100ms**
- Postgres Changes latency: **50-200ms**
- Elixir/Phoenix can handle millions of concurrent connections on commodity hardware
- Supabase managed layer adds connection/message limits that must be planned for
- Use Postgres Changes for persistent data; Broadcast for ephemeral high-frequency data (>10 events/sec per user)

---

## 4. Supabase Edge Functions

### What Supabase Offers
- Serverless functions running on **Deno runtime** (JavaScript/TypeScript)
- Deployed globally, close to users
- Background tasks, ephemeral storage, and WebSocket support (added December 2024)
- Can import npm packages from private registries (October 2024)
- Static files support (January 2025)
- Deno 2.1 compatible across all regions (August 2025)
- Can be deployed from Dashboard or CLI
- Self-hostable via **Supabase Edge Runtime** (open-sourced March 2022)

### Architecture & Technology Choices
- **Deno runtime** — modern, secure V8-based runtime with TypeScript support
- Uses V8 isolates (not containers) for fast cold starts
- Supports JSR modules (May 2024)
- `deno.json` configuration supported (November 2024)
- Can write in pure JavaScript (November 2024)
- Rate limits on recursive/nested Edge Function calls (March 2026)

### Known Limitations / Gaps
- **Cold start times: 200-400ms** (requires warming strategies for latency-sensitive endpoints)
- Not as fast as Cloudflare Workers (5ms) or Fastly Compute (35μs–1ms)
- Free tier: **500,000 Edge Function invocations/month**
- Deno runtime means some Node.js-specific packages may not work (compatibility is good but not 100%)
- For high-frequency inference, dedicated GPU infrastructure may be needed
- Recursive/nested calls have rate limits to prevent runaway execution
- Edge Functions have a size limit and memory limit typical of serverless functions

### Resource Overhead & Performance
- Cold start: **200-400ms** (2x smaller, 3x faster boot after September 2024 optimization)
- Warm invocation latency: typically under 100ms
- Good for: webhooks, third-party API calls, light preprocessing, lightweight inference
- Not ideal for: heavy compute, long-running processes, GPU workloads
- Mozilla Llamafile can run inside Edge Functions for local LLM inference (August 2024)

---

## 5. Supabase Storage

### What Supabase Offers
- S3-compatible object storage service
- Three specialized bucket types (as of 2025):
  - **File buckets** — traditional storage
  - **Analytics buckets** — Iceberg format for data lakes
  - **Vector buckets** — AI embeddings with similarity search (Public Alpha)
- Three interoperable protocols:
  - Standard uploads (multipart/form-data, up to ~6MB)
  - Resumable uploads (TUS protocol, GA since April 2024)
  - S3 protocol (Public Alpha since April 2024)
- Global CDN serving files from **285+ cities**
- Smart CDN — automatic cache revalidation within 60 seconds
- Image optimization — resize, compress, WebP conversion on-the-fly
- Supports files up to **500GB** on paid plans
- Cross-bucket transfers (copy/move across buckets)
- Presigned URLs for temporary access
- Metadata stored in Postgres tables (can be joined with application data)

### Architecture & Technology Choices
- **Node.js/TypeScript** service (Apache 2.0 license)
- Stores metadata in regular PostgreSQL tables
- Uses PostgreSQL Large Object functionality for storage engine
- S3-compatible API — can use AWS CLI, rclone, Cyberduck, DuckDB, etc.
- Integrates with Supabase Auth for RLS-based access control
- AWS Signature Version 4 for S3 authentication

### Known Limitations / Gaps
- S3 protocol is **Public Alpha** (as of 2024-2025)
- Not all S3 features implemented: no bucket encryption, lifecycle configuration, CORS management, object locking, SSE-C
- Database backups do NOT include Storage files (only metadata)
- Free tier: **1GB file storage**
- Storage-specific hostname required for 500GB uploads on paid plans
- Standard upload limit of ~6MB for simple multipart uploads

### Resource Overhead & Performance
- Global CDN reduces latency for file serving
- Image transformations are computed on-the-fly (CPU overhead)
- Smart CDN cache invalidation: ~60 seconds globally
- TUS resumable uploads handle unreliable networks well
- RLS policy checks on storage access add minimal overhead

---

## 6. Supabase AI/ML Support

### What Supabase Offers
- **pgvector** extension — store and query high-dimensional vector embeddings natively in PostgreSQL
- HNSW (Hierarchical Navigable Small World) and IVFFlat indexing for efficient similarity search
- Vector search with cosine distance, L2 distance, inner product
- **Hybrid search** — combine vector similarity with full-text search (tsvector)
- **Metadata filtering** — narrow vector search scope with WHERE clauses before similarity comparison
- Edge Functions can run lightweight inference (classification, sentiment, entity extraction)
- **Vector buckets** in Storage (Public Alpha) — store AI embeddings with similarity search
- Integration with LangChain, AutoGen, and other AI frameworks
- RAG (Retrieval-Augmented Generation) pipeline support
- Caching strategies for embeddings and AI responses

### Architecture & Technology Choices
- `pgvector` is a native PostgreSQL extension (C language)
- Embeddings stored alongside relational data in the same table
- HNSW indexes provide fast approximate nearest neighbor search
- SQL-based vector queries with `<=>` (cosine distance), `<->` (L2 distance), `<#>` (inner product)
- RPC functions can encapsulate common vector search patterns

### Known Limitations / Gaps
- **Moderate vector scale**: recommended for under 1-5 million vectors
- For massive vector scale (millions+), dedicated vector databases (Pinecone, Weaviate, Qdrant) may outperform pgvector
- No built-in GPU inference — heavy inference requires external services (OpenAI, Cohere, Replicate) or Edge Functions with Llamafile
- No built-in embedding generation — must call external APIs or use Edge Functions
- Edge Functions have limited compute/memory for local inference
- Vector buckets are still in Public Alpha

### Resource Overhead & Performance
- Sub-50ms query times for indexed vector searches (tested with 100K+ documents)
- HNSW indexes are memory-intensive — balance between recall and memory consumption
- IVFFlat uses less memory but has slower build times and lower recall
- Filter selectivity matters: very selective WHERE clauses reduce vector search space dramatically
- Good for: RAG, semantic search, recommendation engines, AI application backends

---

## 7. Supabase Deployment Options

### Managed Cloud (Supabase Cloud)
- **Free tier**: 500MB DB, 1GB storage, 2GB bandwidth, 50K MAU, 500K edge function invocations. Projects auto-pause after 7 days of inactivity.
- **Pro**: $25/month — 8GB DB, 100GB bandwidth, 2M auth users, unlimited API requests, priority support
- **Team**: $599/month — SOC2, daily backups, SSO, 14-day backup retention
- **Enterprise**: Custom pricing
- Runs on AWS infrastructure
- EU region hosting available for data residency
- AWS Marketplace availability
- Point-in-time recovery (PITR) for databases >4GB on paid plans

### Self-Hosted (Docker)
- **Official Docker Compose** setup — the recommended path
- Minimum requirements: **4GB RAM, 2 CPU cores, 40GB SSD** (recommended: 8GB RAM, 4+ cores, 80GB+ SSD)
- Linux server/VPS, macOS, Windows with Docker Desktop
- One-command setup script for Linux: `curl -fsSL https://supabase.link/setup.sh | sh`
- Full stack: Postgres, Kong, GoTrue, PostgREST, Realtime, Storage, Studio, Edge Runtime
- Optional: Logflare (analytics), Vector (log collection) — adds resource overhead
- HTTPS via reverse proxy (Caddy or Nginx) recommended for production
- No telemetry collected in self-hosted Docker setup

### Self-Hosted (Kubernetes / Advanced)
- Community-supported Kubernetes deployments
- Pigsty provides a one-click self-hosting solution with full PostgreSQL monitoring, IaC, PITR, high availability
- For true HA: minimum 3 nodes for ETCD, 3 nodes for PostgreSQL synchronous commit, multi-node MinIO cluster
- Stateless Supabase containers can scale out; stateful parts (Postgres, MinIO) need HA configuration

### Self-Hosted vs Managed — Feature Gaps
Platform-only features **unavailable** in self-hosted:
- Database branching
- Advanced metrics beyond logs
- Managed backups and PITR (you manage your own)
- Analytics and vector buckets
- ETL tools
- Platform management API
- Studio does not support multiple organizations or projects (single project only)

### Resource Requirements Summary
| Component | Minimum | Recommended |
|-----------|---------|-------------|
| RAM | 4 GB | 8 GB+ |
| CPU | 2 cores | 4 cores+ |
| Disk | 40 GB SSD | 80 GB+ SSD |
| Network | Static IPv4 | Static IPv4 + DNS |

---

## 8. Supabase GraphQL Support

### What Supabase Offers
- GraphQL API via the **`pg_graphql`** PostgreSQL extension
- Auto-reflection of SQL schema into GraphQL schema — no separate schema definition needed
- Supports CRUD operations, deep relationships, views, materialized views, foreign tables, computed columns
- Exposed via `graphql.resolve(...)` SQL function
- Interoperates with PostgREST — can call via RPC over HTTP/S
- Supports Row Level Security, roles, and grants
- Any language that can connect to PostgreSQL can use the GraphQL API

### Architecture & Technology Choices
- `pg_graphql` is a PostgreSQL extension written in Rust (by Supabase)
- Resolver runs **inside the database** — no additional GraphQL server needed, reducing network hops
- Schema automatically updates when SQL schema changes
- Uses PostgREST's RPC mechanism to expose `graphql.resolve()` over HTTP

### Known Limitations / Gaps
- GraphQL support is less mature than REST/PostgREST
- No native GraphQL subscriptions for real-time (use Realtime WebSockets or Postgres Changes instead)
- Schema reflection is automatic but may not perfectly map complex Postgres features
- No built-in GraphQL federation or schema stitching
- Error handling follows PostgreSQL patterns, not standard GraphQL error format
- Performance depends on query complexity and underlying SQL query optimization
- Some developers report that complex nested queries can strain CPU
- File uploads via GraphQL require workarounds (GraphQL spec doesn't natively support multipart uploads)

### Resource Overhead & Performance
- Queries resolve to a single SQL statement — fast for simple queries
- Complex deep queries can be CPU-intensive (same as any GraphQL resolver)
- Indexing is critical for performance
- Load testing recommended before production deployment
- Consider using REST for simple CRUD and GraphQL for complex relationship queries

---

## 9. Supabase REST API (PostgREST)

### What Supabase Offers
- Auto-generated RESTful API from your PostgreSQL schema
- Full CRUD operations via HTTP
- Complex filters, joins, aggregations via URL parameters
- Count functionality (exact, planned, estimated)
- Supports PostgreSQL functions as RPC endpoints
- Automatic OpenAPI/Swagger documentation generation
- Deep integration with RLS — API respects database security policies automatically
- Works with any HTTP client
- Supports bulk inserts, upserts, and returning data after insert/update
- Pagination via `limit`/`offset` and cursor-based approaches

### Architecture & Technology Choices
- **PostgREST** is a standalone Haskell web server
- Directly converts PostgreSQL schema to REST API — no code generation step needed
- Uses PostgreSQL's native role-based access control and RLS
- Supports embedding related tables (foreign key relationships) in a single request
- Transactional — single HTTP request maps to a single SQL transaction

### Known Limitations / Gaps
- URL-based query syntax can become verbose for complex queries
- No built-in request/response transformation (must use database views or functions)
- File uploads not natively supported (use Storage API instead)
- Rate limiting is at the platform level, not per-endpoint in self-hosted
- No built-in request caching (requires external CDN or Redis)
- Some advanced PostgreSQL features require RPC calls rather than direct REST endpoints
- Query complexity limits enforced to prevent resource exhaustion

### Resource Overhead & Performance
- Typical REST API latency: **20-50ms** for indexed queries
- PostgREST is highly optimized — can handle thousands of requests per second
- Connection pooling via Supavisor manages database connections efficiently
- No code generation overhead — schema changes reflect immediately
- Embedded resource fetching reduces N+1 query problems

---

## 10. Supabase Security Model

### What Supabase Offers
- **Row Level Security (RLS)** — the cornerstone of Supabase security. Policies are SQL rules evaluated at the database level for every query.
- **JWT-based authentication** with configurable expiration
- **Vault** (`supabase_vault` extension) — encrypted secrets storage within Postgres
- **SSL enforcement** for data in transit
- **Network restrictions** — limit database access to trusted networks/IP ranges
- **RLS policies for Storage** — same SQL-based access control for files
- **RLS policies for Realtime** — Broadcast and Presence authorization via `realtime.messages` table policies
- **Audit trail** exposed in dashboard and GoTrue logs
- **SOC 2 Type II** compliance on Team/Enterprise plans
- **EU region hosting** for GDPR/data residency
- **Database encryption at rest** (managed cloud)
- **Custom SMTP** for auth emails

### Architecture & Technology Choices
- RLS policies are enforced **inside PostgreSQL** — not in application middleware. This means even if the API key is compromised, data access is still restricted by policies.
- RLS uses `USING` (for SELECT/UPDATE/DELETE) and `WITH CHECK` (for INSERT/UPDATE) clauses
- `auth.uid()` and `auth.role()` helper functions for policy definitions
- Vault encrypts secrets using pgsodium (libsodium bindings for Postgres)
- GoTrue uses bcrypt for password hashing
- Supabase uses Kong API gateway for rate limiting and request routing

### Known Limitations / Gaps
- **RLS is powerful but has a steep learning curve** — many developers find it the hardest part of Supabase
- **No built-in RBAC** — you must manually implement roles via RLS policies or custom tables
- No native organization/team-level access control in Auth
- No access scopes in JWT tokens
- RLS policies can be bypassed if developers accidentally disable them or use `SECURITY DEFINER` functions incorrectly
- RLS performance impact: complex policies can slow down queries significantly
- Realtime Broadcast/Presence authorization is Public Beta (as of late 2024)
- Self-hosted: you are responsible for security hardening, OS updates, and secret rotation

### Resource Overhead & Performance
- RLS policy evaluation adds ~1-10ms per query depending on complexity
- Simple policies (e.g., `auth.uid() = user_id`) have negligible overhead
- Complex policies with subqueries or function calls can add significant overhead
- Realtime authorization checks RLS on channel subscription (cached for the connection)
- Policy caching in Realtime minimizes latency after initial connection

---

## Comparative Summary Table

| Feature | Supabase | Firebase | Neon | Notes |
|---------|----------|----------|------|-------|
| Database | PostgreSQL (full) | Firestore (NoSQL) | Serverless Postgres | Supabase = full SQL |
| Auth | GoTrue (JWT, built-in) | Firebase Auth | None (bring your own) | Supabase integrates with RLS |
| Realtime | WebSocket (Elixir) | Firestore listeners | None | Supabase supports 10K+ connections |
| Edge Functions | Deno runtime | Cloud Functions | None | Supabase cold start 200-400ms |
| Storage | S3-compatible | Cloud Storage | None | Supabase has RLS + CDN |
| Vector/AI | pgvector (built-in) | Vector search (2024+) | pgvector (supported) | Supabase = integrated backend |
| GraphQL | pg_graphql (in-DB) | No native | No native | Supabase auto-reflects schema |
| REST API | PostgREST (auto-gen) | No native | No native | Supabase schema-driven |
| Self-hostable | Yes (Docker) | No | No | Supabase = open source |
| Pricing | Predictable | Usage-based | Usage-based | Supabase = $25/month Pro |

---

## Sources & References

1. [Supabase Official Architecture Docs](https://supabase.com/docs/guides/getting-started/architecture) — Official architecture overview
2. [Supabase Auth Docs](https://supabase.com/docs/guides/auth) — Authentication features and providers
3. [Supabase Database Overview](https://supabase.com/docs/guides/database/overview) — Database features and extensions
4. [Supabase Realtime Authorization Blog](https://supabase.com/blog/supabase-realtime-broadcast-and-presence-authorization) — Broadcast/Presence RLS (Aug 2024)
5. [Supabase Realtime GitHub](https://github.com/supabase/realtime) — Realtime server source and status
6. [Supabase Realtime in Production Guide](https://www.agilesoftlabs.com/blog/2026/05/supabase-realtime-in-production-what) — Production scaling insights (2026)
7. [Supabase Storage S3 Compatibility](https://supabase.com/blog/s3-compatible-storage) — S3 protocol support (Apr 2024)
8. [Supabase Storage Features](https://supabase.com/features/file-storage) — Storage capabilities and limits
9. [Supabase Edge Functions Changelog](https://supabase.com/changelog?tags=edge%20functions) — Edge Functions updates
10. [Supabase Self-Hosting Docker Docs](https://supabase.com/docs/guides/self-hosting/docker) — Docker deployment guide
11. [Supabase Self-Hosting Overview](https://supabase.com/docs/guides/self-hosting) — Self-hosting vs managed differences
12. [Supabase GraphQL Docs](https://www.restack.io/docs/supabase-knowledge-supabase-graphql-docs) — pg_graphql implementation
13. [Supabase AI/Vector Guide](https://zenvanriel.com/ai-engineer-blog/supabase-for-ai-applications/) — AI application patterns
14. [Supabase Vector Storage Deep Dive](https://sparkco.ai/blog/mastering-supabase-vector-storage-a-2025-deep-dive) — pgvector best practices (2025)
15. [Supabase vs Firebase 2026](https://cadence.withremote.ai/blog/supabase-vs-firebase) — Architectural comparison
16. [Supabase Review 2026](https://hackceleration.com/labs/review/supabase) — Feature and performance review
17. [Supabase Auth Build vs Buy](https://supabase.com/blog/supabase-auth-build-vs-buy) — Auth architecture and cost analysis
18. [Supabase vs Neon vs PlanetScale 2026](https://apiscout.dev/guides/supabase-vs-neon-vs-planetscale-serverless-db-2026) — Database comparison
19. [Clerk Auth Comparison](https://clerk.com/articles/the-best-apis-for-secure-user-authentication) — Supabase Auth limitations (no passkeys, archived UI)
20. [Supabase on AI Limitations](https://blog.logto.io/zh-TW/supabase-ai-limitation) — RBAC gaps and multi-tenant limitations

---

*Document compiled for research purposes. Data current as of June 2025 based on 2024-2025 sources.*
