import 'dart:async';
import 'dart:convert';
import 'package:http/http.dart' as http;

/// Pure Dart SSE client — no Gemini/Firebase dependency.
class SseClient {
  final String url;
  final Map<String, String> headers;

  SseClient({required this.url, required this.headers});

  Stream<Map<String, dynamic>> connect() {
    final controller = StreamController<Map<String, dynamic>>();

    Future<void> listen() async {
      final client = http.Client();
      try {
        final request = http.Request('GET', Uri.parse(url))
          ..headers.addAll({...headers, 'Accept': 'text/event-stream'});

        final response = await client.send(request);
        if (response.statusCode != 200) {
          controller.addError('SSE connection failed: ${response.statusCode}');
          return;
        }

        String buffer = '';
        await for (final chunk in response.stream.transform(utf8.decoder)) {
          buffer += chunk;
          final lines = buffer.split('\n');
          buffer = lines.removeLast();

          String dataLine = '';
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
                // skip malformed data
              }
              dataLine = '';
            }
          }
        }
      } catch (e) {
        if (!controller.isClosed) {
          controller.addError(e);
        }
      } finally {
        client.close();
        if (!controller.isClosed) {
          controller.close();
        }
      }
    }

    listen();
    return controller.stream;
  }
}
