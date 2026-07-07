# Current Waypoint — Flint Forge

## Active Phase
**p5-a2ui-registry** — Flint A2UI Component Registry

## Phase State
- Completed changes: p5-c001, p5-c002, p5-c003, p5-c007, p5-c009, p5-c010, p5-c011, p5-c014
- Next change to implement: **p5-c008-protocol-surfaces**

## Immediate Next Action
1. Review `openspec/changes/p5-c008-protocol-surfaces/proposal.md` and `tasks.md`.
2. Implement protocol surface generation and message dispatch.
3. Run `cargo clippy -p fdb-gateway -- -D warnings` after implementation.

## Why A2UI before Kiln
OpenSpec changes already use p5-* IDs for A2UI; the revised phase plan (RFC-FORGE-PHASES-002) validates this ordering. Kiln runtime moves to Phase 6.

## Recorded Decisions
- Wiki: `flint-forge/phase-numbering-a2ui-before-kiln`
- Memory: `egn27z6kazo7nl7wr7fl`
