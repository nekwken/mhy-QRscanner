import 'package:flutter/material.dart';

/// Win11 云母 (Mica) 配套主题，层次参照 BetterGI / WPF-UI：
/// 窗体最暗（Mica 透出）→ 内容区稍亮 → 卡片再亮一层。
///
/// 字体统一用 Microsoft YaHei UI：Windows 默认的 Latin 字体与 CJK 回退字重
/// 不一致（中文发虚），整套走雅黑后中英文同源、字重一致。
ThemeData buildTheme(Brightness brightness) {
  final isDark = brightness == Brightness.dark;
  final scheme = ColorScheme.fromSeed(
    seedColor: const Color(0xFF3B6EA5),
    brightness: brightness,
  ).copyWith(
    // 中性表面（不带种子色染色），贴近 WinUI 深浅两版的灰阶
    surface: isDark ? const Color(0xFF1E1E20) : const Color(0xFFF3F3F3),
    surfaceContainerHighest:
        isDark ? const Color(0xFF2A2A2C) : const Color(0xFFE8E8E8),
  );
  const cjk = 'Microsoft YaHei UI';

  ThemeData base = ThemeData(
    colorScheme: scheme,
    useMaterial3: true,
    fontFamily: cjk,
    fontFamilyFallback: const ['Microsoft YaHei', 'PingFang SC', 'sans-serif'],
    scaffoldBackgroundColor: Colors.transparent,
    splashFactory: InkSparkle.splashFactory,
  );

  final text =
      base.textTheme.apply(bodyColor: scheme.onSurface, displayColor: scheme.onSurface);

  base = base.copyWith(
    textTheme: text,
    cardTheme: CardThemeData(
      elevation: 0,
      color: (isDark ? Colors.white : Colors.white)
          .withValues(alpha: isDark ? 0.055 : 0.6),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
      margin: EdgeInsets.zero,
    ),
    dialogTheme: DialogThemeData(
      backgroundColor: isDark ? const Color(0xFF2B2B2B) : const Color(0xFFF9F9F9),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
    ),
    navigationBarTheme: NavigationBarThemeData(
      backgroundColor: Colors.transparent,
    ),
    dividerTheme: DividerThemeData(
      color: scheme.outlineVariant.withValues(alpha: 0.35),
    ),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: (isDark ? Colors.white : Colors.white)
          .withValues(alpha: isDark ? 0.05 : 0.65),
      border: OutlineInputBorder(borderRadius: BorderRadius.circular(6)),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(6),
        borderSide: BorderSide(color: scheme.outlineVariant.withValues(alpha: 0.5)),
      ),
    ),
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6)),
      ),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6)),
      ),
    ),
    switchTheme: SwitchThemeData(
      trackColor: WidgetStateProperty.resolveWith(
        (states) => states.contains(WidgetState.selected)
            ? scheme.primary
            : scheme.outlineVariant,
      ),
    ),
  );
  return base;
}
