# SP6 — Web SPA + Release Pipeline

**Date:** 2026-05-10  
**Status:** Approved

## Scope

Add a Preact SPA served by the existing Axum binary (`jot serve`), and a GitHub Actions release pipeline that produces 5 static binaries on every `v*` tag.

## Architecture

```
jot/
├── spa/                        ← Preact source
│   ├── src/
│   │   ├── main.tsx
│   │   ├── components/
│   │   │   ├── BoardList.tsx
│   │   │   ├── NoteList.tsx
│   │   │   └── DeviceRegister.tsx
│   │   └── api.ts              ← typed fetch + WebSocket client
│   ├── package.json            ← bun + vite
│   └── dist/                   ← gitignored, built by scripts/build-spa.sh
├── scripts/
│   └── build-spa.sh
├── crates/api/
│   └── src/
│       ├── routes/spa.rs       ← new: serve embedded assets
│       └── (rust-embed reads spa/dist at compile time)
└── .github/workflows/
    ├── ci.yml                  ← existing, unchanged
    └── release.yml             ← new: tag → 5 binaries → GH Release
```

The SPA is embedded at compile time via `rust-embed`. The Axum router registers API routes first, then `/ws`, then a `fallback` to the SPA handler. The SPA communicates with the existing `/api/...` endpoints on the same origin (no CORS required in production).

## SPA

**Views (hash-based routing, no router library):**

| Hash | Component | Purpose |
|------|-----------|---------|
| `#/register` | `DeviceRegister` | Show QR / token, poll until device linked |
| `#/` | `BoardList` | List boards, click to select |
| `#/board/:id` | `NoteList` | List + create + delete notes, live WS updates |

**State:** `@preact/signals` — no Redux/Zustand. Auth token in `localStorage`, sent as `Authorization: Bearer` on every fetch.

**`api.ts` exports:**
- `fetchBoards()`, `fetchNotes(boardId)`, `createNote(boardId, text)`, `deleteNote(id)`
- `connectWs(token, onEvent)` — opens `ws://host/ws?token=...`, calls `onEvent` on each JSON frame

**Build:** Vite + bun. `bun run build` outputs `spa/dist/index.html` + hashed `assets/main-[hash].js`.

```bash
# scripts/build-spa.sh
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../spa"
bun install --frozen-lockfile
bun run build
```

## rust-embed Integration

Dependencies added to `crates/api/Cargo.toml`:
```toml
rust-embed = { version = "8", features = ["mime-guess"] }
mime_guess = "2"
```

`crates/api/src/routes/spa.rs` — serves exact asset matches by MIME type; all other paths fall back to `index.html`:

```rust
pub async fn spa_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            ([(CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => {
            let index = Assets::get("index.html").unwrap();
            ([(CONTENT_TYPE, "text/html")], index.data).into_response()
        }
    }
}
```

Router registration (lowest priority):
```rust
Router::new()
    .nest("/api", api_routes())
    .route("/ws", get(ws::handler))
    .fallback(spa::spa_handler)
    .with_state(state)
```

## Cross-Compilation

| Target | Runner | Tool |
|--------|--------|------|
| `x86_64-unknown-linux-musl` | `ubuntu-latest` | `cargo-zigbuild` |
| `aarch64-unknown-linux-musl` | `ubuntu-latest` | `cargo-zigbuild` |
| `x86_64-pc-windows-gnu` | `ubuntu-latest` | `cargo-zigbuild` |
| `x86_64-apple-darwin` | `macos-latest` | `cargo build` (native) |
| `aarch64-apple-darwin` | `macos-latest` | `cargo build` (native) |

`cargo-zigbuild` is installed via `pip install ziglang && cargo install cargo-zigbuild`. No Docker required. macOS jobs use native runners for correct Apple SDK support.

`SQLX_OFFLINE=true` + committed `.sqlx/` query cache required for all cross-compilation targets (no SQLite available during cross builds).

## Release Workflow (`.github/workflows/release.yml`)

Triggers on `v*` tag push. Two jobs:

1. **`build` (matrix of 5):** checkout → rust toolchain → bun → build SPA → install cargo-zigbuild (Linux only) → cargo build → rename binary → upload artifact
2. **`release` (needs: build):** download all artifacts → `softprops/action-gh-release` uploads all `jot-*` files

Binary naming convention:
- `jot-linux-x86_64`
- `jot-linux-aarch64`
- `jot-windows-x86_64.exe`
- `jot-macos-x86_64`
- `jot-macos-aarch64`

## sqlx Offline Cache

Before cross-compilation works, the `.sqlx/` directory must be generated locally:
```bash
cargo sqlx prepare --workspace
git add .sqlx
git commit -m "add sqlx offline query cache"
```

This is a one-time step done before the first release tag.

## Error Handling

- Missing `spa/dist/` at compile time: `rust-embed` will panic with a clear path error — developer must run `build-spa.sh` first.
- WS disconnect in SPA: `connectWs` reconnects with exponential backoff (max 30s).
- API errors: SPA shows inline error messages, no global error boundary needed for this scope.

## Out of Scope

- Code signing / notarization for macOS binaries
- Windows MSI installer
- CDN / asset caching headers
- Dark mode / theming
