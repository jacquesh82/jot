import 'dart:convert';
import 'dart:ffi';
import 'dart:io' show Platform;
import 'dart:typed_data';

import 'package:ffi/ffi.dart';

// ── raw native types ─────────────────────────────────────────────────────────

final class JotBuffer extends Struct {
  external Pointer<Uint8> data;
  @Size()
  external int len;
}

typedef _BufferFreeNative = Void Function(JotBuffer);
typedef _BufferFreeDart = void Function(JotBuffer);

typedef _EncryptNative = Int32 Function(
    Pointer<Uint8>, Pointer<Uint8>, Size, Pointer<JotBuffer>);
typedef _EncryptDart = int Function(
    Pointer<Uint8>, Pointer<Uint8>, int, Pointer<JotBuffer>);

typedef _DeriveNative = Int32 Function(
    Pointer<Uint8>, Pointer<Utf8>, Pointer<Uint8>);
typedef _DeriveDart = int Function(
    Pointer<Uint8>, Pointer<Utf8>, Pointer<Uint8>);

typedef _WrapKeyNative = Int32 Function(Pointer<Uint8>, Pointer<Uint8>);
typedef _WrapKeyDart = int Function(Pointer<Uint8>, Pointer<Uint8>);

typedef _KeypairNative = Int32 Function(Pointer<Uint8>, Pointer<Uint8>);
typedef _KeypairDart = int Function(Pointer<Uint8>, Pointer<Uint8>);

typedef _DhNative = Int32 Function(
    Pointer<Uint8>, Pointer<Uint8>, Pointer<Uint8>);
typedef _DhDart = int Function(Pointer<Uint8>, Pointer<Uint8>, Pointer<Uint8>);

typedef _ExtractLinksNative = Int32 Function(
    Pointer<Utf8>, Pointer<JotBuffer>);
typedef _ExtractLinksDart = int Function(Pointer<Utf8>, Pointer<JotBuffer>);

typedef _GenDekNative = Int32 Function(Pointer<Uint8>);
typedef _GenDekDart = int Function(Pointer<Uint8>);

// ── library loader ───────────────────────────────────────────────────────────

DynamicLibrary _open() {
  if (Platform.isAndroid) {
    return DynamicLibrary.open('libjot_mobile_ffi.so');
  }
  if (Platform.isLinux) {
    return DynamicLibrary.open('libjot_mobile_ffi.so');
  }
  if (Platform.isMacOS) {
    return DynamicLibrary.open('libjot_mobile_ffi.dylib');
  }
  if (Platform.isIOS) {
    return DynamicLibrary.process();
  }
  throw UnsupportedError('libjot_mobile_ffi not available on ${Platform.operatingSystem}');
}

/// Idiomatic, allocation-safe wrapper around the C ABI.
class JotFfi {
  JotFfi._(this._lib);

  final DynamicLibrary _lib;
  static JotFfi? _instance;

  static JotFfi get instance => _instance ??= JotFfi._(_open());

  late final _BufferFreeDart _bufferFree =
      _lib.lookupFunction<_BufferFreeNative, _BufferFreeDart>('jot_buffer_free');

  late final _EncryptDart _encrypt =
      _lib.lookupFunction<_EncryptNative, _EncryptDart>('jot_encrypt');

  late final _EncryptDart _decrypt =
      _lib.lookupFunction<_EncryptNative, _EncryptDart>('jot_decrypt');

  late final _GenDekDart _genDek =
      _lib.lookupFunction<_GenDekNative, _GenDekDart>('jot_generate_dek');

  late final _DeriveDart _deriveBek =
      _lib.lookupFunction<_DeriveNative, _DeriveDart>('jot_derive_bek');

  late final _DeriveDart _deriveDek =
      _lib.lookupFunction<_DeriveNative, _DeriveDart>('jot_derive_dek');

  late final _WrapKeyDart _deriveWrapKey =
      _lib.lookupFunction<_WrapKeyNative, _WrapKeyDart>('jot_derive_wrap_key');

  late final _KeypairDart _genStatic = _lib
      .lookupFunction<_KeypairNative, _KeypairDart>('jot_generate_static_keypair');

  late final _WrapKeyDart _pubFromPriv = _lib
      .lookupFunction<_WrapKeyNative, _WrapKeyDart>('jot_pubkey_from_privkey');

  late final _DhDart _staticDh =
      _lib.lookupFunction<_DhNative, _DhDart>('jot_static_diffie_hellman');

  late final _ExtractLinksDart _extractLinks = _lib
      .lookupFunction<_ExtractLinksNative, _ExtractLinksDart>('jot_extract_links');

  // ── high-level helpers (own allocations, free on exit) ─────────────────────

  Uint8List encrypt(Uint8List key, Uint8List plaintext) {
    return _withKey32(key, (kPtr) => _withBytes(plaintext, (ptPtr, ptLen) {
          final out = calloc<JotBuffer>();
          try {
            final rc = _encrypt(kPtr, ptPtr, ptLen, out);
            if (rc != 0) throw StateError('jot_encrypt rc=$rc');
            return _copyAndFree(out);
          } finally {
            calloc.free(out);
          }
        }));
  }

  Uint8List decrypt(Uint8List key, Uint8List blob) {
    return _withKey32(key, (kPtr) => _withBytes(blob, (bPtr, bLen) {
          final out = calloc<JotBuffer>();
          try {
            final rc = _decrypt(kPtr, bPtr, bLen, out);
            if (rc != 0) throw StateError('jot_decrypt rc=$rc');
            return _copyAndFree(out);
          } finally {
            calloc.free(out);
          }
        }));
  }

  Uint8List generateDek() => _into32((p) => _genDek(p));

  Uint8List deriveBek(Uint8List privKey, String boardId) =>
      _deriveWith(privKey, boardId, _deriveBek);

  Uint8List deriveDek(Uint8List bek, String noteId) =>
      _deriveWith(bek, noteId, _deriveDek);

  Uint8List deriveWrapKey(Uint8List sharedSecret) =>
      _withKey32(sharedSecret, (sPtr) => _into32((p) => _deriveWrapKey(sPtr, p)));

  ({Uint8List priv, Uint8List pub}) generateStaticKeyPair() {
    final priv = calloc<Uint8>(32);
    final pub = calloc<Uint8>(32);
    try {
      final rc = _genStatic(priv, pub);
      if (rc != 0) throw StateError('keypair rc=$rc');
      return (priv: _copy32(priv), pub: _copy32(pub));
    } finally {
      calloc.free(priv);
      calloc.free(pub);
    }
  }

  Uint8List publicFromPrivate(Uint8List priv) =>
      _withKey32(priv, (pPtr) => _into32((out) => _pubFromPriv(pPtr, out)));

  Uint8List sharedSecret(Uint8List myPriv, Uint8List peerPub) =>
      _withKey32(myPriv, (privPtr) =>
          _withKey32(peerPub, (pubPtr) =>
              _into32((out) => _staticDh(privPtr, pubPtr, out))));

  List<Map<String, dynamic>> extractLinks(String markdown) {
    final s = markdown.toNativeUtf8();
    final out = calloc<JotBuffer>();
    try {
      final rc = _extractLinks(s, out);
      if (rc != 0) return const [];
      final bytes = _copyAndFree(out);
      final decoded = jsonDecode(utf8.decode(bytes)) as List;
      return decoded.cast<Map<String, dynamic>>();
    } finally {
      calloc.free(s);
      calloc.free(out);
    }
  }

  // ── internals ──────────────────────────────────────────────────────────────

  T _withKey32<T>(Uint8List key, T Function(Pointer<Uint8>) body) {
    if (key.length != 32) throw ArgumentError('expected 32-byte key');
    final p = calloc<Uint8>(32);
    try {
      for (var i = 0; i < 32; i++) {
        p[i] = key[i];
      }
      return body(p);
    } finally {
      calloc.free(p);
    }
  }

  T _withBytes<T>(Uint8List bytes, T Function(Pointer<Uint8>, int) body) {
    if (bytes.isEmpty) return body(nullptr, 0);
    final p = calloc<Uint8>(bytes.length);
    try {
      for (var i = 0; i < bytes.length; i++) {
        p[i] = bytes[i];
      }
      return body(p, bytes.length);
    } finally {
      calloc.free(p);
    }
  }

  Uint8List _into32(int Function(Pointer<Uint8>) call) {
    final p = calloc<Uint8>(32);
    try {
      final rc = call(p);
      if (rc != 0) throw StateError('rc=$rc');
      return _copy32(p);
    } finally {
      calloc.free(p);
    }
  }

  Uint8List _deriveWith(Uint8List key, String uuid, _DeriveDart fn) =>
      _withKey32(key, (kPtr) {
        final s = uuid.toNativeUtf8();
        try {
          return _into32((out) => fn(kPtr, s, out));
        } finally {
          calloc.free(s);
        }
      });

  Uint8List _copy32(Pointer<Uint8> p) {
    final out = Uint8List(32);
    for (var i = 0; i < 32; i++) {
      out[i] = p[i];
    }
    return out;
  }

  Uint8List _copyAndFree(Pointer<JotBuffer> out) {
    final buf = out.ref;
    if (buf.data == nullptr || buf.len == 0) {
      return Uint8List(0);
    }
    final copy = Uint8List(buf.len);
    for (var i = 0; i < buf.len; i++) {
      copy[i] = buf.data[i];
    }
    _bufferFree(buf);
    return copy;
  }
}
