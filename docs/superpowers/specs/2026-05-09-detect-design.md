# Spec : crate detect — détection de type par magic bytes

**Date :** 2026-05-09
**Sous-projet :** 2 / 8
**Statut :** approuvé

---

## 1. Périmètre

Implémenter `crate detect` : détection automatique du type d'un payload (image, audio, texte) depuis ses magic bytes. Remplace le stub existant dans `crates/detect/`.

Dépend de `jot-core` pour le type `NoteType`. Aucune autre dépendance interne.

---

## 2. Interface publique

```rust
// crates/detect/src/lib.rs

use jot_core::models::NoteType;

#[derive(Debug, thiserror::Error)]
pub enum DetectError {
    #[error("input too short to detect type (got {0} bytes, need at least 16)")]
    TooShort(usize),
    #[error("unknown file format")]
    UnknownFormat,
    #[error("invalid UTF-8 text content")]
    InvalidUtf8,
}

pub fn detect(bytes: &[u8]) -> Result<NoteType, DetectError>
```

- `bytes` : payload complet (le caller lit tout le contenu avant d'appeler `detect`)
- Retourne `NoteType::{Image, Voice, Text}` ou une erreur
- Pas d'I/O dans `detect` — le caller gère la lecture

---

## 3. Logique de détection

Ordre strict : image → audio → UTF-8 → erreur.

```
1. bytes.len() < 16              → DetectError::TooShort(bytes.len())
2. Check image magic bytes       → Ok(NoteType::Image)
3. Check audio magic bytes       → Ok(NoteType::Voice)
4. std::str::from_utf8(bytes) OK → Ok(NoteType::Text)
5. Sinon                         → Err(DetectError::UnknownFormat)
```

### 3.1 Formats image (10)

| Format | Condition |
|---|---|
| JPEG   | `bytes[0..3] == [0xFF, 0xD8, 0xFF]` |
| PNG    | `bytes[0..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]` |
| WebP   | `bytes[0..4] == b"RIFF" && bytes[8..12] == b"WEBP"` |
| GIF    | `bytes[0..4] == b"GIF8"` |
| BMP    | `bytes[0..2] == [0x42, 0x4D]` |
| TIFF   | `bytes[0..4] == [0x49,0x49,0x2A,0x00]` ou `[0x4D,0x4D,0x00,0x2A]` |
| HEIC   | `bytes[4..8] == b"ftyp" && (bytes[8..12] == b"heic" \| bytes[8..12] == b"heix")` |
| AVIF   | `bytes[4..8] == b"ftyp" && (bytes[8..12] == b"avif" \| bytes[8..12] == b"avis")` |
| ICO    | `bytes[0..4] == [0x00, 0x00, 0x01, 0x00]` |
| PDF    | `bytes[0..4] == b"%PDF"` |

### 3.2 Formats audio (11 signatures, 10 formats)

| Format | Condition |
|---|---|
| OGG/Opus | `bytes[0..4] == b"OggS"` |
| MP3 (ID3)| `bytes[0..3] == b"ID3"` |
| MP3 (raw)| `bytes[0] == 0xFF && bytes[1] & 0xE0 == 0xE0` |
| WAV      | `bytes[0..4] == b"RIFF" && bytes[8..12] == b"WAVE"` |
| FLAC     | `bytes[0..4] == b"fLaC"` |
| AAC      | `bytes[0..2] == [0xFF, 0xF1]` ou `[0xFF, 0xF9]` |
| M4A      | `bytes[4..8] == b"ftyp" && bytes[8..12] == b"M4A "` |
| AIFF     | `bytes[0..4] == b"FORM" && bytes[8..12] == b"AIFF"` |
| WebM     | `bytes[0..4] == [0x1A, 0x45, 0xDF, 0xA3]` |
| AMR      | `bytes[0..5] == b"#!AMR"` |
| WMA      | `bytes[0..4] == [0x30, 0x26, 0xB2, 0x75]` |

### 3.3 Priorités de déambiguïsation

- `RIFF` prefix : WebP checké **avant** WAV (WebP → Image, WAV → Voice)
- `ftyp` prefix : HEIC checké **avant** AVIF, AVIF **avant** M4A
- `[0xFF, 0xD8]` (JPEG) checké **avant** MP3 raw (`0xFF && 0xE0`) — JPEG a 3 bytes distinctifs, pas d'ambiguïté réelle mais l'ordre est explicite

---

## 4. Dépendances `detect/Cargo.toml`

```toml
[dependencies]
jot-core = { path = "../core" }
thiserror = "1"
```

---

## 5. Tests

### 5.1 Tests par format image (10 tests)
Un test par format avec magic bytes minimaux padded à 16 octets.

### 5.2 Tests par format audio (10 tests)
Un test par format audio.

### 5.3 Tests de collision (3 tests)
- `RIFF+WEBP` → `Image` (pas `Voice`)
- `RIFF+WAVE` → `Voice` (pas `Image`)
- `ftyp+heic` → `Image` (pas confondu avec M4A)

### 5.4 Tests texte et erreurs (3 tests)
- UTF-8 valide → `Text`
- `bytes.len() < 16` → `DetectError::TooShort`
- Zéros (`[0u8; 16]`) → `DetectError::UnknownFormat`

---

## 6. Hors périmètre

- Détection depuis un chemin de fichier ou un `Read` trait — le caller lit le contenu
- Formats supplémentaires — les 20 formats de PROJECT.MD sont la liste complète pour v1
- WMA — inclus dans la liste mais la signature `[0x30, 0x26, 0xB2, 0x75]` couvre l'ASF container (WMA/WMV) ; suffisant pour v1
