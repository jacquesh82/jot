# Spec : crate cli — Clap + reqwest + Ratatui TUI

**Date :** 2026-05-10
**Sous-projet :** 5 / 8
**Statut :** approuvé

---

## 1. Périmètre

Implémenter `crate cli` : binaire `jot` exposant des commandes Clap et un TUI Ratatui.
- Architecture async-first : tokio runtime unique partagé
- Commandes : `add`, `list`, `boards`, `serve`, `tui`
- Pipe mode : `echo "note" | jot add` détecté automatiquement via `atty`
- HTTP client reqwest vers l'API locale (crate api)
- TUI complet : navigation boards/notes + création inline + suppression

Dépend de `jot-core`, `storage`, `api`. Binaire nommé `jot`.

---

## 2. Structure

```
crates/cli/
├── Cargo.toml
└── src/
    ├── main.rs           # tokio::main, Clap dispatch
    ├── config.rs         # Config + token JWT (~/.config/jot/)
    ├── client.rs         # JotClient — reqwest wrapper
    ├── commands/
    │   ├── mod.rs        # Cli struct Clap (derive)
    │   ├── add.rs        # jot add [text...] + pipe mode
    │   ├── list.rs       # jot list [--board <id>]
    │   ├── boards.rs     # jot boards
    │   └── serve.rs      # jot serve [--port]
    └── tui/
        ├── mod.rs        # run_tui() — event loop principal
        ├── app.rs        # App state machine
        └── ui.rs         # render() — layout Ratatui
```

---

## 3. Dépendances

```toml
[package]
name = "cli"
version.workspace = true
edition.workspace = true

[[bin]]
name = "jot"
path = "src/main.rs"

[dependencies]
jot-core = { path = "../core" }
storage  = { path = "../storage" }
api      = { path = "../api" }
clap     = { version = "4", features = ["derive"] }
tokio    = { version = "1", features = ["full"] }
reqwest  = { version = "0.12", features = ["json"] }
ratatui  = "0.26"
crossterm = "0.27"
serde    = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
dirs     = "5"
toml     = "0.8"
uuid     = { version = "1", features = ["v4"] }
atty     = "0.2"
chrono   = "0.4"
```

Pas de `[lib]` — binaire pur.

---

## 4. Config (`config.rs`)

Fichier : `~/.config/jot/config.toml`

```toml
server_url = "http://127.0.0.1:3000"
token      = "<jwt>"
device_id  = "<uuid>"
identity_id = "<uuid>"
```

```rust
#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub server_url: Option<String>,
    pub token: Option<String>,
    pub device_id: Option<String>,
    pub identity_id: Option<String>,
}

impl Config {
    pub fn load() -> Self            // lit ~/.config/jot/config.toml, defaults si absent
    pub fn save(&self) -> Result<()> // écrit le fichier
    pub fn server_url(&self) -> &str // "http://127.0.0.1:3000" par défaut
}
```

---

## 5. Client HTTP (`client.rs`)

```rust
pub struct JotClient {
    base_url: String,
    token: Option<String>,
    inner: reqwest::Client,
}

impl JotClient {
    pub fn new(config: &Config) -> Self
    pub fn from_config() -> Self      // raccourci: Config::load() + new()

    pub async fn get_boards(&self) -> Result<Vec<BoardSummary>, CliError>
    pub async fn get_notes(&self, board_id: Uuid) -> Result<Vec<NoteSummary>, CliError>
    pub async fn create_note(&self, board_id: Uuid, content: &str) -> Result<Uuid, CliError>
    // 2 étapes : POST /notes → { id } puis PUT /notes/{id}/blob
    pub async fn delete_note(&self, note_id: Uuid) -> Result<(), CliError>

    // Auth
    pub async fn register_device(
        &self,
        device_id: Uuid,
        identity_id: Uuid,
        pub_key_x25519: &str,
        pub_key_ed25519: &str,
        name: &str,
    ) -> Result<String, CliError>  // → JWT
}

#[derive(Serialize, Deserialize)]
pub struct BoardSummary { pub id: Uuid, pub name: String, pub position: i32 }

#[derive(Serialize, Deserialize)]
pub struct NoteSummary { pub id: Uuid, pub note_type: String, pub blob_key: String, pub color: String, pub position: i32 }
```

Si token absent lors d'un appel auth-requis → `CliError::NotAuthenticated` avec message :
> `"Run 'jot serve' first to register this device."`

---

## 6. Commandes

### 6.1 `jot add [TEXT...] [--board <id>]`

- Si `TEXT` fourni → note texte = args joints par espace
- Si stdin non-terminal (`!atty::is(Stream::Stdin)`) → lit stdin entier
- Si rien → ouvre `$EDITOR` avec fichier temp, lit le résultat
- Board cible : `--board <id>` OU premier board de la liste
- Appelle `client.create_note(board_id, content).await`

### 6.2 `jot list [--board <id>]`

- `client.get_notes(board_id).await`
- Affichage : `<id>  <type>  <position>` (une note par ligne)

### 6.3 `jot boards`

- `client.get_boards().await`
- Affichage : `<id>  <name>  (<position>)` (un board par ligne)

### 6.4 `jot serve [--port <port>]`

Séquence au premier démarrage :
1. Génère identity_id + device_id (Uuid::new_v4)
2. Génère keypair Ed25519 via `jot_core::crypto::signing::generate_device_keypair()`
3. Initialise DB (`storage::Db::connect`) + LocalStore
4. Construit `AppState` + `build_router(state)` (crate api)
5. Appelle `POST /auth/device` en local → récupère JWT
6. Sauvegarde token + ids dans `Config`
7. Lance `axum::Server` sur `0.0.0.0:<port>` (default 3000)
8. Bloque jusqu'à Ctrl-C

Si config.token déjà présent → saute les étapes 1-6 (réutilise l'identité existante).

### 6.5 `jot tui`

Lance `tui::run_tui(client).await`.

---

## 7. TUI (`tui/`)

### 7.1 Layout

```
┌─── Boards (30%) ─────┬──── Notes (70%) ───────────────┐
│ > Board A            │ > Note 1 (text)                 │
│   Board B            │   Note 2 (voice)                │
│   Board C            │   Note 3 (image)                │
├──────────────────────┴─────────────────────────────────┤
│ [q]uit  [n]ew  [d]elete  [Tab] switch panel  [r]efresh │
└────────────────────────────────────────────────────────┘
```

En mode Input : popup centré "New note: ▌" remplace la status bar.

### 7.2 State machine (`app.rs`)

```rust
pub enum Focus { Boards, Notes }
pub enum Mode {
    Normal,
    Input(String),           // saisie nouvelle note
    Confirm(ConfirmAction),  // confirmation suppression
}
pub enum ConfirmAction { DeleteNote(Uuid) }

pub struct App {
    pub boards: Vec<BoardSummary>,
    pub selected_board: usize,
    pub notes: Vec<NoteSummary>,
    pub selected_note: usize,
    pub focus: Focus,
    pub mode: Mode,
    pub status: String,
    pub client: JotClient,
}
```

### 7.3 Raccourcis clavier

| Touche | Action |
|--------|--------|
| `j` / `↓` | Suivant dans panel focalisé |
| `k` / `↑` | Précédent |
| `Tab` | Changer focus Boards ↔ Notes |
| `Enter` | Sélectionner board (charge notes) |
| `n` | → Mode::Input("") |
| `d` | → Mode::Confirm(DeleteNote(id)) |
| `y` (en Confirm) | Confirme + API delete |
| `Esc` / `n` (en Confirm) | Annule |
| `r` | Rafraîchit boards + notes |
| `q` | Quitte |
| Tout char (en Input) | Append au buffer |
| `Enter` (en Input) | POST /notes → refresh |
| `Backspace` (en Input) | Supprime dernier char |
| `Esc` (en Input) | Annule |

### 7.4 Event loop (`tui/mod.rs`)

```rust
pub async fn run_tui(client: JotClient) -> Result<(), CliError> {
    // setup terminal (raw mode + alternate screen)
    // charger boards au démarrage
    // loop { tokio::select! { event = crossterm_stream => ..., _ = tick => draw } }
    // cleanup terminal
}
```

Tick interval : 250ms (pour les refresh de statut).

---

## 8. Gestion d'erreurs

```rust
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("not authenticated — run 'jot serve' first")]
    NotAuthenticated,
    #[error("server error: {0}")]
    Server(String),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("config error: {0}")]
    Config(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
```

Erreurs CLI : affichées sur stderr avec `eprintln!` + exit code 1.
Erreurs TUI : affichées dans `app.status` (sans quitter).

---

## 9. Tests

```rust
// config.rs
#[test] fn config_load_save_round_trip()   // tempdir, write + read
#[test] fn config_defaults_when_missing()  // absent → defaults

// tui/app.rs
#[test] fn navigation_j_k()               // selected_board incrémente/décrémente
#[test] fn mode_transition_normal_input()  // n → Input(""), Esc → Normal
#[test] fn mode_transition_input_confirm() // d → Confirm, y → Normal
```

Pas de tests réseau (reqwest vers serveur réel) — trop cher à orchestrer en CI.

---

## 10. Amendement API requis

`POST /notes` doit retourner `201 { "id": "<uuid>" }` au lieu de juste `201` (pour que le client puisse enchaîner `PUT /notes/{id}/blob`). Modification minimale dans `crates/api/src/routes/notes.rs` : changer le type de retour de `Result<StatusCode, ApiError>` en `Result<(StatusCode, Json<serde_json::Value>), ApiError>` et retourner `json!({ "id": note.id })`.

---

## 11. Hors périmètre

- Chiffrement des blobs côté client → SP6
- Upload binaires (images, audio) via CLI
- TLS / HTTPS
- `jot delete`, `jot search`, `jot rename` — post-v1
- Autocomplétion shell (clap_complete)
- Multi-serveur / federation
