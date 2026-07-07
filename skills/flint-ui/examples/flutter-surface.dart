import 'package:flutter/material.dart';
import 'package:flint_genui/flint_genui.dart';

/// Example: Full Flutter app with a FlintSurface rendering an orders data grid.
///
/// The surface is assembled on the server via POST /agents/v1/{runId}/surfaces/assemble
/// and streamed back to the client as an AG-UI Custom("a2ui:surface") event.
void main() {
  runApp(const FlintExampleApp());
}

class FlintExampleApp extends StatelessWidget {
  const FlintExampleApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Flint Example',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: const Color(0xFF2563EB)),
        useMaterial3: true,
      ),
      home: const OrdersSurfacePage(),
    );
  }
}

class OrdersSurfacePage extends StatefulWidget {
  const OrdersSurfacePage({super.key});

  @override
  State<OrdersSurfacePage> createState() => _OrdersSurfacePageState();
}

class _OrdersSurfacePageState extends State<OrdersSurfacePage> {
  late Future<String> _runIdFuture;
  // In production this comes from your auth service
  static const _bearerToken = 'your-jwt-token-here';
  static const _gatewayUrl  = 'https://api.example.com';
  static const _catalogUrl  = '$_gatewayUrl/a2ui/v1/catalog/flint-base/1.0';

  @override
  void initState() {
    super.initState();
    _runIdFuture = _startRun();
  }

  /// Start a new AG-UI run and trigger surface assembly.
  Future<String> _startRun() async {
    // 1. Create a run
    final startResp = await FlintHttp.post(
      '$_gatewayUrl/agents/v1/runs',
      bearerToken: _bearerToken,
    );
    final runId = startResp['run_id'] as String;

    // 2. Trigger surface assembly for the orders table
    await FlintHttp.post(
      '$_gatewayUrl/agents/v1/$runId/surfaces/assemble',
      bearerToken: _bearerToken,
      body: {
        'event_type': 'mount',
        'event_context': {
          'data_source': {'schema': 'public', 'table': 'orders'},
        },
      },
    );

    return runId;
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Orders'),
        backgroundColor: Theme.of(context).colorScheme.inversePrimary,
      ),
      body: FutureBuilder<String>(
        future: _runIdFuture,
        builder: (context, snapshot) {
          if (snapshot.hasError) {
            return Center(child: Text('Error: ${snapshot.error}'));
          }
          if (!snapshot.hasData) {
            return const Center(child: CircularProgressIndicator());
          }

          final runId = snapshot.data!;
          return FlintSurface(
            transport: FlintA2uiTransport(
              catalogUrl: _catalogUrl,
              gatewayUrl: _gatewayUrl,
              bearerToken: _bearerToken,
              runId: runId,
              eventsUrl: '$_gatewayUrl/agents/v1/$runId/events',
            ),
            surfaceId: 'mount',
            theme: const FlintThemeData(
              primaryColor: Color(0xFF2563EB),
              surfaceColor: Colors.white,
              textColor:    Color(0xFF0F172A),
              fontFamily:   'Inter',
            ),
            loadingBuilder: (context) => const Center(
              child: CircularProgressIndicator(),
            ),
            errorBuilder: (context, error) => Center(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  const Icon(Icons.error_outline, color: Colors.red, size: 48),
                  const SizedBox(height: 16),
                  Text('Could not load surface: $error'),
                ],
              ),
            ),
          );
        },
      ),
    );
  }
}
