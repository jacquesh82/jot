enum LinkStatus { pending, approved, denied, expired }

LinkStatus linkStatusFromString(String s) {
  switch (s) {
    case 'approved':
      return LinkStatus.approved;
    case 'denied':
      return LinkStatus.denied;
    case 'expired':
      return LinkStatus.expired;
    default:
      return LinkStatus.pending;
  }
}

class LinkSession {
  final String token;
  final LinkStatus status;
  final String? deviceId;
  final String? jwt;

  const LinkSession({
    required this.token,
    required this.status,
    this.deviceId,
    this.jwt,
  });

  factory LinkSession.fromJson(Map<String, dynamic> j) => LinkSession(
        token: j['token'] as String? ?? '',
        status: linkStatusFromString(j['status'] as String? ?? 'pending'),
        deviceId: j['device_id'] as String?,
        jwt: j['jwt'] as String?,
      );
}
