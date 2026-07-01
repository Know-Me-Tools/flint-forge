import 'package:flutter/material.dart';
import '../../tokens/flint_theme.dart';

class FlintDataGrid extends StatelessWidget {
  final List<Map<String, dynamic>> columns;
  final List<Map<String, dynamic>> data;
  final bool loading;

  const FlintDataGrid({super.key, required this.columns, required this.data, this.loading = false});

  static Widget fromProps(Map<String, dynamic> p) {
    return FlintDataGrid(
      columns: ((p['columns'] as List<dynamic>?) ?? []).cast<Map<String, dynamic>>(),
      data: ((p['data'] as List<dynamic>?) ?? []).cast<Map<String, dynamic>>(),
      loading: (p['loading'] as bool?) ?? false,
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = FlintThemeData.of(context);
    if (loading) return const Center(child: CircularProgressIndicator());
    return Semantics(
      label: 'Data grid',
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: DataTable(
          columns: columns.map((col) => DataColumn(label: Semantics(header: true, child: Text(col['header'] as String? ?? '', style: theme.textBase.copyWith(fontWeight: FontWeight.w700))))).toList(),
          rows: data.map((row) => DataRow(
            cells: columns.map((col) {
              final field = col['field'] as String? ?? '';
              return DataCell(Text(row[field]?.toString() ?? ''));
            }).toList(),
          )).toList(),
        ),
      ),
    );
  }
}

class FlintChart extends StatelessWidget {
  final String type;
  final List<Map<String, dynamic>> data;
  final String? title;

  const FlintChart({super.key, this.type = 'bar', required this.data, this.title});

  static Widget fromProps(Map<String, dynamic> p) {
    return FlintChart(
      type: (p['type'] as String?) ?? 'bar',
      data: ((p['data'] as List<dynamic>?) ?? []).cast<Map<String, dynamic>>(),
      title: p['title'] as String?,
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = FlintThemeData.of(context);
    final maxValue = data.map((d) => (d['value'] as num?)?.toDouble() ?? 0).fold(0.0, (a, b) => a > b ? a : b);
    return Semantics(
      label: title ?? 'Chart',
      child: SizedBox(
        height: 160,
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.end,
          children: data.map((item) {
            final value = (item['value'] as num?)?.toDouble() ?? 0;
            final fraction = maxValue > 0 ? value / maxValue : 0.0;
            return Expanded(
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 2),
                child: Column(
                  mainAxisAlignment: MainAxisAlignment.end,
                  children: [
                    Flexible(
                      child: FractionallySizedBox(
                        heightFactor: fraction,
                        child: Semantics(
                          label: '${item['label']}: $value',
                          child: Container(
                            decoration: BoxDecoration(color: theme.primary, borderRadius: BorderRadius.vertical(top: Radius.circular(theme.radiusSm))),
                          ),
                        ),
                      ),
                    ),
                    const SizedBox(height: 4),
                    Text(item['label'] as String? ?? '', style: theme.textSm, textAlign: TextAlign.center, overflow: TextOverflow.ellipsis),
                  ],
                ),
              ),
            );
          }).toList(),
        ),
      ),
    );
  }
}

class FlintTimeline extends StatelessWidget {
  final List<Map<String, dynamic>> events;

  const FlintTimeline({super.key, required this.events});

  static Widget fromProps(Map<String, dynamic> p) {
    return FlintTimeline(events: ((p['events'] as List<dynamic>?) ?? []).cast<Map<String, dynamic>>());
  }

  @override
  Widget build(BuildContext context) {
    final theme = FlintThemeData.of(context);
    return Column(
      children: events.map((event) => Padding(
        padding: const EdgeInsets.symmetric(vertical: 8),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            SizedBox(
              width: 80,
              child: Text(event['timestamp'] as String? ?? '', style: theme.textSm.copyWith(color: theme.muted)),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Semantics(label: event['label'] as String?, child: Text(event['label'] as String? ?? '', style: theme.textBase.copyWith(fontWeight: FontWeight.w600))),
                  if (event['description'] != null) Text(event['description'] as String, style: theme.textSm.copyWith(color: theme.muted)),
                ],
              ),
            ),
          ],
        ),
      )).toList(),
    );
  }
}

class FlintKanban extends StatelessWidget {
  final List<Map<String, dynamic>> columns;

  const FlintKanban({super.key, required this.columns});

  static Widget fromProps(Map<String, dynamic> p) {
    return FlintKanban(columns: ((p['columns'] as List<dynamic>?) ?? []).cast<Map<String, dynamic>>());
  }

  @override
  Widget build(BuildContext context) {
    final theme = FlintThemeData.of(context);
    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: columns.map((col) {
          final cards = ((col['cards'] as List<dynamic>?) ?? []).cast<Map<String, dynamic>>();
          return Semantics(
            label: col['title'] as String?,
            child: Container(
              width: 240,
              margin: const EdgeInsets.only(right: 12),
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(color: theme.surface, borderRadius: BorderRadius.circular(theme.radiusMd), border: Border.all(color: theme.border)),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(col['title'] as String? ?? '', style: theme.textBase.copyWith(fontWeight: FontWeight.w700)),
                  const SizedBox(height: 8),
                  ...cards.map((card) => Container(
                    margin: const EdgeInsets.only(bottom: 8),
                    padding: const EdgeInsets.all(8),
                    decoration: BoxDecoration(color: Colors.white, borderRadius: BorderRadius.circular(theme.radiusSm), boxShadow: [BoxShadow(color: Colors.black.withOpacity(0.06), blurRadius: 4)]),
                    child: Text(card['title'] as String? ?? '', style: theme.textSm),
                  )),
                ],
              ),
            ),
          );
        }).toList(),
      ),
    );
  }
}

class FlintMetric extends StatelessWidget {
  final String label;
  final dynamic value;
  final String? unit;
  final String? trend;

  const FlintMetric({super.key, required this.label, required this.value, this.unit, this.trend});

  static Widget fromProps(Map<String, dynamic> p) {
    return FlintMetric(label: p['label'] as String? ?? '', value: p['value'], unit: p['unit'] as String?, trend: p['trend'] as String?);
  }

  @override
  Widget build(BuildContext context) {
    final theme = FlintThemeData.of(context);
    final trendIcon = trend == 'up' ? '↑' : trend == 'down' ? '↓' : trend == 'flat' ? '→' : null;
    return Semantics(
      label: label,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(label, style: theme.textSm.copyWith(color: theme.muted)),
          Row(
            children: [
              Text('$value', style: theme.textLg.copyWith(fontWeight: FontWeight.w700, fontSize: 28)),
              if (unit != null) Text(' $unit', style: theme.textBase.copyWith(color: theme.muted)),
              if (trendIcon != null) Semantics(label: 'Trend: $trend', child: Text(' $trendIcon', style: theme.textBase)),
            ],
          ),
        ],
      ),
    );
  }
}

class FlintBadge extends StatelessWidget {
  final String label;
  final String variant;

  const FlintBadge({super.key, required this.label, this.variant = 'default'});

  static Widget fromProps(Map<String, dynamic> p) {
    return FlintBadge(label: p['label'] as String? ?? '', variant: (p['variant'] as String?) ?? 'default');
  }

  @override
  Widget build(BuildContext context) {
    final theme = FlintThemeData.of(context);
    final color = switch (variant) {
      'success' => theme.success,
      'error' => theme.error,
      'warning' => Colors.orange,
      'info' => theme.primary,
      _ => theme.border,
    };
    return Semantics(
      label: label,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
        decoration: BoxDecoration(color: color.withOpacity(0.15), borderRadius: BorderRadius.circular(999), border: Border.all(color: color.withOpacity(0.5))),
        child: Text(label, style: theme.textSm.copyWith(color: color, fontWeight: FontWeight.w600)),
      ),
    );
  }
}
