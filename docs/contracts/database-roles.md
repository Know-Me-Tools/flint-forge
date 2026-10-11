# Contract: Database Role Identity for `fdb-gateway`

## Status: RESOLVED — the gateway's `DATABASE_URL` must name the extension owner

### The problem this documents

`fdb-gateway` applies `migrations/` on every startup (`crates/fdb-gateway/src/bootstrap.rs`, via
`sqlx::migrate!`). Migration `0003_a2ui_triggers.sql` does:

```sql
CREATE TRIGGER a2ui_auto_bind_tables
    AFTER INSERT ON flint_meta.cache_tables
    ...
```

`flint_meta.cache_tables` belongs to the `ext-flint-meta` pgrx extension. In Postgres,
**`CREATE TRIGGER` requires ownership of the target table — no `GRANT` substitutes for it**, even
`GRANT ALL`. An extension is owned by whichever role executed `CREATE EXTENSION`, which for this
image is whichever role ran the `/docker-entrypoint-initdb.d/` scripts — i.e. `POSTGRES_USER` on the
Postgres image's first boot.

So the requirement is narrow and absolute: **`DATABASE_URL` must authenticate as the role that owns
`ext-flint-meta`** (and every other pgrx extension this image installs — `ext-flint-auth`,
`flint_vault`, `flint_llm`). Any other role fails migration 3 with:

```
error: database migration failed: ExecuteMigration(Database(PgDatabaseError {
  code: "42501", message: "permission denied for schema flint_meta", ... }))
```

This repository's own `docker-compose.yml` already gets it right — `POSTGRES_USER: flint` and
`DATABASE_URL: postgres://flint:flint@db:5432/flint` are the same role — which is why this has never
surfaced here. It surfaces the moment a consumer runs the image with a different `POSTGRES_USER` and
connects the gateway as a separate, narrower role (for example a dedicated non-superuser schema owner
for their own application tables). That pairing is reasonable — least privilege for application
schemas is correct practice — and the gateway currently gives no indication of why it fails.

### Verified

- `service_role` granted `GRANT ALL ON ALL TABLES IN SCHEMA flint_meta` still fails
  `CREATE TRIGGER ... ON flint_meta.cache_tables` with `permission denied for schema flint_meta`
  (schema-level `CREATE`/`USAGE` is what's missing, and granting *that* still does not confer
  ownership of objects the extension already created).
- Transferring `flint_meta`'s *schema* ownership to a different role does not fix it either: the
  failure changes from `permission denied for schema flint_meta` to `permission denied for table
  cache_tables` — the same object, now correctly diagnosed as a **table-ownership** requirement, not a
  schema-privilege one. `ALTER TABLE ... OWNER TO` on every extension-created object is the only grant
  path that would work, and doing that piecemeal is functionally re-deriving "connect as the owner"
  one object at a time.
- Connecting as the role that ran the image's init scripts (`POSTGRES_USER`) clears every migration.

### The contract

1. **`DATABASE_URL` for `fdb-gateway` MUST authenticate as `POSTGRES_USER`** (or, in a deployment
   that does not use this image's init flow, whichever role owns every extension the schema
   provisioning docs (`FFS-001`) list as required). This is not a recommendation; migrations fail
   deterministically otherwise.
2. **A consumer that wants a separate, least-privilege application role** (e.g. a `NOSUPERUSER` owner
   for its own schema, so `FORCE ROW LEVEL SECURITY` actually binds it — extension roles are
   frequently superusers or extension-owners and are exempt from `FORCE RLS` the same way a
   superuser is) **creates that role in addition to, not instead of**, the extension-owning
   connection the gateway uses. Two roles, two jobs: one migrates and owns Forge's own schemas, the
   other owns and is constrained on the consumer's application schema.
3. This is orthogonal to `FFS-001`'s `flint_provisioner` role, which governs **runtime, caller-driven
   DDL** through the provisioning API. `flint_provisioner` is deliberately weaker than the gateway's
   own bootstrap identity (`NOLOGIN`, namespace-allowlisted, no database-level `CREATE`) — see
   `docs/FFS-001-SCHEMA-PROVISIONING.md` D3. The gateway's own migrations, run at every startup, are a
   different, earlier, and necessarily more privileged step: they install and own the extensions that
   `flint_provisioner` and everything downstream depend on.

### What this document does not change

No code changes here. See `crates/fdb-gateway/src/bootstrap.rs` for the companion preflight check
that turns a misconfigured `DATABASE_URL` into a named startup error instead of an opaque `42501` on
migration 3.

### Provenance

Found integrating `fdb-gateway` against a Postgres image that installs its extensions under a
`POSTGRES_USER` other than the one the gateway was pointed at. Three deliberately wrong role
assignments were tried and each failed a documented way before the correct one — connecting as the
image's own bootstrap role — cleared every migration. See go-mark `phase-2-design-to-fullstack-scaffold`
child `c-local-stack`, change `gc01-compose-identity-and-data`.
