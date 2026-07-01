import 'package:flutter/material.dart';
import '../../tokens/flint_theme.dart';

class FlintNavMenu extends StatelessWidget {
  final List<Map<String, dynamic>> items;
  final Axis orientation;
  final String? ariaLabel;

  const FlintNavMenu({super.key, required this.items, this.orientation = Axis.horizontal, this.ariaLabel});

  static Widget fromProps(Map<String, dynamic> p) {
    return FlintNavMenu(
      items: ((p['items'] as List<dynamic>?) ?? []).cast<Map<String, dynamic>>(),
      orientation: (p['orientation'] as String?) == 'vertical' ? Axis.vertical : Axis.horizontal,
      ariaLabel: p['ariaLabel'] as String?,
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = FlintThemeData.of(context);
    final children = items.map((item) {
      final active = (item['active'] as bool?) ?? false;
      return Semantics(
        selected: active,
        label: item['label'] as String?,
        child: InkWell(
          onTap: () {},
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
            child: Text(item['label'] as String? ?? '', style: theme.textBase.copyWith(color: active ? theme.primary : theme.textColor, fontWeight: active ? FontWeight.w600 : FontWeight.normal)),
          ),
        ),
      );
    }).toList();
    return Semantics(
      label: ariaLabel ?? 'Navigation',
      child: orientation == Axis.horizontal ? Row(children: children) : Column(crossAxisAlignment: CrossAxisAlignment.start, children: children),
    );
  }
}

class FlintCommandPalette extends StatefulWidget {
  final bool open;
  final VoidCallback onClose;
  final List<Map<String, dynamic>> commands;
  final String placeholder;

  const FlintCommandPalette({super.key, required this.open, required this.onClose, required this.commands, this.placeholder = 'Search commands…'});

  static Widget fromProps(Map<String, dynamic> p) {
    return FlintCommandPalette(open: (p['open'] as bool?) ?? false, onClose: () {}, commands: ((p['commands'] as List<dynamic>?) ?? []).cast<Map<String, dynamic>>(), placeholder: (p['placeholder'] as String?) ?? 'Search commands…');
  }

  @override
  State<FlintCommandPalette> createState() => _FlintCommandPaletteState();
}

class _FlintCommandPaletteState extends State<FlintCommandPalette> {
  final _controller = TextEditingController();
  String _query = '';

  @override
  Widget build(BuildContext context) {
    if (!widget.open) return const SizedBox.shrink();
    final theme = FlintThemeData.of(context);
    final filtered = widget.commands.where((c) => (c['label'] as String? ?? '').toLowerCase().contains(_query.toLowerCase())).toList();
    return Semantics(
      label: 'Command palette',
      child: GestureDetector(
        onTap: widget.onClose,
        child: Container(
          color: Colors.black54,
          alignment: Alignment.topCenter,
          padding: const EdgeInsets.only(top: 80),
          child: GestureDetector(
            onTap: () {},
            child: Container(
              width: 560,
              constraints: const BoxConstraints(maxHeight: 480),
              decoration: BoxDecoration(color: theme.surface, borderRadius: BorderRadius.circular(theme.radiusLg), boxShadow: [BoxShadow(color: Colors.black.withOpacity(0.2), blurRadius: 40)]),
              child: Column(
                children: [
                  Padding(
                    padding: const EdgeInsets.all(12),
                    child: TextField(
                      controller: _controller,
                      autofocus: true,
                      decoration: InputDecoration(hintText: widget.placeholder, border: InputBorder.none, prefixIcon: const Icon(Icons.search)),
                      onChanged: (v) => setState(() => _query = v),
                    ),
                  ),
                  Divider(height: 1, color: theme.border),
                  Expanded(
                    child: filtered.isEmpty
                        ? Center(child: Text('No results', style: theme.textBase.copyWith(color: theme.muted)))
                        : ListView.builder(
                            itemCount: filtered.length,
                            itemBuilder: (ctx, i) {
                              final cmd = filtered[i];
                              return ListTile(
                                title: Semantics(button: true, label: cmd['label'] as String?, child: Text(cmd['label'] as String? ?? '')),
                                trailing: cmd['shortcut'] != null ? Container(padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2), decoration: BoxDecoration(border: Border.all(color: theme.border), borderRadius: BorderRadius.circular(4)), child: Text(cmd['shortcut'] as String, style: theme.textSm)) : null,
                                onTap: () { widget.onClose(); },
                              );
                            },
                          ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class FlintFilterBar extends StatelessWidget {
  final List<Map<String, dynamic>> filters;
  final Map<String, String> values;
  final void Function(String id, String val) onChange;
  final VoidCallback? onReset;

  const FlintFilterBar({super.key, required this.filters, required this.values, required this.onChange, this.onReset});

  static Widget fromProps(Map<String, dynamic> p) {
    return FlintFilterBar(
      filters: ((p['filters'] as List<dynamic>?) ?? []).cast<Map<String, dynamic>>(),
      values: (p['values'] as Map?)?.cast<String, String>() ?? {},
      onChange: (_, __) {},
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = FlintThemeData.of(context);
    return Semantics(
      label: 'Filters',
      child: Wrap(
        spacing: 12,
        runSpacing: 8,
        crossAxisAlignment: WrapCrossAlignment.end,
        children: [
          ...filters.map((filter) {
            final id = filter['id'] as String? ?? '';
            return SizedBox(
              width: 160,
              child: TextField(
                decoration: InputDecoration(labelText: filter['label'] as String? ?? '', border: OutlineInputBorder(borderRadius: BorderRadius.circular(theme.radiusMd)), contentPadding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8)),
                controller: TextEditingController(text: values[id] ?? ''),
                onChanged: (v) => onChange(id, v),
              ),
            );
          }),
          if (onReset != null)
            TextButton(onPressed: onReset, child: const Text('Reset')),
        ],
      ),
    );
  }
}

class FlintBreadcrumb extends StatelessWidget {
  final List<Map<String, dynamic>> items;

  const FlintBreadcrumb({super.key, required this.items});

  static Widget fromProps(Map<String, dynamic> p) {
    return FlintBreadcrumb(items: ((p['items'] as List<dynamic>?) ?? []).cast<Map<String, dynamic>>());
  }

  @override
  Widget build(BuildContext context) {
    final theme = FlintThemeData.of(context);
    final widgets = <Widget>[];
    for (int i = 0; i < items.length; i++) {
      if (i > 0) widgets.add(Padding(padding: const EdgeInsets.symmetric(horizontal: 6), child: Text('/', style: theme.textSm.copyWith(color: theme.muted))));
      final isLast = i == items.length - 1;
      widgets.add(Semantics(
        currentPage: isLast,
        label: items[i]['label'] as String?,
        child: Text(items[i]['label'] as String? ?? '', style: theme.textSm.copyWith(color: isLast ? theme.textColor : theme.primary, fontWeight: isLast ? FontWeight.w600 : FontWeight.normal)),
      ));
    }
    return Semantics(
      label: 'Breadcrumb',
      child: Row(children: widgets),
    );
  }
}
