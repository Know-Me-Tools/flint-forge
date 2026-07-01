import 'package:flutter_test/flutter_test.dart';
import 'package:flint_genui/flint_genui.dart';

void main() {
  group('FlintCatalog', () {
    late FlintCatalog catalog;

    setUp(() {
      catalog = FlintCatalog.build();
    });

    test('registers all 40 base components', () {
      expect(catalog.size, greaterThanOrEqualTo(40));
    });

    test('resolves layout/stack', () {
      final builder = catalog.resolve('layout/stack');
      expect(builder, isNotNull);
    });

    test('resolves action/button', () {
      final builder = catalog.resolve('action/button');
      expect(builder, isNotNull);
    });

    test('resolves agent/agent-chat', () {
      final builder = catalog.resolve('agent/agent-chat');
      expect(builder, isNotNull);
    });

    test('resolves navigation/breadcrumb', () {
      final builder = catalog.resolve('navigation/breadcrumb');
      expect(builder, isNotNull);
    });

    test('returns null for unknown slug', () {
      expect(catalog.resolve('unknown/slug'), isNull);
    });

    test('build with overrides replaces component', () {
      final customBuilder = (Map<String, dynamic> p) => const FlintBadge(label: 'custom', variant: 'success');
      final withOverride = FlintCatalog.build(overrides: {'action/button': customBuilder});
      expect(withOverride.resolve('action/button'), equals(customBuilder));
    });
  });

  group('FlintComponentSchemas', () {
    test('contains 40 schemas', () {
      expect(FlintComponentSchemas.all.length, greaterThanOrEqualTo(40));
    });

    test('all schemas have required fields', () {
      for (final schema in FlintComponentSchemas.all) {
        expect(schema['slug'], isNotNull, reason: 'Missing slug in ${schema['name']}');
        expect(schema['type'], equals('object'));
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
      final widget = FlintButton.fromProps({'label': 'Click me', 'variant': 'primary'});
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

    testWidgets('FlintStreamingText shows cursor when streaming', (tester) async {
      final widget = FlintStreamingText.fromProps({'text': 'Hello', 'streaming': true});
      await tester.pumpWidget(wrapWidget(widget));
      expect(find.textContaining('Hello'), findsOneWidget);
    });

    testWidgets('FlintMetric renders value and label', (tester) async {
      final widget = FlintMetric.fromProps({'label': 'Requests', 'value': 42, 'unit': 'rps', 'trend': 'up'});
      await tester.pumpWidget(wrapWidget(widget));
      expect(find.text('42'), findsOneWidget);
      expect(find.text('Requests'), findsOneWidget);
    });

    testWidgets('FlintNavMenu renders items horizontally', (tester) async {
      final widget = FlintNavMenu.fromProps({
        'items': [{'label': 'Home', 'active': true}, {'label': 'About'}],
      });
      await tester.pumpWidget(wrapWidget(widget));
      expect(find.text('Home'), findsOneWidget);
      expect(find.text('About'), findsOneWidget);
    });
  });
}

Widget wrapWidget(Widget child) {
  return MaterialApp(
    home: Scaffold(
      body: Theme(
        data: ThemeData.light().copyWith(
          extensions: const [
            // FlintThemeData injected as ThemeExtension
          ],
        ),
        child: Builder(
          builder: (context) {
            return child;
          },
        ),
      ),
    ),
  );
}
