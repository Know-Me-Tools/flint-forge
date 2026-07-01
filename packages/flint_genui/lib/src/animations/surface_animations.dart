import 'package:flutter/material.dart';

class SurfaceEntranceAnimation extends StatefulWidget {
  final Widget child;

  const SurfaceEntranceAnimation({super.key, required this.child});

  @override
  State<SurfaceEntranceAnimation> createState() => _SurfaceEntranceAnimationState();
}

class _SurfaceEntranceAnimationState extends State<SurfaceEntranceAnimation> with SingleTickerProviderStateMixin {
  late final AnimationController _controller;
  late final Animation<double> _opacity;
  late final Animation<Offset> _slide;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(vsync: this, duration: const Duration(milliseconds: 280));
    _opacity = CurvedAnimation(parent: _controller, curve: Curves.easeOut);
    _slide = Tween<Offset>(begin: const Offset(0, 0.04), end: Offset.zero).animate(CurvedAnimation(parent: _controller, curve: Curves.easeOutCubic));
    _controller.forward();
  }

  @override
  void dispose() { _controller.dispose(); super.dispose(); }

  @override
  Widget build(BuildContext context) {
    return FadeTransition(
      opacity: _opacity,
      child: SlideTransition(position: _slide, child: widget.child),
    );
  }
}

class SurfaceExitAnimation extends StatefulWidget {
  final Widget child;
  final VoidCallback onComplete;

  const SurfaceExitAnimation({super.key, required this.child, required this.onComplete});

  @override
  State<SurfaceExitAnimation> createState() => _SurfaceExitAnimationState();
}

class _SurfaceExitAnimationState extends State<SurfaceExitAnimation> with SingleTickerProviderStateMixin {
  late final AnimationController _controller;
  late final Animation<double> _opacity;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(vsync: this, duration: const Duration(milliseconds: 180));
    _opacity = CurvedAnimation(parent: _controller, curve: Curves.easeIn);
    _controller.forward().whenComplete(widget.onComplete);
  }

  @override
  void dispose() { _controller.dispose(); super.dispose(); }

  @override
  Widget build(BuildContext context) {
    return FadeTransition(opacity: ReverseAnimation(_opacity), child: widget.child);
  }
}

class ComponentUpdateAnimation extends StatefulWidget {
  final Widget child;

  const ComponentUpdateAnimation({super.key, required this.child});

  @override
  State<ComponentUpdateAnimation> createState() => _ComponentUpdateAnimationState();
}

class _ComponentUpdateAnimationState extends State<ComponentUpdateAnimation> with SingleTickerProviderStateMixin {
  late final AnimationController _controller;
  late final Animation<double> _opacity;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(vsync: this, duration: const Duration(milliseconds: 150));
    _opacity = CurvedAnimation(parent: _controller, curve: Curves.easeInOut);
    _controller.forward();
  }

  @override
  void dispose() { _controller.dispose(); super.dispose(); }

  @override
  Widget build(BuildContext context) {
    return FadeTransition(opacity: _opacity, child: widget.child);
  }
}

class StreamingTextAnimation extends StatefulWidget {
  final Widget child;
  final bool active;

  const StreamingTextAnimation({super.key, required this.child, required this.active});

  @override
  State<StreamingTextAnimation> createState() => _StreamingTextAnimationState();
}

class _StreamingTextAnimationState extends State<StreamingTextAnimation> with SingleTickerProviderStateMixin {
  late final AnimationController _controller;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(vsync: this, duration: const Duration(milliseconds: 600));
    if (widget.active) _controller.repeat(reverse: true);
  }

  @override
  void didUpdateWidget(StreamingTextAnimation oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.active && !_controller.isAnimating) _controller.repeat(reverse: true);
    if (!widget.active && _controller.isAnimating) _controller.stop();
  }

  @override
  void dispose() { _controller.dispose(); super.dispose(); }

  @override
  Widget build(BuildContext context) => widget.child;
}
