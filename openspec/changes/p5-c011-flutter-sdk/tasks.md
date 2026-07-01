# p5-c011 Tasks — Flint Flutter SDK

## Tasks

- [x] Initialize `packages/flint_genui/` directory with `pubspec.yaml` (http ^1.2.0, web_socket_channel ^3.0.0; genui/cue excluded per OQ-13)
- [x] Implement `FlintA2uiTransport` — pure Dart SSE client to fdb-gateway, no Gemini/Firebase dependency
- [x] Implement `FlintCatalog.build()` — registers all 40 Flint component slugs across 6 categories with JSON schemas
- [x] Implement `FlintSurface` widget — SSE-driven surface renderer using FlintA2uiTransport + FlintCatalog
- [x] Implement ALL layout widgets (FlintStack, FlintCard, FlintGrid, FlintTabs, FlintDrawer, FlintModal, ...)
- [x] Implement ALL data-display widgets (FlintDataGrid, FlintChart, FlintTimeline, FlintKanban, FlintMetric, FlintBadge)
- [x] Implement ALL input widgets (FlintForm, FlintTextField, FlintSelect, FlintDatePicker, FlintSearch, FlintFileUpload)
- [x] Implement ALL action widgets (FlintButton, FlintConfirm, FlintWizard, FlintBulkAction, FlintActionBar)
- [x] Implement ALL agent widgets (FlintAgentChat, FlintToolCall, FlintStreamingText, FlintDecision, FlintProgressLog, FlintArtifact)
- [x] Implement ALL navigation widgets (FlintNavMenu, FlintCommandPalette, FlintFilterBar, FlintBreadcrumb)
- [x] Implement `surface_animations.dart` — entrance, exit, update, streaming transitions (AnimationController; cue excluded per OQ-13)
- [x] Implement `FlintThemeData` (ThemeExtension) + `fromCatalog()` async factory for `GET /a2ui/v1/catalog/:id`
- [x] Add `Semantics` wrappers to all interactive components for a11y
- [x] Gate test: render all 40 base components with minimal props in widget tests
- [x] Gate test: FlintCatalog registers ≥ 40 components at `build()` time
- [x] Gate test: no Firebase/Gemini imports anywhere in package
- [ ] Publish to pub.dev as `flint_genui` (after alpha stabilization; deferred)
