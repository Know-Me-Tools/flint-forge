---
name: forge-client-contracts
description: "Maintain Flint Forge CLI and React/Flutter client contracts for generated interfaces, reconnect behavior and authorization errors."
license: MIT
metadata:
  author: prometheus
  version: "1.0.0"
---

# forge-client-contracts

Read packages/flint-react, packages/flint_genui and crates/forge-cli. Consumer contracts include REST/GraphQL, MCP/AG-UI/A2UI reflection outputs, version negotiation, cancellation, pagination, typed errors and auth refresh. Do not assume a Supabase client is a drop-in replacement until its exact requests pass the compatibility matrix.

Keep server-authoritative data separate from local optimistic state. For KnowMe/offline consumers represent pending, accepted, committed and projected distinctly; reconcile replay/idempotency outcomes. For The Boss control surfaces distinguish managed vs external instances and do not stop an unowned service on disconnect.

Validate role-scoped schema/SDK generation without embedding privileged schema or service keys into clients. Coordinate generator/metadata changes with the metadata owner and API semantics with Quarry. Use existing React/Flutter tooling rather than adding a UI framework. Developer examples are clients, not the database or execution authority.

Sources: Forge packages and https://supabase.com/docs/guides/getting-started/architecture .
