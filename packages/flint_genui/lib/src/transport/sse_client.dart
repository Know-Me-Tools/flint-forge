import 'dart:async';
import 'dart:convert';
import 'package:http/http.dart' as http;

/// Factory function type for creating [http.Client] instances.
/// Injectable in tests; defaults to [http.Client.new].
typedef _ClientFactory = http.Client Function();

/// Pure Dart SSE client with exponential backoff reconnect.
///
/// ## Reconnect behaviour
///
/// When [_listen] throws (non-200 status or any network error) the [connect]
/// loop:
///   1. Emits a [reconnectingEventType] control event
///   2. Waits for the current backoff duration
///   3. Doubles the backoff (capped at 60 s)
///   4. Retries by calling [_listen] again
///
/// A clean EOF from the server (stream ends without error) breaks the loop
/// and closes the broadcast stream.
///
/// ## Testing
///
/// Pass a custom [clientFactory] (and optionally a short [initialBackoff]) to
/// avoid real network calls and eliminate slow delays in tests:
///
/// ```dart
/// final client = SseClient(
///   url: 'http://test.local/sse',
///   headers: {},
///   clientFactory: () => _MyMockHttpClient(),
///   initialBackoff: const Duration(milliseconds: 1),
/// );
/// ```
class SseClient {
  final String url;
  final Map<String, String> headers;

  final _ClientFactory _clientFactory;
  final Duration _initialBackoff;

  static const Duration _maxBackoff = Duration(seconds: 60);

  /// Sentinel event type emitted to the stream before each reconnect attempt.
  ///
  /// Payload shape:
  /// ```json
  /// {"type": "__reconnecting", "attempt": 1, "backoffMs": 3000}
  /// ```
  ///
  /// Callers can filter these out:
  /// ```dart
  /// stream.where((e) => e['type'] != SseClient.reconnectingEventType)
  /// ```
  static const String reconnectingEventType = '__reconnecting';

  SseClient({
    required this.url,
    required this.headers,
    /// Injectable HTTP client factory. Defaults to [http.Client.new].
    _ClientFactory? clientFactory,
    /// Initial backoff before the first reconnect attempt. Defaults to 3 s per
    /// spec. Override to a tiny value in tests to avoid slow delays.
    Duration initialBackoff = const Duration(seconds: 3),
  })  : _clientFactory = clientFactory ?? http.Client.new,
        _initialBackoff = initialBackoff;

  /// Returns a broadcast stream of parsed SSE event objects.
  ///
  /// Multiple listeners are allowed. The stream stays open across reconnects.
  Stream<Map<String, dynamic>> connect() {
    final controller = StreamController<Map<String, dynamic>>.broadcast();

    Future<void> run() async {
      var backoff = _initialBackoff;
      var attempts = 0;

      while (!controller.isClosed) {
        try {
          await _listen(controller);
          // Clean EOF — server closed the connection intentionally; stop.
          break;
        } catch (_) {
          if (controller.isClosed) break;
          attempts++;
          if (!controller.isClosed) {
            controller.add({
              'type': reconnectingEventType,
              'attempt': attempts,
              'backoffMs': backoff.inMilliseconds,
            });
          }
          await Future.delayed(backoff);
          backoff = Duration(
            milliseconds:
                (backoff.inMilliseconds * 2).clamp(0, _maxBackoff.inMilliseconds),
          );
        }
      }
    }

    run().whenComplete(() {
      if (!controller.isClosed) controller.close();
    });

    return controller.stream;
  }

  /// Opens a single SSE connection and forwards parsed events to [controller].
  ///
  /// Throws [StateError] on non-200 HTTP status; re-throws any network
  /// exception. The caller ([connect]) catches and triggers the reconnect loop.
  Future<void> _listen(
    StreamController<Map<String, dynamic>> controller,
  ) async {
    final client = _clientFactory();
    try {
      final request = http.Request('GET', Uri.parse(url))
        ..headers.addAll({...headers, 'Accept': 'text/event-stream'});

      final response = await client.send(request);
      if (response.statusCode != 200) {
        throw StateError('SSE connection failed: ${response.statusCode}');
      }

      var buffer = '';
      await for (final chunk in response.stream.transform(utf8.decoder)) {
        buffer += chunk;
        final lines = buffer.split('\n');
        buffer = lines.removeLast(); // keep the incomplete trailing fragment

        var dataLine = '';
        for (final line in lines) {
          if (line.startsWith('data: ')) {
            dataLine = line.substring(6);
          } else if (line.isEmpty && dataLine.isNotEmpty) {
            try {
              final event = json.decode(dataLine) as Map<String, dynamic>;
              if (!controller.isClosed) {
                controller.add(event);
              }
            } catch (_) {
              // skip malformed JSON
            }
            dataLine = '';
          }
        }
      }
      // Fell off the end of the stream → clean EOF, _listen returns normally.
    } finally {
      client.close();
    }
  }
}
