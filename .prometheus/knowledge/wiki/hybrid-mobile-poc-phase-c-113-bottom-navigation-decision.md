---
type: Reference
id: hybrid-mobile-poc-phase-c-113-bottom-navigation-decision
title: Hybrid Mobile PoC Phase C-113 Bottom Navigation Decision
tags:
- hybrid-mobile-architecture
- phase-tracking
- codegen-ci
- navigation-design
- tauri
- flutter
- react
- memory-ui
sources:
- stdin
timestamp: 2026-07-16T21:48:37.778961+00:00
created_at: 2026-07-16T21:48:37.778961+00:00
updated_at: 2026-07-16T21:48:37.778961+00:00
revision: 0
---

## Phase Context

- Project: Hybrid Mobile Architecture
- Phase: `phase-codegen-and-ci-verification`
- Status: `executing`
- Progress: `8/10`
- KBD root: `/Users/gqadonis/Projects/hybrid-mobile-architecture-src`
- Captured: `2026-07-16T21:40:51Z`

## Revised Phase Goal

The phase target was revised on `2026-07-15`: the end result is a working proof-of-concept application in `apps/<name>/`, not only pipeline verification.

The PoC must use the repository scaffolds and skills, be based on KnowMe reference documentation in `docs/reference-app/`, and prove the skill package end-to-end while showcasing the broadest practical supported capability set:

- Streaming `ContentBlock` chat
- PEM entity management
- SurrealDB graph-RAG memory
- Local-first sync
- Cross-platform Flutter, Tauri, and web surfaces from one Rust core
- Feature subset selected using web research on showcase-app best practices and 2026 on-device AI feasibility

## Supporting Objectives

The original codegen/CI objectives remain supporting goals that the PoC should prove in passing:

- Run the real codegen pipeline on the PoC:
  - `flutter_rust_bridge_codegen generate`
  - `dart run build_runner build`
  - full `flutter pub get`
  - full `pnpm install`
- Confirm pre-codegen warnings clear after generated code and sibling packages exist.
- Resolve or work around the PEM install blocker: `@prometheus-ags/entity-graph-core@workspace:*` is unresolvable outside the PEM monorepo.
- Verify the PoC builds and runs on at least one real target per surface:
  - macOS Tauri desktop
  - iOS simulator or Android emulator for Flutter
- Wire CI to run on every push:
  - `cargo clippy --workspace`
  - `audit.sh all`
  - boundary test suites against the PoC

## C-113 Outcome

C-113 is complete, including tasks T1–T6, and has been pushed.

The original proposal premise was rejected. It claimed iOS and Android disagree on top-vs-bottom navigation. Sourced platform research showed convergence instead:

- Apple HIG places tab bars at the bottom for top-level sections.
- Material 3 navigation bars are always placed at the bottom for top-level destinations.
- Android top tabs are a different component for a different purpose. Material 3 distinguishes them as: use navigation for distinct pages and tabs for related content within a page.

## Navigation Decision

Ratified decision:

- Use one convention: bottom navigation.
- Do not perform platform detection for navigation.
- Do not maintain separate iOS and Android navigation trees.
- React should match Flutter responsively.
- Layout variation is based on form factor, not operating system.
- Width breakpoint is `600px`, matching the Material 3 boundary, rather than Tailwind's default `640px`.

Rationale:

- Platform-adaptive navigation would add UA sniffing and duplicate navigation structure without improving conformance.
- The meaningful adaptation is compact vs expanded layout.

## Verification Performed

Verified visually on screen:

- Bottom navigation appears at `375px` width.
- Navigation rail appears at `1024px` width.
- Layout switches correctly between bottom bar and rail.
- `/memory` navigation works.

Not claimed as verified:

- Flutter simulator conformance. Flutter was asserted from reading `router.dart`; it was already correct and unchanged.
- Earlier WebLLM generation. It remains unproven and tracked as open.

## Bugs Found by Making Memory Reachable

Making the Memory screen reachable exposed three real defects:

1. **Desktop memory commands were still stubs**
   - `memory_search` and `graph_expand` returned empty vectors on desktop.
   - C-104 wired the mobile FFI path but left Tauri commands behind.
   - Result: desktop memory search silently produced no results.

2. **React/Rust `MemoryHit` shape drifted**
   - React expected `name`/`snippet`.
   - Rust had moved to `text`/`kind`.
   - An `as unknown as` cast, documented as a known tracked mismatch, kept TypeScript quiet after the mismatch should have been resolved.

3. **`memoryStore` tests mocked the wrong boundary**
   - Zero invocations reached the intended module boundary.
   - Fixing this resolved 2 of the 4 known failing tests.
   - The remaining 2 failing tests belong to a spawned session.

Additional finding:

- `MemoryPanel` was fully built and hook-wired but had never been mounted.
- Because it was unreachable, it had no styling until navigation exposed it.

## Pending Work

Wave 2 still has pending items:

- C-108: MCP + agent
- C-109: settings/model admin; depends on C-105 and C-106
- C-111: memory UI + corpus
- C-106 sync: owned by a concurrent session
- Spawned tasks running independently:
  - `startupStore` tests
  - WebLLM browser check

# Citations

1. stdin