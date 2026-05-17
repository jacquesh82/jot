import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../services/crypto.dart';
import '../state/providers.dart';

class NoteScreen extends ConsumerStatefulWidget {
  const NoteScreen({super.key, required this.noteId});
  final String noteId;

  @override
  ConsumerState<NoteScreen> createState() => _State();
}

class _State extends ConsumerState<NoteScreen> {
  String? _content;
  String? _error;
  bool _loading = true;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final api = await ref.read(apiClientProvider.future);
      final meta = await api.note(widget.noteId);
      final blob = await api.noteBlob(widget.noteId);

      String text;
      try {
        // Owner path: derive the board key, then the note key, then AES-GCM
        // decrypt. Same hierarchy as the SPA and CLI.
        final bek = await JotCrypto.instance.deriveBek(meta.boardId);
        final dek = await JotCrypto.instance.deriveDekFromBek(bek, meta.id);
        final pt = await JotCrypto.instance.decrypt(dek, blob);
        text = utf8.decode(pt);
      } catch (_) {
        // Either the blob isn't encrypted (legacy note) or we don't have the
        // right key — fall back to raw UTF-8 so the user sees something.
        try {
          text = utf8.decode(blob);
        } catch (_) {
          text = '[chiffré — clé inaccessible sur cet appareil]';
        }
      }

      if (!mounted) return;
      setState(() {
        _content = text;
        _loading = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = '$e';
        _loading = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text('Note #${widget.noteId.substring(0, 8)}'),
        actions: [
          IconButton(icon: const Icon(Icons.refresh), onPressed: () {
            setState(() => _loading = true);
            _load();
          }),
        ],
      ),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : _error != null
              ? Center(child: Text(_error!))
              : SingleChildScrollView(
                  padding: const EdgeInsets.all(16),
                  child: SelectableText(_content ?? ''),
                ),
    );
  }
}
