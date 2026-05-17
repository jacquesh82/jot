import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../models/note.dart';
import '../services/crypto.dart';
import '../state/providers.dart';
import '../widgets/empty_state.dart';

class NotesScreen extends ConsumerWidget {
  const NotesScreen({super.key, required this.boardId});
  final String boardId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final notes = ref.watch(notesProvider(boardId));

    return Scaffold(
      appBar: AppBar(title: const Text('Notes')),
      body: RefreshIndicator(
        onRefresh: () async => ref.invalidate(notesProvider(boardId)),
        child: notes.when(
          loading: () => const Center(child: CircularProgressIndicator()),
          error: (e, _) => EmptyState(message: 'Erreur : $e'),
          data: (list) {
            if (list.isEmpty) {
              return const EmptyState(
                icon: Icons.sticky_note_2_outlined,
                message: 'Aucune note. Touche + pour en créer une.',
              );
            }
            return ListView.separated(
              padding: const EdgeInsets.all(12),
              itemCount: list.length,
              separatorBuilder: (_, __) => const SizedBox(height: 8),
              itemBuilder: (_, i) => _NoteTile(list[i]),
            );
          },
        ),
      ),
      floatingActionButton: FloatingActionButton(
        onPressed: () => _quickAdd(context, ref),
        child: const Icon(Icons.add),
      ),
    );
  }

  Future<void> _quickAdd(BuildContext context, WidgetRef ref) async {
    final text = await showDialog<String>(
      context: context,
      builder: (_) => _QuickAddDialog(),
    );
    if (text == null || text.isEmpty) return;
    try {
      final api = await ref.read(apiClientProvider.future);
      // Encrypt with the derived per-note DEK. The note ID is allocated server
      // side, so we follow the SPA's pattern: encrypt under a fresh random DEK
      // and store both, then the server replaces it on first save. For the
      // initial happy path we use the board's DEK directly — the server has
      // already lifted notes onto block storage and can re-key on read.
      final crypto = JotCrypto.instance;
      final bek = await crypto.deriveBek(boardId);
      // Use a deterministic dek seeded from board+random; v1 stores the blob
      // unwrapped under the board key while we wait for per-note DEKs to land.
      final cipher = await crypto.encrypt(bek, Uint8List.fromList(utf8.encode(text)));
      await api.createNote(boardId: boardId, cipherBlob: cipher);
      ref.invalidate(notesProvider(boardId));
    } catch (e) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text('$e')));
      }
    }
  }
}

class _NoteTile extends StatelessWidget {
  const _NoteTile(this.note);
  final Note note;

  @override
  Widget build(BuildContext context) {
    final title = note.titleB64 != null ? '[chiffré]' : (note.snippet ?? note.id.substring(0, 8));
    return Card(
      child: ListTile(
        leading: Icon(switch (note.noteType) {
          'voice' => Icons.mic_outlined,
          'image' => Icons.image_outlined,
          _ => Icons.sticky_note_2_outlined,
        }),
        title: Text(title, maxLines: 1, overflow: TextOverflow.ellipsis),
        subtitle: Text('#${note.id.substring(0, 8)} · pos ${note.position}'),
        trailing: note.shared ? const Icon(Icons.people_outline) : null,
        onTap: () => context.push('/notes/${note.id}'),
      ),
    );
  }
}

class _QuickAddDialog extends StatefulWidget {
  @override
  State<_QuickAddDialog> createState() => _QuickAddDialogState();
}

class _QuickAddDialogState extends State<_QuickAddDialog> {
  final _ctrl = TextEditingController();
  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Nouvelle note'),
      content: TextField(
        controller: _ctrl,
        autofocus: true,
        maxLines: 5,
        decoration: const InputDecoration(hintText: 'Écris…'),
      ),
      actions: [
        TextButton(onPressed: () => Navigator.pop(context), child: const Text('Annuler')),
        FilledButton(
          onPressed: () => Navigator.pop(context, _ctrl.text),
          child: const Text('Ajouter'),
        ),
      ],
    );
  }
}
