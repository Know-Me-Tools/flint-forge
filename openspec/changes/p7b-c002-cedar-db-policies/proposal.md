# p7b-c002 — Cedar Policies from Postgres (`flint_kiln.cedar_policies`)

**Phase:** 7b — Kiln Production Hardening
**Priority:** P0
**Depends on:** none (migration runs at startup; `AllowAllPolicySource` stays until replaced)
**Blocks:** p7b-c003 (BGW publisher identity is meaningless until Cedar is real)

## What this change delivers

Replaces `AllowAllPolicySource` in `fke-server` with a DB-backed
`DbKilnPolicySource` that loads Cedar policies from a new
`flint_kiln.cedar_policies` table. The Kiln Cedar gate now enforces
real policy instead of permitting everything.

## Design

### Migration `0009_flint_kiln_cedar_policies.sql`

Same schema as `flint_meta.cedar_policies` (used by the Quarry):

```sql
CREATE TABLE IF NOT EXISTS flint_kiln.cedar_policies (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    policy_text text NOT NULL,
    enabled     boolean NOT NULL DEFAULT true,
    description text,
    created_at  timestamptz NOT NULL DEFAULT now()
);

-- Bootstrap: permit every kiln:invoke so the system works
-- before an operator adds real policies.
INSERT INTO flint_kiln.cedar_policies (policy_text, description)
VALUES (
    'permit(principal, action, resource);',
    'bootstrap allow-all — replace with scoped policies'
)
ON CONFLICT DO NOTHING;
```

### `crates/fke-server/src/kiln_db_policy.rs`

Direct port of `fdb-gateway/src/policy_source.rs`, changing the table name
from `flint_meta.cedar_policies` to `flint_kiln.cedar_policies`:

```rust
pub struct DbKilnPolicySource { pool: PgPool }

impl DbKilnPolicySource {
    pub fn new(pool: PgPool) -> Self { ... }
}

#[async_trait]
impl PolicySource for DbKilnPolicySource {
    async fn load(&self) -> Result<Vec<PolicyEntry>, PolicyLoadError> {
        // SELECT id::text, policy_text, enabled
        // FROM flint_kiln.cedar_policies WHERE enabled = true
    }
}
```

### `fke-server/src/main.rs` wiring

Replace:
```rust
let pep = CedarPolicyEngine::new(Arc::new(kiln_policy::AllowAllPolicySource)).await;
```
With:
```rust
let policy_source = Arc::new(kiln_db_policy::DbKilnPolicySource::new(pool.clone()));
let pep = CedarPolicyEngine::new(policy_source).await;
```

The `AllowAllPolicySource` file can be deleted or kept for testing.
