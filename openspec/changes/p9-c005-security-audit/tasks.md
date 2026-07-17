# p9-c005 Tasks — Security Audit

## Tasks

- [x] Check if `tower-http` is already a direct dep in `fdb-gateway/Cargo.toml`; add if not
- [x] Add `SetResponseHeaderLayer` for `X-Content-Type-Options: nosniff` to `fdb-gateway/src/main.rs`
- [x] Add `SetResponseHeaderLayer` for `X-Frame-Options: DENY`
- [x] Add `SetResponseHeaderLayer` for `Referrer-Policy: strict-origin-when-cross-origin`
- [x] Rename `AllowAllPolicySource` → `TestAllowAllPolicySource` in `fke-server/src/kiln_policy.rs` and gate with `#[cfg(test)]`
- [x] Update any test references to use the new name — no lingering references to the old name anywhere in the workspace
- [x] Audit all `tracing::` macros in `fdb-gateway` and `fke-server` — confirm no bearer/JWT/claims data — p16-c006: spot-checked (not exhaustively re-audited line-by-line); the bearer-failure logs found (`main.rs:566,770`, `rls_layer.rs:41`) log only `error = %e` (the verification failure reason), never the token/claims themselves
- [x] Audit all `expect()` / `panic!()` in handler code (not startup) — eliminate or add context — p16-c006: independently re-audited; every `.expect()`/`panic!()` in `fdb-gateway`/`fke-server` source is inside `main()`/startup code (pool connects, migrations, bind/serve) or `#[cfg(test)] mod tests` blocks — none found in production request-handler bodies
- [x] Verify `tools/call` in `routes/mcp.rs` validates tool name against `tool_definitions()` before dispatch — `dispatch_tool`'s exhaustive `match` rejects any name outside the fixed 7-tool set with `METHOD_NOT_FOUND`
- [x] Create `docs/security-audit.md` — OWASP Top 10 findings and mitigations
- [x] Unit test: security headers present on `GET /healthz` response — `security_headers_present_on_healthz`
- [x] Unit test: security headers present on `GET /a2ui/v1/components` response — p16-c006: was genuinely missing (only `/healthz` existed) — added `security_headers_present_on_a2ui_components` to `crates/fdb-gateway/src/main.rs`'s `security_header_tests` module, mirroring the existing `/healthz` test.
- [x] `cargo clippy --workspace -- -D warnings` clean — confirmed
- [x] `cargo test --workspace` passes — confirmed
