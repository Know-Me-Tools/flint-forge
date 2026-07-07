---
type: Reference
id: flint-gate-agent-authorization-control-plane-phase-status
title: flint-gate Agent Authorization Control Plane Phase Status
tags:
- flint-gate
- agent-gateway
- mcp
- rate-limiting
- budget-enforcement
- oauth-resource-server
- phase-tracking
sources:
- stdin
- manual:flint-gate/agent-authz-control-plane
timestamp: 2026-07-03T15:03:22.796974+00:00
created_at: 2026-07-03T15:03:22.796974+00:00
updated_at: 2026-07-03T15:03:22.796974+00:00
revision: 0
---

## Phase Context

- Project: `flint-gate`
- Phase: `agent-authz-control-plane`
- Status: `execution_ready`
- Progress: `changes 1/8`
- Captured at: `2026-07-03T15:00:01Z`
- KBD root: `/Users/gqadonis/Projects/prometheus/flint-gate`
- Seed brief: `.kbd-orchestrator/evolution-briefs/ai-agent-gateway-parity.md`
- Criteria profile: `effort-impact`

## Phase Objective

Turn `flint-gate` from an auth proxy into an MCP-era agent gateway by adding an agent-authorization control plane on top of existing streaming enforcement:

- Mid-stream SSE token metering
- Session watchdog
- AG-UI/A2UI processing

The phase is authorization-first and explicitly excludes off-identity LLM-ops capabilities such as semantic caching, multi-LLM routing, and multimodal processing.

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
   - Return `403 insufficient_scope` for step-up authorization.
   - Prevent confused-deputy behavior by avoiding token passthrough to upstreams.
   - Gap: `G1`; critical credibility gate.

3. **Embedded policy engine + per-tool-call authorization**
   - Evaluate an embedded native-Rust policy engine, likely Cedar core or `casbin-rs`; no sidecar.
   - Authorize each MCP tool call inline in the stream by:
     - tool name
     - parameters
     - identity claims
   - Filter unauthorized tools from `list_tools` responses using the agentgateway pattern.
   - Add `PreRequestHook::Authorize`.
   - Add stream-level tool-call gate.
   - Gap: `G2`; critical strategic core.

## Completed Change: `add-budget-rate-limiting`

Change 1 of 8 is complete, QA-passed, verified, and archived.

- Change: `add-budget-rate-limiting`
- Gap: `G3`
- Tasks: `8/8` complete
- Archive path: `archive/2026-07-03-add-budget-rate-limiting`
- Branch: `feat/agent-authz-budget-rate-limiting`
- Commit status: not committed at capture time

### Shipped Implementation

- Added in-process request-rate limiting with:
  - `governor`
  - `tower_governor`
  - credential key extractor
  - IP fallback
- Added budget/rate-limit config types:
  - `BudgetWindow`
  - `BudgetScope`
  - minute/hour/day windows
  - user/team scopes
  - backward-compatible serde defaults
- Added `ratelimit/` module with:
  - Redis Lua window counters
  - reuse of existing Redis connection manager; no new Redis dependency
  - shared cross-replica enforcement when `redis-l2` is enabled
  - Postgres `usage_events` windowed-sum fallback when `redis-l2` is disabled
  - fail-open behavior on backend errors
- Added pipeline enforcement that blocks quota breaches with HTTP `429 quota_exceeded`.

### Verification

All verification was independently confirmed:

- `cargo test --workspace`
  - `137 passed`
  - `0 failed`
  - `3 ignored`
  - Ignored tests are live Redis/Postgres tests and are correctly gated.
- `cargo clippy --workspace` is clean under:
  - `--all-features`
  - `--no-default-features`
- QA gate passed all 5 blocking constraints:
  - no secrets
  - admin port untouched
  - no broken tests
  - config precedence intact
  - no production `unwrap`
- QA result logged to `.refiner/`.
- `openspec validate --strict` passes.
- Added delta spec: `specs/rate-limiting/spec.md`.
- Archiving promoted the delta into main capability spec: `openspec/specs/rate-limiting`.

## Follow-Up Notes

- Archived proposals warn on missing `## Why` and `## What Changes` sections. This is non-blocking, but remaining proposals should include those sections.
- Windowed budgets accumulate only on the streaming path, matching existing lifetime-budget behavior; this was flagged for reflection.
- Next planned change: `/kbd-apply add-mcp-resource-server`.
  - This is change 2 of 8.
  - It is security-sensitive and should receive a `security-reviewer` pass before archive.
- Open operational decision at capture time:
  - whether to commit change 1 to the feature branch before continuing;
  - whether to keep the same delegate-to-rust-subagents cadence for remaining changes.

# Citations

1. stdin
2. manual:flint-gate/agent-authz-control-plane