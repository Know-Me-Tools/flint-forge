import 'package:flutter/material.dart';
import '../../tokens/flint_theme.dart';

class FlintTextField extends StatelessWidget {
  final String label;
  final String name;
  final String? value;
  final ValueChanged<String>? onChanged;
  final String? placeholder;
  final bool required;
  final String? error;
  final TextInputType? keyboardType;

  const FlintTextField({super.key, required this.label, required this.name, this.value, this.onChanged, this.placeholder, this.required = false, this.error, this.keyboardType});

  static Widget fromProps(Map<String, dynamic> p) {
    return FlintTextField(label: p['label'] as String? ?? '', name: p['name'] as String? ?? '', value: p['value'] as String?, placeholder: p['placeholder'] as String?, required: (p['required'] as bool?) ?? false);
  }

  @override
  Widget build(BuildContext context) {
    final theme = FlintThemeData.of(context);
    return Semantics(
      label: label,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(children: [
            Text('$label', style: theme.textBase.copyWith(fontWeight: FontWeight.w500)),
            if (required) Text(' *', style: TextStyle(color: theme.error)),
          ]),
          const SizedBox(height: 4),
          TextField(
            decoration: InputDecoration(
              hintText: placeholder,
              errorText: error,
              border: OutlineInputBorder(borderRadius: BorderRadius.circular(theme.radiusMd)),
              contentPadding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
            ),
            controller: value != null ? (TextEditingController(text: value)..selection = TextSelection.collapsed(offset: value!.length)) : null,
            onChanged: onChanged,
            keyboardType: keyboardType,
          ),
        ],
      ),
    );
  }
}

class _SelectOption { final String label; final String value; const _SelectOption(this.label, this.value); }

class FlintSelect extends StatefulWidget {
  final String label;
  final String name;
  final List<_SelectOption> options;
  final String? value;
  final ValueChanged<String?>? onChanged;

  const FlintSelect({super.key, required this.label, required this.name, required this.options, this.value, this.onChanged});

  static Widget fromProps(Map<String, dynamic> p) {
    final opts = ((p['options'] as List<dynamic>?) ?? []).cast<Map<String, dynamic>>().map((o) => _SelectOption(o['label'] as String? ?? '', o['value'] as String? ?? '')).toList();
    return FlintSelect(label: p['label'] as String? ?? '', name: p['name'] as String? ?? '', options: opts, value: p['value'] as String?);
  }

  @override
  State<FlintSelect> createState() => _FlintSelectState();
}

class _FlintSelectState extends State<FlintSelect> {
  String? _value;

  @override
  void initState() { super.initState(); _value = widget.value; }

  @override
  Widget build(BuildContext context) {
    final theme = FlintThemeData.of(context);
    return Semantics(
      label: widget.label,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(widget.label, style: theme.textBase.copyWith(fontWeight: FontWeight.w500)),
          const SizedBox(height: 4),
          DropdownButtonFormField<String>(
            value: _value,
            decoration: InputDecoration(border: OutlineInputBorder(borderRadius: BorderRadius.circular(theme.radiusMd)), contentPadding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10)),
            items: widget.options.map((o) => DropdownMenuItem(value: o.value, child: Text(o.label))).toList(),
            onChanged: (v) { setState(() => _value = v); widget.onChanged?.call(v); },
          ),
        ],
      ),
    );
  }
}

class FlintDatePicker extends StatelessWidget {
  final String label;
  final String name;
  final String? value;
  final ValueChanged<String>? onChanged;
  final bool required;

  const FlintDatePicker({super.key, required this.label, required this.name, this.value, this.onChanged, this.required = false});

  static Widget fromProps(Map<String, dynamic> p) {
    return FlintDatePicker(label: p['label'] as String? ?? '', name: p['name'] as String? ?? '', value: p['value'] as String?, required: (p['required'] as bool?) ?? false);
  }

  @override
  Widget build(BuildContext context) {
    return FlintTextField(label: label, name: name, value: value, onChanged: onChanged, required: required, keyboardType: TextInputType.datetime, placeholder: 'YYYY-MM-DD');
  }
}

class FlintSearch extends StatelessWidget {
  final String? value;
  final ValueChanged<String>? onChanged;
  final String placeholder;

  const FlintSearch({super.key, this.value, this.onChanged, this.placeholder = 'Search…'});

  static Widget fromProps(Map<String, dynamic> p) {
    return FlintSearch(value: p['value'] as String?, placeholder: (p['placeholder'] as String?) ?? 'Search…');
  }

  @override
  Widget build(BuildContext context) {
    final theme = FlintThemeData.of(context);
    return Semantics(
      label: placeholder,
      child: TextField(
        decoration: InputDecoration(
          hintText: placeholder,
          prefixIcon: const Icon(Icons.search),
          border: OutlineInputBorder(borderRadius: BorderRadius.circular(999)),
          contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
        ),
        controller: value != null ? TextEditingController(text: value) : null,
        onChanged: onChanged,
      ),
    );
  }
}

class FlintFileUpload extends StatelessWidget {
  final String label;
  final String name;
  final String? accept;
  final bool multiple;

  const FlintFileUpload({super.key, required this.label, required this.name, this.accept, this.multiple = false});

  static Widget fromProps(Map<String, dynamic> p) {
    return FlintFileUpload(label: p['label'] as String? ?? '', name: p['name'] as String? ?? '', accept: p['accept'] as String?, multiple: (p['multiple'] as bool?) ?? false);
  }

  @override
  Widget build(BuildContext context) {
    final theme = FlintThemeData.of(context);
    return Semantics(
      label: label,
      button: true,
      child: DottedBorderBox(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            const Icon(Icons.upload, size: 32),
            const SizedBox(height: 8),
            Text(label, style: theme.textBase),
            Text('Tap to upload', style: theme.textSm.copyWith(color: theme.muted)),
          ],
        ),
      ),
    );
  }
}

class DottedBorderBox extends StatelessWidget {
  final Widget child;
  const DottedBorderBox({super.key, required this.child});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(32),
      decoration: BoxDecoration(border: Border.all(color: Colors.grey.shade400, style: BorderStyle.solid), borderRadius: BorderRadius.circular(8)),
      child: child,
    );
  }
}

class FlintForm extends StatefulWidget {
  final List<Map<String, dynamic>> fields;
  final ValueChanged<Map<String, String>>? onSubmit;
  final String submitLabel;

  const FlintForm({super.key, required this.fields, this.onSubmit, this.submitLabel = 'Submit'});

  static Widget fromProps(Map<String, dynamic> p) {
    return FlintForm(fields: ((p['fields'] as List<dynamic>?) ?? []).cast<Map<String, dynamic>>(), submitLabel: (p['submitLabel'] as String?) ?? 'Submit');
  }

  @override
  State<FlintForm> createState() => _FlintFormState();
}

class _FlintFormState extends State<FlintForm> {
  final _values = <String, String>{};

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        ...widget.fields.map((field) {
          final name = field['name'] as String? ?? '';
          final label = field['label'] as String? ?? '';
          final type = field['type'] as String? ?? 'text';
          if (type == 'select') {
            return Padding(
              padding: const EdgeInsets.only(bottom: 16),
              child: FlintSelect(
                label: label, name: name,
                options: ((field['options'] as List<dynamic>?) ?? []).cast<Map<String, dynamic>>().map((o) => _SelectOption(o['label'] as String, o['value'] as String)).toList(),
                value: _values[name],
                onChanged: (v) => setState(() => _values[name] = v ?? ''),
              ),
            );
          }
          return Padding(
            padding: const EdgeInsets.only(bottom: 16),
            child: FlintTextField(label: label, name: name, value: _values[name], onChanged: (v) => setState(() => _values[name] = v), required: (field['required'] as bool?) ?? false),
          );
        }),
        Align(
          alignment: Alignment.centerRight,
          child: FlintButton(label: widget.submitLabel, onPressed: () => widget.onSubmit?.call(_values)),
        ),
      ],
    );
  }
}
