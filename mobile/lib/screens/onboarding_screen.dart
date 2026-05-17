import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../services/crypto.dart';
import '../services/secure_storage.dart';
import '../state/providers.dart';

/// Two-path entry screen, mirrors `spa/src/components/DeviceRegister.tsx`:
/// (1) register a brand new identity on this server, or
/// (2) link this device to an existing identity (handled in [LinkScreen]).
class OnboardingScreen extends ConsumerStatefulWidget {
  const OnboardingScreen({super.key});
  @override
  ConsumerState<OnboardingScreen> createState() => _State();
}

class _State extends ConsumerState<OnboardingScreen> {
  final _device = TextEditingController(text: 'Android');
  final _invite = TextEditingController();
  bool _busy = false;
  String? _error;

  Future<void> _register() async {
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final api = await ref.read(apiClientProvider.future);

      // Generate identity keypair locally — the server never sees the priv key.
      final kp = await JotCrypto.instance.generateIdentityKeyPair();
      // The device key (ed25519) is used for token signing; the SPA also
      // generates one but the server-side `/register` route accepts a single
      // pair for our v1. We send the X25519 pub for both slots for now.
      final r = await api.register(
        deviceName: _device.text.trim().isEmpty ? 'Android' : _device.text.trim(),
        publicKeyX25519: JotCrypto.hex(kp.pubKey),
        publicKeyEd25519: JotCrypto.hex(kp.pubKey),
        inviteToken: _invite.text.trim().isEmpty ? null : _invite.text.trim(),
      );

      final token = r['token'] as String?;
      final identityId = r['identity_id'] as String?;
      if (token == null) throw StateError('server returned no token');

      await SecureStore.setToken(token);
      if (identityId != null) await SecureStore.setIdentityId(identityId);
      await SecureStore.setIdentityPrivKey(base64.encode(kp.privKey));
      JotCrypto.instance.clearCache();

      ref.read(authStateProvider.notifier).state = AuthState.loggedIn;
      ref.invalidate(boardsProvider);
      if (!mounted) return;
      context.go('/boards');
    } catch (e) {
      setState(() => _error = '$e');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Bienvenue')),
      body: SafeArea(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            children: [
              Card(
                child: Padding(
                  padding: const EdgeInsets.all(16),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      Text('Nouveau compte', style: Theme.of(context).textTheme.titleMedium),
                      const SizedBox(height: 12),
                      TextField(
                        controller: _device,
                        decoration: const InputDecoration(labelText: 'Nom de cet appareil'),
                      ),
                      const SizedBox(height: 12),
                      TextField(
                        controller: _invite,
                        decoration: const InputDecoration(
                          labelText: 'Token d\'invitation (optionnel)',
                        ),
                      ),
                      if (_error != null) ...[
                        const SizedBox(height: 12),
                        Text(_error!,
                            style: TextStyle(color: Theme.of(context).colorScheme.error)),
                      ],
                      const SizedBox(height: 16),
                      FilledButton(
                        onPressed: _busy ? null : _register,
                        child: _busy
                            ? const SizedBox(
                                height: 18, width: 18, child: CircularProgressIndicator(strokeWidth: 2))
                            : const Text('Créer mon identité'),
                      ),
                    ],
                  ),
                ),
              ),
              const SizedBox(height: 16),
              Card(
                child: ListTile(
                  title: const Text('Lier cet appareil'),
                  subtitle: const Text('Scanner un QR code depuis un autre appareil déjà connecté'),
                  trailing: const Icon(Icons.qr_code_scanner),
                  onTap: () => context.push('/link'),
                ),
              ),
              const Spacer(),
              TextButton(
                onPressed: () => context.go('/server'),
                child: const Text('Changer de serveur'),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
