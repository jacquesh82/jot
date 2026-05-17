class Note {
  final String id;
  final String noteType;
  final int position;
  final bool shared;
  final bool encrypted;
  final int schemaVersion;
  final String? titleB64;
  final String? snippet;

  const Note({
    required this.id,
    required this.noteType,
    required this.position,
    this.shared = false,
    this.encrypted = true,
    this.schemaVersion = 0,
    this.titleB64,
    this.snippet,
  });

  factory Note.fromJson(Map<String, dynamic> j) => Note(
        id: j['id'] as String,
        noteType: j['note_type'] as String? ?? 'text',
        position: (j['position'] as num?)?.toInt() ?? 0,
        shared: j['shared'] as bool? ?? false,
        encrypted: j['encrypted'] as bool? ?? true,
        schemaVersion: (j['schema_version'] as num?)?.toInt() ?? 0,
        titleB64: j['title_b64'] as String?,
        snippet: j['snippet'] as String?,
      );
}

class NoteMeta {
  final String id;
  final String boardId;
  final String noteType;
  final String blobKey;
  final int schemaVersion;
  final String? titleB64;

  const NoteMeta({
    required this.id,
    required this.boardId,
    required this.noteType,
    required this.blobKey,
    this.schemaVersion = 0,
    this.titleB64,
  });

  factory NoteMeta.fromJson(Map<String, dynamic> j) => NoteMeta(
        id: j['id'] as String,
        boardId: j['board_id'] as String,
        noteType: j['note_type'] as String? ?? 'text',
        blobKey: j['blob_key'] as String? ?? '',
        schemaVersion: (j['schema_version'] as num?)?.toInt() ?? 0,
        titleB64: j['title_b64'] as String?,
      );
}
