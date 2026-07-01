import 'dart:async';
import 'dart:convert';
import 'package:http/http.dart' as http;
import 'sse_client.dart';

/// AG-UI event types received from fdb-gateway.
enum AgUiEventType {
  runStarted,
  runFinished,
  runError,
  textMessageContent,
  toolCallStart,
  toolCallEnd,
  custom,
  unknown,
}

AgUiEventType _parseEventType(String? type) {
  switch (type) {
    case 'RunStarted': return AgUiEventType.runStarted;
    case 'RunFinished': return AgUiEventType.runFinished;
    case 'RunError': return AgUiEventType.runError;
    case 'TextMessageContent': return AgUiEventType.textMessageContent;
    case 'ToolCallStart': return AgUiEventType.toolCallStart;
    case 'ToolCallEnd': return AgUiEventType.toolCallEnd;
    case 'Custom': return AgUiEventType.custom;
    default: return AgUiEventType.unknown;
  }
}

/// An A2UI surface event payload received via AG-UI Custom event.
class A2uiSurfaceEvent {
  final String action;
  final String surfaceId;
  final List<Map<String, dynamic>> components;

  const A2uiSurfaceEvent({
    required this.action,
    required this.surfaceId,
    required this.components,
  });

  factory A2uiSurfaceEvent.fromJson(Map<String, dynamic> json) {
    final value = json['value'] as Map<String, dynamic>? ?? {};
    return A2uiSurfaceEvent(
      action: value['action'] as String? ?? '',
      surfaceId: value['surfaceId'] as String? ?? '',
      components: (value['components'] as List<dynamic>?)
              ?.cast<Map<String, dynamic>>() ??
          [],
    );
  }
}

/// Connects to fdb-gateway AG-UI SSE stream — no Gemini dependency.
class FlintA2uiTransport {
  final String endpoint;
  final String applicationId;
  final String jwt;

  StreamSubscription<Map<String, dynamic>>? _subscription;
  final _surfaceController = StreamController<A2uiSurfaceEvent>.broadcast();

  FlintA2uiTransport({
    required this.endpoint,
    required this.applicationId,
    required this.jwt,
  });

  Stream<A2uiSurfaceEvent> get surfaceEvents => _surfaceController.stream;

  void connect() {
    final sseUrl = '$endpoint/realtime/v1/sse?application_id=${Uri.encodeComponent(applicationId)}';
    final client = SseClient(
      url: sseUrl,
      headers: {'Authorization': 'Bearer $jwt'},
    );

    _subscription = client.connect().listen(
      (event) {
        final type = _parseEventType(event['type'] as String?);
        if (type == AgUiEventType.custom) {
          final name = event['name'] as String?;
          if (name == 'a2ui:surface') {
            _surfaceController.add(A2uiSurfaceEvent.fromJson(event));
          }
        }
      },
      onError: (Object e) {
        // Surface stream stays open — transport will reconnect
      },
    );
  }

  /// POST /a2ui/v1/surfaces/assemble and return the A2UI JSON response.
  Future<String> assembleSurface(String eventJson) async {
    final response = await http.post(
      Uri.parse('$endpoint/a2ui/v1/surfaces/assemble'),
      headers: {
        'Authorization': 'Bearer $jwt',
        'Content-Type': 'application/json',
      },
      body: json.encode({'event': eventJson}),
    );
    if (response.statusCode != 200) {
      throw Exception('assembleSurface failed: ${response.statusCode}');
    }
    return response.body;
  }

  void dispose() {
    _subscription?.cancel();
    _surfaceController.close();
  }
}
