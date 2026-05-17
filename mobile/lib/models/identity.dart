class Identity {
  final String id;
  final String? name;
  final String? lang;
  final String? publicKeyX25519;

  const Identity({required this.id, this.name, this.lang, this.publicKeyX25519});

  factory Identity.fromJson(Map<String, dynamic> j) => Identity(
        id: j['id'] as String,
        name: j['name'] as String?,
        lang: j['lang'] as String?,
        publicKeyX25519: j['public_key_x25519'] as String?,
      );
}

class Contact {
  final String identityId;
  final String name;
  final String? publicKeyX25519;

  const Contact({required this.identityId, required this.name, this.publicKeyX25519});

  factory Contact.fromJson(Map<String, dynamic> j) => Contact(
        identityId: j['identity_id'] as String,
        name: j['name'] as String? ?? j['identity_id'] as String,
        publicKeyX25519: j['public_key_x25519'] as String?,
      );
}
