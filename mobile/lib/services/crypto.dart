import 'dart:convert';
import 'dart:typed_data';

import 'package:cryptography/cryptography.dart';

import 'secure_storage.dart';

/// E2E crypto for the mobile app.
///
/// Mirrors the algorithm choices documented in `docs/sharing-crypto-design.md`
/// and implemented in `crates/core/src/crypto/` (Rust) and `spa/src/crypto.ts`
/// (TypeScript):
///
/// - X25519 ECDH for share-key wrapping
/// - HKDF-SHA256 to derive BEK (per-board) and DEK (per-note) from the
///   identity private key
/// - AES-256-GCM with a random 12-byte nonce for blob encryption
///
/// The identity private key is stored once via [SecureStore.setIdentityPrivKey]
/// (sourced from `GET /identity/me/privkey` after registration / linking),
/// then cached in memory for the lifetime of the process.
class JotCrypto {
  JotCrypto._();
  static final instance = JotCrypto._();

  final _x25519 = Cryptography.instance.x25519();
  final _hkdf = Hkdf(hmac: Hmac.sha256(), outputLength: 32);
  final _aes = AesGcm.with256bits(nonceLength: 12);

  Uint8List? _privKeyCache;

  Future<Uint8List> _privKey() async {
    if (_privKeyCache != null) return _privKeyCache!;
    final b64 = await SecureStore.identityPrivKey();
    if (b64 == null) {
      throw StateError('identity private key not loaded — register or link first');
    }
    _privKeyCache = Uint8List.fromList(base64.decode(b64));
    return _privKeyCache!;
  }

  void clearCache() => _privKeyCache = null;

  // ── X25519 keypair ─────────────────────────────────────────────────────────

  Future<({Uint8List privKey, Uint8List pubKey})> generateIdentityKeyPair() async {
    final kp = await _x25519.newKeyPair();
    final privBytes = await kp.extractPrivateKeyBytes();
    final pub = await kp.extractPublicKey();
    return (
      privKey: Uint8List.fromList(privBytes),
      pubKey: Uint8List.fromList(pub.bytes),
    );
  }

  Future<Uint8List> publicKeyFromPrivate(Uint8List priv) async {
    final kp = await _x25519.newKeyPairFromSeed(priv);
    final pub = await kp.extractPublicKey();
    return Uint8List.fromList(pub.bytes);
  }

  // ── BEK / DEK derivation ───────────────────────────────────────────────────

  Future<Uint8List> deriveBek(String boardId) async {
    final priv = await _privKey();
    final info = Uint8List(12 + 16)
      ..setRange(0, 12, utf8.encode('jot-board-v1'))
      ..setRange(12, 28, _uuidBytes(boardId));
    final out = await _hkdf.deriveKey(
      secretKey: SecretKey(priv),
      info: info,
      nonce: const <int>[],
    );
    return Uint8List.fromList(await out.extractBytes());
  }

  Future<Uint8List> deriveDekFromBek(Uint8List bek, String noteId) async {
    final info = Uint8List(11 + 16)
      ..setRange(0, 11, utf8.encode('jot-note-v1'))
      ..setRange(11, 27, _uuidBytes(noteId));
    final out = await _hkdf.deriveKey(
      secretKey: SecretKey(bek),
      info: info,
      nonce: const <int>[],
    );
    return Uint8List.fromList(await out.extractBytes());
  }

  Future<Uint8List> deriveDek(String boardId, String noteId) async {
    final bek = await deriveBek(boardId);
    return deriveDekFromBek(bek, noteId);
  }

  Future<Uint8List> deriveWrapKey(Uint8List sharedSecret) async {
    final out = await _hkdf.deriveKey(
      secretKey: SecretKey(sharedSecret),
      info: utf8.encode('jot-share-v1'),
      nonce: const <int>[],
    );
    return Uint8List.fromList(await out.extractBytes());
  }

  // ── ECDH ───────────────────────────────────────────────────────────────────

  Future<Uint8List> sharedSecret({
    required Uint8List myPriv,
    required Uint8List peerPub,
  }) async {
    final kp = await _x25519.newKeyPairFromSeed(myPriv);
    final remote = SimplePublicKey(peerPub, type: KeyPairType.x25519);
    final shared = await _x25519.sharedSecretKey(keyPair: kp, remotePublicKey: remote);
    return Uint8List.fromList(await shared.extractBytes());
  }

  // ── AES-256-GCM ────────────────────────────────────────────────────────────

  Future<Uint8List> encrypt(Uint8List key, Uint8List plaintext) async {
    final box = await _aes.encrypt(plaintext, secretKey: SecretKey(key));
    // wire format: nonce || cipher || mac (matches the Rust crate)
    return Uint8List.fromList([...box.nonce, ...box.cipherText, ...box.mac.bytes]);
  }

  Future<Uint8List> decrypt(Uint8List key, Uint8List blob) async {
    if (blob.length < 12 + 16) {
      throw const FormatException('ciphertext too short');
    }
    final nonce = blob.sublist(0, 12);
    final mac = blob.sublist(blob.length - 16);
    final cipher = blob.sublist(12, blob.length - 16);
    final box = SecretBox(cipher, nonce: nonce, mac: Mac(mac));
    final pt = await _aes.decrypt(box, secretKey: SecretKey(key));
    return Uint8List.fromList(pt);
  }

  // ── helpers ────────────────────────────────────────────────────────────────

  static Uint8List _uuidBytes(String uuid) {
    final hex = uuid.replaceAll('-', '');
    if (hex.length != 32) {
      throw FormatException('invalid uuid: $uuid');
    }
    final out = Uint8List(16);
    for (var i = 0; i < 16; i++) {
      out[i] = int.parse(hex.substring(i * 2, i * 2 + 2), radix: 16);
    }
    return out;
  }

  static String hex(Uint8List bytes) {
    final sb = StringBuffer();
    for (final b in bytes) {
      sb.write(b.toRadixString(16).padLeft(2, '0'));
    }
    return sb.toString();
  }

  static Uint8List hexDecode(String s) {
    if (s.length.isOdd) throw const FormatException('odd-length hex');
    final out = Uint8List(s.length ~/ 2);
    for (var i = 0; i < out.length; i++) {
      out[i] = int.parse(s.substring(i * 2, i * 2 + 2), radix: 16);
    }
    return out;
  }
}
