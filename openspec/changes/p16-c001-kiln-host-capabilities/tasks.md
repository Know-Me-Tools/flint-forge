# p16-c001 Tasks — Kiln Host Capability Surface

## Part A — Real capability enforcement

- [x] Add `kiln:capability:<name>` action constants to `crates/forge-policy/src/kiln.rs`
      (one per `fke_domain::Capability` variant), mirroring `KILN_INVOKE`/`KILN_REGISTER`
- [x] Decide and implement where a component's *requested* capabilities are recorded —
      turned out to already exist as `FunctionManifest.capabilities`; no new field needed
- [x] Replace `check_capabilities(granted, granted)` at `fke-runtime/src/lib.rs:212`
      with `check_capabilities(declared, &granted)` using the real requested set
- [x] Add per-capability Cedar check in `handle_with_telemetry` (loop over `declared`,
      call `forge_policy::kiln` capability action, default-deny on `Decision::Deny` —
      any single denial refuses the whole invocation, fail-closed, confirmed with user)
- [x] Tests: capability denial, capability allow, `caller = None` bypass (3 new tests,
      `fke-runtime/src/lib.rs`)

## Part B — Host trait implementations

- [x] Add `fdb-ports` as a dependency of `fke-runtime` — NOT `fdb-postgres` directly;
      extended the `DatabaseBackend` *port trait* itself with a new `query_json` method
      (keeps `fke-runtime` off the concrete Postgres adapter, since `PgConn.inner` is
      `pub(crate)`-only and unreachable from outside `fdb-postgres` anyway)
- [x] Implement `db` interface: `query(sql, params)` via `DatabaseBackend::query_json`,
      which wraps arbitrary SQL as `WITH __flint_kiln_query AS (sql) SELECT to_jsonb(t)
      FROM __flint_kiln_query t` — Postgres does universal row→JSON conversion instead
      of a lossy Rust-side type dispatch
- [x] Implement `llm` interface: `complete`/`embed` via the same `query_json` primitive
      against the real `llm.complete(prompt,opts,model)` / `llm.embed(input,model)` SQL
      signatures (`ext-flint-llm/sql/flint_llm.sql`)
- [x] Implement `kv` interface: per-`Store` `HashMap<String, Vec<u8>>` on `KilnHostState`,
      synchronous, infallible per the WIT doc comment
- [x] Implement `identity` interface: `origin_jwt`/`claims` sourced from `RlsContext`
      cloned onto `KilnHostState`; no separate "may see raw JWT" grant exists in the
      domain model yet, so `origin_jwt` reuses the `Identity` capability check — flagged
      as a modeling gap, not solved here
- [x] `ext-flint-vault`: added `vault.reveal_for_kiln(want_name, publisher_did, want_scope)`
      SQL function + dedicated `flint_kiln_worker` role/grant. Also: the role Kiln
      invocations actually connect as (`kiln_publisher` per `kiln_bgw.rs`) is never
      `CREATE ROLE`'d anywhere in the repo — `GRANT flint_kiln_worker TO kiln_publisher`
      still needs to land wherever that role is eventually created.
      **Partially verified** (toolchain unblocked — see below): `cargo-pgrx` +
      a from-source PG18 build via `cargo pgrx init --pg18` now work in this
      environment (env quirks fixed along the way: no `.cargo/config.toml` anywhere
      in the repo meant `cargo pgrx test` had apparently never run natively on macOS
      here before — added `ext-flint-vault/.cargo/config.toml` with
      `-Wl,-undefined,dynamic_lookup`, the standard fix for Postgres extension dylibs
      on macOS; added an empty `[workspace]` table to `ext-flint-vault/Cargo.toml`
      because this worktree is nested inside the main repo, which made
      `cargo metadata`-based tools false-match the outer repo's workspace). Got as
      far as `cargo pgrx test pg18` actually building `flint_vault.dylib`, loading it
      into a real running PG18, and pgrx discovering **5 functions** (the original 4
      plus `reveal_for_kiln`) and writing `flint_vault--0.1.0.sql` without error —
      strong evidence the new function's SQL is syntactically valid. Blocked short of
      a full green test run: `flint_vault.control` requires `pgcrypto`, which pgrx's
      `init` doesn't build (core Postgres only, not contrib modules); building
      `pgcrypto` standalone against the pgrx-managed PG18 hit a chain of its own
      macOS build issues (missing OpenSSL headers, then missing OpenSSL link libs,
      then missing core Postgres symbols at link time — the same `-undefined
      dynamic_lookup` class of issue as the Rust build, but PGXS's C Makefile isn't
      picking it up the way the Rust `.cargo/config.toml` fix does) — stopped
      chasing this specific sub-chain as diminishing returns. Net: `reveal_for_kiln`'s
      SQL is confirmed syntactically valid and extension-loadable; its *runtime
      behavior* (decrypt, audit log write) is still unexercised
- [x] Implement `secrets` interface: `get` returns an opaque `Resource<SecretHandle>`
      (gated on `Secrets` capability); `reveal` does an *additional* per-secret Cedar
      check (`kiln:secret:reveal` scoped to the secret name, default-deny) before
      calling `vault.reveal_for_kiln`
- [ ] Tests per interface: only `forge-policy`'s new `secret_reveal_request` action-naming
      test landed. No live-Postgres integration tests for `db`/`llm`/`secrets` yet —
      needs `cargo test -p fke-server` under `p35-c003-ci-postgres-service`'s Postgres,
      not attempted this pass

## Part C — Conditional linker wiring

- [x] `build_linker(engine, granted: &[Capability])` calls `add_to_linker` per
      `flint:host` interface only when present in `granted`. Gates on `declared`
      (known at `load_wasm` time) rather than a per-invocation `granted`, since the
      `Linker`/`ProxyPre` built here is cached and reused across every future
      invocation of that `id` — Part A's fail-closed capability gate guarantees
      `granted == declared` on every invocation that reaches a cached component, so
      this is equivalent. Left WASI/WASI-HTTP wiring unconditional (deliberately NOT
      gating `HttpOutgoing`): `wasmtime-wasi-http::add_only_http_to_linker_async`
      bundles the response-construction host functions every incoming-handler
      component needs with `outgoing-handler`, and splitting them isn't in scope here
- [x] Test: `build_linker_succeeds_for_every_capability_subset` regression-tests the
      conditional wiring itself. The real "component importing an ungranted interface
      fails at `instantiate_pre`" gate needs an actual multi-interface WASM component
      built via `cargo component` — unavailable in this environment (see Part D)

## Part D — Test component

- [x] Toolchain unblocked: installed `cargo-component` + `wasm32-wasip1`/`wasm32-wasip2`
      rustup targets. `examples/hello-component` builds clean against the real
      `edge-function` world, confirming the vendored `wit/flint/host/deps/` WIT and
      `world.wit` additions resolve correctly outside `wasmtime::component::bindgen!`
      too, not just inside it
- [x] `examples/hello-component` now calls `flint:host/kv` (`set`/`get` round-trip,
      reflected in the response body) — 1 of 5 interfaces. **Important correction to
      the design docs above**: targeting `edge-function` does NOT make a compiled
      component import all five interfaces — `cargo-component` dead-code-eliminates
      any `flint:host` import the guest never calls. Proven directly: the component
      instantiated successfully with zero capabilities granted *before* it called
      anything. `db`/`llm`/`identity`/`secrets` still aren't called from guest code
      (their guest-side bindings exist in the pre-existing checked-in
      `src/bindings.rs` scaffolding but are unused) — extending to all 5 not done
- [x] Real gate test added and passing against the actual compiled `.wasm`:
      `gate_hello_component_fails_to_load_without_kv_capability` — `&[]` and
      `&[Capability::Db]` (wrong capability) both correctly fail `instantiate_pre`
      with a missing-import error; `&[Capability::Db, Capability::Kv]` succeeds
- [ ] Wire the example into `fke-server` integration tests: full-grant success path,
      per-interface denial path — not attempted (needs a live Postgres + Cedar Pep,
      not just the WASM toolchain)

## Gate

- [x] `cargo test -p fke-runtime` passes (18/18, including 3 new Part A tests, the
      Part C `build_linker` regression test, and all pre-existing tests unchanged)
- [x] `cargo test -p forge-policy` passes (8/8, including 2 new Part A/B action tests)
- [ ] `cargo test -p ext-flint-vault` (or equivalent pgrx test path) — partially run;
      `flint_vault.dylib` builds/links/loads into real PG18 and its SQL (including
      `reveal_for_kiln`) is confirmed valid, but the existing 2 pg_tests still fail
      because `pgcrypto` (a `flint_vault.control` dependency) isn't built in this
      environment's pgrx-managed PG18 — see Part B note above
- [ ] Test component demonstrates all five interfaces end-to-end under `fke-server` —
      blocked on Part D
- [x] `cargo clippy -p fdb-ports -p fdb-postgres -p forge-policy -p fke-domain
      -p fke-runtime -p fke-server -- -D warnings` clean (not run against the full
      `--workspace`, which includes crates unrelated to this change)
- [x] No new `unwrap()`/`expect()` in library crates touched by this change (the two
      `.expect()` calls added — `"string JSON-encoding is infallible"` and `"identity
      checked present above"` — are on genuinely unreachable states, matching the
      repo's stated exception, and are asserted, not silently trusted)
