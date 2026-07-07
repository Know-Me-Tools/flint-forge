# p8-c005 Tasks — Design Token Export

## Tasks

- [ ] Add `GET /a2ui/v1/design-systems/:id/tokens` handler in `crates/fdb-gateway/src/routes/a2ui.rs`
- [ ] Wire the route in `fdb-gateway/src/main.rs` behind `require_rls`
- [ ] Create `packages/flint-react/src/tokens/exportDesignSyncTokens.ts`
- [ ] Export `exportDesignSyncTokens` from `packages/flint-react/src/index.ts`
- [ ] Unit test (Rust): missing design system ID returns 404
- [ ] Unit test (TypeScript): `exportDesignSyncTokens` rejects on non-200 response
- [ ] `cargo clippy -p fdb-gateway -- -D warnings` clean
- [ ] `npm run typecheck` in `packages/flint-react/` passes
