import 'dart:async';
import 'package:flutter/material.dart';
import 'catalog/flint_catalog.dart';
import 'transport/flint_transport.dart';
import 'animations/surface_animations.dart';

class FlintSurface extends StatefulWidget {
  final String surfaceId;
  final String endpoint;
  final String applicationId;
  final String jwt;
  final FlintCatalog? catalog;
  final Widget? loadingWidget;
  final Widget? errorWidget;

  const FlintSurface({
    super.key,
    required this.surfaceId,
    required this.endpoint,
    required this.applicationId,
    required this.jwt,
    this.catalog,
    this.loadingWidget,
    this.errorWidget,
  });

  @override
  State<FlintSurface> createState() => _FlintSurfaceState();
}

class _FlintSurfaceState extends State<FlintSurface> {
  late FlintCatalog _catalog;
  late FlintA2uiTransport _transport;
  StreamSubscription<A2uiSurfaceEvent>? _subscription;

  List<Map<String, dynamic>> _components = [];
  bool _loading = true;
  String? _error;

  @override
  void initState() {
    super.initState();
    _catalog = widget.catalog ?? FlintCatalog.build();
    _transport = FlintA2uiTransport(endpoint: widget.endpoint, applicationId: widget.applicationId, jwt: widget.jwt);
    _connect();
  }

  void _connect() {
    _subscription = _transport.connect().listen(
      (event) {
        if (!mounted) return;
        if (event.surfaceId != widget.surfaceId) return;
        setState(() {
          _loading = false;
          switch (event.action) {
            case 'createSurface':
              _components = event.components ?? [];
            case 'updateComponents':
              final updates = {for (final c in (event.components ?? [])) c['id']: c};
              _components = _components.map((c) => updates[c['id']] ?? c).toList();
            case 'deleteSurface':
              _components = [];
          }
        });
      },
      onError: (Object e) {
        if (mounted) setState(() { _loading = false; _error = e.toString(); });
      },
    );
  }

  @override
  void dispose() {
    _subscription?.cancel();
    _transport.dispose();
    super.dispose();
  }

  Widget _renderComponent(Map<String, dynamic> spec) {
    final slug = spec['slug'] as String? ?? '';
    final id = spec['id'] as String? ?? slug;
    final props = (spec['props'] as Map<String, dynamic>?) ?? {};
    final children = (spec['children'] as List<dynamic>?)?.cast<Map<String, dynamic>>() ?? [];

    final builder = _catalog.resolve(slug);
    if (builder == null) {
      return Semantics(key: ValueKey(id), label: 'Unknown component: $slug', child: const SizedBox.shrink());
    }
    final widget = builder(props);
    if (children.isEmpty) return ComponentUpdateAnimation(key: ValueKey(id), child: widget);

    return Column(
      key: ValueKey(id),
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        ComponentUpdateAnimation(child: widget),
        ...children.map(_renderComponent),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    if (_loading) {
      return widget.loadingWidget ?? const Center(child: CircularProgressIndicator());
    }
    if (_error != null) {
      return widget.errorWidget ?? Semantics(label: 'Surface error: $_error', child: Center(child: Text('Error: $_error')));
    }
    return Semantics(
      label: 'Surface ${widget.surfaceId}',
      child: SurfaceEntranceAnimation(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: _components.map(_renderComponent).toList(),
        ),
      ),
    );
  }
}
