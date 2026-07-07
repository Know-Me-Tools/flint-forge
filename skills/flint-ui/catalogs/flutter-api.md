# flint_genui — Flutter API Reference

Dart package extending `genui ^0.9.2`. Pure SSE transport, no Gemini or Firebase.
Supports iOS 14+, Android API 26+, and desktop.

---

## Installation

```yaml
# pubspec.yaml
dependencies:
  flint_genui: ^0.9.2
  genui: ^0.9.2
```

---

## FlintSurface

Top-level widget. Subscribes to an AG-UI SSE run and renders the assembled surface.

```dart
import 'package:flint_genui/flint_genui.dart';

FlintSurface(
  transport: FlintA2uiTransport(
    catalogUrl: 'https://api.example.com/a2ui/v1/catalog/flint-base/1.0',
    gatewayUrl: 'https://api.example.com',
    bearerToken: userToken,
    runId: 'run-abc123',
    eventsUrl: 'https://api.example.com/agents/v1/run-abc123/events',
  ),
  surfaceId: 'orders-view',
  loadingBuilder: (context) => const CircularProgressIndicator(),
  errorBuilder: (context, error) => Text('Error: $error'),
)
```

### FlintSurface Parameters

| Parameter | Type | Required | Description |
|---|---|---|---|
| `transport` | `FlintA2uiTransport` | ✅ | SSE transport + catalog configuration |
| `surfaceId` | `String` | ✅ | Matches the A2UI `createSurface` message surfaceId |
| `loadingBuilder` | `WidgetBuilder?` | — | Widget shown while loading |
| `errorBuilder` | `ErrorWidgetBuilder?` | — | Widget shown on error |
| `theme` | `FlintThemeData?` | — | Design token overrides |

---

## FlintA2uiTransport

Configures the SSE + REST connection to fdb-gateway. Pure SSE — do NOT use WebSocket.

```dart
final transport = FlintA2uiTransport(
  catalogUrl: 'https://api.example.com/a2ui/v1/catalog/flint-base/1.0',
  gatewayUrl: 'https://api.example.com',
  bearerToken: () => authService.currentToken,  // can be a function for refresh
  runId: 'run-abc123',
  eventsUrl: 'https://api.example.com/agents/v1/run-abc123/events',
  reconnectDelay: const Duration(seconds: 3),
  maxReconnectAttempts: 5,
);
```

---

## FlintCatalog

Load and query the component catalog independently:

```dart
import 'package:flint_genui/flint_genui.dart';

final catalog = await FlintCatalog.load(
  'https://api.example.com/a2ui/v1/catalog/flint-base/1.0',
  bearerToken: userToken,
);

final dataGrid = catalog.getComponent('data-grid');
final inputs = catalog.listComponents(category: 'input');
```

---

## FlintThemeData

Design token overrides for your brand:

```dart
FlintSurface(
  transport: transport,
  surfaceId: 'orders-view',
  theme: FlintThemeData(
    primaryColor: const Color(0xFF2563EB),
    surfaceColor: Colors.white,
    textColor: const Color(0xFF0F172A),
    spacing: const FlintSpacing(unit: 4.0),
    borderRadius: const FlintBorderRadius(md: 6.0),
    fontFamily: 'Inter',
  ),
)
```

---

## Individual Widgets

All 55 Flint components are available as Flutter widgets:

```dart
import 'package:flint_genui/components.dart';

// DataGrid
FlintDataGrid(
  columns: [
    FlintColumn(name: 'status', type: FlintColumnType.text, sortable: true),
    FlintColumn(name: 'total',  type: FlintColumnType.number, format: 'currency'),
  ],
  data: rows,
  onRowTap: (row) => context.push('/orders/${row["id"]}'),
  pagination: const FlintPagination(pageSize: 25),
)

// Form
FlintForm(
  fields: [
    FlintField.text(name: 'email', label: 'Email', required: true),
    FlintField.select(name: 'role', label: 'Role', options: roleOptions),
  ],
  onSubmit: (data) async { await createUser(data); },
  submitLabel: 'Create User',
)

// Button
FlintButton(
  label: 'Submit',
  variant: FlintButtonVariant.primary,
  onPressed: handleSubmit,
)
```

---

## Animations — cue ^0.3.11

Use `cue` for component entrance/exit animations:

```dart
import 'package:cue/cue.dart';

CueAnimated(
  animation: CueAnimation.fadeSlideUp,
  duration: const Duration(milliseconds: 300),
  child: FlintDataGrid(...),
)
```

---

## TypeDefs

```dart
// From flint_genui/types.dart

typedef FlintBearerToken = String Function();

class FlintColumn {
  final String name;
  final FlintColumnType type;
  final bool sortable;
  final String? format;   // 'currency' | 'date' | 'percent'
  const FlintColumn({required this.name, required this.type, ...});
}

class FlintField {
  final String name;
  final String label;
  final String type;      // 'text' | 'email' | 'number' | 'select' | 'textarea' | ...
  final bool required;
  final List<FlintOption>? options;

  const FlintField.text({...});
  const FlintField.select({...});
  const FlintField.email({...});
}

enum FlintButtonVariant { primary, secondary, outline, ghost, destructive }
enum FlintColumnType    { text, number, uuid, boolean, date, json }
```
