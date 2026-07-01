import 'package:flutter/material.dart';
import '../../tokens/flint_theme.dart';

class FlintStack extends StatelessWidget {
  final List<Widget> children;
  final Axis direction;
  final double gap;
  final MainAxisAlignment mainAxisAlignment;

  const FlintStack({
    super.key,
    required this.children,
    this.direction = Axis.vertical,
    this.gap = 8.0,
    this.mainAxisAlignment = MainAxisAlignment.start,
  });

  static Widget fromProps(Map<String, dynamic> p) {
    return FlintStack(
      direction: (p['direction'] as String?) == 'horizontal' ? Axis.horizontal : Axis.vertical,
      gap: ((p['gap'] as num?) ?? 2) * 4.0,
      children: const [],
    );
  }

  @override
  Widget build(BuildContext context) {
    final gapped = <Widget>[];
    for (int i = 0; i < children.length; i++) {
      if (i > 0) gapped.add(SizedBox(width: direction == Axis.horizontal ? gap : 0, height: direction == Axis.vertical ? gap : 0));
      gapped.add(children[i]);
    }
    return direction == Axis.vertical
        ? Column(mainAxisAlignment: mainAxisAlignment, crossAxisAlignment: CrossAxisAlignment.stretch, children: gapped)
        : Row(mainAxisAlignment: mainAxisAlignment, children: gapped);
  }
}

class FlintCard extends StatelessWidget {
  final String? title;
  final bool elevated;
  final Widget? child;

  const FlintCard({super.key, this.title, this.elevated = false, this.child});

  static Widget fromProps(Map<String, dynamic> p) {
    return FlintCard(title: p['title'] as String?, elevated: (p['elevated'] as bool?) ?? false);
  }

  @override
  Widget build(BuildContext context) {
    final theme = FlintThemeData.of(context);
    return Semantics(
      container: true,
      label: title,
      child: Container(
        decoration: BoxDecoration(
          color: theme.surface,
          borderRadius: BorderRadius.circular(theme.radiusMd),
          border: Border.all(color: theme.border),
          boxShadow: elevated ? [BoxShadow(color: Colors.black.withOpacity(0.08), blurRadius: 24)] : null,
        ),
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            if (title != null) ...[
              Text(title!, style: theme.textLg.copyWith(fontWeight: FontWeight.w600)),
              const SizedBox(height: 12),
            ],
            if (child != null) child!,
          ],
        ),
      ),
    );
  }
}

class FlintGrid extends StatelessWidget {
  final int columns;
  final double gap;
  final List<Widget> children;

  const FlintGrid({super.key, this.columns = 3, this.gap = 8.0, required this.children});

  static Widget fromProps(Map<String, dynamic> p) {
    return FlintGrid(columns: (p['columns'] as num?)?.toInt() ?? 3, gap: ((p['gap'] as num?) ?? 2) * 4.0, children: const []);
  }

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(builder: (context, constraints) {
      final colWidth = (constraints.maxWidth - gap * (columns - 1)) / columns;
      return Wrap(spacing: gap, runSpacing: gap, children: children.map((c) => SizedBox(width: colWidth, child: c)).toList());
    });
  }
}

class FlintTabs extends StatefulWidget {
  final List<Map<String, dynamic>> items;
  final String? defaultValue;

  const FlintTabs({super.key, required this.items, this.defaultValue});

  static Widget fromProps(Map<String, dynamic> p) {
    return FlintTabs(items: ((p['items'] as List<dynamic>?) ?? []).cast<Map<String, dynamic>>(), defaultValue: p['defaultValue'] as String?);
  }

  @override
  State<FlintTabs> createState() => _FlintTabsState();
}

class _FlintTabsState extends State<FlintTabs> {
  late String _active;

  @override
  void initState() {
    super.initState();
    _active = widget.defaultValue ?? (widget.items.isNotEmpty ? widget.items[0]['value'] as String? ?? '' : '');
  }

  @override
  Widget build(BuildContext context) {
    final theme = FlintThemeData.of(context);
    final activeItem = widget.items.firstWhere((i) => i['value'] == _active, orElse: () => widget.items.isNotEmpty ? widget.items[0] : {});
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: widget.items.map((item) {
            final val = item['value'] as String? ?? '';
            final selected = val == _active;
            return Semantics(
              selected: selected,
              child: GestureDetector(
                onTap: () => setState(() => _active = val),
                child: Container(
                  padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                  decoration: BoxDecoration(border: Border(bottom: BorderSide(color: selected ? theme.primary : Colors.transparent, width: 2))),
                  child: Text(item['label'] as String? ?? '', style: theme.textBase.copyWith(color: selected ? theme.primary : theme.textColor, fontWeight: selected ? FontWeight.w600 : FontWeight.normal)),
                ),
              ),
            );
          }).toList(),
        ),
        const SizedBox(height: 12),
        if (activeItem.isNotEmpty) const SizedBox.shrink(), // content would come from children in real use
      ],
    );
  }
}

class FlintAccordion extends StatefulWidget {
  final List<Map<String, dynamic>> items;
  final bool allowMultiple;

  const FlintAccordion({super.key, required this.items, this.allowMultiple = false});

  static Widget fromProps(Map<String, dynamic> p) {
    return FlintAccordion(items: ((p['items'] as List<dynamic>?) ?? []).cast<Map<String, dynamic>>(), allowMultiple: (p['allowMultiple'] as bool?) ?? false);
  }

  @override
  State<FlintAccordion> createState() => _FlintAccordionState();
}

class _FlintAccordionState extends State<FlintAccordion> {
  final _open = <String>{};

  @override
  Widget build(BuildContext context) {
    final theme = FlintThemeData.of(context);
    return Column(
      children: widget.items.map((item) {
        final val = item['value'] as String? ?? '';
        final isOpen = _open.contains(val);
        return Semantics(
          expanded: isOpen,
          child: Column(
            children: [
              InkWell(
                onTap: () => setState(() {
                  if (!widget.allowMultiple) _open.clear();
                  isOpen ? _open.remove(val) : _open.add(val);
                }),
                child: Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
                  child: Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      Text(item['label'] as String? ?? '', style: theme.textBase),
                      Icon(isOpen ? Icons.expand_less : Icons.expand_more),
                    ],
                  ),
                ),
              ),
              if (isOpen)
                Padding(padding: const EdgeInsets.all(16), child: const SizedBox.shrink()),
              Divider(height: 1, color: theme.border),
            ],
          ),
        );
      }).toList(),
    );
  }
}

class FlintModal extends StatelessWidget {
  final bool open;
  final String? title;
  final VoidCallback? onClose;
  final Widget? child;

  const FlintModal({super.key, required this.open, this.title, this.onClose, this.child});

  static Widget fromProps(Map<String, dynamic> p) => FlintModal(open: (p['open'] as bool?) ?? false, title: p['title'] as String?);

  @override
  Widget build(BuildContext context) {
    if (!open) return const SizedBox.shrink();
    final theme = FlintThemeData.of(context);
    return Stack(
      children: [
        GestureDetector(onTap: onClose, child: Container(color: Colors.black54)),
        Center(
          child: Semantics(
            label: title,
            child: Material(
              borderRadius: BorderRadius.circular(theme.radiusLg),
              color: theme.surface,
              child: Container(
                padding: const EdgeInsets.all(24),
                constraints: const BoxConstraints(minWidth: 320, maxWidth: 560),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    if (title != null) Text(title!, style: theme.textLg.copyWith(fontWeight: FontWeight.w700)),
                    if (child != null) child!,
                  ],
                ),
              ),
            ),
          ),
        ),
      ],
    );
  }
}

class FlintDrawer extends StatelessWidget {
  final bool open;
  final String side;
  final Widget? child;
  final VoidCallback? onClose;

  const FlintDrawer({super.key, required this.open, this.side = 'right', this.child, this.onClose});

  static Widget fromProps(Map<String, dynamic> p) => FlintDrawer(open: (p['open'] as bool?) ?? false, side: (p['side'] as String?) ?? 'right');

  @override
  Widget build(BuildContext context) {
    if (!open) return const SizedBox.shrink();
    final theme = FlintThemeData.of(context);
    return Row(
      children: [
        if (side == 'right') Expanded(child: GestureDetector(onTap: onClose, child: Container(color: Colors.black38))),
        Container(width: 320, color: theme.surface, padding: const EdgeInsets.all(24), child: child ?? const SizedBox.shrink()),
        if (side == 'left') Expanded(child: GestureDetector(onTap: onClose, child: Container(color: Colors.black38))),
      ],
    );
  }
}
