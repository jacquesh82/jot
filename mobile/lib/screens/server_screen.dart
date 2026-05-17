import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../services/secure_storage.dart';
import '../state/providers.dart';

class ServerScreen extends ConsumerStatefulWidget {
  const ServerScreen({super.key});
  @override
  ConsumerState<ServerScreen> createState() => _State();
}

class _State extends ConsumerState<ServerScreen> {
  final _ctrl = TextEditingController(text: 'http://10.0.2.2:3000');
  bool _busy = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    SecureStore.serverUrl().then((url) {
      if (url != null && mounted) setState(() => _ctrl.text = url);
    });
  }

  Future<void> _connect() async {
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final url = _ctrl.text.trim();
      if (!url.startsWith('http')) {
        throw const FormatException("L'URL doit commencer par http(s)://");
      }
      await SecureStore.setServerUrl(url);
      ref.invalidate(apiClientProvider);
      // Probe /health so we fail fast before the user even tries to log in.
      final api = await ref.read(apiClientProvider.future);
      await api.health();
      if (!mounted) return;
      final tok = await SecureStore.token();
      if (tok != null && tok.isNotEmpty) {
        ref.read(authStateProvider.notifier).state = AuthState.loggedIn;
        context.go('/boards');
      } else {
        context.go('/onboarding');
      }
    } catch (e) {
      setState(() => _error = '$e');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: SafeArea(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text('jot', style: Theme.of(context).textTheme.displaySmall),
              const SizedBox(height: 8),
              Text(
                'Post-it numérique universel — chiffré, anonyme, partout.',
                style: Theme.of(context).textTheme.bodyMedium,
              ),
              const SizedBox(height: 32),
              TextField(
                controller: _ctrl,
                keyboardType: TextInputType.url,
                autocorrect: false,
                decoration: const InputDecoration(
                  labelText: 'Serveur jot',
                  hintText: 'https://jot.example.com',
                ),
              ),
              if (_error != null) ...[
                const SizedBox(height: 12),
                Text(_error!, style: TextStyle(color: Theme.of(context).colorScheme.error)),
              ],
              const SizedBox(height: 24),
              FilledButton(
                onPressed: _busy ? null : _connect,
                child: _busy
                    ? const SizedBox(
                        height: 18, width: 18, child: CircularProgressIndicator(strokeWidth: 2))
                    : const Text('Se connecter'),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
