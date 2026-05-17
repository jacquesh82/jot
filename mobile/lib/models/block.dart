import 'dart:convert' show base64;

enum BlockType { text, heading, todo, quote, code, embed, divider }

BlockType blockTypeFromString(String s) {
  switch (s) {
    case 'heading':
      return BlockType.heading;
    case 'todo':
      return BlockType.todo;
    case 'quote':
      return BlockType.quote;
    case 'code':
      return BlockType.code;
    case 'embed':
      return BlockType.embed;
    case 'divider':
      return BlockType.divider;
    default:
      return BlockType.text;
  }
}

String blockTypeAsString(BlockType t) => t.name;

class Block {
  final String id;
  final String noteId;
  final String? parentBlockId;
  final double position;
  final BlockType blockType;

  /// Encrypted ciphertext bytes (base64-encoded over the wire).
  final List<int> contentCipher;

  /// Plaintext, populated lazily once the per-note DEK has decrypted
  /// `contentCipher`. Null while still encrypted.
  final String? content;

  final List<int>? metadataCipher;
  final bool collapsed;
  final DateTime createdAt;
  final DateTime updatedAt;

  const Block({
    required this.id,
    required this.noteId,
    required this.parentBlockId,
    required this.position,
    required this.blockType,
    required this.contentCipher,
    this.content,
    this.metadataCipher,
    this.collapsed = false,
    required this.createdAt,
    required this.updatedAt,
  });

  Block copyWith({String? content, BlockType? blockType, bool? collapsed}) => Block(
        id: id,
        noteId: noteId,
        parentBlockId: parentBlockId,
        position: position,
        blockType: blockType ?? this.blockType,
        contentCipher: contentCipher,
        content: content ?? this.content,
        metadataCipher: metadataCipher,
        collapsed: collapsed ?? this.collapsed,
        createdAt: createdAt,
        updatedAt: updatedAt,
      );

  factory Block.fromJson(Map<String, dynamic> j) {
    final content = j['content_b64'] as String?;
    final metadata = j['metadata_b64'] as String?;
    return Block(
      id: j['id'] as String,
      noteId: j['note_id'] as String,
      parentBlockId: j['parent_block_id'] as String?,
      position: (j['position'] as num?)?.toDouble() ?? 0,
      blockType: blockTypeFromString(j['block_type'] as String? ?? 'text'),
      contentCipher: content == null ? const [] : base64.decode(content),
      metadataCipher: metadata == null ? null : base64.decode(metadata),
      collapsed: j['collapsed'] as bool? ?? false,
      createdAt: DateTime.parse(j['created_at'] as String),
      updatedAt: DateTime.parse(j['updated_at'] as String),
    );
  }
}
