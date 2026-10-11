---
name: forge-metadata-contracts
description: "Design or review Flint Forge reflection and schema administration against postgres-meta behavior and PostgreSQL catalog semantics."
license: MIT
metadata:
  author: prometheus
  version: "1.0.0"
---

# forge-metadata-contracts

Separate privileged database administration from application data APIs. postgres-meta lists/manages schemas, tables, columns, constraints, indexes, functions, roles, policies and extensions; it is a reference surface, not proof that Forge has each endpoint or permission. Build a per-operation present/partial/unsupported matrix with evidence.

Read crates/ext-flint-meta/sql and src, crates/fdb-reflection, and Quarry provisioning consumers. The Rust call graph does not expand extension_sql_file! into SQL semantics. Inspect catalog SQL directly for composite-key order, quoted identifiers, dropped columns, domains/enums, generated/identity columns, arrays, partitioning, overloads and extension-owned objects.

Preserve one source model with explicit compiler outputs for REST/OpenAPI, GraphQL subscriptions, MCP, AG-UI and A2UI. Version the schema and invalidate dependent snapshots after authorized DDL. Do not invent hot reload if the accepted endpoint reports restartRequired. Avoid leaking schemas or policy definitions through unprivileged reflection.

Changes to reflection data types need coordinated ports/domain and client owners; metadata cannot claim ownership of API route files. Provisioning must detect live drift, use bounded authorized namespaces, distinguish plan from apply, and keep DDL and its audit ledger atomic. Do not execute arbitrary SQL from agent descriptions.

Sources: https://github.com/supabase/postgres-meta ; https://www.postgresql.org/docs/18/catalogs.html ; https://www.postgresql.org/docs/18/information-schema.html .
