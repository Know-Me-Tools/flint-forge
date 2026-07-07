# p8-c002 — Flutter SDK SSE Reconnect + Catalog Refresh

**Phase:** 8 — SDK Completeness
**Priority:** P0
**Depends on:** none

## What this change delivers

- `SseClient` reconnects with exponential backoff on connection error
- `FlintCatalog.refresh()` forces a catalog reload without restarting the transport
- `FlintThemeData` token overrides wired from `component_overrides` REST endpoint

## Design

### Reconnect loop in `SseClient.connect()`

```dart
Stream<Map<String, dynamic>> connect() {
  final controller = StreamController<Map<String, dynamic>>();

  Future<void> run() async {
    var backoff = const Duration(seconds: 3);
    const maxBackoff = Duration(seconds: 60);
    var attempts = 0;

    while (!controller.isClosed) {
      try {
        await _listen(controller);
        break; // clean close — don't reconnect
      } catch (e) {
        if (controller.isClosed) break;
        attempts++;
        controller.add({'type': '__reconnecting', 'attempt': attempts, 'backoffMs': backoff.inMilliseconds});
        await Future.delayed(backoff);
        backoff = Duration(milliseconds: (backoff.inMilliseconds * 2).clamp(0, maxBackoff.inMilliseconds));
      }
    }
  }

  run();
  return controller.stream;
}
```

Rename current `listen()` body to `_listen(controller)`.

### `FlintCatalog.refresh()`

```dart
Future<void> refresh() async {
  _catalog = await _fetchCatalog(_catalogUrl);
  notifyListeners(); // or emit on a stream
}
```

### `FlintThemeData` token overrides

When `applicationId` is set, call `GET /a2ui/v1/design-systems?app_id=<id>` to fetch
overrides and merge `component_overrides.css_vars` into the rendered widget's decoration.
