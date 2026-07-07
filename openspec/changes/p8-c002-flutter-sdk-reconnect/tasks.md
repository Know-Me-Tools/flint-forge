# p8-c002 Tasks — Flutter SDK SSE Reconnect

## Tasks

- [ ] Rename `listen()` to `_listen(StreamController controller)` in `sse_client.dart`
- [ ] Wrap `_listen()` in a reconnect loop with exponential backoff (3 s → 6 s → 12 s → 24 s, cap 60 s)
- [ ] Emit `{'type': '__reconnecting', 'attempt': N, 'backoffMs': M}` internal event before each retry
- [ ] Add `reconnectAttempts` getter to `SseClient` for observability
- [ ] Add `FlintCatalog.refresh()` method: re-fetches catalog URL and updates internal state
- [ ] Wire `FlintThemeData` overrides: when `applicationId != null`, call `GET /a2ui/v1/design-systems` and merge `component_overrides.css_vars`
- [ ] Add unit tests in `test/flint_genui_test.dart`:
  - Reconnect fires after simulated disconnect
  - Backoff increases exponentially
  - `refresh()` updates the component list
- [ ] Run `flutter test` in `packages/flint_genui/` — all tests pass
