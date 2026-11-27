# File metadata HTTP contract v1

Status: authored and mounted in the isolated source snapshot, not compiled or runtime-verified. Migration 0021 is authored, not applied. All activation is held.

## Authentication and availability
Every endpoint uses the existing gateway verified `Authorization: Bearer <JWT>` middleware. Only authenticated tenant-bound subjects are accepted. Cookies alone are not sufficient. There is no supplied WebUI session-to-Forge-token bridge. Owner/tenant values never come from request fields. Missing or invalid bearer: existing 401 response. Missing tenant/subject or another role: 403. Default unset `FLINT_FILE_METADATA_ENABLED` gives 503 to authenticated requests; `true` enables the configured adapter. Any other non-false value is a configuration error. This flag controls only the new HTTP surface, not database privileges or generic APIs.

Mutation permission is mandatory existing Cedar, with optional deployment-mode Keto. Resources: `flint_files.types` action `insert`; `flint_files.records` actions `insert` or `update` according to expected revision. No permit policy is seeded. No new database role is created. The authored migration's grants and RLS require independent security review before applying. Generic API writes retain their existing authorization behavior but cannot bypass the new database schema/revision/owner invariants.

## Endpoints
- GET /files/v1/capabilities: profile, grammar restrictions, query/type bounds.
- GET /files/v1/types: all visible immutable versions, ordered by type_id then version. More than 1000 versions fails rather than silently truncating selection.
- PUT /files/v1/types: full definition below. Equal existing definition succeeds; changed version content conflicts.
- POST /files/v1/types/match: `{ "mime_type":"application/pdf", "filename":"invoice.pdf" }`. Returns definition or null. All matching versions count; multiple matches return ambiguity instead of selecting latest.
- GET /files/v1/records/{file_id}: one visible instance.
- PUT /files/v1/records/{file_id}: full metadata replacement below. CID is immutable per instance. A changed schema version/type must validate replacement attributes. No bytes, pinning, indexing or storage placement change.
- POST /files/v1/records/query: `{ "cid":null, "type_id":null, "tags":[], "categories":[], "attributes":{}, "limit":100, "after":null }`. Arrays use all-of containment. Attributes use PostgreSQL JSONB containment, not arbitrary comparisons. Order/cursor is file_id. Returned list length equal to limit may mean more data; resume after the last returned file_id.

## Definition
```json
{
  "type_id":"invoice","version":1,"schema_profile":"file-schema-v1",
  "schema":{"type":"object","properties":{"amount":{"type":"integer"}},"required":["amount"],"additionalProperties":false},
  "match":{"mime_types":["application/pdf"],"filename_suffixes":[".pdf"]}
}
```

The schema profile supports type, properties, required, additionalProperties, items, enum, title, description only. Every schema node requires a single string type from object/array/string/number/integer/boolean/null. Root must be object. Boolean schemas, type arrays, schema-valued additionalProperties, tuple items and all other keywords (including `$schema`, `$id`, `$ref`, format, pattern, numeric/string limits and combinators) fail registration. This is a strict subset, not full Draft 2020-12. Supported keywords retain standard JSON Schema semantics, including object/array keywords having no effect on other instance types and integral JSON numbers satisfying integer. required and enum reject duplicate values; enum must be nonempty. Depth maximum 16. Unknown keywords never silently disappear. Schema validation is authoritative in PostgreSQL, not a handwritten UI validator.

Criteria groups combine AND; alternatives within each group combine OR. Empty groups impose no constraint; both empty means explicit-only type. Registered MIME criteria are case-insensitive exact or type/*. Match inputs must be concrete nonempty type/subtype tokens in the same supported alphabet; wildcard inputs, extra slashes, parameters and whitespace are rejected; filename suffix is literal and case-sensitive. No regex, sniffing, path traversal or filesystem effects.

## Record replacement
```json
{
 "cid":"bafy...","path":"invoices/one.pdf","mime_type":"application/pdf",
 "type_id":"invoice","type_version":1,"attributes":{"amount":10},
 "tags":["paid"],"categories":["finance"],
 "expected_revision":0,"mutation_id":"upload-one"
}
```

file_id/type_id/mutation_id use 1-128 ASCII alphanumeric, dot, underscore or hyphen. Initial expected_revision=0 produces revision 1; update requires exact current revision. Requests are full replacement, not merge patches. Revision/key conflicts are 409. Same owner+mutation_id+request JSON returns the original successful response even after later edits. Changed request with the same key conflicts. Invalid writes roll back both record and receipt. A file record is an owner-local instance, not a global CID object. CID is an opaque bounded, whitespace-free content-producer assertion: this slice does not decode the CID or prove IPFS existence/ownership. Path is descriptive, not an access instruction. Maximum request 64 KiB, tags/categories each at most 64 entries of 128 characters (app uses UTF-8 byte limit), path at most 4096 UTF-8 bytes at the HTTP boundary.

## Envelopes
Success: `{ "data": value }`, HTTP 200, where value is a definition, a record, an array or null for no match. Record is the replacement fields plus `file_id` and `revision`; no owner identifiers are emitted.

Domain errors: `{ "error": { "code": "..." } }`:
- 422 invalid_metadata: invalid schema/attributes/shape/bounds, or PostgreSQL JSONB untranslatable-character SQLSTATE 22P05 (including U+0000). This additional mapping is specific, not a blanket mapping of all database data errors. Runtime verification remains outstanding.
- 409 metadata_conflict: immutable definition, CAS, or retry conflict.
- 409 ambiguous_type: more than one match; select an explicit version.
- 404 metadata_not_found: missing or invisible instance.
- 403 metadata_denied: identity or policy denied.
- 503 metadata_unavailable: disabled or persistence unavailable.

No raw database details or claims appear in these responses. Axum malformed JSON, extraction and body-limit failures retain native 400/413/422 bodies. Existing JWT middleware retains its existing 401 body. Clients must not assume every non-2xx body is a domain envelope.
