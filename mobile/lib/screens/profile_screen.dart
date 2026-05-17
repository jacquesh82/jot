import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../services/crypto.dart';
import '../services/secure_storage.dart';
import '../state/providers.dart';

class ProfileScreen extends ConsumerWidget {
  const ProfileScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final me = ref.watch(meProvider);
    return Scaffold(
      appBar: AppBar(title: const Text('Profil')),
      body: me.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Center(child: Text('$e')),
        data: (id) => ListView(
          padding: const EdgeInsets.all(16),
          children: [
            Card(
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text('Identité', style: Theme.of(context).textTheme.titleSmall),
                    const SizedBox(height: 8),
                    SelectableText(id.id, style: const TextStyle(fontFamily: 'monospace')),
                    if (id.name != null) ...[
                      const SizedBox(height: 8),
                      Text('Nom : ${id.name}'),
                    ],
                  ],
                ),
              ),
            ),
            const SizedBox(height: 12),
            FilledButton.tonal(
              onPressed: () async {
                JotCrypto.instance.clearCache();
                await SecureStore.setToken(null);
                ref.read(authStateProvider.notifier).state = AuthState.loggedOut;
                if (context.mounted) context.go('/onboarding');
              },
              child: const Text('Se déconnecter'),
            ),
          ],
        ),
      ),
    );
  }
}
