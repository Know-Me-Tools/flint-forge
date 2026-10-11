---
name: forge-wasm-ember
description: "Develop or review Flint Kiln WASM execution or Anvil Ember in-database model jobs while preserving their separate execution contracts."
license: MIT
metadata:
  author: prometheus
  version: "1.0.0"
---

# forge-wasm-ember

For Kiln work, read fke-runtime, fke-server, store/signing adapters and WIT contracts. Enforce signature verification, content digest identity, capability-limited host imports, Cedar effect checks, fuel/epoch/cancellation limits, bounded memory and tenant-scoped secrets. Verify actual Wasmtime major and WIT world pins with UAR/KnowMe owners; do not align dependency versions blindly or share incompatible AOT caches.

For Ember work, read ext-flint-llm and its SQL contracts. Separate synchronous SQL/model calls from queued jobs. Account for database transaction duration, pool budgets, retries, token/cost limits, provider outages and rollback. The ext-flint-hooks owner owns shared capture/outbox changes; Ember consumes the agreed contract. Never duplicate a committed external effect by retrying a whole transaction without reconciliation.

Model/provider metadata comes from a configured liter-llm/UAR endpoint or a provenance-bearing catalog. Missing price/capability data stays unknown. No embedded credentials in SQL, prompts, traces or agent files. Do not send production rows to a provider as part of testing without authorization.

The harness owns agent execution. An edge-function runner and a SQL model call are not permission to start another agent loop. Cross-project runtime reuse must preserve standalone Forge operation and explicit external instance selection.

Sources: https://docs.wasmtime.dev/security.html ; https://docs.wasmtime.dev/api/wasmtime/ ; Forge docs/FLINT-FORGE-SPEC.md and WIT/extension manifests.
