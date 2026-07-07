---
type: Reference
id: flint-gate-embedded-cedar-policy-engine-security-review-status
title: flint-gate Embedded Cedar Policy Engine Security Review Status
tags:
- flint-gate
- agent-gateway
- cedar-policy
- authorization
- policy-engine
- security-hardening
- phase-tracking
links:
- flint-gate-agent-authorization-control-plane-phase-status
- flint-gate-mcp-resource-server-change-settled-at-2-8-progress
- flint-gate-mcp-oauth-resource-server-security-fix-status
sources:
- stdin
- manual:flint-gate/agent-authz-control-plane
timestamp: 2026-07-03T18:44:45.317911+00:00
created_at: 2026-07-03T18:44:45.317911+00:00
updated_at: 2026-07-03T18:44:45.317911+00:00
revision: 0
---

## Phase Context

- Project: `flint-gate`
- Phase: `agent-authz-control-plane`
- Status: `execution_ready`
- Progress: `changes 2/8`; change 3 is `6/8` tasks complete with security fixes in flight
- Captured at: `2026-07-03T18:38:39Z`
- KBD root: `/Users/gqadonis/Projects/prometheus/flint-gate`
- Seed brief: `.kbd-orchestrator/evolution-briefs/ai-agent-gateway-parity.md`
- Criteria profile: `effort-impact`

This updates the broader [flint-gate Agent Authorization Control Plane Phase Status](/flint-gate-agent-authorization-control-plane-phase-status.md) after change 2 was settled in [flint-gate MCP Resource Server Change Settled at 2/8 Progress](/flint-gate-mcp-resource-server-change-settled-at-2-8-progress.md). Change 3, embedded policy engine and per-tool-call authorization, is partially implemented but remains blocked on security fixes.

## Phase Objective

Turn `flint-gate` from an auth proxy into an MCP-era agent gateway by adding an agent-authorization control plane on top of existing streaming enforcement:

- Mid-stream SSE token metering
- Session watchdog
- AG-UI/A2UI processing

The phase remains authorization-first and explicitly excludes off-identity LLM-ops work:

- Semantic caching
- Multi-LLM routing
- Multimodal processing

## Impact-Weighted Build Order

1. **Budget enforcement + windowed rate limiting**
   - Extend existing `usage_events` and lifetime `MaxTokenBudget` hook.
   - Add per-key/per-team rolling-window token budgets for minute/hour/day windows.
   - Add request-rate limits.
   - Block threshold violations with clear errors.
   - Gap: `G3`; fastest win and highest feasibility.
2. **MCP OAuth 2.1 resource-server support**
   - Implement RFC 9728 protected-resource metadata.
   - Return `WWW-Authenticate: resource_metadata` on `401`.
   - Support RFC 8414/OIDC authorization-server discovery.
   - Verify PKCE S256.
   - Validate RFC 8707 `resource`/audience.
   - Return `403 insufficient_scope` for step-up flows.
   - Prevent token passthrough to upstreams to avoid confused-deputy risks.
   - Gap: `G1`; critical credibility gate.
3. **Embedded policy engine + per-tool-call authorization**
   - Evaluate embedded native-Rust policy engine: Cedar core or `casbin-rs`; no sidecar.
   - Authorize inline in the stream.
   - Authorize each MCP tool call by tool name, parameters, and identity claims.
   - Filter unauthorized tools out of `list_tools` responses, matching the agentgateway pattern.
   - Add `PreRequestHook::Authorize` and stream-level tool-call gate.
   - Gap: `G2`; critical strategic core.

## Change 3 Implementation Status

Change 3, `add-policy-engine`, is 6/8 tasks complete.

Implemented by the delegated Rust agent:

- Added dependencies:
  - `cedar-policy v4.10.0`
  - `arc-swap v1.9.1`
- Added embedded authz module:
  - `authz/`
  - `CedarBundle` stored behind `ArcSwap`
  - Hot-reload with parse-before-swap semantics
- Added persistence:
  - `authz_policies` table
  - Admin CRUD for policy rows
  - Write-time `Validator`
- Added request enforcement:
  - `PreRequestHook::Authorize`
  - Default `enforce=true`
  - Shadow-mode support
  - `403` block on denied authorization
- Validation before security fixes:
  - `215` tests passing
  - Clippy clean across both feature sets

## Verified Authorization Invariants

The fail-closed invariant was independently inspected and confirmed in-process:

- `authorize` returns `Allow` only for explicit Cedar `Decision::Allow`.
- Every evaluation error returns `Deny`.
- Empty policy set defaults to deny.
- Hot-reload retains the last-good bundle when parsing a new bundle fails.

## Security Review Findings

Security review confirmed the in-process fail-closed behavior, but found one critical issue, three high-severity issues, and additional guardrail work.

### Critical

- **C1: Multi-replica hot-reload gap**
  - `pg_notify('policies')` is emitted.
  - The listener never reloads the Cedar engine.
  - Result: peer replicas can continue serving stale policy after policy changes.

### High

- **H1: `entities_json` unvalidated at write**
  - A poisoned entity blob can disable the Cedar bundle.
- **H2: Startup all-or-nothing**
  - One bad policy row can cause deny-all behavior for the whole bundle at startup.
- **H3: Admin default bind is `0.0.0.0`**
  - Violates the project blocking constraint: never expose admin to the internet.

## Current Work in Flight

Five fixes plus expanded tests have been dispatched to the Rust agent:

- Wire `pg_notify('policies')` to actual engine reload for all replicas.
- Validate `entities_json` at write time.
- Avoid startup all-or-nothing behavior from one bad row.
- Change or guard the admin bind default so admin is not exposed externally.
- Expand security regression tests around the above fixes and guardrails.

## Next Steps

After the agent returns:

1. Re-run the full test suite independently.
2. Inspect the C1 NOTIFY-to-reload wiring for multi-replica policy freshness.
3. Inspect the H3 admin bind default and exposure guard.
4. Close tasks 7–8 for change 3.
5. Run QA gate.
6. Produce delta spec.
7. Run strict validation.
8. Archive change 3.
9. Commit.
10. Continue remaining phase work: changes 4–8, then reflect.

## Security Process Note

The security-review gate has caught a real must-fix defect on all three authz-sensitive changes completed or in progress in this phase, including the MCP OAuth resource-server work tracked in [flint-gate MCP OAuth Resource Server Security Fix Status](/flint-gate-mcp-oauth-resource-server-security-fix-status.md). Keep the gate mandatory for subsequent authorization-sensitive changes.

# Citations

1. stdin
2. manual:flint-gate/agent-authz-control-plane