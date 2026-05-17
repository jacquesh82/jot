import 'dart:async';
import 'dart:convert';

import 'package:web_socket_channel/web_socket_channel.dart';

import 'secure_storage.dart';

class WsEvent {
  final String type;
  final Map<String, dynamic> payload;
  const WsEvent(this.type, this.payload);
}

/// Subscribes to `/ws` and re-emits server events as a broadcast stream.
///
/// The connection is best-effort: the controller keeps emitting from the most
/// recent socket; consumers shouldn't assume it survived backgrounding. Reopen
/// via [reconnect] when the app returns to the foreground.
class JotWebSocket {
  JotWebSocket._();
  static final instance = JotWebSocket._();

  WebSocketChannel? _ch;
  final _controller = StreamController<WsEvent>.broadcast();
  Stream<WsEvent> get events => _controller.stream;

  Future<void> connect() async {
    await _ch?.sink.close();
    final base = await SecureStore.serverUrl();
    final token = await SecureStore.token();
    if (base == null || token == null) return;
    final uri = Uri.parse(base.replaceFirst(RegExp(r'^http'), 'ws')).replace(
      path: '/ws',
      queryParameters: {'token': token},
    );
    _ch = WebSocketChannel.connect(uri);
    _ch!.stream.listen((raw) {
      try {
        final j = jsonDecode(raw as String) as Map<String, dynamic>;
        final type = j['event'] as String? ?? 'unknown';
        _controller.add(WsEvent(type, j));
      } catch (_) {
        // malformed frame — drop silently
      }
    }, onDone: () => _ch = null, onError: (_) => _ch = null);
  }

  Future<void> reconnect() => connect();

  Future<void> close() async {
    await _ch?.sink.close();
    _ch = null;
  }
}
