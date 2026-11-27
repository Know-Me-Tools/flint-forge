# Validation receipt and held acceptance

## Actual evidence
- Source snapshot copied 1429 non-symlink tracked/source files, preserving dirty cancellation cleanup source/tests, without shared .git/KBD/owner ledger or ignored credentials. Per-file hashes in source-provenance.json.
- New source formatting through pinned Rust 1.96 rustfmt completed exit 0 (session 6004). This is parsing/formatting evidence, not a compile verdict.
- First cargo check through rustup shim: session 67146, no output, cancelled exit 130.
- Pinned Cargo/rustc offline scoped check: session 53686, no output across polls, cancelled exit 130. Reason for silence is unknown. No compile pass/fail established; no further retry performed.
- Process listing with ps denied by sandbox; not retried or bypassed.
- No tests, PostgreSQL connections, migrations, service probes, deployments or UAR pin changes executed.
- New Rust source/test files are below 500 lines. bootstrap.rs already exceeded 500 in the baseline; only composition wiring was changed, no unrelated split/refactor.

## Authored tests, not executed
`crates/fdb-gateway/tests/file_metadata_live_pg.rs` is ignored by default and uses a preprovisioned disposable PostgreSQL fixture only. It proves intended checks for committed persistence across a separate pool, invalid schemas, immutable version conflicts, invalid value rollback/no receipt, two-tenant and same-tenant/two-subject isolation, same-CID privacy, CAS races, concurrent same-key replay, historical replay, changed-key conflict, JSONB/tag/category discovery, pagination, direct-SQL validation and receipt-forgery rejection, HTTP read wiring, deny-all Cedar and default-off availability. It does not verify real JWT issuance/JWKS, generic GraphQL behavior, service-role/owner/BYPASSRLS deployment posture, cancellation timing or all profile keyword combinations. These remain required review/acceptance gaps, not claimed passes.

## Exact commands, held pending review
Run from `/Volumes/T7/Projects/isolated/flint-forge-file-metadata-20261115`.

Permitted compile attempt, not a runtime test:
`RUSTC=/Users/gqadonis/.rustup/toolchains/1.96-aarch64-apple-darwin/bin/rustc /Users/gqadonis/.rustup/toolchains/1.96-aarch64-apple-darwin/bin/cargo check --offline -p fdb-gateway -p fdb-postgres -p fdb-app -p fdb-domain -p fdb-ports`

After reviewing the completed source boundary, scoped lint/test compilation:
`cargo clippy --offline -p fdb-gateway -p fdb-postgres -p fdb-app -p fdb-domain -p fdb-ports --all-targets -- -D warnings`
`cargo test --offline -p fdb-gateway --test file_metadata_live_pg --no-run`

Real PostgreSQL acceptance, only after approval to provision an isolated fixture, apply reviewed migration 0021, and execute writes:
`FILE_METADATA_TEST_ALLOW_WRITES=isolated-disposable cargo test --offline -p fdb-gateway --test file_metadata_live_pg -- --ignored --nocapture`

The operator must supply `FILE_METADATA_TEST_DATABASE_URL` via an approved secret channel. It must name a disposable database ending `_file_metadata_test`, with the existing authenticated role and reviewed migration installed. The test intentionally does not provision a database, apply migrations, guess credentials, use DATABASE_URL, or clean up another fixture. It leaves synthetic owner-scoped rows for inspection. Disposable DB teardown needs its own authorized fixture lifecycle. The command alone does not authorize fixture creation, grants or DDL.

Full workspace checks and repository CI remain outstanding after scoped checks. Compilation and all tests must be observed before declaring this implementation ready.

## Independent-review correction batch
Source-only corrections: reject non-string definition type_id in SQL; validate concrete match MIME grammar; narrowly map PostgreSQL SQLSTATE 22P05 to invalid_metadata. Authored acceptance now checks numeric type-ID SQLSTATE 23514 followed by working registry list/match, successful HTTP type/record writes and independent persistence, wildcard/case-insensitive matching, no-match/ambiguity/malformed inputs, exact SQLSTATE 23514 for direct-write guard failures, explicit U+0000 SQLSTATE 22P05 and HTTP 422/no record, and deny-policy unchanged revision. The test-only permit is restricted to the exact production resource/action pairs.

No compile, tests, DDL, runtime or formatter invoked for this correction batch. Earlier formatting exit 0 applies to the prior revision, not these later edits. Source remains not compile-verified. Generic database grants can bypass HTTP-only Cedar/default-off gates; deployment review must preserve that disclosed boundary. No live grants or policies were changed. Coordinator reported a configured sccache rustc wrapper and external build directory; the prior silent-compilation cause is still unknown, and no global configuration/process/cache action was taken.
