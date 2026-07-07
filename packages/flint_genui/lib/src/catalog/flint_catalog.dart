import 'dart:async';
import 'package:flutter/material.dart';
import '../components/layout/index.dart';
import '../components/data_display/index.dart';
import '../components/input/index.dart';
import '../components/action/index.dart';
import '../components/agent/index.dart';
import '../components/navigation/index.dart';
import 'component_schemas.dart';

typedef FlintWidgetBuilder = Widget Function(
    BuildContext context, Map<String, dynamic> props);

/// A registered Flint component entry in the catalog.
class FlintCatalogItem {
  final String slug;
  final String category;
  final String primitiveType;
  final Map<String, dynamic> schema;
  final FlintWidgetBuilder builder;

  const FlintCatalogItem({
    required this.slug,
    required this.category,
    required this.primitiveType,
    required this.schema,
    required this.builder,
  });
}

/// The Flint component catalog — equivalent to the @flint/react FlintRegistry.
///
/// ## Refresh
///
/// Call [refresh] to re-register all base components and re-apply any
/// overrides that were passed to [build]. The [onRefresh] stream emits `null`
/// after each completed refresh, so widgets can react:
///
/// ```dart
/// _catalog.onRefresh.listen((_) => setState(() {}));
/// ```
class FlintCatalog {
  // ── Private constructor ──────────────────────────────────────────────────

  FlintCatalog._({Map<String, FlintWidgetBuilder>? overrides})
      : _overrides = Map.unmodifiable(overrides ?? const {});

  // ── Instance state ───────────────────────────────────────────────────────

  /// Overrides stored so [refresh] can re-apply them after re-registration.
  final Map<String, FlintWidgetBuilder> _overrides;

  // ── Shared static state ──────────────────────────────────────────────────

  static final Map<String, FlintCatalogItem> _items = {};

  /// Broadcast controller for catalog-refresh notifications.
  /// Never closed; its lifetime matches the process.
  static final StreamController<void> _refreshController =
      StreamController<void>.broadcast();

  // ── Public API ───────────────────────────────────────────────────────────

  /// Emits `null` each time [refresh] completes.
  Stream<void> get onRefresh => _refreshController.stream;

  /// Register all 40 base Flint components, then apply any [overrides].
  static FlintCatalog build({
    Map<String, FlintWidgetBuilder>? overrides,
  }) {
    _registerAll();
    final catalog = FlintCatalog._(overrides: overrides);
    catalog._applyOverrides();
    return catalog;
  }

  /// Forces a full catalog reload.
  ///
  /// Clears all registered components, re-runs the base registration, then
  /// re-applies the overrides that were supplied to [build]. Notifies
  /// [onRefresh] listeners after completion.
  ///
  /// Because [_items] is static (shared across all instances), calling
  /// [refresh] on any instance resets and repopulates the shared map.
  Future<void> refresh() async {
    _items.clear();
    _registerAll();
    _applyOverrides();
    if (!_refreshController.isClosed) {
      _refreshController.add(null);
    }
  }

  /// Renders the component registered under [slug] with the given [props].
  ///
  /// Returns an invisible placeholder with an accessibility label when the
  /// slug is not found.
  Widget build2(
      BuildContext context, String slug, Map<String, dynamic> props) {
    final item = _items[slug];
    if (item == null) {
      return Semantics(
        label: 'Unknown Flint component: $slug',
        child: const SizedBox.shrink(),
      );
    }
    return item.builder(context, props);
  }

  static FlintCatalogItem? resolve(String slug) => _items[slug];
  static int get size => _items.length;
  static Iterable<String> get slugs => _items.keys;

  // ── Private helpers ──────────────────────────────────────────────────────

  /// Applies the instance-level overrides on top of the currently registered
  /// items. Called from both [build] and [refresh].
  void _applyOverrides() {
    for (final entry in _overrides.entries) {
      final existing = _items[entry.key];
      if (existing != null) {
        _items[entry.key] = FlintCatalogItem(
          slug: existing.slug,
          category: existing.category,
          primitiveType: existing.primitiveType,
          schema: existing.schema,
          builder: entry.value,
        );
      }
    }
  }

  static void _reg(
    String slug,
    String category,
    String primitiveType,
    FlintWidgetBuilder builder,
  ) {
    _items[slug] = FlintCatalogItem(
      slug: slug,
      category: category,
      primitiveType: primitiveType,
      schema: FlintComponentSchemas.all[slug] ?? {},
      builder: builder,
    );
  }

  static void _registerAll() {
    // Layout
    _reg('stack', 'layout', 'Container', (ctx, p) => FlintStack.fromProps(p));
    _reg('card', 'layout', 'Container', (ctx, p) => FlintCard.fromProps(p));
    _reg('grid', 'layout', 'Container', (ctx, p) => FlintGrid.fromProps(p));
    _reg('tabs', 'layout', 'Navigation', (ctx, p) => FlintTabs.fromProps(p));
    _reg('accordion', 'layout', 'Container',
        (ctx, p) => FlintAccordion.fromProps(p));
    _reg('modal', 'layout', 'Overlay', (ctx, p) => FlintModal.fromProps(p));
    _reg('drawer', 'layout', 'Overlay', (ctx, p) => FlintDrawer.fromProps(p));

    // Data Display
    _reg('data-grid', 'data-display', 'Table',
        (ctx, p) => FlintDataGrid.fromProps(p));
    _reg('chart', 'data-display', 'Chart',
        (ctx, p) => FlintChart.fromProps(p));
    _reg('timeline', 'data-display', 'List',
        (ctx, p) => FlintTimeline.fromProps(p));
    _reg('kanban', 'data-display', 'Board',
        (ctx, p) => FlintKanban.fromProps(p));
    _reg('metric', 'data-display', 'Metric',
        (ctx, p) => FlintMetric.fromProps(p));
    _reg('badge', 'data-display', 'Badge',
        (ctx, p) => FlintBadge.fromProps(p));

    // Input
    _reg('form', 'input', 'Form', (ctx, p) => FlintForm.fromProps(p));
    _reg('text-field', 'input', 'Input',
        (ctx, p) => FlintTextField.fromProps(p));
    _reg('select', 'input', 'Select', (ctx, p) => FlintSelect.fromProps(p));
    _reg('search', 'input', 'Input', (ctx, p) => FlintSearch.fromProps(p));
    _reg('date-picker', 'input', 'Input',
        (ctx, p) => FlintDatePicker.fromProps(p));
    _reg('file-upload', 'input', 'Input',
        (ctx, p) => FlintFileUpload.fromProps(p));

    // Action
    _reg('button', 'action', 'Button',
        (ctx, p) => FlintButton.fromProps(p));
    _reg('confirm', 'action', 'Dialog',
        (ctx, p) => FlintConfirm.fromProps(p));
    _reg('wizard', 'action', 'MultiStep',
        (ctx, p) => FlintWizard.fromProps(p));
    _reg('bulk-action', 'action', 'Toolbar',
        (ctx, p) => FlintBulkAction.fromProps(p));
    _reg('action-bar', 'action', 'Toolbar',
        (ctx, p) => FlintActionBar.fromProps(p));

    // Agent
    _reg('agent-chat', 'agent', 'Chat',
        (ctx, p) => FlintAgentChat.fromProps(p));
    _reg('tool-call', 'agent', 'ToolCall',
        (ctx, p) => FlintToolCall.fromProps(p));
    _reg('streaming-text', 'agent', 'Text',
        (ctx, p) => FlintStreamingText.fromProps(p));
    _reg('decision', 'agent', 'Decision',
        (ctx, p) => FlintDecision.fromProps(p));
    _reg('progress-log', 'agent', 'Log',
        (ctx, p) => FlintProgressLog.fromProps(p));
    _reg('artifact', 'agent', 'Artifact',
        (ctx, p) => FlintArtifact.fromProps(p));

    // Navigation
    _reg('nav-menu', 'navigation', 'Navigation',
        (ctx, p) => FlintNavMenu.fromProps(p));
    _reg('command-palette', 'navigation', 'Search',
        (ctx, p) => FlintCommandPalette.fromProps(p));
    _reg('filter-bar', 'navigation', 'Filter',
        (ctx, p) => FlintFilterBar.fromProps(p));
    _reg('breadcrumb', 'navigation', 'Navigation',
        (ctx, p) => FlintBreadcrumb.fromProps(p));
  }
}
