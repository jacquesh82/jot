# SP6 Implementation Plan — Web SPA + Release Pipeline

**Spec:** `docs/superpowers/specs/2026-05-10-sp6-web-release-design.md`  
**Date:** 2026-05-10

## Tasks

### Task 1 — Scaffold Preact SPA project

**Files to create:**
- `spa/package.json`
- `spa/vite.config.ts`
- `spa/tsconfig.json`
- `spa/index.html`
- `spa/src/main.tsx`
- `spa/.gitignore` (ignore `dist/` and `node_modules/`)

`package.json`:
```json
{
  "name": "jot-spa",
  "private": true,
  "scripts": {
    "build": "vite build",
    "dev": "vite"
  },
  "dependencies": {
    "preact": "^10.19.0",
    "@preact/signals": "^1.2.3"
  },
  "devDependencies": {
    "@preact/preset-vite": "^2.8.1",
    "typescript": "^5.3.0",
    "vite": "^5.1.0"
  }
}
```

`vite.config.ts`:
```ts
import { defineConfig } from "vite";
import preact from "@preact/preset-vite";

export default defineConfig({
  plugins: [preact()],
  build: { outDir: "dist", emptyOutDir: true },
});
```

`tsconfig.json`:
```json
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "jsx": "react-jsx",
    "jsxImportSource": "preact",
    "strict": true,
    "skipLibCheck": true
  },
  "include": ["src"]
}
```

`index.html` — minimal shell that mounts `#app`.

`src/main.tsx` — renders `<App />` into `#app`.

---

### Task 2 — Write SPA api.ts

**File:** `spa/src/api.ts`

Typed wrappers around the existing REST API + WebSocket:

```ts
const BASE = "/api";

function token(): string {
  return localStorage.getItem("token") ?? "";
}

function headers(): HeadersInit {
  return { Authorization: `Bearer ${token()}`, "Content-Type": "application/json" };
}

export interface Board { id: string; name: string; position: number }
export interface Note  { id: string; note_type: string; position: number }
export type WsEvent = { event: string; [key: string]: unknown }

export async function fetchBoards(): Promise<Board[]> {
  const r = await fetch(`${BASE}/boards`, { headers: headers() });
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function fetchNotes(boardId: string): Promise<Note[]> {
  const r = await fetch(`${BASE}/notes?board_id=${boardId}`, { headers: headers() });
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function createNote(boardId: string, text: string): Promise<{ id: string }> {
  const r = await fetch(`${BASE}/notes`, {
    method: "POST",
    headers: headers(),
    body: JSON.stringify({ board_id: boardId, note_type: "text", color: null, position: 0 }),
  });
  if (!r.ok) throw new Error(await r.text());
  const { id } = await r.json();
  await fetch(`${BASE}/notes/${id}/blob`, {
    method: "PUT",
    headers: { Authorization: `Bearer ${token()}`, "Content-Type": "text/plain" },
    body: text,
  });
  return { id };
}

export async function deleteNote(id: string): Promise<void> {
  await fetch(`${BASE}/notes/${id}`, { method: "DELETE", headers: headers() });
}

export function connectWs(onEvent: (e: WsEvent) => void): () => void {
  const proto = location.protocol === "https:" ? "wss" : "ws";
  let ws: WebSocket;
  let delay = 1000;
  let stopped = false;

  function connect() {
    ws = new WebSocket(`${proto}://${location.host}/ws?token=${token()}`);
    ws.onmessage = (e) => { try { onEvent(JSON.parse(e.data)); } catch {} };
    ws.onclose = () => { if (!stopped) setTimeout(connect, Math.min(delay *= 2, 30000)); };
    ws.onerror = () => ws.close();
  }
  connect();
  return () => { stopped = true; ws.close(); };
}
```

---

### Task 3 — Write SPA components

**Files:**
- `spa/src/components/DeviceRegister.tsx`
- `spa/src/components/BoardList.tsx`
- `spa/src/components/NoteList.tsx`
- `spa/src/App.tsx`

**DeviceRegister.tsx** — on mount calls `POST /api/auth/device` if no token in localStorage, displays the returned token for manual entry on the CLI, polls `GET /api/link/status/:token` every 2s until confirmed, then saves JWT to localStorage and redirects to `#/`.

**BoardList.tsx** — calls `fetchBoards()` on mount, renders list, click navigates to `#/board/:id`.

**NoteList.tsx** — reads `boardId` from hash, calls `fetchNotes()`, renders list with delete button per note, has a text input + submit to `createNote()`. Calls `connectWs()` and refreshes notes list on `note_created` / `note_deleted` events.

**App.tsx** — reads `location.hash`, renders the right component based on hash prefix. No deps beyond Preact signals.

---

### Task 4 — Add build script

**File:** `scripts/build-spa.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../spa"
bun install --frozen-lockfile
bun run build
```

`chmod +x scripts/build-spa.sh`.

---

### Task 5 — Add rust-embed to crates/api

**Edit `crates/api/Cargo.toml`** — add:
```toml
rust-embed = { version = "8", features = ["mime-guess"] }
mime_guess = "2"
```

**Create `crates/api/src/routes/spa.rs`:**

```rust
use axum::{http::{header::CONTENT_TYPE, Uri}, response::IntoResponse};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/../../spa/dist"]
struct Assets;

pub async fn spa_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            ([(CONTENT_TYPE, mime.as_ref().to_owned())], content.data.into_owned()).into_response()
        }
        None => {
            let index = Assets::get("index.html")
                .expect("spa/dist/index.html not found — run scripts/build-spa.sh");
            ([(CONTENT_TYPE, "text/html".to_owned())], index.data.into_owned()).into_response()
        }
    }
}
```

**Edit `crates/api/src/routes/mod.rs`** — add `pub mod spa;` and wire fallback:

```rust
pub mod spa;
// in build():
.fallback(spa::spa_handler)
// remove the explicit .with_state(state) last — add it after fallback
```

The `.with_state(state)` must remain the last call.

---

### Task 6 — Generate sqlx offline cache

Run locally (with the server not running):
```bash
cargo sqlx prepare --workspace
git add .sqlx
```

This generates `.sqlx/*.json` query descriptors needed for `SQLX_OFFLINE=true` cross builds.

---

### Task 7 — Write release workflow

**File:** `.github/workflows/release.yml`

```yaml
name: Release

on:
  push:
    tags: ["v*"]

jobs:
  build:
    name: Build ${{ matrix.bin }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - target: x86_64-unknown-linux-musl
            os: ubuntu-latest
            bin: jot-linux-x86_64
          - target: aarch64-unknown-linux-musl
            os: ubuntu-latest
            bin: jot-linux-aarch64
          - target: x86_64-pc-windows-gnu
            os: ubuntu-latest
            bin: jot-windows-x86_64.exe
          - target: x86_64-apple-darwin
            os: macos-latest
            bin: jot-macos-x86_64
          - target: aarch64-apple-darwin
            os: macos-latest
            bin: jot-macos-aarch64

    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - uses: Swatinem/rust-cache@v2
        with:
          key: ${{ matrix.target }}

      - uses: oven-sh/setup-bun@v2

      - name: Build SPA
        run: bash scripts/build-spa.sh

      - name: Install cargo-zigbuild
        if: runner.os == 'Linux'
        run: pip install ziglang && cargo install cargo-zigbuild

      - name: Build (Linux)
        if: runner.os == 'Linux'
        env:
          SQLX_OFFLINE: "true"
        run: cargo zigbuild --release -p cli --target ${{ matrix.target }}

      - name: Build (macOS)
        if: runner.os == 'macOS'
        env:
          SQLX_OFFLINE: "true"
        run: cargo build --release -p cli --target ${{ matrix.target }}

      - name: Rename binary
        shell: bash
        run: |
          SRC="target/${{ matrix.target }}/release/jot"
          [ -f "${SRC}.exe" ] && SRC="${SRC}.exe"
          mv "$SRC" "${{ matrix.bin }}"

      - uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.bin }}
          path: ${{ matrix.bin }}

  release:
    name: Create GitHub Release
    needs: build
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/download-artifact@v4
        with:
          merge-multiple: true

      - uses: softprops/action-gh-release@v2
        with:
          files: "jot-*"
          generate_release_notes: true
```

---

### Task 8 — Update CI workflow

**Edit `.github/workflows/ci.yml`** — add `SQLX_OFFLINE: "true"` env to the test step so CI doesn't need a running database:

```yaml
      - name: test
        env:
          SQLX_OFFLINE: "true"
        run: cargo test --workspace
```

Also add a check that the SPA builds cleanly (optional but useful):
```yaml
      - uses: oven-sh/setup-bun@v2
      - name: Build SPA
        run: bash scripts/build-spa.sh
```
Place the bun + SPA build steps before the cargo steps.

---

### Task 9 — Commit spec, plan, and all new files

```bash
git add docs/superpowers/ spa/ scripts/ crates/api/ .github/workflows/release.yml .sqlx/
git commit -m "SP6: Preact SPA embedded via rust-embed + 5-target release pipeline"
```

---

## Order of execution

1. Task 1 — scaffold `spa/`
2. Task 2 — `api.ts`
3. Task 3 — components + App
4. Task 4 — `build-spa.sh` + run it locally to verify `spa/dist/` is produced
5. Task 5 — rust-embed in `crates/api`, `spa.rs`, wire router
6. Task 6 — `cargo sqlx prepare`
7. Task 7 — `release.yml`
8. Task 8 — update `ci.yml`
9. Task 9 — commit everything

## Verification

- `bash scripts/build-spa.sh` produces `spa/dist/index.html`
- `cargo build -p cli` compiles without errors (rust-embed finds the dist)
- `jot serve` — open `http://localhost:3000` — SPA loads in browser
- `cargo test --workspace` — all tests still pass with `SQLX_OFFLINE=true`
- Push a `v0.1.0` tag → GitHub Actions → 5 release assets appear on the Release page
