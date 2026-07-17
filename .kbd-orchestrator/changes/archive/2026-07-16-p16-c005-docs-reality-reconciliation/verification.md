# Verification — p16-c005

## Gate
No documented feature lacks a working default configuration.

## Evidence to record on completion
- [x] `grep -n polling .env.example` returns nothing (already fixed as part of
      p16-c002 t4; re-confirmed clean here)
- [x] rest/mod.rs:62 no longer claims CRUD handlers are todo!() stubs — doc
      comment now says handlers "execute parameterized SQL against the
      reflected schema, gated by `is_safe_identifier`"
- [x] README subscription claims match post-c002 behavior — the GraphQL
      Subscription bullet now states the default is `listen` (working) and
      `fabric` is opt-in and fails closed pending OQ-FRF-1; the "Subscription
      RLS enforcement" section no longer says "from the fabric" specifically
      (was stale even before c002 — the RLS re-query applies to whichever
      `ChangeStreamSource` is active)
- [x] `grep -rn "todo!()" crates/` — only comment/doc-string matches
      (`fdb-gateway/tests/mounts_reflection_router.rs`); zero live stubs

## Additional evidence (2026-07-16, sweep task t5)
Found and fixed one contradiction outside the four named spec sites:
`crates/fdb-gateway/tests/mounts_reflection_router.rs` had stale comments
claiming `handle_list` "is still `todo!()`" and that the handler "panics" —
false since the CRUD handlers were implemented. The test's actual assertion
(non-404) was still correct either way (it tolerates panic-or-error-response
against the lazy/unreachable pool), so this was a comment-only fix, not a
behavior change. Re-ran the test after the fix: `cargo test -p fdb-gateway
--test mounts_reflection_router` — 1/1 passed.

Also ran the spec's exact verification commands directly:
```
grep -n "polling" .env.example                                    # PASS: nothing
grep -n "todo!()" crates/fdb-reflection/src/compilers/rest/mod.rs  # PASS: nothing
cargo doc -p fdb-reflection -p fdb-gateway --no-deps 2>&1 | grep -i warn  # PASS: nothing
```
`cargo clippy -p fdb-gateway -p fdb-reflection -- -D warnings` — clean.

## Status
COMPLETE — 5/5 tasks. All 5 acceptance criteria met.
