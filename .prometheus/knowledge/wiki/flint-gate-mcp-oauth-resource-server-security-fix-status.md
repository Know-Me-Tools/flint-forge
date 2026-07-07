---
type: Reference
id: flint-gate-mcp-oauth-resource-server-security-fix-status
title: flint-gate MCP OAuth Resource Server Security Fix Status
tags:
- flint-gate
- agent-gateway
- mcp
- oauth-resource-server
- jwks
- security-hardening
- phase-tracking
links:
- flint-gate-agent-authorization-control-plane-phase-status
sources:
- stdin
- manual:flint-gate/agent-authz-control-plane
- .kbd-orchestrator/evolution-briefs/ai-agent-gateway-parity.md
timestamp: 2026-07-03T16:54:15.060474+00:00
created_at: 2026-07-03T16:54:15.060474+00:00
updated_at: 2026-07-03T16:54:15.060474+00:00
revision: 0
---

## Phase Context

- Project: `flint-gate`
- Phase: `agent-authz-control-plane`
- Status: `execution_ready`
- Progress: `changes 1/8`
- Current change: **Change 2 — MCP OAuth 2.1 resource-server support**
- Task progress: `7/9` tasks complete for change 2
- Captured at: `2026-07-03T16:32:31Z`
- KBD root: `/Users/gqadonis/Projects/prometheus/flint-gate`
- Seed brief: `.kbd-orchestrator/evolution-briefs/ai-agent-gateway-parity.md`
- Criteria profile: `effort-impact`

This updates the broader [flint-gate Agent Authorization Control Plane Phase Status](/flint-gate-agent-authorization-control-plane-phase-status.md): change 2 is in security-hardening validation, not yet ready to archive.

## Phase Objective

Turn `flint-gate` from an auth proxy into an MCP-era agent gateway by adding an agent-authorization control plane on top of existing streaming enforcement:

- Mid-stream SSE token metering
- Session watchdog
- AG-UI/A2UI processing

The phase remains **authorization-first** and deliberately excludes off-identity LLM-ops capabilities:

- Semantic caching
- Multi-LLM routing
- Multimodal processing

## Planned Build Order

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
   - Validate RFC 8707 `resource` / audience values.
   - Return `403 insufficient_scope` for step-up authorization.
   - Prevent token passthrough to upstreams to avoid confused-deputy behavior.
   - Gap: `G1`; critical credibility gate.

3. **Embedded policy engine + per-tool-call authorization**
   - Evaluate an embedded native-Rust policy engine: Cedar core or `casbin-rs`; no sidecar.
   - Authorize inline in the stream.
   - Authorize each MCP tool call by:
     - Tool name
     - Parameters
     - Identity claims
   - Filter unauthorized tools from `list_tools` responses, following the agentgateway pattern.
   - Add new `PreRequestHook::Authorize`.
   - Add stream-level tool-call gate.
   - Gap: `G2`; strategic core.

## Change 2 Security Hardening Status

All 6 reported security findings are addressed in code:

- `C1`
- `H1`
- `H2`
- `M1`
- `M2`
- `M3`

Independent verification confirmed that the `M1` algorithm allowlist is now checked **before** key resolution. This is the correct fail-closed ordering because disallowed algorithms are rejected before any key-selection path can accept or infer a key.

## Independent Test Findings

A manual re-run caught **3 failing tests** that were missed by the executor report.

Two failures are serious availability regressions:

- The `H2` key selector, `select_asymmetric_key`, is wrongly rejecting valid RSA JWKs.
- If shipped, this would break authentication for legitimate tokens backed by valid RSA keys.

The security fixes are directionally correct, but introduced JWKS key-selector defects that would make the OAuth resource-server implementation reject valid production tokens.

## Required Next Steps

When the agent returns with fixes:

1. Re-run:

   ```bash
   cargo test --workspace
   ```

2. Confirm `0` test failures.
3. Re-verify security properties, especially:
   - Algorithm allowlist is enforced before key resolution.
   - Valid RSA JWKs are accepted by `select_asymmetric_key`.
   - Invalid or disallowed key/algorithm combinations fail closed.
4. Close tasks 8–9 for change 2.
5. Run the QA gate.
6. Add delta spec.
7. Run strict OpenSpec validation:

   ```bash
   openspec validate --strict
   ```

8. Archive change 2.
9. Commit.

## Remaining Phase Work

- Finish change 2.
- Complete changes 3–8.
- Reflect after phase execution.

# Citations

1. stdin
2. manual:flint-gate/agent-authz-control-plane
3. .kbd-orchestrator/evolution-briefs/ai-agent-gateway-parity.md