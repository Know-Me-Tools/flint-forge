---
type: Reference
id: flint-gate-mcp-resource-server-change-settled-at-2-8-progress
title: flint-gate MCP Resource Server Change Settled at 2/8 Progress
tags:
- flint-gate
- agent-gateway
- mcp
- oauth-resource-server
- jwt-validation
- security-hardening
- phase-tracking
links:
- flint-gate-agent-authorization-control-plane-phase-status
- flint-gate-mcp-oauth-resource-server-security-fix-status
sources:
- stdin
- manual:flint-gate/agent-authz-control-plane
timestamp: 2026-07-03T17:47:52.787213+00:00
created_at: 2026-07-03T17:47:52.787213+00:00
updated_at: 2026-07-03T17:47:52.787213+00:00
revision: 0
---

## Phase Context

- Project: `flint-gate`
- Phase: `agent-authz-control-plane`
- Status: `execution_ready`
- Progress: `changes 2/8`
- Captured at: `2026-07-03T17:44:14Z`
- KBD root: `/Users/gqadonis/Projects/prometheus/flint-gate`
- Seed brief: `.kbd-orchestrator/evolution-briefs/ai-agent-gateway-parity.md`
- Criteria profile: `effort-impact`
- Current branch: `feat/agent-authz-budget-rate-limiting`

This updates the broader [flint-gate Agent Authorization Control Plane Phase Status](/flint-gate-agent-authorization-control-plane-phase-status.md) and supersedes the prior change-2 hardening status in [flint-gate MCP OAuth Resource Server Security Fix Status](/flint-gate-mcp-oauth-resource-server-security-fix-status.md): change 2 is now reconciled, verified, committed, and archived.

## Phase Objective

Turn `flint-gate` from an auth proxy into an MCP-era agent gateway by adding an agent-authorization control plane on top of existing streaming enforcement:

- Mid-stream SSE token metering
- Session watchdog
- AG-UI/A2UI processing

The phase remains authorization-first and explicitly excludes off-identity LLM-ops work:

- Semantic caching
- Multi-LLM routing
- Multimodal processing

## Build Order

1. **Budget enforcement + windowed rate limiting**
   - Extend existing `usage_events` and lifetime `MaxTokenBudget` hook.
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
   - Return `403 insufficient_scope` for step-up authorization.
   - Prevent token passthrough to upstreams to avoid confused-deputy behavior.
   - Gap: `G1`; critical credibility gate.

3. **Embedded policy engine + per-tool-call authorization**
   - Evaluate an embedded native-Rust policy engine: Cedar core or `casbin-rs`; no sidecar.
   - Authorize inline in the stream.
   - Authorize each MCP tool call by tool name, parameters, and identity claims.
   - Filter unauthorized tools out of `list_tools` responses, following the agentgateway pattern.
   - Add `PreRequestHook::Authorize` and a stream-level tool-call gate.
   - Gap: `G2`; critical strategic core.

## Reconciled Change 2 Follow-up

Commit `b8f9957` reconciled late background-agent edits with the already-committed MCP resource-server work. The agent independently found the same root causes for the three test failures:

- Bad fixture modulus.
- Bracketed-IPv6 SSRF validation gap.

The reconciled follow-up added these improvements:

- **`mcp.rs` correctness fix**
  - `jsonwebtoken@9` raises `InvalidAlgorithm` when `validation.algorithms` spans mixed key families such as RSA and EC.
  - Validation is now pinned to the JWT header algorithm.
  - The M1 security gate remains intact: `ALLOWED_ALGS.contains` is checked before key resolution.

- **`jwks.rs` cleanup**
  - Removed duplicate `#[test]` attributes left behind by probe injection.

- **`kratos.rs` and `processor.rs` lint cleanup**
  - Fixed test-only clippy issues.
  - `clippy --tests` is green on both feature sets.

## MCP E2E Coverage

`mcp_e2e.rs` covers all required resource-server handshake cases:

1. Valid token authorizes.
2. Wrong audience is rejected per RFC 8707.
3. Missing scope returns `insufficient_scope`.
4. RFC 9728 metadata is exposed.
5. Tampered signature is rejected.

## Validation State

Verified after reconciliation:

- `191` tests pass.
- `clippy --tests` clean on both feature sets.
- Formatting clean.
- Security properties present and preserved.

## Git State

On branch `feat/agent-authz-budget-rate-limiting`:

- `e4b871b` — change 1: budget enforcement + rate limiting.
- `708191f` — change 2: MCP OAuth 2.1 resource-server support.
- `b8f9957` — change 2 follow-up: algorithm-family fix + lint cleanup.

## Next Action

Next command: `/kbd-apply add-policy-engine`.

Change 3 of 8 is the strategic core:

- Embedded Cedar engine.
- `ArcSwap` hot reload.
- `authz_policies` table.
- Write-time policy validation.
- Security-sensitive implementation requiring a `security-reviewer` pass.

Remaining in phase: `6` changes. Continue the delegate/verify/security-review cadence and independently rerun the test suite rather than relying on agent reports, since independent verification caught real issues on both authorization changes.

# Citations

1. stdin
2. manual:flint-gate/agent-authz-control-plane