import 'package:flutter_secure_storage/flutter_secure_storage.dart';

/// Thin wrapper around `flutter_secure_storage` so the rest of the app only
/// depends on a typed key surface. Backed by Android Keystore on real devices.
class SecureStore {
  SecureStore._();

  static const _store = FlutterSecureStorage(
    aOptions: AndroidOptions(encryptedSharedPreferences: true),
  );

  // ── server identity ────────────────────────────────────────────────────────

  static Future<String?> serverUrl() => _store.read(key: 'server_url');
  static Future<void> setServerUrl(String url) =>
      _store.write(key: 'server_url', value: url);

  static Future<String?> token() => _store.read(key: 'jwt');
  static Future<void> setToken(String? jwt) async {
    if (jwt == null) {
      await _store.delete(key: 'jwt');
    } else {
      await _store.write(key: 'jwt', value: jwt);
    }
  }

  // ── identity key material ──────────────────────────────────────────────────
  //
  // We persist the raw 32-byte X25519 private key as base64 so the FFI layer
  // can pull it back in and re-derive BEK/DEK without ever exposing the bytes
  // to logs or to disk-as-plaintext.

  static Future<String?> identityPrivKey() => _store.read(key: 'identity_priv_x25519');
  static Future<void> setIdentityPrivKey(String b64) =>
      _store.write(key: 'identity_priv_x25519', value: b64);

  static Future<String?> identityId() => _store.read(key: 'identity_id');
  static Future<void> setIdentityId(String id) =>
      _store.write(key: 'identity_id', value: id);

  static Future<void> clear() => _store.deleteAll();
}
