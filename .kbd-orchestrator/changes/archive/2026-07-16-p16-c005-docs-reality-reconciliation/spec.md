# p16-c005 — Reconcile documentation with the code

**Phase:** p16-v1.0-release-closure
**Priority:** P1 — Tier 1, beta-blocker
**Scope:** `README.md`, `crates/fdb-reflection/src/compilers/rest/mod.rs`,
`crates/fdb-realtime/src/lib.rs`, `.env.example`
**Delivery model:** self-hosted OSS

---

## Problem

Documentation disagrees with the code **in both directions**. A self-hosted
operator has only the docs — there is no vendor to correct them.

### Overstatements (docs promise more than the code delivers)

- `README.md:40` advertises "**GraphQL Subscription** — `async-graphql` over
  `graphql-transport-ws`" unconditionally. Outside Helm, subscriptions deliver
  nothing (see p16-c002).
- `README.md:104` promises RLS enforcement on "every query / subscription
  event." With the default source there *are* no subscription events.
- `.env.example:58` says `listen` replaces "polling." There is no polling. The
  alternative is an empty stream.

### Understatements (docs claim less than the code delivers)

- `crates/fdb-reflection/src/compilers/rest/mod.rs:62` — "CRUD handlers remain
  `todo!()` stubs pending the query-builder landing." **False.**
  `handle_insert`, `handle_update`, `handle_delete` are fully implemented in
  `compilers/rest/mutations.rs`. The comment is stale and undersells the product.

### Verified non-problem

There are **zero live `todo!()` or `unimplemented!()` calls** in any crate. All
grep matches are inside comments and doc-strings. Recorded because an earlier
pass of this analysis wrongly flagged them as stubs.

## Change

Correct each site to match the code. Where the code is wrong (subscriptions),
p16-c002 fixes the code and this change updates the prose *afterward* — do not
paper over a defect with a caveat.

Sequence: **c002 first, then c005.** Otherwise c005 documents behavior that
c002 is about to change.

## Acceptance Criteria

1. `README.md` describes subscription behavior that matches the shipped default
   after c002 lands.
2. The `rest/mod.rs:62` doc-comment no longer claims CRUD handlers are stubs.
3. `.env.example` contains no reference to "polling."
4. `grep -rn "todo!()" crates/ --include=*.rs` returns only comment/doc-string
   matches (i.e. no regression to live stubs).
5. No documented feature lacks a working default configuration.

## Non-Goals

- Rewriting `docs/` wholesale. Scope is the four sites above plus anything a
  reviewer finds contradicted while fixing them.

## Verification Command

```bash
grep -n "polling" .env.example        # must return nothing
grep -n "todo!()" crates/fdb-reflection/src/compilers/rest/mod.rs
cargo doc --workspace --no-deps 2>&1 | grep -i warn
```

## Risk

**Low.** Prose only. The dependency on c002 is the only sequencing hazard.
