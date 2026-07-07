# Plan — p8-sdk-completeness

**Phase:** p8-sdk-completeness
**Planned:** 2026-07-04
**Change backend:** OpenSpec (`openspec/changes/`)
**Assessment:** `.kbd-orchestrator/phases/p8-sdk-completeness/assessment.md`

---

## Ordering rationale

G3 (CI), G5 (token export), and G6 (ZIP import) are the smallest changes
and can run first in parallel — they unblock nothing but deliver immediate
value. G1 (`@flint/react`) and G2 (Flutter reconnect) are medium effort and
independent. G4 (HTMX 48 renderers) is the largest and should be
parallelised across 3 subagents by category. G7 goes last (depends on G1
export accuracy and a live DB).

```
Session 1 (parallel):   c003 (CI)          c005 (token export)   c006 (ZIP import)
Session 2 (parallel):   c001 (@flint/react) c002 (Flutter reconnect)
Session 3 (parallel):   c004a (input+action) c004b (nav+feedback)  c004c (data-display+layout)
Session 4:              c007 (skill gate tests)
```

---

## Change list

| # | Change ID | Title | Priority | Domain | Effort |
|---|---|---|---|---|---|
| 1 | **p8-c003-ci-pipeline** | GitHub Actions + Docker images | P0 | DevOps | Medium |
| 2 | **p8-c005-design-token-export** | `exportDesignSyncTokens()` + REST endpoint | P1 | Rust + TypeScript | Small-Med |
| 3 | **p8-c006-opendesign-zip-import** | `claude_design_zip` format via `zip` crate | P1 | Rust | Small |
| 4 | **p8-c001-react-sdk-completeness** | Slug map, `useFlintRegistry()`, bundle audit | P0 | TypeScript | Medium |
| 5 | **p8-c002-flutter-sdk-reconnect** | SSE reconnect + `FlintCatalog.refresh()` | P0 | Dart | Medium |
| 6 | **p8-c004-htmx-remaining-components** | 48 DaisyUI renderers + module split | P1 | Rust | High |
| 7 | **p8-c007-claude-skill-gate-tests** | Slug accuracy integration test + docs | P2 | Rust + docs | Low |

---

## Constraint notes (from AGENTS.md)

- `#![forbid(unsafe_code)]` in all Rust crates
- Files under 500 lines — `htmx.rs` **must** be split before adding renderers (c004)
- New workspace deps go in `[workspace.dependencies]` first
- No `unwrap()`/`expect()` in library code
- TypeScript: strict mode, no `any`

---

## New workspace/package deps required

| Dep | Version | Used by | OQ |
|---|---|---|---|
| `zip` crate | `"2"` | c006 | Check `cargo search zip` — confirm v2 API |
| `cargo-chef` image | latest | c003 Dockerfiles | Used in multi-stage build, not a Cargo dep |
| `dtolnay/rust-toolchain` action | `@stable` | c003 CI | GitHub Actions |

All TypeScript deps for `@flint/react` are already in `package.json`.

---

## Phase gate

- [ ] React SDK exports all 55 slugs, bundle < 80 KB
- [ ] Flutter SSE reconnects with exponential backoff
- [ ] CI workflow passes on a test PR
- [ ] Docker images build locally
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` clean

---

## Recommended first action

```
/kbd-build p8-c003-ci-pipeline and p8-c005-design-token-export and p8-c006-opendesign-zip-import concurrently
```

Start with the three small/medium independent changes in parallel. They touch
different parts of the codebase (DevOps, Rust REST, Rust routes) and have no
shared state. Completing them first creates momentum and closes the easiest
gaps before the larger G1/G2/G4 work.
