class DeviceSummary {
  final String id;
  final String name;
  final DateTime lastSeen;

  const DeviceSummary({required this.id, required this.name, required this.lastSeen});

  factory DeviceSummary.fromJson(Map<String, dynamic> j) => DeviceSummary(
        id: j['id'] as String,
        name: j['name'] as String? ?? '',
        lastSeen: DateTime.parse(j['last_seen'] as String),
      );
}
