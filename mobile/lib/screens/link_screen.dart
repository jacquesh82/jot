import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:mobile_scanner/mobile_scanner.dart';

import '../models/link_session.dart';
import '../services/secure_storage.dart';
import '../state/providers.dart';

/// Two-step linking flow:
///
/// 1. Scan a QR code displayed by an existing device — the QR encodes a
///    one-time `token` produced by `POST /link/init` on the source device.
///    (Falls back to a manual token field for accessibility / no-camera
///    devices.)
/// 2. Poll `/link/status/<token>` until the source approves the link, then
///    persist the returned JWT and identity bytes locally.
class LinkScreen extends ConsumerStatefulWidget {
  const LinkScreen({super.key});
  @override
  ConsumerState<LinkScreen> createState() => _State();
}

class _State extends ConsumerState<LinkScreen> {
  final _tokenCtrl = TextEditingController();
  Timer? _poll;
  bool _busy = false;
  String? _status;

  @override
  void dispose() {
    _poll?.cancel();
    super.dispose();
  }

  Future<void> _useToken(String token) async {
    _tokenCtrl.text = token;
    setState(() {
      _busy = true;
      _status = 'En attente d\'approbation…';
    });
    _poll?.cancel();
    _poll = Timer.periodic(const Duration(seconds: 2), (t) async {
      final api = await ref.read(apiClientProvider.future);
      try {
        final s = await api.linkStatus(token);
        if (s.status == LinkStatus.approved && s.jwt != null) {
          t.cancel();
          await SecureStore.setToken(s.jwt!);
          if (s.deviceId != null) await SecureStore.setIdentityId(s.deviceId!);
          ref.read(authStateProvider.notifier).state = AuthState.loggedIn;
          ref.invalidate(boardsProvider);
          if (!mounted) return;
          context.go('/boards');
        } else if (s.status == LinkStatus.denied || s.status == LinkStatus.expired) {
          t.cancel();
          if (!mounted) return;
          setState(() {
            _busy = false;
            _status = 'Lien ${s.status.name}.';
          });
        }
      } catch (e) {
        // network blip — keep polling, but surface the latest error
        if (mounted) setState(() => _status = 'Erreur réseau : $e');
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Lier cet appareil')),
      body: SafeArea(
        child: Column(
          children: [
            Expanded(
              child: MobileScanner(
                onDetect: (capture) {
                  if (_busy) return;
                  final raw = capture.barcodes.firstOrNull?.rawValue;
                  if (raw == null || raw.isEmpty) return;
                  // QR may contain either the raw token or the full URL
                  // `http://host/#/register?invite=<token>` — extract the bit
                  // after `?invite=` or fall back to the raw value.
                  final m = RegExp(r'(?:invite|token)=([^&]+)').firstMatch(raw);
                  _useToken(m?.group(1) ?? raw);
                },
              ),
            ),
            Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                children: [
                  TextField(
                    controller: _tokenCtrl,
                    decoration: const InputDecoration(
                      labelText: 'Ou colle un token manuellement',
                    ),
                  ),
                  const SizedBox(height: 8),
                  Row(children: [
                    Expanded(
                      child: FilledButton(
                        onPressed: _busy || _tokenCtrl.text.trim().isEmpty
                            ? null
                            : () => _useToken(_tokenCtrl.text.trim()),
                        child: const Text('Utiliser ce token'),
                      ),
                    ),
                  ]),
                  if (_status != null) ...[
                    const SizedBox(height: 12),
                    Text(_status!),
                  ],
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
