# jot — Project Status

> État du projet, historique des sous-projets et roadmap.

## Sous-projets complétés

### SP1 — Workspace + crate core ✅
Workspace Cargo multi-crates, modèles de données (`Note`, `Board`, `Device`,
`LinkSession`), cryptographie complète : HKDF-SHA256, AES-256-GCM, Ed25519,
X25519. Stubs pour les crates suivants.

### SP2 — crate detect ✅
Détection de type par magic bytes : 10 formats image, 10 formats audio.
Aucune dépendance externe.

### SP3 — crate storage ✅
Couche de persistance SQLite via `sqlx` avec migrations versionnées
(`_sqlx_migrations`). Trait `BlobStore` avec deux backends :
- `LocalStore` — système de fichiers local
- `S3Store` — AWS S3 / Cloudflare R2 / MinIO

Migrations :
- `0001` — boards, notes, devices, link_sessions
- `0002` — identities, note_shares
- `0003` — invite_tokens
- `0004` — board_shares

### SP4 — crate api ✅
Serveur Axum avec :
- Auth JWT EdDSA (`jsonwebtoken` + `ed25519-dalek`), middleware `AuthenticatedDevice`
  qui vérifie l'existence du device en base à chaque requête
- Device linking par QR code (init → confirm → status polling)
- Endpoints : boards, notes (CRUD + blob upload/download), devices, health,
  identity, shares (notes + boards), invites, export, WebSocket
- Broadcast WebSocket temps réel
- SPA Preact embarquée via `rust-embed`
- Tests d'intégration (SQLite en mémoire)

### SP5 — crate cli ✅
Binaire `jot` :
- `jot serve [--port N] [--open-registration]` — démarre le serveur API
- `jot add [texte]` — ajoute une note (args / stdin / `$EDITOR`)
- `jot list [--boards] [--devices]` — liste notes, boards ou devices
- `jot new board <nom>` — crée un board
- `jot read <id>` — lit le contenu d'une note
- `jot link <token>` — approuve un device link depuis le terminal
- `jot whoami` — affiche identité et device courant
- `jot invite [--label L]` — génère un token d'invitation
- `jot migrate` — applique les migrations sans démarrer le serveur
- `jot tui` — lance le TUI Ratatui (navigation boards/notes)
- Pipe mode : `echo "note" | jot add` via `std::io::IsTerminal`
- Config persistante `~/.config/jot/config.toml`

### SP6 — SPA web + release pipeline ✅
- SPA Preact 10 avec `@preact/signals`, routing hash-based, thème clair/sombre
- Vues : register/link, boards, notes (list + card), devices, stats, profile, shared
- WebSocket temps réel pour les mises à jour de notes
- Panel de note redimensionnable (localStorage)
- Workflow GitHub Actions CI (`cargo fmt`, `clippy`, tests)
- Structure de release multi-plateforme prête (`release.yml` à déclencher sur tag `v*`)

---

## Fonctionnalités ajoutées après SP6

### Authentification & Multi-utilisateurs
- Inscription ouverte (`--open-registration`) ou par token d'invitation
- Écran de choix à la première connexion : nouveau compte vs lier un device existant
- Bouton de déconnexion explicite
- Redirection globale vers `/register` sur 401 (wrapper `authedFetch`)
- Révocation d'accès immédiate à la suppression d'un device (vérification DB dans le middleware)

### Identités & Contacts
- Friendly name par identité (lookup insensible à la casse)
- Génération de nom aléatoire `adjectif-nom-NNN` côté client
- Contacts récents : les personnes avec qui on a partagé apparaissent en chips dans le panel de partage
- `GET /identity/contacts` — endpoint dédié

### Partage
- Partage de boards (propriétaire garde l'écriture, partagé en lecture seule)
- Badge "partagé" sur les notes et boards déjà partagés
- Sidebar auto-refresh via signal `@preact/signals` quand un nouveau partage apparaît
- Révocation de partage (notes et boards)

### Export
- `GET /export` — export JSON de toutes les données (boards + notes avec contenu)
- Chiffrement optionnel côté client : AES-256-GCM + PBKDF2 (200 000 rounds), fichier `.jote`

### Versioning du schéma
- `Db::schema_version()` et `Db::migrate_with_version()` — rapport before/after
- `jot serve` affiche `Database schema migrated: v3 → v4` ou `up to date (v4)`
- `jot migrate` — commande standalone pour pré-migrer sans démarrer le serveur
- `GET /health` inclut `schema_version`

### Block structure (notes v1)
- Schéma `blocks` (parent/position/type/contenu chiffré), migration lazy
  `schema_version 0 → 1` côté SPA et via `jot block migrate`
- API : `/notes/:id/blocks`, `/blocks/:id` (CRUD + move/indent/outdent),
  `/blocks/:id/links`, `/(notes|blocks)/:id/backlinks`, `/tags`
- SPA : éditeur block-tree, Intellisense (`[[ ]]`, `(( ))`, `#tag`),
  pages Journal / Todo / Graph / About / BlocksPreview
- Partage par bloc : `/blocks/:id/share[s]`, `/shared/blocks` (re-chiffrement
  par destinataire via X25519 + AES-GCM)

### Parité CLI ↔ SPA / API
Toutes les routes API exposées par la SPA ont désormais un équivalent `jot`.
Ajouts récents :
- `jot board {rename, delete, move, reorder-notes}`
- `jot device {rename, delete}`
- `jot block {share, unshare, shares, shared}` (mêmes briques crypto que `share_note`)
- `jot invites` / `jot invite-revoke <token>`
- `jot link-init` / `jot link-status <token>`
- `jot tag {list, blocks, set}`
- `jot export [--out file]`, `jot contacts`, `jot backlinks --note|--block`
- `jot whoami --set-name / --set-lang` (PATCH /identity/me)
- Agrégations client-side : `jot journal [--date]`, `jot todo [--tag]`, `jot stats`

TUI volontairement non porté pour ces fonctions de gestion (refonte event-loop
intrusive — la CLI couvre la parité fonctionnelle).

---

## Roadmap — Ce qui reste à faire

### Priorité haute

**Chiffrement de bout en bout** (promesse centrale du projet — non implémenté)
Le design est documenté dans `docs/sharing-crypto-design.md`.
- Générer une clé X25519 par identité, stocker la clé privée localement
- Chiffrer les blobs avec AES-256-GCM côté client (SPA + CLI)
- Wrapper le DEK via ECDH pour le partage sécurisé
- Utiliser `window.crypto.subtle` dans la SPA, `x25519-dalek` dans le CLI

### Priorité moyenne

**CLI — flags manquants du README**
- `jot --server <url>` flag global
- `JOT_SERVER` variable d'environnement
- `jot list --format json`

**Notes audio et image**
- L'infrastructure `crate detect` est là, mais l'UI ne propose pas encore
  de créer des notes de type `voice` ou `image`
- Lecture audio dans la SPA

**Release pipeline**
- Déclencher `release.yml` sur un tag `v*` pour produire les 5 binaires statiques
- Générer le cache `.sqlx/` offline avant le premier tag

### Priorité basse

**Mobile**
- Flutter Android + FFI Rust (mentionné dans le README, rien de codé)

**Pro AI**
- LanceDB embeddings (`--features pro-ai`)

**Qualité**
- `jot server` → renommer en `jot serve` dans le README (déjà le cas dans le code)
- Meilleure gestion des erreurs réseau dans le TUI
