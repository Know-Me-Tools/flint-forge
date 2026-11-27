# File metadata v1 implementation plan

This isolated change implements the explicit file-type/attribute request. Decisions below are implementation proposals, not previously user-approved architecture. Source provenance is recorded at source-provenance.json. No shared checkout, owner ledger, KBD state, Git administration, credentials, services or deployments are changed. The source project pins native-kbd; this receipt is isolated and does not claim a canonical phase transition.

## Boundary and model
A file record is a private file instance, keyed by verified tenant + subject + caller-supplied file_id. CID is immutable content reference, not global metadata identity. File bytes, IPFS storage/pinning, embeddings and UI are outside this change. Existing CID attachment is supported without claiming possession or verifying existence. Path is an optional descriptive string, never a filesystem operation.

Type definitions are immutable data keyed by tenant + subject + type_id + positive version. Schemas must be objects using the strict file-schema-v1 JSON Schema profile: type (object,array,string,number,integer,boolean,null), properties, required, additionalProperties (boolean), items, enum, title and description. Unsupported keywords including $ref fail closed. Root is object. Nested schemas bounded to depth 16 and total encoded payload 64 KiB. Same type/version/body replay succeeds; any changed definition conflicts. MIME exact values or type/* and literal filename suffixes are deterministic; criteria groups AND, alternatives within each group OR. Multiple matching definitions return 409 rather than selecting a winner.

PUT /files/v1/records/{file_id} accepts cid,path,mime_type,type_id,type_version,attributes,tags,categories,expected_revision,mutation_id. Initial revision expectation 0 creates revision 1. Updates require exact existing revision and keep CID unchanged. Mutation key is per owner and request JSON equality, including file_id, is authoritative. Receipt and mutation commit atomically, transaction advisory locks serialize same mutation and file. Replay returns original response even after later edits. Changed payload with same key conflicts. Reads remain private. POST /files/v1/records/query accepts optional cid/type_id/after, tags/categories arrays, JSON object attributes and limit <=100; AND semantics, JSONB containment, file_id cursor/order.

## Architecture and security
Domain DTOs in fdb-domain; FileMetadataStore port in fdb-ports; input validation/use-case orchestration in fdb-app; PgFileMetadata adapter in fdb-postgres using existing PgBackend::acquire and cancellation-safe PgConn; Axum router in fdb-gateway attached to existing verified JWT middleware and composition root. All routes mounted; unset FLINT_FILE_METADATA_ENABLED returns unavailable after authentication. Exact true enables, invalid values fail startup. Existing authenticated role only; tenant and subject must be nonempty. Cedar is mandatory and deny-by-default for mutations, existing optional Keto remains independent. No policy seed authorizes these resources.

Postgres schema flint_files holds types, records, mutation receipts. FORCE RLS compares both tenant and subject against transaction-local verified claims. No anonymous or service-role grants added. Triggers enforce schema/record invariants even on generic GraphQL/REST writes; record updates cannot change identity/CID and must advance revision by one. Type definitions cannot update/delete. JSON schema profile validated authoritatively in the database; the app boundary validates typed DTOs and request bounds. All SQL uses bind parameters. No sensitive values logged. New migration authored only; never applied here. Generic CRUD access is not a bypass for schema validation, ownership or revision invariants. Direct CRUD is not covered by API replay semantics.

## Acceptance
Complete all load-bearing modules and mount router before checks/tests. One scoped compile, then phase-boundary tests, then independent review. Database acceptance must prove persisted read after new connection, invalid attribute rejection with no partial update, both owner/tenant isolation, same-key replay and changed-key conflicts, concurrent CAS, filtering and generic mutation validation bypass attempts. Compile, lint and SQL integration remain unrun until specifically permitted where required. No service probes, database connections, migrations or fixture setup in this task.

## Work sequence
1. Domain/port/application validation.
2. PostgreSQL schema, triggers and adapter, atomic replay/CAS.
3. Authenticated default-off routes and composition.
4. Compile, repair, authored live PostgreSQL acceptance, boundary tests and reviewer handoff.
