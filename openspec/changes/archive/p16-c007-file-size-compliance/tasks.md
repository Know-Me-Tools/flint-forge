# p16-c007 Tasks — 500-Line File-Size Compliance

## Tasks

- [x] Re-measure all `crates/**/*.rs` line counts after p16-c001–c004 land (list above is pre-remediation) — re-measured 2026-07-13 via `find crates -name "*.rs" | xargs wc -l | sort -rn`; several files grew due to intervening p16 work. Updated list below (new/changed sizes noted; two files newly crossed the 500-line threshold and are added).
- [x] Split `crates/fdb-gateway/src/routes/htmx/renderers.rs` (1267, unchanged) into a directory module
- [x] Split `crates/fdb-gateway/src/main.rs` (990 → now 1062 — grew from p16-c003/c004/c006 work) — extract composition sub-steps into `bootstrap/` or similar
- [x] Split `crates/fdb-gateway/src/routes/a2ui.rs` (802, unchanged)
- [x] Split `crates/fdb-realtime/src/listen.rs` (769, unchanged) — split into `listen/{mod,config,error,source,watch,listen_loop,payload,validate,tests}.rs`, all under 500 lines; cargo check/clippy/test clean; verified public API preserved via a scratch crate against fdb-gateway's exact import usage (fdb-gateway itself was transiently unbuildable from an unrelated concurrent split)
- [x] Split `crates/fdb-gateway/src/routes/mcp.rs` (732, unchanged)
- [x] Split `crates/fke-runtime/src/lib.rs` (674 → now 820 — grew from p16-c003 capability-check changes) — coordinate with p16-c003 changes to this file (p16-c003 is now archived/done)
- [x] Split `crates/fdb-reflection/src/compilers/mcp.rs` (668, unchanged)
- [x] Split `crates/fdb-gateway/src/routes/htmx/mod.rs` (636, unchanged)
- [x] Split `crates/fdb-query/src/operator.rs` (632, unchanged)
- [x] Split `crates/fdb-reflection/src/compilers/rest/mod.rs` (605 → now 743 — grew from p16-c001 RLS wiring + p16-c006 doc fixes) — coordinate with p16-c001 changes to this file (p16-c001 is now archived/done)
- [x] Split `crates/fdb-gateway/src/routes/a2a.rs` (552, unchanged)
- [x] Split `crates/fdb-app/src/a2ui/design_md_parser.rs` (536, unchanged)
- [x] Split `crates/fdb-query/src/plan.rs` (532, unchanged)
- [x] Split `crates/forge-cli/src/main.rs` (528 → now 579)
- [ ] Split `crates/ext-flint-vault/src/lib.rs` (513, unchanged) — pgrx crate, verify via `cargo pgrx` not `cargo check` — DEFERRED: `cargo pgrx` requires `$PGRX_HOME` (set up via `cargo pgrx init`, which needs network access to build a local Postgres) — confirmed genuinely unavailable in this environment (tried `cargo check` directly in the crate dir too: fails at pgrx-pg-sys's build script with "$PGRX_HOME does not exist"), matching the same pre-existing toolchain gap as `cargo-component` for WASM builds elsewhere in this phase. This is the most security-critical crate in the repo (envelope-encrypted secret store, KMS-wrapped DEK) — splitting it with zero ability to verify correctness (not even a bare compile check) was judged too high a risk to attempt blind. `pgrx::pg_module_magic!()` must also stay in the crate-root `lib.rs` specifically (a real pgrx constraint), adding further risk to an unverified split. Left as open debt for whoever next has pgrx tooling available.
- [x] Split `crates/fdb-reflection/src/compilers/a2ui.rs` (510, unchanged)
- [x] Split `crates/fdb-gateway/src/routes/agui.rs` (508, unchanged)
- [x] Split `crates/fke-server/src/main.rs` (557) — p16-c006 reconcile: newly over 500 lines (auth middleware from p16-c003), not in the original list. Added.
- [x] Split `crates/fdb-postgres/src/lib.rs` (541) — newly over 500 lines, not in the original list. Added.
- [x] Re-run the 500-line grep/wc check workspace-wide; confirm zero violations — confirmed 2026-07-14: only `crates/ext-flint-vault/src/lib.rs` (513) remains, which is the deliberately-deferred pgrx crate above (documented, not an oversight)
- [x] `cargo check --workspace` clean after each file (compile economy) — final full-workspace `cargo check --workspace` clean (also had to fix one real, non-transient bug along the way: a `routes/htmx/renderers/` split left several `render_*` functions without correct cross-module visibility, breaking the whole crate — resolved by substituting a verified-clean split produced in an isolated worktree)
- [x] `cargo clippy --workspace --all-targets -- -D warnings` clean — confirmed clean, zero warnings, full workspace
- [x] `cargo test --workspace` passes (no behavior change) — confirmed: 76/76 `test result: ok` summaries, zero failures

<!-- p16-c007 completion note (2026-07-14): 17 of 18 originally-identified over-500-line files split into directory modules as pure mechanical refactors (zero behavior change, verified via cargo check/clippy/test per crate plus a final full-workspace pass). This was executed via ~15 parallel subagents (one per file/crate) since the splits are largely independent — Rust resolves `mod foo;` to either `foo.rs` or `foo/mod.rs` automatically, so most splits required zero changes to parent files, eliminating most cross-agent conflicts by construction. One real conflict did occur: two independent concurrent sessions both split `routes/htmx/renderers.rs`, and the version that landed in the main working tree had a genuine visibility bug (several `render_*` functions not marked accessible to the dispatch table, breaking the whole `fdb-gateway` crate) — resolved by discovering a second, fully-verified split of the same file sitting in an isolated agent worktree and substituting it in. `crates/ext-flint-vault/src/lib.rs` (pgrx crate) was deliberately NOT split: `cargo-pgrx` requires `$PGRX_HOME` (via `cargo pgrx init`, needing network access to build a local Postgres), confirmed genuinely unavailable in this environment — this is the most security-critical crate in the repo (envelope-encrypted secret store, KMS-wrapped DEK) and splitting it with zero ability to verify correctness, not even a bare compile check, was judged too high a risk. Left as tracked open debt for whoever next has pgrx tooling available.
