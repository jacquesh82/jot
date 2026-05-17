import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'screens/boards_screen.dart';
import 'screens/devices_screen.dart';
import 'screens/link_screen.dart';
import 'screens/note_screen.dart';
import 'screens/notes_screen.dart';
import 'screens/onboarding_screen.dart';
import 'screens/profile_screen.dart';
import 'screens/server_screen.dart';
import 'screens/settings_screen.dart';
import 'state/providers.dart';

GoRouter buildRouter(WidgetRef ref) {
  return GoRouter(
    initialLocation: '/',
    redirect: (ctx, state) {
      final auth = ref.read(authStateProvider);
      final loc = state.matchedLocation;
      final public = loc == '/' || loc == '/server' || loc.startsWith('/onboarding') || loc.startsWith('/link');
      if (auth == AuthState.loggedOut && !public) return '/onboarding';
      if (auth == AuthState.loggedIn && (loc == '/' || loc.startsWith('/onboarding'))) {
        return '/boards';
      }
      return null;
    },
    routes: [
      GoRoute(path: '/', builder: (_, __) => const ServerScreen()),
      GoRoute(path: '/server', builder: (_, __) => const ServerScreen()),
      GoRoute(path: '/onboarding', builder: (_, __) => const OnboardingScreen()),
      GoRoute(path: '/link', builder: (_, __) => const LinkScreen()),
      GoRoute(path: '/boards', builder: (_, __) => const BoardsScreen()),
      GoRoute(
        path: '/boards/:boardId/notes',
        builder: (_, s) => NotesScreen(boardId: s.pathParameters['boardId']!),
      ),
      GoRoute(
        path: '/notes/:noteId',
        builder: (_, s) => NoteScreen(noteId: s.pathParameters['noteId']!),
      ),
      GoRoute(path: '/devices', builder: (_, __) => const DevicesScreen()),
      GoRoute(path: '/profile', builder: (_, __) => const ProfileScreen()),
      GoRoute(path: '/settings', builder: (_, __) => const SettingsScreen()),
    ],
  );
}
