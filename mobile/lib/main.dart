import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'router.dart';
import 'state/providers.dart';
import 'theme.dart';

void main() {
  runApp(const ProviderScope(child: JotApp()));
}

class JotApp extends ConsumerWidget {
  const JotApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    // Resolve token presence once on cold start so the redirect sees a
    // settled auth state before painting the first frame.
    ref.watch(bootstrapProvider);
    final router = buildRouter(ref);

    return MaterialApp.router(
      title: 'jot',
      theme: jotLight(),
      darkTheme: jotDark(),
      routerConfig: router,
      debugShowCheckedModeBanner: false,
    );
  }
}
