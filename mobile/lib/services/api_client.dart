import 'dart:convert';
import 'dart:typed_data';

import 'package:dio/dio.dart';

import '../models/block.dart';
import '../models/board.dart';
import '../models/device.dart';
import '../models/identity.dart';
import '../models/link_session.dart';
import '../models/note.dart';
import 'secure_storage.dart';

class ApiException implements Exception {
  final int statusCode;
  final String message;
  const ApiException(this.statusCode, this.message);
  @override
  String toString() => 'ApiException($statusCode): $message';
}

/// Single instance of the API client used across the app. The base URL and
/// JWT are pulled from [SecureStore] on every request through an interceptor
/// so they can change at runtime (server switch, re-link) without rebuilding.
class ApiClient {
  ApiClient._(this._dio);

  final Dio _dio;
  void Function()? onUnauthorized;

  static Future<ApiClient> create() async {
    final dio = Dio(BaseOptions(
      connectTimeout: const Duration(seconds: 15),
      receiveTimeout: const Duration(seconds: 30),
      responseType: ResponseType.json,
    ));
    final client = ApiClient._(dio);

    dio.interceptors.add(InterceptorsWrapper(
      onRequest: (options, handler) async {
        final base = await SecureStore.serverUrl();
        if (base != null && !options.uri.hasAuthority) {
          options.baseUrl = base;
        }
        final tok = await SecureStore.token();
        if (tok != null && tok.isNotEmpty) {
          options.headers['Authorization'] = 'Bearer $tok';
        }
        handler.next(options);
      },
      onError: (e, handler) {
        if (e.response?.statusCode == 401) {
          client.onUnauthorized?.call();
        }
        handler.next(e);
      },
    ));

    return client;
  }

  // ── helpers ────────────────────────────────────────────────────────────────

  Future<T> _json<T>(
    String method,
    String path, {
    Object? body,
    Map<String, dynamic>? query,
    T Function(dynamic data)? parse,
  }) async {
    try {
      final r = await _dio.request<dynamic>(
        path,
        data: body,
        queryParameters: query,
        options: Options(method: method, contentType: 'application/json'),
      );
      if (parse != null) return parse(r.data);
      return r.data as T;
    } on DioException catch (e) {
      throw ApiException(
        e.response?.statusCode ?? -1,
        e.response?.data?.toString() ?? e.message ?? 'network error',
      );
    }
  }

  // ── auth / identity ────────────────────────────────────────────────────────

  Future<Identity> me() => _json(
        'GET',
        '/identity/me',
        parse: (d) => Identity.fromJson(d as Map<String, dynamic>),
      );

  Future<String> mePrivateKey() => _json(
        'GET',
        '/identity/me/privkey',
        parse: (d) => (d as Map)['private_key_x25519'] as String,
      );

  Future<void> registerPubkey(String pubkeyHex) =>
      _json<void>('PUT', '/identity/me/pubkey',
          body: {'public_key_x25519': pubkeyHex});

  Future<void> updateIdentity({String? name, String? lang}) => _json<void>(
        'PATCH',
        '/identity/me',
        body: {
          if (name != null) 'name': name,
          if (lang != null) 'lang': lang,
        },
      );

  Future<List<Contact>> contacts() => _json(
        'GET',
        '/identity/contacts',
        parse: (d) => (d as List)
            .map((e) => Contact.fromJson(e as Map<String, dynamic>))
            .toList(),
      );

  // ── registration / linking ─────────────────────────────────────────────────

  Future<Map<String, dynamic>> register({
    required String deviceName,
    required String publicKeyX25519,
    required String publicKeyEd25519,
    String? inviteToken,
  }) =>
      _json('POST', '/register', body: {
        'device_name': deviceName,
        'public_key_x25519': publicKeyX25519,
        'public_key_ed25519': publicKeyEd25519,
        if (inviteToken != null) 'invite_token': inviteToken,
      });

  Future<LinkSession> linkInit(String deviceName) => _json(
        'POST',
        '/link/init',
        body: {'device_name': deviceName},
        parse: (d) => LinkSession.fromJson(d as Map<String, dynamic>),
      );

  Future<LinkSession> linkStatus(String token) => _json(
        'GET',
        '/link/status/$token',
        parse: (d) => LinkSession.fromJson(d as Map<String, dynamic>),
      );

  Future<void> linkConfirm(String token) =>
      _json<void>('POST', '/link/confirm', body: {'token': token});

  // ── boards ─────────────────────────────────────────────────────────────────

  Future<List<Board>> boards() => _json(
        'GET',
        '/boards',
        parse: (d) => (d as List)
            .map((e) => Board.fromJson(e as Map<String, dynamic>))
            .toList(),
      );

  Future<Board> createBoard(String name) => _json(
        'POST',
        '/boards',
        body: {'name': name, 'position': 0},
        parse: (d) => Board.fromJson(d as Map<String, dynamic>),
      );

  Future<void> renameBoard(String id, String name) =>
      _json<void>('PATCH', '/boards/$id', body: {'name': name});

  Future<void> deleteBoard(String id) => _json<void>('DELETE', '/boards/$id');

  Future<Map<String, dynamic>?> boardKey(String boardId) async {
    try {
      return await _json<Map<String, dynamic>>(
        'GET',
        '/boards/$boardId/key',
        parse: (d) => d as Map<String, dynamic>,
      );
    } on ApiException catch (e) {
      if (e.statusCode == 404) return null;
      rethrow;
    }
  }

  // ── notes ──────────────────────────────────────────────────────────────────

  Future<List<Note>> notes(String boardId) => _json(
        'GET',
        '/notes',
        query: {'board_id': boardId},
        parse: (d) => (d as List)
            .map((e) => Note.fromJson(e as Map<String, dynamic>))
            .toList(),
      );

  Future<NoteMeta> note(String id) => _json(
        'GET',
        '/notes/$id',
        parse: (d) => NoteMeta.fromJson(d as Map<String, dynamic>),
      );

  Future<Uint8List> noteBlob(String id) async {
    try {
      final r = await _dio.get<List<int>>(
        '/notes/$id/blob',
        options: Options(responseType: ResponseType.bytes),
      );
      return Uint8List.fromList(r.data ?? const []);
    } on DioException catch (e) {
      throw ApiException(e.response?.statusCode ?? -1, e.message ?? '');
    }
  }

  Future<Note> createNote({
    required String boardId,
    required Uint8List cipherBlob,
    String noteType = 'text',
    String? titleB64,
  }) async {
    final form = FormData.fromMap({
      'board_id': boardId,
      'note_type': noteType,
      if (titleB64 != null) 'title_b64': titleB64,
      'blob': MultipartFile.fromBytes(cipherBlob, filename: 'note.bin'),
    });
    try {
      final r = await _dio.post<dynamic>(
        '/notes',
        data: form,
        options: Options(contentType: 'multipart/form-data'),
      );
      return Note.fromJson(r.data as Map<String, dynamic>);
    } on DioException catch (e) {
      throw ApiException(e.response?.statusCode ?? -1, e.message ?? '');
    }
  }

  Future<void> deleteNote(String id) => _json<void>('DELETE', '/notes/$id');

  // ── blocks ─────────────────────────────────────────────────────────────────

  Future<List<Block>> blocks(String noteId) => _json(
        'GET',
        '/notes/$noteId/blocks',
        parse: (d) => (d as List)
            .map((e) => Block.fromJson(e as Map<String, dynamic>))
            .toList(),
      );

  Future<Block> createBlock({
    required String noteId,
    required BlockType type,
    required Uint8List contentCipher,
    String? parentId,
    double? position,
  }) =>
      _json(
        'POST',
        '/notes/$noteId/blocks',
        body: {
          'block_type': blockTypeAsString(type),
          'content_b64': base64.encode(contentCipher),
          if (parentId != null) 'parent_block_id': parentId,
          if (position != null) 'position': position,
        },
        parse: (d) => Block.fromJson(d as Map<String, dynamic>),
      );

  Future<Block> updateBlock(String blockId, {
    BlockType? type,
    Uint8List? contentCipher,
    bool? collapsed,
  }) =>
      _json(
        'PATCH',
        '/blocks/$blockId',
        body: {
          if (type != null) 'block_type': blockTypeAsString(type),
          if (contentCipher != null) 'content_b64': base64.encode(contentCipher),
          if (collapsed != null) 'collapsed': collapsed,
        },
        parse: (d) => Block.fromJson(d as Map<String, dynamic>),
      );

  Future<void> deleteBlock(String blockId) =>
      _json<void>('DELETE', '/blocks/$blockId');

  Future<void> moveBlock(String blockId, {String? parentId, double? position}) =>
      _json<void>('POST', '/blocks/$blockId/move', body: {
        if (parentId != null) 'parent_block_id': parentId,
        if (position != null) 'position': position,
      });

  Future<void> indentBlock(String blockId) =>
      _json<void>('POST', '/blocks/$blockId/indent');
  Future<void> outdentBlock(String blockId) =>
      _json<void>('POST', '/blocks/$blockId/outdent');

  // ── devices ────────────────────────────────────────────────────────────────

  Future<List<DeviceSummary>> devices() => _json(
        'GET',
        '/devices',
        parse: (d) => (d as List)
            .map((e) => DeviceSummary.fromJson(e as Map<String, dynamic>))
            .toList(),
      );

  Future<void> renameDevice(String id, String name) =>
      _json<void>('PATCH', '/devices/$id', body: {'name': name});

  Future<void> deleteDevice(String id) =>
      _json<void>('DELETE', '/devices/$id');

  // ── sharing ────────────────────────────────────────────────────────────────

  Future<void> shareNote({
    required String noteId,
    required String recipientId,
    required String encryptedDekHex,
    required String ephemeralPubkeyHex,
  }) =>
      _json<void>('POST', '/notes/$noteId/share', body: {
        'recipient_id': recipientId,
        'encrypted_dek': encryptedDekHex,
        'ephemeral_pubkey_x25519': ephemeralPubkeyHex,
      });

  Future<void> unshareNote(String noteId, String recipientId) =>
      _json<void>('DELETE', '/notes/$noteId/share/$recipientId');

  Future<List<dynamic>> sharedWithMe() =>
      _json('GET', '/shared', parse: (d) => d as List);

  // ── invites ────────────────────────────────────────────────────────────────

  Future<Map<String, dynamic>> createInvite({String? label}) => _json(
        'POST',
        '/invites',
        body: {if (label != null) 'label': label},
        parse: (d) => d as Map<String, dynamic>,
      );

  // ── health / export ────────────────────────────────────────────────────────

  Future<Map<String, dynamic>> health() => _json(
        'GET',
        '/health',
        parse: (d) => d as Map<String, dynamic>,
      );

  Future<Map<String, dynamic>> export() =>
      _json('GET', '/export', parse: (d) => d as Map<String, dynamic>);
}
