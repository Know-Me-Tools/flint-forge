# Tasks — p35-c003-ci-postgres-service

- [x] docker/postgres/Dockerfile: PG18 + pgvector (v0.8.0) + pg_graphql (v1.5.11), pinned.
- [x] scripts/ci-test.sh: unit stage (always) + db-integration stage (DATABASE_URL-gated,
      migrate + cargo test --include-ignored). Verified: shellcheck clean, no-DB path runs green.
- [x] .dagger/main.go: CheckDb — pinned PG service binding + DATABASE_URL + ci-test.sh (gofmt clean).
- [~] Run CheckDb / build the image end-to-end — BLOCKED in this env (no dagger CLI, no Docker
      daemon). Must be verified on a host with both.
- [~] flint_meta bootstrap for the full migration set — surfaced as a follow-up (migrations
      assume the ext-flint-meta pgrx schema exists); wire before CheckDb can be fully green.
