import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../services/secure_storage.dart';

class SettingsScreen extends ConsumerWidget {
  const SettingsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Scaffold(
      appBar: AppBar(title: const Text('Paramètres')),
      body: ListView(
        children: [
          FutureBuilder<String?>(
            future: SecureStore.serverUrl(),
            builder: (_, snap) => ListTile(
              leading: const Icon(Icons.dns_outlined),
              title: const Text('Serveur'),
              subtitle: Text(snap.data ?? '—'),
              onTap: () => context.push('/server'),
            ),
          ),
          ListTile(
            leading: const Icon(Icons.devices),
            title: const Text('Appareils'),
            onTap: () => context.push('/devices'),
          ),
          ListTile(
            leading: const Icon(Icons.person_outline),
            title: const Text('Profil'),
            onTap: () => context.push('/profile'),
          ),
          const Divider(),
          ListTile(
            leading: const Icon(Icons.info_outline),
            title: const Text('À propos'),
            subtitle: const Text('jot mobile — v0.1.0'),
          ),
        ],
      ),
    );
  }
}
