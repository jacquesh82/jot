import 'package:flutter/material.dart';

// Colour palette mirrors `spa/src/style.css`. The web SPA derives its accents
// from a single seed (`--accent: #ff5c5c`) — we use that same seed so the
// mobile app feels visually consistent.
const _seed = Color(0xFFFF5C5C);

ThemeData jotLight() {
  final scheme = ColorScheme.fromSeed(seedColor: _seed, brightness: Brightness.light);
  return ThemeData(
    useMaterial3: true,
    colorScheme: scheme,
    visualDensity: VisualDensity.adaptivePlatformDensity,
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: scheme.surfaceContainerHighest,
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(10),
        borderSide: BorderSide.none,
      ),
    ),
    cardTheme: CardTheme(
      elevation: 0,
      color: scheme.surfaceContainerLow,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
    ),
  );
}

ThemeData jotDark() {
  final scheme = ColorScheme.fromSeed(seedColor: _seed, brightness: Brightness.dark);
  return ThemeData(
    useMaterial3: true,
    colorScheme: scheme,
    visualDensity: VisualDensity.adaptivePlatformDensity,
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: scheme.surfaceContainerHighest,
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(10),
        borderSide: BorderSide.none,
      ),
    ),
    cardTheme: CardTheme(
      elevation: 0,
      color: scheme.surfaceContainerLow,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
    ),
  );
}
