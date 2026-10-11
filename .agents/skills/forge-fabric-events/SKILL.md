---
name: forge-fabric-events
description: "Implement or review Forge realtime subscriptions, outbox delivery and Flint Fabric integration with tenant-safe replay."
license: MIT
metadata:
  author: prometheus
  version: "1.0.0"
---

# forge-fabric-events

Forge owns authoritative domain transactions and outbox intent. Flint Realtime Fabric owns its transport/streaming contracts; Gate owns its identity plane. The Boss is a controller; UAR owns a selected execution loop; BossFang owns channel/orchestration integration or its explicitly selected native loop. None of these is required merely to run Forge database APIs.

Read fdb-realtime and ext-flint-hooks plus the gateway subscription consumer. The current source says ListenChangeSource is the rollout default, with FabricChangeSource optional. Do not assume Fabric is always running. Record which backend each test exercises. Preserve the authorized WatchEntityType protocol, resume cursor and advertised capabilities rather than inventing transport fields.

WAL/notifications alone do not enforce row visibility. Recheck the row as the subscriber before disclosure, handle deletes without leaking previous rows, and specify revocation and outage behavior. Admission, delivery, payload expansion and actions are separate checks. Test tenant A/B, replay duplicates, cursor gaps, ordering, schema change, reconnect and authorization revocation.

Use stable event/idempotency identities and reconcile uncertain outcomes; no blanket exactly-once claim. Observers receive their own scoped subscription/inbox and cannot acquire an execution loop merely by observing. Prevent self-trigger loops and budget amplification. An origin token is provenance, not perpetual authority.

KnowMe offline intent remains pending until authoritative acceptance/commit; replicated personal documents are a separate consistency domain. Cross-project work is a request to the owning repo, not write permission here. Align actual protocol and dependency pins at a release checkpoint.

Sources: https://supabase.com/docs/guides/realtime/postgres-changes ; project crates/fdb-realtime/src/lib.rs and docs/agent-teams/flint-forge/RESEARCH.md .
