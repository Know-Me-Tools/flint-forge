# p3-c006 — Keto Sync: FRF Iggy keto_changes → flint_meta.keto_tuples

## Change ID
`p3-c006-keto-sync`

## Phase
`p3-graphql-hybrid-engine`

## Priority
P0 — Depends on OQ-8 (FRF Iggy `keto_changes` event type confirmation)

## Open Question Gate

**OQ-8 must be resolved before this change can be coded.**

OQ-8: Does `flint-realtime-fabric` publish a `keto_changes` Iggy topic when
Keto relation tuples change? What is the message schema?

Resolution path: Read `flint-realtime-fabric` source — look for Iggy publisher
code that emits keto-related events, or check if FRF delegates Keto change
propagation to a different bus.

If OQ-8 resolves to "no `keto_changes` topic exists": this change is
redesigned around polling Keto's `/relation-tuples` API on a schedule instead.

## Problem Statement

The four-layer auth model (§2.3) requires Postgres RLS to be authoritative.
RLS policies reference `flint_meta.keto_tuples` to evaluate row-level
visibility. This table must stay in sync with Keto's relationship store.

Current state: `flint_meta.keto_tuples` is defined (in `flint_auth` or schema
migrations) but has no sync mechanism. Keto changes are not propagated.

## Scope

### In Scope (assuming OQ-8 = keto_changes topic exists)
- Subscribe to FRF Iggy `keto_changes` topic from Quarry
- Parse `KetoTupleChange` events (namespace, object, relation, subject_id, op)
- Apply upsert/delete to `flint_meta.keto_tuples` in Postgres
- Implement in a new background task within `fdb-gateway` (Tokio task, not a
  separate process)
- The sync task restarts automatically on Iggy connection drop

### In Scope (if OQ-8 = polling only)
- Periodic poll of Keto `/relation-tuples` API (configurable interval, default 60s)
- Full-replace sync: fetch all tuples for configured namespaces, diff against
  `flint_meta.keto_tuples`, apply batch upsert/delete

### Out of Scope
- Subscribe-time Keto check in `FabricChangeSource` (p3-c002)
- Cedar policy evaluation (forge-policy)
- Keto administration API (creating/deleting tuples from Quarry — Quarry is
  read-only from Keto's perspective)

## Design (Iggy-path — provisional on OQ-8)

### KetoSyncTask (fdb-gateway/src/keto_sync.rs)

```rust
pub struct KetoSyncTask {
    iggy_client: IggyClient,
    db: Arc<dyn DatabaseBackend>,
    stream: String,   // "keto_changes" or configurable
    consumer_group: String,
}

impl KetoSyncTask {
    pub async fn run(self) {
        loop {
            match self.iggy_client.subscribe(&self.stream, &self.consumer_group).await {
                Ok(mut stream) => {
                    while let Some(msg) = stream.next().await {
                        if let Ok(evt) = serde_json::from_slice::<KetoTupleChange>(&msg.payload) {
                            if let Err(e) = self.apply_change(&evt).await {
                                tracing::error!(error = ?e, "keto tuple sync failed");
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(error = ?e, "iggy keto_changes disconnected — retrying in 5s");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    async fn apply_change(&self, evt: &KetoTupleChange) -> Result<(), SyncError> {
        match evt.op {
            Op::Upsert => {
                self.db.execute(
                    "INSERT INTO flint_meta.keto_tuples (namespace, object, relation, subject_id)
                     VALUES ($1, $2, $3, $4)
                     ON CONFLICT (namespace, object, relation, subject_id) DO NOTHING",
                    &[&evt.namespace, &evt.object, &evt.relation, &evt.subject_id],
                ).await
            }
            Op::Delete => {
                self.db.execute(
                    "DELETE FROM flint_meta.keto_tuples
                     WHERE namespace=$1 AND object=$2 AND relation=$3 AND subject_id=$4",
                    &[&evt.namespace, &evt.object, &evt.relation, &evt.subject_id],
                ).await
            }
        }
    }
}
```

### Background task spawn (fdb-gateway/src/main.rs)

```rust
tokio::spawn(keto_sync_task.run());
```

Spawned after the Axum router is built, before `serve()`. Failure of the sync
task does NOT crash the gateway — it logs and retries.

### KetoTupleChange schema (provisional — confirm with OQ-8)

```rust
#[derive(serde::Deserialize)]
struct KetoTupleChange {
    namespace: String,
    object: String,
    relation: String,
    subject_id: String,
    op: Op,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "lowercase")]
enum Op { Upsert, Delete }
```

## Security Contracts
- Keto sync task MUST NOT log `subject_id` values (PII)
- `flint_meta.keto_tuples` is trusted by RLS policies — a corrupt sync would
  allow unauthorized data access. The upsert uses exact tuple matching; no
  wildcard or range operations
- The sync task MUST run as a privileged Postgres role that can write to
  `flint_meta.keto_tuples` — this is a service-level operation, NOT a
  user-context operation. It MUST NOT use `SET LOCAL ROLE authenticated`
- Iggy connection credentials MUST be sourced from environment variables (never
  hardcoded)

## Acceptance Criteria
- `KetoSyncTask::run()` implemented (or polling equivalent if OQ-8 = polling)
- Background task spawned in `fdb-gateway/src/main.rs`
- OQ-8 documented with resolution in `current-waypoint.json`
- Unit test `test_keto_sync_applies_upsert_and_delete` (mock `DatabaseBackend`)
- `cargo check --workspace` GREEN; clippy pedantic passes
