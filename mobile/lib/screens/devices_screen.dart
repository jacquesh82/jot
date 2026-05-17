import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/device.dart';
import '../state/providers.dart';
import '../widgets/empty_state.dart';

class DevicesScreen extends ConsumerWidget {
  const DevicesScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final devices = ref.watch(devicesProvider);
    return Scaffold(
      appBar: AppBar(title: const Text('Appareils')),
      body: RefreshIndicator(
        onRefresh: () async => ref.invalidate(devicesProvider),
        child: devices.when(
          loading: () => const Center(child: CircularProgressIndicator()),
          error: (e, _) => EmptyState(message: 'Erreur : $e'),
          data: (list) => list.isEmpty
              ? const EmptyState(message: 'Aucun appareil', icon: Icons.devices_other)
              : ListView.separated(
                  padding: const EdgeInsets.all(12),
                  itemCount: list.length,
                  separatorBuilder: (_, __) => const SizedBox(height: 4),
                  itemBuilder: (_, i) => _DeviceTile(list[i]),
                ),
        ),
      ),
    );
  }
}

class _DeviceTile extends ConsumerWidget {
  const _DeviceTile(this.device);
  final DeviceSummary device;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Card(
      child: ListTile(
        leading: const Icon(Icons.phone_android),
        title: Text(device.name),
        subtitle: Text('Vu ${device.lastSeen.toLocal()}'),
        trailing: IconButton(
          icon: const Icon(Icons.delete_outline),
          onPressed: () async {
            final ok = await showDialog<bool>(
              context: context,
              builder: (_) => AlertDialog(
                title: Text('Supprimer ${device.name} ?'),
                content: const Text('Le jeton sera révoqué immédiatement.'),
                actions: [
                  TextButton(
                      onPressed: () => Navigator.pop(context, false),
                      child: const Text('Annuler')),
                  FilledButton(
                      style: FilledButton.styleFrom(
                          backgroundColor: Theme.of(context).colorScheme.error),
                      onPressed: () => Navigator.pop(context, true),
                      child: const Text('Supprimer')),
                ],
              ),
            );
            if (ok != true) return;
            final api = await ref.read(apiClientProvider.future);
            await api.deleteDevice(device.id);
            ref.invalidate(devicesProvider);
          },
        ),
      ),
    );
  }
}
