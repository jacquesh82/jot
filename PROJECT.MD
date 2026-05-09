# jot — Cahier de Spécifications Techniques
**Universal Encrypted Note System**
*v0.2 — Avril 2026 — Confidentiel, usage interne*

---

## Table des matières

1. [Vision du projet](#1-vision-du-projet)
2. [Identité et authentification](#2-identité-et-authentification)
3. [Architecture cryptographique](#3-architecture-cryptographique)
4. [Modèle de données](#4-modèle-de-données)
5. [Stack technique](#5-stack-technique)
6. [API REST](#6-api-rest)
7. [Interface CLI](#7-interface-cli)
8. [Déploiement](#8-déploiement)
9. [Features IA (tier Pro)](#9-features-ia-tier-pro)
10. [Questions ouvertes](#10-questions-ouvertes)
11. [Roadmap](#11-roadmap)

---

## 1. Vision du projet

### 1.1 Concept

jot est un gestionnaire de post-its numériques universels, chiffré de bout en bout, multi-device et sans compte utilisateur traditionnel. L'identité est anonyme, le contenu est opaque pour le serveur, et la philosophie d'usage est inspirée d'Unix : simple, composable, pipeable.

> **Tagline** : Post-it numérique universel — chiffré, anonyme, disponible partout.

### 1.2 Principes directeurs

- **Encrypted by design** — le serveur ne voit jamais le contenu des notes
- **Anonymous by default** — pas d'email, pas de mot de passe, identité UUID
- **Universal** — un binaire unique pour Linux, macOS, Windows ; app Flutter pour Android
- **Unix-friendly** — stdin/stdout, pipe, format JSON/TSV
- **Open Source** — core Rust, clients communautaires
- **Self-hostable** — Docker Compose, zéro dépendance externe obligatoire

### 1.3 Modèle commercial

| Tier | Périmètre | Modèle |
|---|---|---|
| Free | Self-hosted illimité + cloud 1 device | Communauté / adoption |
| Pro | Multi-device cloud + features IA | Abonnement mensuel SaaS |
| Team | Workspaces partagés + admin | Abonnement équipe |

---

## 2. Identité et authentification

### 2.1 Modèle d'identité

Aucun email ni mot de passe n'est requis. L'identité est générée localement au premier lancement.

| Champ | Description |
|---|---|
| `uuid` | Identifiant unique généré côté client (UUID v4) |
| `pseudo` | Nom choisi librement, non unique globalement |
| `avatar` | URL ou seed procédural (ex. : identicon, dicebear) |
| `devices[]` | Liste des appareils liés à cette identité |
| `created_at` | Timestamp de création locale |

> ⚠️ **Attention** : Si tous les devices sont perdus sans backup, la clé symétrique est irrécupérable. Un mécanisme d'export explicite est requis.

### 2.2 Mécanismes d'authentification

L'authentification repose exclusivement sur le QR code ou un flux URL + code court, sans aucune saisie de mot de passe.

#### 2.2.1 Flux QR code (device riche)

- Le device existant affiche un QR code encodant une URL éphémère
- Le nouveau device scanne → échange de clés ECDH X25519
- La clé symétrique est transmise chiffrée, le shared secret est jeté après usage

#### 2.2.2 Flux URL + code court (terminal / RFC 8628)

```
$ jot link

Ouvrez cette URL sur un device déjà lié :
https://jot.example.com/link/a3f9k2

Code de vérification : 7842

En attente... ⠋
```

Le QR code encode simplement la même URL — les deux flux utilisent le même endpoint backend.

#### 2.2.3 Protocole de liaison (séquence)

| Étape | Acteur | Action |
|---|---|---|
| 1 | Device B | Génère une paire X25519 éphémère |
| 2 | Device B | Affiche QR ou URL + code (TTL 5 min, polling 5s) |
| 3 | Device A | Scanne le QR ou ouvre l'URL, confirme le code |
| 4 | Device A | ECDH → chiffre la clé symétrique avec le shared secret |
| 5 | Serveur | Stocke le blob chiffré (canal éphémère) |
| 6 | Device B | Récupère le blob, déchiffre → obtient la clé symétrique |
| 7 | Device B | Stocke la clé localement, sync les notes |

---

## 3. Architecture cryptographique

### 3.1 Principe général

Le serveur stocke uniquement des blobs chiffrés. Il ne peut jamais accéder au contenu des notes, ni aux clés de déchiffrement, sauf opt-in explicite pour les features IA (voir §3.4).

### 3.2 Algorithmes retenus

| Usage | Algorithme |
|---|---|
| Échange de clés | X25519 (ECDH) — RustCrypto |
| Signature / identité device | Ed25519 — RustCrypto |
| Chiffrement symétrique | AES-256-GCM — RustCrypto |
| Dérivation de clés | HKDF-SHA256 — RustCrypto |
| QR / transport éphémère | TLS 1.3 + ECDH éphémère |

### 3.3 Dérivation des clés

Toutes les clés fonctionnelles sont dérivées depuis une clé maître via HKDF. La clé maître ne quitte jamais le device.

```
master_key  (jamais transmise)
    │
    ├── HKDF("jot-v1-notes")  →  notes_key   (chiffre les blobs)
    └── HKDF("jot-v1-ai")     →  ai_key      (opt-in Pro, voir §3.4)
```

Exemple Rust (crate `hkdf`) :

```rust
use hkdf::Hkdf;
use sha2::Sha256;

let hk = Hkdf::<Sha256>::new(None, &master_key);

let mut notes_key = [0u8; 32];
hk.expand(b"jot-v1-notes", &mut notes_key).unwrap();

let mut ai_key = [0u8; 32];
hk.expand(b"jot-v1-ai", &mut ai_key).unwrap();
```

### 3.4 Clé dérivée IA (tier Pro)

Les features IA nécessitent que le worker serveur puisse lire le contenu des notes. L'utilisateur opt-in explicitement et transmet la `ai_key` au serveur via canal chiffré. Cette clé est stockée dans un HSM ou une enclave côté serveur.

- La `notes_key` n'est **jamais** transmise au serveur IA
- La `ai_key` peut être révoquée à tout moment (`jot pro revoke-ai`)
- Les métadonnées IA (tags, embeddings, tâches) sont stockées chiffrées avec `ai_key`
- La rotation de clé IA est possible sans toucher aux blobs notes

> **Consentement explicite** : L'activation des features IA affiche un avertissement clair précisant que le serveur déchiffre temporairement les notes. Aucune donnée n'est conservée en clair.

### 3.5 Backup de la clé maître

Un mécanisme d'export de la clé maître est requis pour éviter la perte définitive des données. Options envisagées :

- Export chiffré par passphrase (fichier téléchargeable)
- Paper key style BIP39 (liste de mots mnémotechniques)
- QR code imprimable contenant la clé chiffrée

---

## 4. Modèle de données

### 4.1 Note (post-it)

Une note est un post-it : courte, typée, organisée manuellement dans un board. Le contenu est toujours un blob chiffré côté serveur.

| Champ | Description |
|---|---|
| `id` | UUID v4 — identifiant unique |
| `type` | Enum : `text` \| `voice` \| `image` |
| `content` | Blob chiffré AES-256-GCM (opaque serveur) |
| `thumbnail` | Optionnel — preview chiffrée (image/voice) |
| `duration_ms` | Durée en ms — voice uniquement |
| `color` | Couleur du post-it (hex ou enum) |
| `board_id` | Référence au board parent |
| `position` | Entier — ordre manuel dans le board |
| `s3_key` | Clé RustFS du blob (UUID de la note) |
| `size` | Taille du blob en octets |
| `created_at` | Timestamp de création |
| `updated_at` | Timestamp de dernière modification |

### 4.2 Limites de taille

| Type | Limite |
|---|---|
| `text` | 2 000 caractères maximum |
| `voice` | 60 secondes maximum (≈ 500 Ko Opus) |
| `image` | 5 Mo maximum (compression JPEG/WebP avant chiffrement) |

> **Note** : La compression est effectuée côté client, avant chiffrement. Le serveur ne peut pas compresser un blob chiffré.

### 4.3 Board

| Champ | Description |
|---|---|
| `id` | UUID v4 |
| `identity_id` | Référence à l'identité propriétaire |
| `name` | Nom du board (ex. : Idées, Work, Personnel) |
| `position` | Ordre manuel entre boards |
| `created_at` | Timestamp |

### 4.4 Device

| Champ | Description |
|---|---|
| `id` | UUID v4 |
| `identity_id` | Référence à l'identité |
| `pub_key_x25519` | Clé publique X25519 du device (hex) |
| `pub_key_ed25519` | Clé publique Ed25519 du device (hex) |
| `name` | Nom lisible (ex. : iPhone de Jacques) |
| `last_seen` | Dernier timestamp de connexion |

### 4.5 Link Session (QR / URL flow)

| Champ | Description |
|---|---|
| `token` | Token URL éphémère (random 128 bits) |
| `code` | Code court numérique à 4 chiffres |
| `status` | Enum : `pending` \| `confirmed` \| `expired` |
| `pub_key_initiator` | Clé X25519 du device demandeur |
| `encrypted_symkey` | Blob ECDH — clé symétrique chiffrée |
| `expires_at` | Timestamp d'expiration (TTL 5 min) |

---

## 5. Stack technique

### 5.1 Vue d'ensemble

```
┌─────────────────────────────────────────────────┐
│                  Docker Compose                  │
│                                                  │
│  ┌───────────────┐  ┌──────────┐  ┌──────────┐  │
│  │   jot-api     │  │  jot-db  │  │  rustfs  │  │
│  │  Rust + Axum  │  │ Postgres │  │  S3 API  │  │
│  │  CLI + Web    │  │          │  │          │  │
│  └───────────────┘  └──────────┘  └──────────┘  │
└─────────────────────────────────────────────────┘
```

### 5.2 Composants backend

| Composant | Technologie |
|---|---|
| Langage | Rust (edition 2021) |
| Framework HTTP | Axum |
| Base de données | PostgreSQL 16 via sqlx |
| Object storage | RustFS (compatible S3) |
| Client S3 | aws-sdk-rust (compatible S3 générique) |
| Cryptographie | RustCrypto (x25519-dalek, aes-gcm, ed25519-dalek, hkdf) |
| CLI | Clap v4 |
| TUI | Ratatui |
| Sérialisation | serde / serde_json |
| Migrations DB | sqlx-cli |
| QR code terminal | qrcode crate (half-blocks Unicode) |

### 5.3 Structure du workspace Rust

```
jot/
├── Cargo.toml              (workspace)
├── crates/
│   ├── core/               (crypto, modèles, logique métier)
│   ├── api/                (Axum — REST + WebSocket)
│   ├── cli/                (Clap + Ratatui + pipe mode)
│   ├── detect/             (magic bytes — détection auto du type)
│   └── storage/            (PostgreSQL sqlx + RustFS S3)
├── docker-compose.yml
├── .github/
│   └── workflows/
│       └── release.yml     (build + GHCR + GitHub Releases)
└── clients/                (communauté)
    ├── flutter/
    └── web/
```

### 5.4 Client mobile

| Composant | Technologie |
|---|---|
| Framework | Flutter (Dart) |
| Plateforme v1 | Android |
| Extension future | iOS sans modification majeure |
| Cryptographie | FFI → crate `core` Rust compilé en `.so` |
| Scanner QR | mobile_scanner (pub.dev) |
| Enregistrement audio | record (pub.dev) |
| Caméra / galerie | image_picker (pub.dev) |

### 5.5 Cross-compilation

| Target Rust | Plateforme | Statut |
|---|---|---|
| `linux/amd64` | Linux x86_64 | ✓ cible principale |
| `linux/arm64` | Linux ARM (serveur, Pi) | ✓ |
| `darwin/amd64` | macOS Intel | ✓ |
| `darwin/arm64` | macOS Apple Silicon | ✓ |
| `windows/amd64` | Windows x86_64 | ✓ |

---

## 6. API REST

L'API suit les conventions REST. Tous les endpoints (sauf `/health` et `/link`) requièrent un device token JWT signé Ed25519. La spec complète est générée au format OpenAPI 3.1 depuis Axum.

### 6.1 Authentification

| Endpoint | Description |
|---|---|
| `POST /auth/register` | Enregistre une nouvelle identité (uuid, pseudo, avatar) |
| `POST /auth/device` | Enregistre un nouveau device (clés publiques) |
| `POST /link/init` | Démarre un flux de liaison (retourne url, code, token) |
| `GET /link/{token}` | Page de confirmation (device existant) |
| `POST /link/confirm` | Soumet la clé symétrique chiffrée |
| `GET /link/status/{token}` | Polling statut (pending/confirmed/expired) |

### 6.2 Notes

| Endpoint | Description |
|---|---|
| `GET /notes` | Liste les notes (metadata, pas le contenu) |
| `POST /notes` | Crée une note (metadata + s3_key) |
| `GET /notes/{id}` | Récupère les metadata d'une note |
| `DELETE /notes/{id}` | Supprime une note et son blob S3 |
| `PATCH /notes/{id}` | Met à jour position, color, board |
| `GET /notes/{id}/blob` | Télécharge le blob chiffré |
| `PUT /notes/{id}/blob` | Upload le blob chiffré |

### 6.3 Boards

| Endpoint | Description |
|---|---|
| `GET /boards` | Liste les boards de l'identité |
| `POST /boards` | Crée un board |
| `PATCH /boards/{id}` | Renomme ou repositionne un board |
| `DELETE /boards/{id}` | Supprime un board et ses notes |
| `PATCH /boards/{id}/reorder` | Réordonne les notes (array de positions) |

### 6.4 Devices

| Endpoint | Description |
|---|---|
| `GET /devices` | Liste les devices liés à l'identité |
| `DELETE /devices/{id}` | Révoque un device |
| `POST /devices/{id}/rename` | Renomme un device |

### 6.5 Pro / IA

| Endpoint | Description |
|---|---|
| `POST /pro/enable-ai` | Active les features IA (transmet ai_key chiffrée) |
| `DELETE /pro/revoke-ai` | Révoque la clé IA côté serveur |
| `GET /pro/tasks` | Liste les tâches extraites par l'IA |
| `GET /pro/search?q=` | Recherche sémantique (embeddings) |
| `GET /pro/notes/{id}/transcript` | Transcription Whisper d'une note vocale |
| `GET /pro/boards/{id}/summary` | Résumé IA d'un board |

---

## 7. Interface CLI

### 7.1 Philosophie Unix

Le binaire `jot` détecte automatiquement s'il est exécuté dans un TTY ou dans un pipe, et adapte son comportement en conséquence.

| Contexte | Comportement |
|---|---|
| TTY détecté | Mode interactif — TUI Ratatui, couleurs ANSI, QR code |
| Pipe détecté | Mode machine — JSON par défaut, pas de couleurs, pas d'UI |

### 7.2 Détection automatique du type de note

Le flag `--type` est absent. Le type de la note est détecté automatiquement depuis les magic bytes des 16 premiers octets du payload stdin. Aucune indication manuelle n'est requise.

```
stdin reçu
  │
  ├── lecture des 16 premiers octets (non bloquant)
  │
  ├── magic bytes reconnus ?
  │     ├── image (JPEG, PNG, WebP, GIF, BMP, TIFF,
  │     │         HEIC, AVIF, ICO, PDF)    → type = image
  │     ├── audio (OGG, MP3, WAV, FLAC, AAC, M4A,
  │     │         AIFF, WebM, AMR, WMA)    → type = voice
  │     └── UTF-8 valide                   → type = text
  │
  └── format inconnu → erreur explicite
```

### 7.3 Formats supportés

#### Images (10 formats)

| Format | Magic bytes | Notes |
|---|---|---|
| JPEG | `FF D8 FF` | Universel |
| PNG | `89 50 4E 47 0D 0A 1A 0A` | Lossless |
| WebP | `RIFF` + `WEBP` (offset 8) | Google — RIFF container |
| GIF | `47 49 46 38` | GIF87a / GIF89a |
| BMP | `42 4D` | Windows bitmap |
| TIFF | `49 49 2A 00` ou `4D 4D 00 2A` | Little / big endian |
| HEIC | `ftyp` + `heic`/`heix` (offset 4) | iPhone — ISO Base Media |
| AVIF | `ftyp` + `avif`/`avis` (offset 4) | Moderne, AV1 |
| ICO | `00 00 01 00` | Windows icon |
| PDF | `25 50 44 46` (`%PDF`) | Document image |

#### Audio (10 formats)

| Format | Magic bytes | Notes |
|---|---|---|
| OGG / Opus | `4F 67 67 53` (`OggS`) | Recommandé pour les notes vocales |
| MP3 | `49 44 33` (`ID3`) ou `FF FB`/`F3`/`F2` | Universel |
| WAV | `RIFF` + `WAVE` (offset 8) | RIFF container |
| FLAC | `66 4C 61 43` (`fLaC`) | Lossless |
| AAC | `FF F1` ou `FF F9` | iOS natif (ADTS) |
| M4A | `ftyp` + `M4A ` (offset 4+8) | ISO Base Media container |
| AIFF | `FORM` + `AIFF` (offset 8) | macOS natif |
| WebM | `1A 45 DF A3` | Matroska / WebM |
| AMR | `23 21 41 4D 52` (`#!AMR`) | Téléphonie mobile |
| WMA | `30 26 B2 75` | Windows Media Audio |

> **Ambiguïté RIFF** : `RIFF` est partagé entre WAV (`RIFF`+`WAVE`) et WebP (`RIFF`+`WEBP`). La distinction se fait sur les octets 8–11. Idem pour le container `ftyp` (ISO Base Media) partagé entre HEIC, AVIF et M4A.

### 7.4 Commandes principales

| Commande | Description |
|---|---|
| `jot server` | Lance l'instance backend (API + SPA web) |
| `jot add [texte]` | Crée une note texte (argument ou stdin) |
| `jot list` | Liste les notes (TTY : tableau, pipe : JSON) |
| `jot get <id>` | Affiche le contenu d'une note (stdout) |
| `jot delete <id>` | Supprime une note |
| `jot boards` | Liste les boards |
| `jot link` | Affiche QR + URL + code pour lier un device |
| `jot devices` | Liste les devices liés |
| `jot export` | Exporte toutes les notes (JSON chiffré) |
| `jot pro enable-ai` | Active les features IA (opt-in explicite) |
| `jot pro revoke-ai` | Révoque la clé IA côté serveur |

### 7.5 Exemples de pipes

```bash
# Texte — détecté automatiquement (UTF-8)
echo "faire les courses" | jot

# Image JPEG — magic bytes FF D8 FF détectés
cat photo.jpg | jot

# Screenshot direct (PNG) — magic bytes 89 50 4E 47 détectés
scrot - | jot

# Audio OGG/Opus — magic bytes OggS détectés
arecord -d 30 -f cd | oggenc - | jot

# Audio MP3 — magic bytes ID3 détectés
rec -t mp3 - trim 0 30 | jot

# Lister et filtrer avec fzf
jot list | fzf | xargs jot get

# Filtrer par type en JSON
jot list --format json | jq '.[] | select(.type == "voice")'

# Jouer une note vocale
jot get <id> | aplay

# Exporter et compresser
jot export | gzip > backup.gz
```

### 7.6 Formats de sortie

| Flag | Description |
|---|---|
| `--format json` | Défaut si pipe — compatible jq |
| `--format table` | Défaut si TTY — tableau coloré |
| `--format tsv` | Pour awk / cut |

### 7.7 Implémentation — crate `detect`

La détection est isolée dans un crate dédié `crates/detect/` avec ses propres tests unitaires pour chaque format. Seuls les 16 premiers octets sont lus avant de streamer le reste du payload — zéro surcoût mémoire.

```rust
pub fn detect_type(buf: &[u8]) -> NoteType {
    // Images
    if buf.starts_with(&[0xFF, 0xD8, 0xFF])               { return Image(Jpeg)  }
    if buf.starts_with(&[0x89,0x50,0x4E,0x47,0x0D,0x0A,
                          0x1A,0x0A])                      { return Image(Png)   }
    if buf.starts_with(b"RIFF") && buf.len() >= 12
       && &buf[8..12] == b"WEBP"                           { return Image(WebP)  }
    if buf.starts_with(b"GIF8")                            { return Image(Gif)   }
    if buf.starts_with(&[0x42, 0x4D])                     { return Image(Bmp)   }
    if buf.starts_with(&[0x49,0x49,0x2A,0x00])
    || buf.starts_with(&[0x4D,0x4D,0x00,0x2A])            { return Image(Tiff)  }
    if buf.starts_with(&[0x00,0x00,0x01,0x00])            { return Image(Ico)   }
    if buf.starts_with(b"%PDF")                            { return Image(Pdf)   }
    // HEIC / AVIF — ftyp box à offset 4
    if buf.len() >= 12 && &buf[4..8] == b"ftyp" {
        match &buf[8..12] {
            b"heic" | b"heix" | b"mif1" => return Image(Heic),
            b"avif" | b"avis"           => return Image(Avif),
            b"M4A "                     => return Voice(M4a),
            _ => {}
        }
    }

    // Audio
    if buf.starts_with(b"OggS")                            { return Voice(Ogg)   }
    if buf.starts_with(b"fLaC")                            { return Voice(Flac)  }
    if buf.starts_with(b"ID3")
    || buf.starts_with(&[0xFF,0xFB])
    || buf.starts_with(&[0xFF,0xF3])
    || buf.starts_with(&[0xFF,0xF2])                       { return Voice(Mp3)   }
    if buf.starts_with(b"RIFF") && buf.len() >= 12
       && &buf[8..12] == b"WAVE"                           { return Voice(Wav)   }
    if buf.starts_with(b"FORM") && buf.len() >= 12
       && &buf[8..12] == b"AIFF"                           { return Voice(Aiff)  }
    if buf.starts_with(&[0xFF,0xF1])
    || buf.starts_with(&[0xFF,0xF9])                       { return Voice(Aac)   }
    if buf.starts_with(&[0x1A,0x45,0xDF,0xA3])            { return Voice(WebM)  }
    if buf.starts_with(b"#!AMR")                           { return Voice(Amr)   }
    if buf.starts_with(&[0x30,0x26,0xB2,0x75])            { return Voice(Wma)   }

    // Fallback
    if std::str::from_utf8(buf).is_ok() { return Text }
    NoteType::Unknown  // → erreur explicite côté CLI
}
```

---

## 8. Déploiement

### 8.1 Docker Compose

```yaml
services:
  api:
    image: ghcr.io/xxx/jot-api:latest
    ports:
      - "8080:8080"
    environment:
      - DATABASE_URL=postgres://jot:secret@db:5432/jot
      - S3_ENDPOINT=http://storage:9000
      - S3_BUCKET=jot
      - S3_ACCESS_KEY=minioadmin
      - S3_SECRET_KEY=minioadmin
      - JOT_JWT_SECRET=changeme
      - JOT_LINK_TTL=300
      - JOT_MAX_BLOB_SIZE=5242880
    depends_on: [db, storage]

  db:
    image: postgres:16-alpine
    volumes:
      - pgdata:/var/lib/postgresql/data
    environment:
      - POSTGRES_DB=jot
      - POSTGRES_USER=jot
      - POSTGRES_PASSWORD=secret

  storage:
    image: rustfs/rustfs:latest
    ports:
      - "9000:9000"
      - "9001:9001"
    volumes:
      - rustfsdata:/data
    environment:
      - RUSTFS_ACCESS_KEY=minioadmin
      - RUSTFS_SECRET_KEY=minioadmin

volumes:
  pgdata:
  rustfsdata:
```

### 8.2 Variables d'environnement

| Variable | Description |
|---|---|
| `JOT_PORT` | Port d'écoute de l'API (défaut : 8080) |
| `DATABASE_URL` | URL de connexion PostgreSQL |
| `S3_ENDPOINT` | URL de l'instance RustFS / S3 |
| `S3_BUCKET` | Nom du bucket S3 |
| `S3_ACCESS_KEY` / `S3_SECRET_KEY` | Credentials S3 |
| `JOT_JWT_SECRET` | Secret de signature des device tokens |
| `JOT_LINK_TTL` | Durée de vie du QR/URL en secondes (défaut : 300) |
| `JOT_MAX_BLOB_SIZE` | Taille max des blobs en octets (défaut : 5 242 880) |

### 8.3 Pipeline CI/CD

- Déclenchement sur tag `vX.Y.Z` (GitHub Actions)
- Build Rust multi-arch (`linux/amd64`, `linux/arm64`, `darwin`, `windows`)
- Build Docker multi-arch → push `ghcr.io`
- Release GitHub avec les binaires attachés en assets
- Build Flutter → APK Android en artefact de release

---

## 9. Features IA (tier Pro)

### 9.1 Périmètre

| Feature | Description |
|---|---|
| Transcription vocale | Whisper — voice → texte indexable |
| Extraction de tâches | LLM — détecte les actions dans le texte libre |
| Tags automatiques | LLM — catégorisation contextuelle |
| Résumé de board | LLM — synthèse des notes d'un board |
| Recherche sémantique | Embeddings — recherche par sens, pas par mot-clé |
| OCR image | Vision — extraction de texte depuis photo/whiteboard |
| Rappels intelligents | LLM — détecte les dates/délais et crée des rappels |

### 9.2 Consentement et transparence

L'activation des features IA est un opt-in explicite :

```
$ jot pro enable-ai

⚠  Les features IA nécessitent que le serveur déchiffre
   temporairement vos notes pour traitement.
   Aucune donnée n'est conservée en clair.
   Voir : https://jot.dev/privacy/ai

Confirmer ? [y/N]
```

### 9.3 Révocation

```
$ jot pro revoke-ai
✓  Clé IA révoquée. Les workers IA n'ont plus accès à vos notes.
```

---

## 10. Questions ouvertes

| Sujet | Question | Statut |
|---|---|---|
| Mode local-only | SQLite autonome sans serveur, sync optionnelle ? | ⏳ À décider |
| Conflits de sync | Last-write-wins ou CRDT pour éditions simultanées offline ? | ⏳ À décider |
| Stockage blobs | En base PostgreSQL (bytea) ou RustFS systématiquement ? | ✅ RustFS retenu |
| Discriminant pseudo | Pseudo libre ou suffixe `#XXXX` pour unicité globale ? | ⏳ À décider |
| Paper key backup | BIP39, QR imprimable, ou fichier chiffré par passphrase ? | ⏳ À décider |
| WebSocket sync | Temps réel multi-device ou pull à l'ouverture ? | ⏳ À décider |
| Nom de domaine | `jot.dev`, `jot.app`, ou autre ? | ⏳ À décider |

---

## 11. Roadmap

### Sprint 0 — Fondations
- Workspace Rust (`core`, `api`, `cli`, `storage`, `detect`)
- Modèle de données PostgreSQL + migrations sqlx
- Intégration RustFS S3
- Crypto core : X25519, AES-256-GCM, Ed25519, HKDF
- Docker Compose de développement

### Sprint 1 — MVP CLI
- Endpoints auth (`register`, `device`, `link`)
- Flux QR + URL/code (RFC 8628)
- CRUD notes texte via CLI
- Pipe mode (stdin/stdout, TTY detection)
- Détection automatique du type via magic bytes (`crate detect`)
- `jot list` / `jot get` / `jot add` / `jot link`

### Sprint 2 — Voice & Image
- Upload/download blobs voice et image
- Compression côté client avant chiffrement
- CLI : pipe audio (`arecord`, `sox`) et image
- Boards et réordonnancement manuel

### Sprint 3 — Web & Mobile
- SPA web embarquée dans le binaire
- App Flutter Android (auth QR, CRUD, voice, image)
- FFI Rust → Flutter pour le core crypto

### Sprint 4 — Pro IA
- Dérivation `ai_key` + opt-in consentement
- Intégration Whisper (transcription)
- Extraction de tâches LLM
- Recherche sémantique (embeddings)
- Pipeline CI/CD GitHub Actions → GHCR

---

*jot — v0.2 — Avril 2026*
