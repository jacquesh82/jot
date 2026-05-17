import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/board.dart';
import '../models/device.dart';
import '../models/identity.dart';
import '../models/note.dart';
import '../services/api_client.dart';
import '../services/secure_storage.dart';

// ── infrastructure ───────────────────────────────────────────────────────────

final apiClientProvider = FutureProvider<ApiClient>((ref) async {
  final c = await ApiClient.create();
  c.onUnauthorized = () {
    // Surfaces a 401 to the rest of the app; UI layer triggers re-link.
    ref.read(authStateProvider.notifier).state = AuthState.loggedOut;
  };
  return c;
});

// ── auth / session ───────────────────────────────────────────────────────────

enum AuthState { unknown, loggedOut, loggedIn }

final authStateProvider = StateProvider<AuthState>((_) => AuthState.unknown);

final bootstrapProvider = FutureProvider<AuthState>((ref) async {
  final tok = await SecureStore.token();
  final state = (tok == null || tok.isEmpty) ? AuthState.loggedOut : AuthState.loggedIn;
  ref.read(authStateProvider.notifier).state = state;
  return state;
});

// ── identity ─────────────────────────────────────────────────────────────────

final meProvider = FutureProvider<Identity>((ref) async {
  final api = await ref.watch(apiClientProvider.future);
  return api.me();
});

// ── boards / notes ───────────────────────────────────────────────────────────

final boardsProvider = FutureProvider<List<Board>>((ref) async {
  final api = await ref.watch(apiClientProvider.future);
  return api.boards();
});

final selectedBoardProvider = StateProvider<Board?>((_) => null);

final notesProvider =
    FutureProvider.family<List<Note>, String>((ref, boardId) async {
  final api = await ref.watch(apiClientProvider.future);
  return api.notes(boardId);
});

// ── devices ──────────────────────────────────────────────────────────────────

final devicesProvider = FutureProvider<List<DeviceSummary>>((ref) async {
  final api = await ref.watch(apiClientProvider.future);
  return api.devices();
});
