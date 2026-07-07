---
type: Reference
id: flint-gate-per-tool-authorization-completed-at-4-8-progress
title: flint-gate Per-Tool Authorization Completed at 4/8 Progress
tags:
- flint-gate
- agent-gateway
- tool-authorization
- cedar-policy
- streaming-enforcement
- security-hardening
- phase-tracking
links:
- flint-gate-agent-authorization-control-plane-phase-status
- flint-gate-mcp-resource-server-change-settled-at-2-8-progress
- flint-gate-embedded-cedar-policy-engine-security-review-status
sources:
- stdin
timestamp: 2026-07-03T22:10:51.620464+00:00
created_at: 2026-07-03T22:10:51.620464+00:00
updated_at: 2026-07-03T22:10:51.620464+00:00
revision: 0
---

## Phase Context

- Project: `flint-gate`
- Phase: `agent-authz-control-plane`
- Status: `execution_ready`
- Progress: `changes 4/8`
- Captured at: `2026-07-03T22:02:12Z`
- KBD root: `/Users/gqadonis/Projects/prometheus/flint-gate`
- Seed brief: `.kbd-orchestrator/evolution-briefs/ai-agent-gateway-parity.md`
- Criteria profile: `effort-impact`
- Completed change: `add-per-tool-authz` / `G2b`
- Commit: `2771cc6`
- Git path: `e4b871b` → `708191f` → `b8f9957` → `f36ce20` → `2771cc6`

This updates the broader [flint-gate Agent Authorization Control Plane Phase Status](/flint-gate-agent-authorization-control-plane-phase-status.md). It follows the settled MCP resource-server work in [flint-gate MCP Resource Server Change Settled at 2/8 Progress](/flint-gate-mcp-resource-server-change-settled-at-2-8-progress.md) and completes the per-tool authorization work that was previously under security review in [flint-gate Embedded Cedar Policy Engine Security Review Status](/flint-gate-embedded-cedar-policy-engine-security-review-status.md).

## Phase Objective

Turn `flint-gate` from an auth proxy into an MCP-era agent gateway by adding an agent-authorization control plane on top of existing streaming enforcement:

- Mid-stream SSE token metering
- Session watchdog
- AG-UI/A2UI processing

The phase remains authorization-first and deliberately excludes off-identity LLM-ops capabilities:

- Semantic caching
- Multi-LLM routing
- Multimodal processing

## Planned Build Order

1. **Budget enforcement + windowed rate limiting**
   - Extend existing `usage_events` and lifetime `MaxTokenBudget` behavior.
   - Add per-key/per-team rolling-window token budgets for minute/hour/day windows.
   - Add request-rate limits.
   - Block threshold violations with clear errors.
   - Gap: `G3`; fastest win and highest feasibility.

2. **MCP OAuth 2.1 resource-server support**
   - Add RFC 9728 protected-resource metadata.
   - Return `WWW-Authenticate: resource_metadata` on `401`.
   - Support RFC 8414/OIDC authorization-server discovery.
   - Verify PKCE S256.
   - Validate RFC 8707 `resource`/audience.
   - Return `403 insufficient_scope` for step-up flows.
   - Prevent token passthrough to upstreams to avoid confused-deputy behavior.
   - Gap: `G1`; critical credibility gate.

3. **Embedded policy engine + per-tool-call authorization**
   - Evaluate and embed a native Rust policy engine, Cedar core or `casbin-rs`, with no sidecar.
   - Evaluate authorization inline in the stream.
   - Authorize each MCP tool call by tool name, parameters, and identity claims.
   - Filter unauthorized tools out of `list_tools` responses.
   - Add `PreRequestHook::Authorize` and a stream-level tool-call gate.
   - Gap: `G2`; strategic core.

## Completed Change: `add-per-tool-authz`

`add-per-tool-authz` completed all `6/6` tasks, was verified, archived, and promoted as a delta spec to `openspec/specs/tool-authorization`.

### Shipped Behavior

- Added per-tool-call authorization on the streaming path.
- Tool calls are authorized against the Cedar engine using complete arguments **before any part of the call is forwarded**.
- Implemented inspect-then-forward semantics:
  - Buffer `START` and `ARGS` by `toolCallId`.
  - Authorize at `TOOL_CALL_END` after complete arguments are available.
  - Flush buffered events only on allow.
  - On deny, emit `RUN_ERROR` and forward **zero arguments**.
- Added `tools/list` visibility filtering.
- Non-tool events continue to stream live.
- Non-authorization routes are unaffected.

## Security Review Findings and Fixes

The initial draft was rejected as security-cosmetic because it streamed tool arguments before authorization and blocked too late. The final implementation corrected that design.

### Rejected Draft Behavior

- Used forward-then-annul semantics.
- Streamed arguments live before the authorization decision.
- Blocked only after sensitive data could already have reached the client.
- Had a critical DoS risk.
- Had a fail-open `tools/list` filter.

### Verified Corrected Behavior

- AG-UI research found clients execute tools only after `TOOL_CALL_END`.
- Industry pattern selected: inspect-then-forward.
- Final design buffers until authorization and only then forwards.
- Denied calls produce `RUN_ERROR` with no forwarded arguments.

### Fixed Issues

- `C1`: Added byte caps to prevent DoS from unbounded buffering.
- `H2`: Changed `tools/list` filtering to fail closed.
- `L2`: Denies unparseable tool-call arguments.

## Verification

- Ran 11 security-critical tests individually.
- Independent build caught a transient mid-edit break that would have been hidden by the agent report.
- Final verification after waiting for the edit to settle:
  - `256` tests pass.
  - `clippy` clean for both feature sets.
  - No production `unwrap`/`expect` remain.

## Engineering Significance

This change converts per-tool authorization from audit-only behavior into a true pre-delivery control. It preserves `flint-gate`'s streaming enforcement moat by enforcing authorization mid-stream without leaking unauthorized tool arguments.

## Next Change

Next planned work: `/kbd-apply add-authz-audit-trail`.

Scope:

- Add `authz_audit` table.
- Add decision write path.
- Add Admin read endpoint.
- Severity: medium.
- Smaller scope than `add-per-tool-authz`.
- No security-review gate required.

Remaining phase progress: `4` changes left, then reflect.

# Citations

1. stdin