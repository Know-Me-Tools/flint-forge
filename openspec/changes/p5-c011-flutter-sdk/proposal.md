# p5-c011 — Flint Flutter SDK (`flint_genui`)

**Phase:** 5 — Flint A2UI Component Registry  
**Priority:** P1 (MVP SDK surface — enables mobile clients without Google Firebase/Gemini lock-in)  
**Depends on:** p5-c002 (base components seed), p5-c006 (REST API)  
**Blocks:** nothing in Phase 5

---

## What this change delivers

A Flutter/Dart package (`flint_genui`) that extends `flutter/genui` (pub: `genui ^0.9.2`) with the full Flint component catalog, animated transitions via `cue ^0.3.11`, and a design token system that maps to `flint_a2ui.design_systems`. This enables Dart/Flutter apps to consume Flint A2UI surfaces natively without binding to Gemini or Firebase.

---

## Research Basis

- **flutter/genui** (`genui` on pub.dev): Official Flutter A2UI implementation by the Flutter team at Google. Alpha, `^0.9.2`, requires Dart 3.9+/Flutter 3.35. Uses `A2uiTransportAdapter`, `SurfaceController`, `Catalog`, `Surface`. See `.firecrawl/flutter-genui-a2ui-2026.md`.
- **cue** (`cue ^0.3.11`): Physics-first Flutter animation library by Milad-Akarie. MIT. Timeline-driven API for polished transitions without imperative `AnimationController`. See `.firecrawl/cue-flutter-animation-2026.md`.
- **genui_a2ui** supplementary package: For server-side agent architectures (where the LLM runs on `fdb-gateway`, not on-device). This is Flint's model.

---

## Architecture

### Package Structure

```
packages/flint_genui/                   # pub: flint_genui
├── lib/
│   ├── src/
│   │   ├── catalog/
│   │   │   ├── flint_catalog.dart      # FlintCatalog.build() — all 63 Flint components as CatalogItem
│   │   │   └── component_schemas.dart  # JSON schemas for each Flint component (mirrors flint_a2ui.components.schema)
│   │   │
│   │   ├── transport/
│   │   │   ├── flint_transport.dart    # FlintA2uiTransport — connects to fdb-gateway SSE, no Gemini dependency
│   │   │   └── sse_client.dart         # Pure Dart SSE client for AG-UI stream
│   │   │
│   │   ├── components/
│   │   │   ├── layout/                 # FlintStack, FlintCard, FlintGrid, FlintTabs, ...
│   │   │   ├── data_display/           # FlintDataGrid, FlintTable, FlintChart, FlintKanban, ...
│   │   │   ├── input/                  # FlintForm, FlintTextField, FlintSelect, FlintDatePicker, ...
│   │   │   ├── action/                 # FlintButton, FlintConfirm, FlintWizard, ...
│   │   │   ├── agent/                  # FlintAgentChat, FlintToolCall, FlintStreamingText, FlintDecision, ...
│   │   │   └── navigation/             # FlintNavMenu, FlintCommandPalette, FlintFilterBar, ...
│   │   │
│   │   ├── animations/
│   │   │   ├── surface_animations.dart # cue-powered surface assembly / destruction transitions
│   │   │   └── component_animations.dart # Per-component animated state transitions using cue
│   │   │
│   │   ├── tokens/
│   │   │   ├── flint_theme.dart        # FlintThemeData (ThemeExtension) — design tokens from design_systems
│   │   │   └── token_resolver.dart     # Fetches GET /a2ui/v1/catalog/:id → ThemeExtension values
│   │   │
│   │   └── flint_surface.dart          # FlintSurface widget: A2uiTransportAdapter + Catalog + Surface wrapped
│   │
│   └── flint_genui.dart                # Public API exports
│
└── pubspec.yaml
```

### pubspec.yaml

```yaml
name: flint_genui
description: Flint A2UI component catalog for Flutter, extending flutter/genui
version: 0.1.0

environment:
  sdk: ">=3.9.0 <4.0.0"
  flutter: ">=3.35.0"

dependencies:
  flutter:
    sdk: flutter
  genui: ^0.9.2
  genui_a2ui: ^0.9.2
  cue: ^0.3.11
  json_schema_builder: ^0.1.5
  http: ^1.2.0
  web_socket_channel: ^3.0.0
```

### Core Widget

```dart
// lib/src/flint_surface.dart
class FlintSurface extends StatefulWidget {
  final String endpoint;          // fdb-gateway base URL
  final String applicationId;
  final String jwt;
  final FlintThemeData? theme;   // Override design tokens
  final Map<String, WidgetBuilder>? componentOverrides; // slug → widget builder

  const FlintSurface({
    super.key,
    required this.endpoint,
    required this.applicationId,
    required this.jwt,
    this.theme,
    this.componentOverrides,
  });
}
```

### Catalog Registration

```dart
// All 63 Flint components registered as CatalogItems
final catalog = FlintCatalog.build(
  overrides: {
    'data-grid': (context, props) => MyCustomDataGrid(props: props),
  },
);

// Use in app
FlintSurface(
  endpoint: 'https://api.myapp.com',
  applicationId: 'app-uuid',
  jwt: userJwt,
  theme: FlintThemeData.fromTokens(tokens),
);
```

### Transport (No Gemini Lock-In)

```dart
// FlintA2uiTransport — uses fdb-gateway SSE, not Gemini
class FlintA2uiTransport extends A2uiTransportAdapter {
  final String endpoint;
  final String jwt;

  @override
  Future<String> sendAndReceive(ChatMessage message) async {
    // POST /a2ui/v1/surfaces/assemble with JWT
    // Returns A2UI surface JSON
    // OR subscribe to AG-UI SSE stream and filter Custom events
    final response = await _client.post(
      Uri.parse('$endpoint/a2ui/v1/surfaces/assemble'),
      headers: {'Authorization': 'Bearer $jwt'},
      body: jsonEncode({'event': message.content}),
    );
    return response.body; // A2UI JSON
  }
}
```

### cue Animations

```dart
// lib/src/animations/surface_animations.dart
class FlintSurfaceEntrance extends StatelessWidget {
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return CueAnimation(
      animation: CueSpec.spring(
        from: const CueValue(opacity: 0.0, scale: 0.95),
        to: const CueValue(opacity: 1.0, scale: 1.0),
        physics: SpringPhysics(stiffness: 300, damping: 25),
      ),
      builder: (context, value, child) => Opacity(
        opacity: value.opacity,
        child: Transform.scale(scale: value.scale, child: child),
      ),
      child: child,
    );
  }
}

// Per-component animations
// FlintStreamingText: character timeline via cue
// FlintToolCall: pending→running→complete spring transitions
// FlintDataGrid: row-insertion spring from top
```

### Design Token System (ThemeExtension)

```dart
@immutable
class FlintThemeData extends ThemeExtension<FlintThemeData> {
  final Color primary;
  final Color surface;
  final Color text;
  final TextStyle textBase;
  final Duration durationNormal;

  // Fetch from GET /a2ui/v1/catalog/:id
  static Future<FlintThemeData> fromCatalog(String endpoint, String catalogId) async {
    final tokens = await _fetchTokens(endpoint, catalogId);
    return FlintThemeData(
      primary: _parseColor(tokens['color']['primary']),
      surface: _parseColor(tokens['color']['surface']),
      ...
    );
  }
}
```

---

## Gate Tests

- [ ] `FlintSurface` renders in a test app connected to a mock fdb-gateway endpoint
- [ ] `FlintCatalog.build()` registers all 63 components — each can render a minimal valid props set
- [ ] `FlintA2uiTransport` sends request to `POST /a2ui/v1/surfaces/assemble` and parses response
- [ ] cue animation plays on surface entry (no `AnimationController` boilerplate)
- [ ] `FlintThemeData.fromCatalog()` fetches and applies design tokens
- [ ] Component override: `componentOverrides: { 'data-grid': myBuilder }` renders custom widget
- [ ] A11y: `Semantics` wrappers present on all interactive components
- [ ] No dependency on `firebase_ai`, `genui_google_generative_ai`, or Gemini SDK
