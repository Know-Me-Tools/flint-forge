# p3-c018 — IntrospectionMerger Verify + pg_graphql PG18 Pre-flight

## Change ID
`p3-c018-introspection-merge-verify`

## Phase
`p3-auth-rls-keto`

## Goal Mapping
**G4 confidence** — verify the pg_graphql schema SDL ∪ subscription SDL merge
actually works. Scope is conditional on OQ-3.

## Problem
`IntrospectionMerger::merge()` is called in the gateway but may be a stub.
OQ-3 asks whether pg_graphql has a tagged PG18 release — if absent, the
passthrough path errors.

## Scope
- **Pre-flight (OQ-3):** run `SELECT extversion FROM pg_extension WHERE
  extname = 'pg_graphql';` against the PG18 container. Record the result.
- **If pg_graphql present:** implement/verify `IntrospectionMerger::merge()`
  produces a single SDL that is the union of pg_graphql output and the
  subscription SDL from `GraphQlCompiler::compile()`. Add a unit test with
  two fixture SDLs asserting the merged output contains all types from both.
- **If pg_graphql absent:** ship a verify-only change. Document a stub
  fallback (gateway returns `501 Not Implemented` on `/graphql` introspection
  with a clear error). Defer full merge to a future change. Record
  `kbd_stage_handoff_skip` note.

## Out of Scope
- Subscription schema compilation (already done in p3-c007-graphql-compiler).
- Live GraphQL query execution testing.

## Acceptance Criteria
- [ ] OQ-3 pre-flight result recorded in this proposal's *Verification Log* section
- [ ] If present: `IntrospectionMerger::merge()` implemented + unit-tested with fixture SDLs
- [ ] If absent: 501 stub documented; handoff-skip recorded; phase gate still achievable
- [ ] `cargo check` + clippy + relevant test green
