class Board {
  final String id;
  final String name;
  final int position;
  final bool shared;

  const Board({
    required this.id,
    required this.name,
    required this.position,
    this.shared = false,
  });

  factory Board.fromJson(Map<String, dynamic> j) => Board(
        id: j['id'] as String,
        name: j['name'] as String,
        position: (j['position'] as num?)?.toInt() ?? 0,
        shared: j['shared'] as bool? ?? false,
      );

  Map<String, dynamic> toJson() => {
        'id': id,
        'name': name,
        'position': position,
        if (shared) 'shared': true,
      };
}
