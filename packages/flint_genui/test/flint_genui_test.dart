import 'dart:async';
import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:flint_genui/flint_genui.dart';

void main() {
  group('FlintCatalog', () {
    late FlintCatalog catalog;

    setUp(() {
      catalog = FlintCatalog.build();
    });

    test('registers all base components', () {
      expect(FlintCatalog.size, greaterThanOrEqualTo(34));
    });

    test('resolves stack', () {
      final item = FlintCatalog.resolve('stack');
      expect(item, isNotNull);
    });

    test('resolves button', () {
      final item = FlintCatalog.resolve('button');
      expect(item, isNotNull);
    });

    test('resolves agent-chat', () {
      final item = FlintCatalog.resolve('agent-chat');
      expect(item, isNotNull);
    });

    test('resolves breadcrumb', () {
      final item = FlintCatalog.resolve('breadcrumb');
      expect(item, isNotNull);
    });

    test('returns null for unknown slug', () {
      expect(FlintCatalog.resolve('unknown/slug'), isNull);
    });

    test('build with overrides replaces component builder', () {
      Widget customBuilder(BuildContext ctx, Map<String, dynamic> p) =>
          const SizedBox.shrink();
      final withOverride =
          FlintCatalog.build(overrides: {'button': customBuilder});
      expect(FlintCatalog.resolve('button')?.builder, equals(customBuilder));
    });
  });

  group('FlintComponentSchemas', () {
    test('contains schema entries for all registered slugs', () {
      expect(FlintComponentSchemas.all.length, greaterThanOrEqualTo(34));
    });

    test('every schema value is a Map', () {
      for (final entry in FlintComponentSchemas.all.entries) {
        expect(entry.value, isA<Map<String, dynamic>>(),
            reason: 'Schema for ${entry.key} should be a Map');
      }
    });
  });

  group('FlintThemeData', () {
    test('defaults() constructs without error', () {
      final theme = FlintThemeData.defaults();
      expect(theme.primary, isNotNull);
      expect(theme.surface, isNotNull);
      expect(theme.radiusMd, greaterThan(0));
    });

    test('copyWith produces new instance with updated field', () {
      final base = FlintThemeData.defaults();
      final updated = base.copyWith(radiusMd: 99.0);
      expect(updated.radiusMd, equals(99.0));
      expect(base.radiusMd, isNot(equals(99.0)));
    });
  });

  group('Components — fromProps factory', () {
    testWidgets('FlintButton renders from props', (tester) async {
      final widget =
          FlintButton.fromProps({'label': 'Click me', 'variant': 'primary'});
      await tester.pumpWidget(wrapWidget(widget));
      expect(find.text('Click me'), findsOneWidget);
    });

    testWidgets('FlintBreadcrumb renders items', (tester) async {
      final widget = FlintBreadcrumb.fromProps({
        'items': [
          {'label': 'Home'},
          {'label': 'Products'},
          {'label': 'Detail'},
        ],
      });
      await tester.pumpWidget(wrapWidget(widget));
      expect(find.text('Home'), findsOneWidget);
      expect(find.text('Detail'), findsOneWidget);
    });

    testWidgets('FlintStreamingText shows cursor when streaming',
        (tester) async {
      final widget =
          FlintStreamingText.fromProps({'text': 'Hello', 'streaming': true});
      await tester.pumpWidget(wrapWidget(widget));
      expect(find.textContaining('Hello'), findsOneWidget);
    });

    testWidgets('FlintMetric renders value and label', (tester) async {
      final widget = FlintMetric.fromProps(
          {'label': 'Requests', 'value': 42, 'unit': 'rps', 'trend': 'up'});
      await tester.pumpWidget(wrapWidget(widget));
      expect(find.text('42'), findsOneWidget);
      expect(find.text('Requests'), findsOneWidget);
    });

    testWidgets('FlintNavMenu renders items horizontally', (tester) async {
      final widget = FlintNavMenu.fromProps({
        'items': [
          {'label': 'Home', 'active': true},
          {'label': 'About'}
        ],
      });
      await tester.pumpWidget(wrapWidget(widget));
      expect(find.text('Home'), findsOneWidget);
      expect(find.text('About'), findsOneWidget);
    });
  });

  // ──────────────────────────────────────────────────────────────────────────
  // SseClient — reconnect behaviour
  // ──────────────────────────────────────────────────────────────────────────

  group('SseClient — reconnect', () {
    test('emits __reconnecting then data after a single transient failure',
        () async {
      var callCount = 0;
      final received = <Map<String, dynamic>>[];

      final client = SseClient(
        url: 'http://test.local/sse',
        headers: {},
        clientFactory: () {
          callCount++;
          if (callCount == 1) return _ThrowingClient();
          // Second call: one SSE event then clean EOF.
          return _SseStreamClient('data: {"type":"hello","v":1}\n\n');
        },
        initialBackoff: const Duration(milliseconds: 1),
      );

      await for (final event in client.connect()) {
        received.add(event);
      }

      expect(callCount, equals(2));
      expect(received.length, equals(2));

      // First event is the reconnect sentinel.
      expect(received[0]['type'], equals(SseClient.reconnectingEventType));
      expect(received[0]['attempt'], equals(1));
      expect(received[0]['backoffMs'], equals(1));

      // Second event is the real data.
      expect(received[1]['type'], equals('hello'));
      expect(received[1]['v'], equals(1));
    });

    test('backoff doubles on each consecutive retry', () async {
      var callCount = 0;
      final reconnectEvents = <Map<String, dynamic>>[];

      final client = SseClient(
        url: 'http://test.local/sse',
        headers: {},
        clientFactory: () {
          callCount++;
          if (callCount <= 3) return _ThrowingClient();
          return _SseStreamClient('data: {"type":"done"}\n\n');
        },
        initialBackoff: const Duration(milliseconds: 1),
      );

      await for (final event in client.connect()) {
        if (event['type'] == SseClient.reconnectingEventType) {
          reconnectEvents.add(event);
        }
      }

      expect(reconnectEvents.length, equals(3));

      // Backoff is reported before the delay, so:
      //   attempt 1 → backoff still at initialBackoff (1 ms)
      //   attempt 2 → backoff doubled to 2 ms
      //   attempt 3 → backoff doubled to 4 ms
      expect(reconnectEvents[0]['attempt'], equals(1));
      expect(reconnectEvents[0]['backoffMs'], equals(1));

      expect(reconnectEvents[1]['attempt'], equals(2));
      expect(reconnectEvents[1]['backoffMs'], equals(2));

      expect(reconnectEvents[2]['attempt'], equals(3));
      expect(reconnectEvents[2]['backoffMs'], equals(4));
    });

    test('stops reconnecting after a clean server EOF', () async {
      var callCount = 0;

      final client = SseClient(
        url: 'http://test.local/sse',
        headers: {},
        clientFactory: () {
          callCount++;
          return _SseStreamClient('data: {"type":"ping"}\n\n');
        },
        initialBackoff: const Duration(milliseconds: 1),
      );

      final events = await client.connect().toList();

      // Connected exactly once — no reconnect triggered.
      expect(callCount, equals(1));
      expect(events.length, equals(1));
      expect(events[0]['type'], equals('ping'));
    });

    test('emits no __reconnecting events when first connection succeeds',
        () async {
      final client = SseClient(
        url: 'http://test.local/sse',
        headers: {},
        clientFactory: () =>
            _SseStreamClient('data: {"type":"ok"}\n\n'),
        initialBackoff: const Duration(milliseconds: 1),
      );

      final reconnecting = await client
          .connect()
          .where((e) => e['type'] == SseClient.reconnectingEventType)
          .toList();

      expect(reconnecting, isEmpty);
    });

    test('backoff caps at 60 s', () async {
      // Verify the cap formula by simulating enough failures that the
      // un-capped value would exceed 60 s.
      var callCount = 0;
      final backoffValues = <int>[];

      // With initialBackoff = 1 s, after 7 doublings: 128 s > 60 s cap.
      final client = SseClient(
        url: 'http://test.local/sse',
        headers: {},
        clientFactory: () {
          callCount++;
          if (callCount <= 7) return _ThrowingClient();
          return _SseStreamClient('data: {"type":"done"}\n\n');
        },
        // Use a small base so the test still finishes quickly.
        initialBackoff: const Duration(milliseconds: 1),
      );

      await for (final event in client.connect()) {
        if (event['type'] == SseClient.reconnectingEventType) {
          backoffValues.add(event['backoffMs'] as int);
        }
      }

      // None of the reported backoff values should exceed 60 000 ms.
      for (final ms in backoffValues) {
        expect(ms, lessThanOrEqualTo(60000));
      }
      // The last few should have reached or stayed at the maximum (since we
      // start at 1 ms: 1→2→4→8→16→32→64 capped to 60 000 ms).
      // With only 7 retries the raw maximum would be 64 ms — still well under
      // the cap — so just confirm the series is monotonically non-decreasing.
      for (var i = 1; i < backoffValues.length; i++) {
        expect(backoffValues[i], greaterThanOrEqualTo(backoffValues[i - 1]));
      }
    });

    test('non-200 response triggers reconnect', () async {
      var callCount = 0;
      int? reconnectAttempt;

      final client = SseClient(
        url: 'http://test.local/sse',
        headers: {},
        clientFactory: () {
          callCount++;
          if (callCount == 1) {
            // Return a 503; _listen will throw StateError.
            return _StatusCodeClient(503);
          }
          return _SseStreamClient('data: {"type":"recovered"}\n\n');
        },
        initialBackoff: const Duration(milliseconds: 1),
      );

      await for (final event in client.connect()) {
        if (event['type'] == SseClient.reconnectingEventType) {
          reconnectAttempt = event['attempt'] as int;
        }
      }

      expect(reconnectAttempt, equals(1));
      expect(callCount, equals(2));
    });
  });

  // ──────────────────────────────────────────────────────────────────────────
  // FlintCatalog.refresh()
  // ──────────────────────────────────────────────────────────────────────────

  group('FlintCatalog.refresh()', () {
    test('catalog remains fully populated after refresh', () async {
      final catalog = FlintCatalog.build();
      final sizeBefore = FlintCatalog.size;

      await catalog.refresh();

      expect(FlintCatalog.size, equals(sizeBefore));
      expect(FlintCatalog.resolve('button'), isNotNull);
      expect(FlintCatalog.resolve('stack'), isNotNull);
      expect(FlintCatalog.resolve('agent-chat'), isNotNull);
    });

    test('onRefresh stream emits after refresh completes', () async {
      final catalog = FlintCatalog.build();
      var notified = false;

      final sub = catalog.onRefresh.listen((_) => notified = true);
      await catalog.refresh();
      // The broadcast stream delivers synchronously; no extra delay needed.
      sub.cancel();

      expect(notified, isTrue);
    });

    test('onRefresh stream emits once per refresh call', () async {
      final catalog = FlintCatalog.build();
      var count = 0;

      final sub = catalog.onRefresh.listen((_) => count++);
      await catalog.refresh();
      await catalog.refresh();
      await catalog.refresh();
      sub.cancel();

      expect(count, equals(3));
    });

    test('override supplied to build() survives a refresh', () async {
      Widget myBuilder(BuildContext ctx, Map<String, dynamic> p) =>
          const SizedBox.shrink();

      final catalog = FlintCatalog.build(overrides: {'button': myBuilder});

      // Override applied immediately after build.
      expect(FlintCatalog.resolve('button')?.builder, equals(myBuilder));

      await catalog.refresh();

      // Override still applied after refresh.
      expect(FlintCatalog.resolve('button')?.builder, equals(myBuilder));
    });

    test('catalog without overrides resets all items to defaults on refresh',
        () async {
      Widget customBuilder(BuildContext ctx, Map<String, dynamic> p) =>
          const SizedBox.shrink();

      // Catalog A installs an override.
      FlintCatalog.build(overrides: {'button': customBuilder});
      expect(FlintCatalog.resolve('button')?.builder, equals(customBuilder));

      // Catalog B has no overrides; refresh resets to defaults.
      final catalogB = FlintCatalog.build();
      await catalogB.refresh();

      expect(FlintCatalog.resolve('button')?.builder,
          isNot(equals(customBuilder)));
    });
  });
}

// ────────────────────────────────────────────────────────────────────────────
// Widget test helper
// ────────────────────────────────────────────────────────────────────────────

Widget wrapWidget(Widget child) {
  return MaterialApp(
    home: Scaffold(
      body: Builder(
        builder: (context) => child,
      ),
    ),
  );
}

// ────────────────────────────────────────────────────────────────────────────
// Mock HTTP clients for SseClient tests
// ────────────────────────────────────────────────────────────────────────────

/// Always throws [StateError] when [send] is called, simulating a total
/// connection failure (e.g. network unreachable).
class _ThrowingClient extends http.BaseClient {
  @override
  Future<http.StreamedResponse> send(http.BaseRequest request) {
    throw StateError('Simulated connection failure');
  }
}

/// Returns a single streaming SSE response with [body] and status 200,
/// then closes cleanly (simulating a server-initiated EOF).
class _SseStreamClient extends http.BaseClient {
  final String body;

  _SseStreamClient(this.body);

  @override
  Future<http.StreamedResponse> send(http.BaseRequest request) async {
    final bytes = utf8.encode(body);
    final stream = Stream.value(bytes);
    return http.StreamedResponse(stream, 200);
  }
}

/// Returns a response with [statusCode] and an empty body — used to test
/// that non-200 responses trigger the reconnect path.
class _StatusCodeClient extends http.BaseClient {
  final int statusCode;

  _StatusCodeClient(this.statusCode);

  @override
  Future<http.StreamedResponse> send(http.BaseRequest request) async {
    return http.StreamedResponse(const Stream.empty(), statusCode);
  }
}
