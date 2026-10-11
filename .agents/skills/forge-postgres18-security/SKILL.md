---
name: forge-postgres18-security
description: "Design or review PostgreSQL 18 SQL, migrations, database performance, RLS, pooled identity and authorization in Flint Forge."
license: MIT
metadata:
  author: prometheus
  version: "1.0.0"
---

# forge-postgres18-security

Treat PostgreSQL 18 as the target and check actual image, pgrx and extension pins before changing them. Include all five Anvil crates even though default cargo checks exclude them. Do not silently upgrade the PG17-targeted auth extension or other pins based on old documentation.

Use the local supabase-postgres-best-practices skill for query/index/pool guidance, then verify correctness with PostgreSQL 18 documentation and actual EXPLAIN evidence. Its examples are suggestions, not Forge contracts: session-level SET must not replace transaction-local context; a row-dependent correlated subquery is not guaranteed to run once; SECURITY DEFINER does not universally bypass RLS; performance multipliers are not measured guarantees.

Trace crates/fdb-postgres/src/backend.rs and conn.rs. Start a transaction before SET LOCAL ROLE and bound set_config(..., true); derive an allowlisted role and tenant from verified identity. Reset/rollback pooled connections on success, errors and cancellation. Never interpolate untrusted role/identifier text or log claims/tokens/tenant IDs. Test a reused physical connection across two tenants after commit, rollback and failed requests.

RLS requires both grants and policies. Cover SELECT/INSERT/UPDATE/DELETE, USING and WITH CHECK, role membership, table ownership, FORCE RLS, BYPASSRLS and the explicit service-role exception. FORCE does not defeat superuser/BYPASSRLS. Security-invoker views, SECURITY DEFINER functions, function EXECUTE grants and a safe search_path need separate review. Treat TRUNCATE, DDL, foreign-key/uniqueness side channels and privileged metadata as separate capabilities. Never claim Cedar replaces PostgreSQL RLS.

Gate issues/validates ecosystem identity; Forge verifies the token for its own audience and maps it to a local RlsContext. Keto relationship admission, PostgreSQL row filtering and Cedar action checks remain independent. Re-evaluate authorization when delivering realtime content and performing effects, not just at subscription creation.

Use additive migration planning with drift guards and transactional audit receipts. Verify failure rollback, lock duration, restartRequired and recovery. Privileged provisioning is not the ordinary CRUD path. Database SQL lives with the PostgreSQL owner; policy changes require the security specialist's review.

Sources: https://www.postgresql.org/docs/18/ddl-rowsecurity.html ; https://www.postgresql.org/docs/18/sql-createpolicy.html ; https://www.postgresql.org/docs/18/sql-createfunction.html ; https://supabase.com/docs/guides/database/postgres/row-level-security .
