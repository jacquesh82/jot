import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../models/board.dart';
import '../state/providers.dart';
import '../widgets/empty_state.dart';

class BoardsScreen extends ConsumerWidget {
  const BoardsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final boards = ref.watch(boardsProvider);

    return Scaffold(
      appBar: AppBar(
        title: const Text('jot'),
        actions: [
          IconButton(icon: const Icon(Icons.devices), onPressed: () => context.push('/devices')),
          IconButton(icon: const Icon(Icons.person), onPressed: () => context.push('/profile')),
          IconButton(icon: const Icon(Icons.settings), onPressed: () => context.push('/settings')),
        ],
      ),
      body: RefreshIndicator(
        onRefresh: () async => ref.invalidate(boardsProvider),
        child: boards.when(
          loading: () => const Center(child: CircularProgressIndicator()),
          error: (e, _) => EmptyState(message: 'Erreur : $e'),
          data: (list) {
            if (list.isEmpty) {
              return const EmptyState(
                message: 'Aucun board. Crée-en un avec le bouton +.',
                icon: Icons.dashboard_outlined,
              );
            }
            return ListView.separated(
              padding: const EdgeInsets.all(12),
              itemCount: list.length,
              separatorBuilder: (_, __) => const SizedBox(height: 8),
              itemBuilder: (_, i) => _BoardTile(list[i]),
            );
          },
        ),
      ),
      floatingActionButton: FloatingActionButton.extended(
        icon: const Icon(Icons.add),
        label: const Text('Nouveau board'),
        onPressed: () => _create(context, ref),
      ),
    );
  }

  Future<void> _create(BuildContext context, WidgetRef ref) async {
    final name = await showDialog<String>(
      context: context,
      builder: (_) => _NameDialog(title: 'Nouveau board'),
    );
    if (name == null || name.isEmpty) return;
    try {
      final api = await ref.read(apiClientProvider.future);
      await api.createBoard(name);
      ref.invalidate(boardsProvider);
    } catch (e) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text('$e')));
      }
    }
  }
}

class _BoardTile extends StatelessWidget {
  const _BoardTile(this.board);
  final Board board;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: ListTile(
        leading: const Icon(Icons.dashboard_outlined),
        title: Text(board.name),
        trailing: board.shared
            ? const Tooltip(message: 'Partagé', child: Icon(Icons.people_outline))
            : const Icon(Icons.chevron_right),
        onTap: () => context.push('/boards/${board.id}/notes'),
      ),
    );
  }
}

class _NameDialog extends StatefulWidget {
  const _NameDialog({required this.title, this.initial = ''});
  final String title;
  final String initial;
  @override
  State<_NameDialog> createState() => _NameDialogState();
}

class _NameDialogState extends State<_NameDialog> {
  late final _ctrl = TextEditingController(text: widget.initial);
  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text(widget.title),
      content: TextField(
        controller: _ctrl,
        autofocus: true,
        decoration: const InputDecoration(hintText: 'Nom'),
        onSubmitted: (_) => Navigator.pop(context, _ctrl.text.trim()),
      ),
      actions: [
        TextButton(onPressed: () => Navigator.pop(context), child: const Text('Annuler')),
        FilledButton(
          onPressed: () => Navigator.pop(context, _ctrl.text.trim()),
          child: const Text('OK'),
        ),
      ],
    );
  }
}
