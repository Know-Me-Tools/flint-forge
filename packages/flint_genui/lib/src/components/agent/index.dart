import 'package:flutter/material.dart';
import '../../tokens/flint_theme.dart';
import '../action/index.dart';

class FlintAgentChat extends StatefulWidget {
  final List<Map<String, dynamic>> messages;
  final bool loading;
  final ValueChanged<String>? onSend;

  const FlintAgentChat({super.key, required this.messages, this.loading = false, this.onSend});

  static Widget fromProps(Map<String, dynamic> p) {
    return FlintAgentChat(messages: ((p['messages'] as List<dynamic>?) ?? []).cast<Map<String, dynamic>>(), loading: (p['loading'] as bool?) ?? false);
  }

  @override
  State<FlintAgentChat> createState() => _FlintAgentChatState();
}

class _FlintAgentChatState extends State<FlintAgentChat> {
  final _controller = TextEditingController();
  final _scrollController = ScrollController();

  @override
  void didUpdateWidget(FlintAgentChat oldWidget) {
    super.didUpdateWidget(oldWidget);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (_scrollController.hasClients) {
        _scrollController.animateTo(_scrollController.position.maxScrollExtent, duration: const Duration(milliseconds: 300), curve: Curves.easeOut);
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final theme = FlintThemeData.of(context);
    return Semantics(
      label: 'Chat',
      child: Column(
        children: [
          Expanded(
            child: ListView.separated(
              controller: _scrollController,
              padding: const EdgeInsets.all(16),
              itemCount: widget.messages.length + (widget.loading ? 1 : 0),
              separatorBuilder: (_, __) => const SizedBox(height: 12),
              itemBuilder: (ctx, i) {
                if (i == widget.messages.length) {
                  return const Align(alignment: Alignment.centerLeft, child: Padding(padding: EdgeInsets.only(left: 12), child: SizedBox.square(dimension: 20, child: CircularProgressIndicator(strokeWidth: 2))));
                }
                final msg = widget.messages[i];
                final isUser = (msg['role'] as String?) == 'user';
                return Align(
                  alignment: isUser ? Alignment.centerRight : Alignment.centerLeft,
                  child: Semantics(
                    label: '${msg['role']}: ${msg['content']}',
                    child: Container(
                      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
                      constraints: const BoxConstraints(maxWidth: 320),
                      decoration: BoxDecoration(
                        color: isUser ? theme.primary : theme.surface,
                        borderRadius: BorderRadius.circular(20),
                        border: !isUser ? Border.all(color: theme.border) : null,
                      ),
                      child: Text(msg['content'] as String? ?? '', style: theme.textBase.copyWith(color: isUser ? Colors.white : theme.textColor)),
                    ),
                  ),
                );
              },
            ),
          ),
          if (widget.onSend != null)
            Padding(
              padding: const EdgeInsets.all(12),
              child: Row(
                children: [
                  Expanded(
                    child: TextField(
                      controller: _controller,
                      decoration: InputDecoration(hintText: 'Type a message…', border: OutlineInputBorder(borderRadius: BorderRadius.circular(999)), contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10)),
                      onSubmitted: (text) { if (text.trim().isNotEmpty) { widget.onSend!(text.trim()); _controller.clear(); } },
                    ),
                  ),
                  const SizedBox(width: 8),
                  FlintButton(label: 'Send', onPressed: () { final text = _controller.text.trim(); if (text.isNotEmpty) { widget.onSend!(text); _controller.clear(); } }),
                ],
              ),
            ),
        ],
      ),
    );
  }
}

class FlintToolCall extends StatelessWidget {
  final String name;
  final String status;
  final String? args;
  final String? result;

  const FlintToolCall({super.key, required this.name, required this.status, this.args, this.result});

  static Widget fromProps(Map<String, dynamic> p) {
    return FlintToolCall(name: p['name'] as String? ?? '', status: p['status'] as String? ?? 'pending', args: p['args'] as String?, result: p['result'] as String?);
  }

  @override
  Widget build(BuildContext context) {
    final theme = FlintThemeData.of(context);
    return Semantics(
      label: 'Tool call $name: $status',
      child: Container(
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(border: Border(left: BorderSide(color: theme.primary, width: 3)), color: theme.surface.withOpacity(0.8)),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(children: [
              Text(name, style: theme.textBase.copyWith(fontWeight: FontWeight.w700, fontFamily: 'monospace')),
              const SizedBox(width: 8),
              Container(padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2), decoration: BoxDecoration(color: theme.border, borderRadius: BorderRadius.circular(4)), child: Text(status, style: theme.textSm)),
            ]),
            if (args != null) Padding(padding: const EdgeInsets.only(top: 8), child: Text(args!, style: theme.textSm.copyWith(fontFamily: 'monospace'))),
            if (result != null) Padding(
              padding: const EdgeInsets.only(top: 4),
              child: Text(result!, style: theme.textSm.copyWith(fontFamily: 'monospace', color: status == 'error' ? theme.error : theme.success)),
            ),
          ],
        ),
      ),
    );
  }
}

class FlintStreamingText extends StatelessWidget {
  final String text;
  final bool streaming;

  const FlintStreamingText({super.key, required this.text, this.streaming = false});

  static Widget fromProps(Map<String, dynamic> p) {
    return FlintStreamingText(text: p['text'] as String? ?? '', streaming: (p['streaming'] as bool?) ?? false);
  }

  @override
  Widget build(BuildContext context) {
    final theme = FlintThemeData.of(context);
    return Semantics(
      liveRegion: streaming,
      label: text,
      child: RichText(
        text: TextSpan(
          text: text,
          style: theme.textBase.copyWith(color: theme.textColor),
          children: streaming ? [const TextSpan(text: '▍', style: TextStyle(color: Colors.blue))] : [],
        ),
      ),
    );
  }
}

class FlintDecision extends StatelessWidget {
  final String question;
  final List<Map<String, dynamic>> options;
  final ValueChanged<String>? onSelect;

  const FlintDecision({super.key, required this.question, required this.options, this.onSelect});

  static Widget fromProps(Map<String, dynamic> p) {
    return FlintDecision(question: p['question'] as String? ?? '', options: ((p['options'] as List<dynamic>?) ?? []).cast<Map<String, dynamic>>());
  }

  @override
  Widget build(BuildContext context) {
    final theme = FlintThemeData.of(context);
    return Semantics(
      label: question,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(question, style: theme.textBase.copyWith(fontWeight: FontWeight.w600)),
          const SizedBox(height: 12),
          ...options.map((opt) => Padding(
            padding: const EdgeInsets.only(bottom: 8),
            child: Semantics(
              button: true,
              label: opt['label'] as String?,
              child: InkWell(
                onTap: () => onSelect?.call(opt['id'] as String? ?? ''),
                borderRadius: BorderRadius.circular(theme.radiusMd),
                child: Container(
                  padding: const EdgeInsets.all(16),
                  decoration: BoxDecoration(border: Border.all(color: theme.border), borderRadius: BorderRadius.circular(theme.radiusMd)),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(opt['label'] as String? ?? '', style: theme.textBase.copyWith(fontWeight: FontWeight.w600)),
                      if (opt['description'] != null) Text(opt['description'] as String, style: theme.textSm.copyWith(color: theme.muted)),
                    ],
                  ),
                ),
              ),
            ),
          )),
        ],
      ),
    );
  }
}

class FlintProgressLog extends StatelessWidget {
  final List<Map<String, dynamic>> entries;
  final String? title;

  const FlintProgressLog({super.key, required this.entries, this.title});

  static Widget fromProps(Map<String, dynamic> p) {
    return FlintProgressLog(entries: ((p['entries'] as List<dynamic>?) ?? []).cast<Map<String, dynamic>>(), title: p['title'] as String?);
  }

  @override
  Widget build(BuildContext context) {
    final theme = FlintThemeData.of(context);
    return Semantics(
      liveRegion: true,
      label: title ?? 'Progress log',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          if (title != null) Text(title!, style: theme.textBase.copyWith(fontWeight: FontWeight.w600)),
          ...entries.map((entry) {
            final level = entry['level'] as String? ?? 'info';
            final color = level == 'error' ? theme.error : level == 'warn' ? Colors.orange : theme.muted;
            return Padding(
              padding: const EdgeInsets.symmetric(vertical: 2),
              child: Row(children: [
                if (entry['timestamp'] != null) Text(entry['timestamp'] as String, style: theme.textSm.copyWith(color: theme.muted, fontFamily: 'monospace')),
                const SizedBox(width: 8),
                Expanded(child: Text(entry['message'] as String? ?? '', style: theme.textSm.copyWith(color: color, fontFamily: 'monospace'))),
              ]),
            );
          }),
        ],
      ),
    );
  }
}

class FlintArtifact extends StatelessWidget {
  final String type;
  final String content;
  final String? language;
  final String? filename;

  const FlintArtifact({super.key, required this.type, required this.content, this.language, this.filename});

  static Widget fromProps(Map<String, dynamic> p) {
    return FlintArtifact(type: p['type'] as String? ?? 'text', content: p['content'] as String? ?? '', language: p['language'] as String?, filename: p['filename'] as String?);
  }

  @override
  Widget build(BuildContext context) {
    final theme = FlintThemeData.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (filename != null) Text(filename!, style: theme.textSm.copyWith(color: theme.muted)),
        type == 'code'
            ? Container(
                padding: const EdgeInsets.all(16),
                decoration: BoxDecoration(color: const Color(0xFF1E1E1E), borderRadius: BorderRadius.circular(theme.radiusMd)),
                child: Semantics(label: 'Code: ${language ?? ''}\n$content', child: SelectableText(content, style: theme.textSm.copyWith(color: Colors.white70, fontFamily: 'monospace'))),
              )
            : Container(
                padding: const EdgeInsets.all(16),
                decoration: BoxDecoration(color: theme.surface, border: Border.all(color: theme.border), borderRadius: BorderRadius.circular(theme.radiusMd)),
                child: Text(content, style: theme.textBase),
              ),
      ],
    );
  }
}
