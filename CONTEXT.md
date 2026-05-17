# CONTEXT — jot

## Current Task
Bootstrap de l'app Android : projet Flutter `mobile/` + crate FFI Rust
`crates/mobile-ffi`, parité visée avec la SPA.

## Key Decisions
- Stack mobile : Flutter + FFI Rust (réutilise `jot-core` pour la crypto).
- ABI C stable hand-rolled (header `mobile/rust/jot_mobile_ffi.h`), bindings
  `dart:ffi` côté Flutter, avec un fallback Dart `package:cryptography` pour
  les builds sans NDK.
- État côté Dart : riverpod + go_router ; storage via flutter_secure_storage
  (Android Keystore) ; HTTP via dio ; QR via mobile_scanner.

## Next Steps
- Éditeur de blocs interactif (équivalent SPA `BlockEditor`).
- Abonnement WebSocket dans les écrans + invalidation des providers riverpod.
- Panneau de partage (notes & blocks) + UI export.
