import 'package:flutter/material.dart';
import '../../tokens/flint_theme.dart';

enum FlintButtonVariant { primary, secondary, ghost, destructive }
enum FlintButtonSize { sm, md, lg }

class FlintButton extends StatelessWidget {
  final String label;
  final VoidCallback? onPressed;
  final FlintButtonVariant variant;
  final FlintButtonSize size;
  final bool loading;
  final bool disabled;

  const FlintButton({
    super.key,
    required this.label,
    this.onPressed,
    this.variant = FlintButtonVariant.primary,
    this.size = FlintButtonSize.md,
    this.loading = false,
    this.disabled = false,
  });

  static Widget fromProps(Map<String, dynamic> p) {
    return FlintButton(
      label: p['label'] as String? ?? '',
      loading: (p['loading'] as bool?) ?? false,
      disabled: (p['disabled'] as bool?) ?? false,
      variant: _parseVariant(p['variant'] as String?),
      size: _parseSize(p['size'] as String?),
    );
  }

  static FlintButtonVariant _parseVariant(String? v) {
    switch (v) {
      case 'secondary': return FlintButtonVariant.secondary;
      case 'ghost': return FlintButtonVariant.ghost;
      case 'destructive': return FlintButtonVariant.destructive;
      default: return FlintButtonVariant.primary;
    }
  }

  static FlintButtonSize _parseSize(String? s) {
    switch (s) {
      case 'sm': return FlintButtonSize.sm;
      case 'lg': return FlintButtonSize.lg;
      default: return FlintButtonSize.md;
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = FlintThemeData.of(context);
    final isDisabled = disabled || loading;
    final bgColor = variant == FlintButtonVariant.primary ? theme.primary
        : variant == FlintButtonVariant.destructive ? theme.error
        : Colors.transparent;
    final fgColor = variant == FlintButtonVariant.ghost || variant == FlintButtonVariant.secondary ? theme.textColor : Colors.white;
    final padding = size == FlintButtonSize.sm ? const EdgeInsets.symmetric(horizontal: 12, vertical: 6)
        : size == FlintButtonSize.lg ? const EdgeInsets.symmetric(horizontal: 32, vertical: 16)
        : const EdgeInsets.symmetric(horizontal: 16, vertical: 10);

    return Semantics(
      button: true,
      label: label,
      enabled: !isDisabled,
      child: Opacity(
        opacity: isDisabled ? 0.6 : 1.0,
        child: GestureDetector(
          onTap: isDisabled ? null : onPressed,
          child: Container(
            padding: padding,
            decoration: BoxDecoration(
              color: bgColor,
              borderRadius: BorderRadius.circular(theme.radiusMd),
              border: variant == FlintButtonVariant.secondary ? Border.all(color: theme.border) : null,
            ),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                if (loading) ...[SizedBox.square(dimension: 14, child: CircularProgressIndicator(strokeWidth: 2, color: fgColor)), const SizedBox(width: 8)],
                Text(label, style: TextStyle(color: fgColor, fontWeight: FontWeight.w600)),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class FlintConfirm extends StatelessWidget {
  final String message;
  final VoidCallback onConfirm;
  final VoidCallback onCancel;
  final String confirmLabel;
  final String cancelLabel;

  const FlintConfirm({super.key, required this.message, required this.onConfirm, required this.onCancel, this.confirmLabel = 'Confirm', this.cancelLabel = 'Cancel'});

  static Widget fromProps(Map<String, dynamic> p) {
    return FlintConfirm(message: p['message'] as String? ?? '', onConfirm: () {}, onCancel: () {}, confirmLabel: p['confirmLabel'] as String? ?? 'Confirm', cancelLabel: p['cancelLabel'] as String? ?? 'Cancel');
  }

  @override
  Widget build(BuildContext context) {
    final theme = FlintThemeData.of(context);
    return Semantics(
      label: message,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(message, style: theme.textBase),
          const SizedBox(height: 16),
          Row(
            mainAxisAlignment: MainAxisAlignment.end,
            children: [
              FlintButton(label: cancelLabel, onPressed: onCancel, variant: FlintButtonVariant.secondary),
              const SizedBox(width: 8),
              FlintButton(label: confirmLabel, onPressed: onConfirm, variant: FlintButtonVariant.destructive),
            ],
          ),
        ],
      ),
    );
  }
}

class FlintWizard extends StatefulWidget {
  final List<Map<String, dynamic>> steps;
  final VoidCallback? onComplete;

  const FlintWizard({super.key, required this.steps, this.onComplete});

  static Widget fromProps(Map<String, dynamic> p) {
    return FlintWizard(steps: ((p['steps'] as List<dynamic>?) ?? []).cast<Map<String, dynamic>>());
  }

  @override
  State<FlintWizard> createState() => _FlintWizardState();
}

class _FlintWizardState extends State<FlintWizard> {
  int _current = 0;

  @override
  Widget build(BuildContext context) {
    final theme = FlintThemeData.of(context);
    final isLast = _current == widget.steps.length - 1;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: List.generate(widget.steps.length, (i) {
            final step = widget.steps[i];
            final done = i < _current;
            final current = i == _current;
            return Expanded(
              child: Semantics(
                label: step['title'] as String? ?? '',
                selected: current,
                child: Column(
                  children: [
                    CircleAvatar(
                      radius: 12,
                      backgroundColor: done || current ? theme.primary : theme.border,
                      child: Text('${i + 1}', style: const TextStyle(color: Colors.white, fontSize: 10)),
                    ),
                    const SizedBox(height: 4),
                    Text(step['title'] as String? ?? '', style: theme.textSm, textAlign: TextAlign.center),
                  ],
                ),
              ),
            );
          }),
        ),
        const SizedBox(height: 24),
        const SizedBox.shrink(), // content placeholder
        const SizedBox(height: 16),
        Row(
          mainAxisAlignment: MainAxisAlignment.end,
          children: [
            if (_current > 0) FlintButton(label: 'Back', onPressed: () => setState(() => _current--), variant: FlintButtonVariant.secondary),
            const SizedBox(width: 8),
            if (!isLast) FlintButton(label: 'Next', onPressed: () => setState(() => _current++)),
            if (isLast) FlintButton(label: 'Complete', onPressed: widget.onComplete),
          ],
        ),
      ],
    );
  }
}

class FlintBulkAction extends StatelessWidget {
  final int selectedCount;
  final List<Map<String, dynamic>> actions;

  const FlintBulkAction({super.key, required this.selectedCount, required this.actions});

  static Widget fromProps(Map<String, dynamic> p) {
    return FlintBulkAction(
      selectedCount: (p['selectedCount'] as num?)?.toInt() ?? 0,
      actions: ((p['actions'] as List<dynamic>?) ?? []).cast<Map<String, dynamic>>(),
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = FlintThemeData.of(context);
    if (selectedCount == 0) return const SizedBox.shrink();
    return Semantics(
      label: '$selectedCount items selected',
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        decoration: BoxDecoration(color: theme.primary, borderRadius: BorderRadius.circular(theme.radiusMd)),
        child: Row(
          children: [
            Text('$selectedCount selected', style: const TextStyle(color: Colors.white)),
            const SizedBox(width: 16),
            ...actions.map((a) => Padding(
              padding: const EdgeInsets.only(right: 8),
              child: FlintButton(label: a['label'] as String? ?? '', variant: (a['destructive'] as bool?) == true ? FlintButtonVariant.destructive : FlintButtonVariant.ghost, size: FlintButtonSize.sm),
            )),
          ],
        ),
      ),
    );
  }
}

class FlintActionBar extends StatelessWidget {
  final List<Map<String, dynamic>> actions;

  const FlintActionBar({super.key, required this.actions});

  static Widget fromProps(Map<String, dynamic> p) {
    return FlintActionBar(actions: ((p['actions'] as List<dynamic>?) ?? []).cast<Map<String, dynamic>>());
  }

  @override
  Widget build(BuildContext context) {
    return Semantics(
      label: 'Action bar',
      child: Row(
        children: actions.map((a) => Padding(
          padding: const EdgeInsets.only(right: 8),
          child: FlintButton(label: a['label'] as String? ?? '', variant: FlintButtonVariant.ghost, size: FlintButtonSize.sm, disabled: (a['disabled'] as bool?) ?? false),
        )).toList(),
      ),
    );
  }
}
