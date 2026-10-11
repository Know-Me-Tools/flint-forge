---
name: forge-api-compat
description: "Implement or verify Flint Quarry REST/RPC and hybrid GraphQL compatibility with PostgREST and pg_graphql."
license: MIT
metadata:
  author: prometheus
  version: "1.0.0"
---

# forge-api-compat

Start a versioned compatibility matrix, not a claim of universal Supabase parity. Pin the chosen PostgREST baseline and actual deployed pg_graphql version for each acceptance run. Research used PostgREST 14 docs; this does not silently select Forge's production compatibility version.

For REST/RPC, test filters and null semantics, ordering, range/count, profile headers, returning and preference headers, upsert conflicts, type casts, error status/body, RPC overload/argument resolution, foreign-key embedding ambiguity and schema-cache invalidation. Compatibility is observable behavior; Forge need not reproduce PostgREST's internal implementation. Unsupported behavior needs an explicit response and documented deviation.

Follow the hybrid split: query/mutation delegates to graphql.resolve() through fdb-postgres under the same RLS transaction; async-graphql handles subscription surfaces. Inspect PgGraphQl::execute and commit behavior: a successful payload without a committed mutation is a bug. Keep operationName, variables, null/error propagation and client-visible shapes compatible.

Introspection and schema visibility are role-sensitive. pg_graphql 1.6+ documentation changes introspection defaults; check the exact deployed version and comments. Never globally enable introspection or weaken grants to make a test work. Use per-role schema snapshots (anon, authenticated tenants, privileged role), operation validation and breaking-change diffing. GraphQL Inspector is an optional tool, not a new runtime dependency; verify installed CLI flags before using community skill examples.

Read crates/fdb-app, fdb-query and fdb-gateway. Database adapter changes go to the PostgreSQL owner; schema reflection changes to the metadata owner. Identity changes go to security. Test real clients and PostgreSQL at the phase boundary, including two tenants and denied mutations.

Sources: https://docs.postgrest.org/en/v14/references/api.html ; https://docs.postgrest.org/en/v14/references/api/resource_embedding.html ; https://docs.postgrest.org/en/v14/references/errors.html ; https://supabase.com/docs/guides/graphql/security ; https://supabase.com/docs/guides/database/extensions/pg_graphql ; https://github.com/graphql-hive/graphql-inspector .
