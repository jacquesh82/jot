# jot

> Post-it numérique universel — chiffré, anonyme, disponible partout.

## What is jot?

jot is a universal encrypted note system — think digital post-its. Designed to
feel like a Unix tool: simple, composable, pipeable.

- No email, no password — identity is a local UUID with a friendly name
- One binary: CLI + API server + web SPA, statically linked
- Runs anywhere: Linux (musl), macOS, Windows
- Multi-user: open registration or invite-token gating
- Board and note sharing with per-identity access control

All note content is **end-to-end encrypted** — the server stores only ciphertext and
never has access to plaintext or key material.

## Quick start

```bash
# Start the server (SQLite + local blob storage, zero dependencies)
jot serve

# Open http://localhost:3000 in your browser — register a device via QR code
# or use the CLI directly after serve has created a token in ~/.config/jot/

# Add a note
echo "faire les courses" | jot add

# Add a note from your $EDITOR
jot add

# List notes (first board)
jot list

# List boards
jot list --boards

# List registered devices
jot list --devices

# Read a note
jot read <note-id>

# Launch the TUI
jot tui
```

## Authentication

jot has no email or password. Identity is generated automatically on first
`jot serve` — a UUID and a cryptographic key pair stored in `~/.config/jot/`.

**First device** — identity is created automatically when you start the server:
```bash
jot serve        # generates identity, registers this device, prints token
```

**Linking a new device** — use the web UI (Profile → "Link a new device") or:
```bash
# On the new device, open the URL shown in the browser
# The web UI displays a QR code and a jot link <token> command to run
jot link <token>
```

**Registration modes:**

```bash
# Open registration — anyone can create an account
jot serve --open-registration

# Invite-only (default) — generate a token for someone
jot invite --label "alice"
# → prints a one-time URL: http://localhost:3000/#/register?invite=<token>
```

## Connecting to a server

By default the client connects to `http://localhost:3000`. To point it at a
remote instance, edit `~/.config/jot/config.toml`:

```toml
server = "https://jot.example.com"
```

## Architecture

```mermaid
graph TB
    subgraph Clients
        CLI["🖥️ jot CLI\nClap · Ratatui · pipe"]
        WEB["🌐 Web Browser\nSPA embarquée"]
    end

    subgraph BIN["⚙️ binaire jot  —  statiquement lié"]
        direction TB
        AXUM["Axum\nAPI REST · WebSocket"]
        EMBED["rust-embed\nSPA assets compilés"]
        CORE["crate core\nRustCrypto · modèles · HKDF"]
        DETECT["crate detect\nmagic bytes auto-type"]
        STORAGE["crate storage\nsqlx · SQLite · blobs"]
    end

    subgraph LOCAL["💾 ~/.local/share/jot/"]
        DB[("jot.db\nSQLite")]
        BLOBS[("blobs/\nfichiers locaux")]
    end

    subgraph S3["☁️ S3-compatible  —  optionnel"]
        S3SVC[("AWS S3\nCloudflare R2\nMinIO · …")]
    end

    CLI -->|pipe / HTTP| AXUM
    WEB -->|HTTP · WS| AXUM

    AXUM --> CORE
    AXUM --> EMBED
    CORE --> DETECT
    CORE --> STORAGE
    STORAGE --> DB
    STORAGE -->|JOT_STORAGE=local| BLOBS
    STORAGE -->|JOT_STORAGE=s3| S3SVC
```

## End-to-end encryption

jot uses a **per-note Data Encryption Key (DEK)** model.  The private key never
leaves the device — the server only stores ciphertext and wrapped keys.

### Primitives

| Role | Algorithm | Size |
|---|---|---|
| Identity key pair | X25519 (ECDH) | 32 bytes |
| Key derivation | HKDF-SHA-256, info `"jot-share-v1"` | → 32-byte wrap key |
| Note encryption | AES-256-GCM, random 12-byte nonce | 256-bit key |
| Blob format | `[nonce 12 B]` \|\| `[ciphertext + GCM tag 16 B]` | — |

### Key lifecycle

The X25519 identity key pair is generated once on first `jot serve` and stored at
`~/.config/jot/identity.key` (chmod 600). The public key is registered in the
database. The SPA fetches the private key over the authenticated local API
(`GET /identity/me/privkey`) so both CLI and browser share the same pair.

### Write flow

```mermaid
flowchart LR
    subgraph CLIENT["Client  (CLI or SPA)"]
        direction TB
        PT["plaintext"] --> AES1
        DEK["DEK\n32 random bytes"] --> AES1["AES-256-GCM\nnonce ‖ ciphertext"]
        DEK --> AES2["AES-256-GCM\nnonce ‖ encrypted_DEK"]
        PRIV["identity.key\n(private)"] --> ECDH["X25519\nself-ECDH"]
        PUB["public key"] --> ECDH
        ECDH --> HKDF["HKDF-SHA-256\njot-share-v1"]
        HKDF --> WK["wrap_key"]
        WK --> AES2
    end
    AES1 -->|"PUT /notes/:id/blob"| BLOB[("blobs/\nciphertext")]
    AES2 -->|"PUT /notes/:id/dek"| DB[("jot.db\nencrypted_DEK")]
```

### Read flow

```mermaid
flowchart LR
    BLOB[("blobs/\nciphertext")] -->|"GET /notes/:id/blob"| AES1
    DB[("jot.db\nencrypted_DEK")] -->|"GET /notes/:id/dek"| AES2

    subgraph CLIENT["Client  (CLI or SPA)"]
        direction TB
        PRIV["identity.key\n(private)"] --> ECDH["X25519\nself-ECDH"]
        PUB["public key"] --> ECDH
        ECDH --> HKDF["HKDF-SHA-256\njot-share-v1"]
        HKDF --> WK["wrap_key"]
        WK --> AES2["AES-256-GCM\ndecrypt → DEK"]
        AES2 --> DEK["DEK"]
        DEK --> AES1["AES-256-GCM\ndecrypt → plaintext"]
        AES1 --> PT["plaintext"]
    end
```

> The server sees only `ciphertext` and `encrypted_DEK`.  It cannot recover the
> plaintext without the private key stored exclusively on the user's device.

## Web SPA

The SPA is served directly by `jot serve` on the same port as the API.
No separate web server needed.

**Features:**
- Board and note management (list view + card view) with block-tree editor
- Real-time updates via WebSocket
- Resizable note editor panel (persisted in localStorage)
- Board, note and per-block sharing with friendly-name resolution
- Recent contacts shown as quick-pick chips when sharing
- Dark / light theme toggle, language selector (en / fr / es / de)
- Journal page — notes grouped by day
- Todo page — all `todo` blocks across notes, with tag filter
- Knowledge graph (force-directed, links + backlinks)
- Intellisense in editor : `[[note]]`, `((block))`, `#tag` autocompletion
- Profile / devices pages : set/generate friendly name, link new devices, manage invites
- Data export to JSON (plain or AES-256-GCM encrypted with PBKDF2)

## TUI

`jot tui` opens a Ratatui interface with keyboard navigation:

- **Views** : `Tab` cycles MyBoards → SharedBoards → SharedNotes
- **Boards** : `n` new, `r` rename, `D` delete, `↑/↓` navigate
- **Notes** : `n` new note (input), `e` open `$EDITOR` on selected block, `d` delete
- **Blocks (v1 notes)** : `j/k` navigate, `e` edit content, `o/O` add below/above,
  `>` indent, `<` outdent, `dd` delete
- **Devices view** : `r` rename, `d` delete
- **Modes** : `Esc` cancels input/confirm

The TUI covers navigation and editing. Sharing, invites, tags, link-init and
aggregations (Journal / Todo / Graph) are CLI- or SPA-only.

## CLI commands

### Server & maintenance

| Command | Description |
|---|---|
| `jot serve [--port N] [--open-registration]` | Start the API server |
| `jot migrate` | Apply pending DB migrations without starting the server |
| `jot tui` | Launch the interactive TUI |
| `jot export [--out file]` | Export all your data as JSON |
| `jot stats` | Counts of boards / notes (per-board breakdown) |

### Notes & blocks

| Command | Description |
|---|---|
| `jot add [text…] [--board ID]` | Add a note (args, stdin pipe, or `$EDITOR`) |
| `jot list [--boards] [--devices]` | List notes, boards, or devices |
| `jot read <id>` | Read a note's content |
| `jot note title <id> <text>` | Set or clear a note's title |
| `jot block <add\|list\|show\|edit\|move\|indent\|outdent\|delete\|ref>` | Block-tree operations |
| `jot block backlinks <id>` | List backlinks pointing at a block |
| `jot block migrate [--all\|--note ID] [--dry-run]` | Migrate legacy notes to block form |
| `jot backlinks --note ID \| --block ID` | Show backlinks for a note or block |
| `jot journal [--date YYYY-MM-DD]` | Notes grouped by day |
| `jot todo [--tag T]` | All todo blocks across notes |

### Boards & devices

| Command | Description |
|---|---|
| `jot new board <name>` | Create a new board |
| `jot board rename <id> <name>` | Rename a board |
| `jot board delete <id>` | Delete a board |
| `jot board move <id> --position N` | Reposition a board in the sidebar |
| `jot board reorder-notes <board> <uuid:pos>...` | Reorder notes inside a board |
| `jot device rename <id> <name>` | Rename a device |
| `jot device delete <id>` | Revoke a device |

### Sharing

| Command | Description |
|---|---|
| `jot share <note> <with> [--permission read\|write\|delete]` | Share a note |
| `jot revoke <note> <identity>` | Revoke a note share |
| `jot shares <note>` | List who a note is shared with |
| `jot board-share <board> <with>` | Share a board (propagates note DEKs) |
| `jot board-revoke <board> <identity>` | Revoke a board share |
| `jot block share <id> <with> [--permission read\|write]` | Share a single block |
| `jot block unshare <id> <identity>` | Revoke a block share |
| `jot block shares <id>` / `jot block shared` | List block shares (out / in) |

### Identity, invites & linking

| Command | Description |
|---|---|
| `jot whoami [--set-name N] [--set-lang L]` | Show or update identity |
| `jot contacts` | List recent contacts |
| `jot link <token>` | Approve a pending device link (poll until done) |
| `jot link-init` | Start a new link session, print token/code/URL |
| `jot link-status <token>` | Print a link token's status |
| `jot invite [--label L]` | Create an invite token |
| `jot invites` | List all invites |
| `jot invite-revoke <token>` | Revoke an invite |

### Tags

| Command | Description |
|---|---|
| `jot tag list` | List all tags |
| `jot tag blocks <name>` | List block ids carrying a tag |
| `jot tag set <name> --json '{"color":"#abc"}'` | Create / update tag metadata |

### Account

| Command | Description |
|---|---|
| `jot delete-account` | Permanently delete this account and all data |

## Schema versioning

jot uses `sqlx` migrations. The current schema version is displayed at startup
and available at `GET /health`:

```
Database schema: up to date (v5)
# or, after an update:
Database schema migrated: v3 → v4
```

To pre-migrate before restarting a production server:
```bash
jot migrate
```

## Stack

| Component | Technology |
|---|---|
| Language | Rust (edition 2021) |
| HTTP framework | Axum 0.7 |
| Database | SQLite via `sqlx` 0.8 |
| Blob storage | Local filesystem (default) or S3-compatible |
| Cryptography | RustCrypto — X25519, AES-256-GCM, Ed25519, HKDF |
| CLI | Clap v4 + Ratatui TUI |
| Web frontend | Preact 10 + Vite 6 + `@preact/signals` |
| Web assets | `rust-embed` (SPA compiled into binary) |

## Data export

From the Profile page in the SPA, you can export all your boards and notes as JSON.
Optionally encrypt the export with a password (AES-256-GCM + PBKDF2, 200k rounds) —
the file is saved as `.jote`.

## Platforms

| Target | Platform | Linking |
|---|---|---|
| `x86_64-unknown-linux-musl` | Linux x86_64 | Static (musl) |
| `aarch64-unknown-linux-musl` | Linux ARM64 | Static (musl) |
| `x86_64-pc-windows-gnu` | Windows x86_64 | Static |
| `x86_64-apple-darwin` | macOS Intel | Dynamic |
| `aarch64-apple-darwin` | macOS Apple Silicon | Dynamic |

## Development

```bash
# Run the server in dev mode
cargo run -p cli -- serve --open-registration

# Run tests
cargo test --workspace

# Build the SPA (required before cargo build)
cd spa && npm run build

# Apply pending migrations without starting the server
cargo run -p cli -- migrate
```

## License

See [LICENSE](LICENSE).
