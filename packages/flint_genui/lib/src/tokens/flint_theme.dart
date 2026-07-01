import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:http/http.dart' as http;

/// Design token data loaded from GET /a2ui/v1/catalog/:id.
/// Exposed as a Flutter ThemeExtension so it integrates natively with Theme.of(context).
@immutable
class FlintThemeData extends ThemeExtension<FlintThemeData> {
  final Color primary;
  final Color surface;
  final Color textColor;
  final Color border;
  final Color muted;
  final Color error;
  final Color success;
  final Duration durationFast;
  final Duration durationNormal;
  final double radiusSm;
  final double radiusMd;
  final double radiusLg;
  final TextStyle textBase;
  final TextStyle textSm;
  final TextStyle textLg;

  const FlintThemeData({
    required this.primary,
    required this.surface,
    required this.textColor,
    required this.border,
    required this.muted,
    required this.error,
    required this.success,
    this.durationFast = const Duration(milliseconds: 150),
    this.durationNormal = const Duration(milliseconds: 300),
    this.radiusSm = 4.0,
    this.radiusMd = 8.0,
    this.radiusLg = 16.0,
    const TextStyle textBase = const TextStyle(fontSize: 16),
    const TextStyle textSm = const TextStyle(fontSize: 14),
    const TextStyle textLg = const TextStyle(fontSize: 20),
  })  : textBase = textBase,
        textSm = textSm,
        textLg = textLg;

  /// Default Flint token set (mirrors DEFAULT_TOKENS in @flint/react).
  factory FlintThemeData.defaults() {
    return FlintThemeData(
      primary: const Color(0xFF6366f1),
      surface: const Color(0xFFFAFAFA),
      textColor: const Color(0xFF1A1A1A),
      border: const Color(0xFFE5E7EB),
      muted: const Color(0xFF9CA3AF),
      error: const Color(0xFFEF4444),
      success: const Color(0xFF22C55E),
    );
  }

  /// Fetch tokens from GET /a2ui/v1/catalog/:catalogId and build a FlintThemeData.
  static Future<FlintThemeData> fromCatalog(
    String endpoint,
    String catalogId,
    String jwt,
  ) async {
    try {
      final response = await http.get(
        Uri.parse('$endpoint/a2ui/v1/catalog/$catalogId'),
        headers: {'Authorization': 'Bearer $jwt'},
      );
      if (response.statusCode != 200) return FlintThemeData.defaults();
      final data = json.decode(response.body) as Map<String, dynamic>;
      final tokens = data['tokens'] as Map<String, dynamic>? ?? {};

      Color parseHex(String key, Color fallback) {
        final raw = tokens[key] as String?;
        if (raw == null) return fallback;
        final hex = raw.replaceFirst('#', '');
        return Color(int.parse('FF$hex', radix: 16));
      }

      final defaults = FlintThemeData.defaults();
      return FlintThemeData(
        primary: parseHex('--flint-color-primary', defaults.primary),
        surface: parseHex('--flint-color-surface', defaults.surface),
        textColor: parseHex('--flint-color-text', defaults.textColor),
        border: parseHex('--flint-color-border', defaults.border),
        muted: parseHex('--flint-color-muted', defaults.muted),
        error: parseHex('--flint-color-error', defaults.error),
        success: parseHex('--flint-color-success', defaults.success),
      );
    } catch (_) {
      return FlintThemeData.defaults();
    }
  }

  @override
  FlintThemeData copyWith({
    Color? primary,
    Color? surface,
    Color? textColor,
    Color? border,
    Color? muted,
    Color? error,
    Color? success,
    Duration? durationFast,
    Duration? durationNormal,
    double? radiusSm,
    double? radiusMd,
    double? radiusLg,
    TextStyle? textBase,
    TextStyle? textSm,
    TextStyle? textLg,
  }) {
    return FlintThemeData(
      primary: primary ?? this.primary,
      surface: surface ?? this.surface,
      textColor: textColor ?? this.textColor,
      border: border ?? this.border,
      muted: muted ?? this.muted,
      error: error ?? this.error,
      success: success ?? this.success,
      durationFast: durationFast ?? this.durationFast,
      durationNormal: durationNormal ?? this.durationNormal,
      radiusSm: radiusSm ?? this.radiusSm,
      radiusMd: radiusMd ?? this.radiusMd,
      radiusLg: radiusLg ?? this.radiusLg,
      textBase: textBase ?? this.textBase,
      textSm: textSm ?? this.textSm,
      textLg: textLg ?? this.textLg,
    );
  }

  @override
  ThemeExtension<FlintThemeData> lerp(FlintThemeData? other, double t) {
    if (other == null) return this;
    return FlintThemeData(
      primary: Color.lerp(primary, other.primary, t)!,
      surface: Color.lerp(surface, other.surface, t)!,
      textColor: Color.lerp(textColor, other.textColor, t)!,
      border: Color.lerp(border, other.border, t)!,
      muted: Color.lerp(muted, other.muted, t)!,
      error: Color.lerp(error, other.error, t)!,
      success: Color.lerp(success, other.success, t)!,
      durationFast: durationFast,
      durationNormal: durationNormal,
      radiusSm: radiusSm,
      radiusMd: radiusMd,
      radiusLg: radiusLg,
    );
  }

  /// Convenience accessor: FlintThemeData.of(context)
  static FlintThemeData of(BuildContext context) {
    return Theme.of(context).extension<FlintThemeData>() ?? FlintThemeData.defaults();
  }
}
