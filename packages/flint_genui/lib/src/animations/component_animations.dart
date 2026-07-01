import 'package:flutter/material.dart';

class FlintFadeIn extends StatefulWidget {
  final Widget child;
  final Duration delay;
  final Duration duration;

  const FlintFadeIn({super.key, required this.child, this.delay = Duration.zero, this.duration = const Duration(milliseconds: 220)});

  @override
  State<FlintFadeIn> createState() => _FlintFadeInState();
}

class _FlintFadeInState extends State<FlintFadeIn> with SingleTickerProviderStateMixin {
  late final AnimationController _controller;
  late final Animation<double> _opacity;
  bool _started = false;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(vsync: this, duration: widget.duration);
    _opacity = CurvedAnimation(parent: _controller, curve: Curves.easeOut);
    if (widget.delay == Duration.zero) {
      _controller.forward();
      _started = true;
    } else {
      Future<void>.delayed(widget.delay, () { if (mounted) { _controller.forward(); setState(() => _started = true); } });
    }
  }

  @override
  void dispose() { _controller.dispose(); super.dispose(); }

  @override
  Widget build(BuildContext context) {
    if (!_started) return const SizedBox.shrink();
    return FadeTransition(opacity: _opacity, child: widget.child);
  }
}

class FlintPressEffect extends StatefulWidget {
  final Widget child;
  final VoidCallback? onTap;

  const FlintPressEffect({super.key, required this.child, this.onTap});

  @override
  State<FlintPressEffect> createState() => _FlintPressEffectState();
}

class _FlintPressEffectState extends State<FlintPressEffect> with SingleTickerProviderStateMixin {
  late final AnimationController _controller;
  late final Animation<double> _scale;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(vsync: this, duration: const Duration(milliseconds: 100), lowerBound: 0.95, upperBound: 1.0, value: 1.0);
    _scale = _controller;
  }

  @override
  void dispose() { _controller.dispose(); super.dispose(); }

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTapDown: (_) => _controller.reverse(),
      onTapUp: (_) { _controller.forward(); widget.onTap?.call(); },
      onTapCancel: () => _controller.forward(),
      child: ScaleTransition(scale: _scale, child: widget.child),
    );
  }
}
