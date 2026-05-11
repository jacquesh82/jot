# Block Structure (Outliner) — Design

**Status:** Approved — ready for implementation plan
**Date:** 2026-05-11
**Scope:** Feature 2 of the knowledge-graph roadmap. Foundational data-model change that subsequent specs (backlinks, journal, graph view) build on.

---

## 1. Goals

Transform `jot` notes from atomic encrypted blobs into **trees of first-class blocks**, while preserving the existing local-first, single-binary, E2E-encrypted deployment model.

The new model must enable:
- Per-line / per-paragraph identity (every block has a global UUID)
- Hierarchical structure with indent, fold, drag-and-drop
- Cross-note block references and embeds (`((id))`, `!((id))`)
- Future backlinks, graph view, and daily journal without further schema changes
- Functional parity across **API / CLI / SPA / TUI**

## 2. Non-goals (deferred to later specs)

- `[[Page]]` and `#tag` resolution UI (Feature 1 — backlinks)
- Interactive graph rendering (Feature 3)
- Auto-created daily pages (Feature 4)
- Real-time CRDT collaboration (table is prepared, integration deferred)
- Per-block permissions (inherit from note for MVP)
- Per-block comments
- Per-block version history

## 3. Stack decision

SQLite remains the right primary store for jot's deployment model (local-first, self-hosted, single-binary). Rationale and tradeoffs are recorded inline:

| Concern | Decision |
|---|---|
| Block volume (~7 M blocks / 20 M edges for a 10-year heavy user) | SQLite handles this with composite indexes |
| Graph traversal 1–2 hops | Recursive CTE, well under 50 ms at this scale |
| Realtime collaboration | Yjs/Automerge CRDT documents stored as BLOBs (integration deferred) |
| Semantic suggestions | Existing `ruvector.db` reused (Feature 1) |
| Deeper graph queries (>4 hops) | Out of scope for MVP; client-side traversal if ever needed |

Migration path if jot ever becomes multi-tenant SaaS: schema ports 1:1 to PostgreSQL.

## 4. Data model

### 4.1 New table — `blocks`

```sql
CREATE TABLE blocks (
    id              TEXT PRIMARY KEY,            -- UUID v4
    note_id         TEXT NOT NULL,
    parent_block_id TEXT,                        -- NULL = root of note
    position        REAL NOT NULL,               -- fractional indexing
    block_type      TEXT NOT NULL DEFAULT 'text',-- text|heading|todo|quote|code|embed|divider
    content         BLOB NOT NULL,               -- AES-256-GCM ciphertext of markdown text
    metadata        BLOB,                        -- AES-256-GCM ciphertext of JSON (todo_state, lang, embed_ref…)
    collapsed       INTEGER NOT NULL DEFAULT 0,  -- local UI preference, plaintext
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    FOREIGN KEY (note_id)         REFERENCES notes(id)  ON DELETE CASCADE,
    FOREIGN KEY (parent_block_id) REFERENCES blocks(id) ON DELETE CASCADE
);

CREATE INDEX idx_blocks_note          ON blocks(note_id);
CREATE INDEX idx_blocks_parent_pos    ON blocks(parent_block_id, position);
CREATE INDEX idx_blocks_note_parent   ON blocks(note_id, parent_block_id);
```

**Fractional indexing**: positions are real numbers. Inserting between 1.0 and 2.0 yields 1.5; drag-and-drop never rewrites siblings.

**Encryption**: a block's `content` and `metadata` are encrypted with the DEK of its parent note (the same DEK already used by `notes.content`). No per-block key management.

### 4.2 New table — `block_links` (graph edges)

```sql
CREATE TABLE block_links (
    id              TEXT PRIMARY KEY,
    source_block_id TEXT NOT NULL,           -- block containing the link
    target_kind     TEXT NOT NULL,           -- 'note' | 'block' | 'tag'
    target_id       TEXT NOT NULL,           -- note.id, block.id, or tag name
    link_kind       TEXT NOT NULL,           -- 'page_ref' | 'block_ref' | 'block_embed' | 'tag'
    created_at      TEXT NOT NULL,
    FOREIGN KEY (source_block_id) REFERENCES blocks(id) ON DELETE CASCADE
);

CREATE INDEX idx_block_links_target ON block_links(target_kind, target_id);
CREATE INDEX idx_block_links_source ON block_links(source_block_id);
```

`target_kind` / `target_id` are **plaintext** UUIDs (or tag names). They reveal the *shape* of the graph to the server but not the content. This is consistent with the existing share model which already exposes board/note IDs.

Edges are recomputed by the client on block save: parse markdown → diff against existing rows → upsert/delete edges in one transaction.

### 4.3 New table — `tags`

```sql
CREATE TABLE tags (
    name        TEXT NOT NULL,         -- plaintext, e.g. 'projet-x'
    identity_id TEXT NOT NULL,
    color       TEXT,
    created_at  TEXT NOT NULL,
    PRIMARY KEY (name, identity_id)
);
```

Tag names are stored in plaintext (decision: tags are short, useful for server-side fuzzy lookup, and the leakage is bounded).

### 4.4 Modifications to `notes`

```sql
ALTER TABLE notes ADD COLUMN title           BLOB;                       -- AES-256-GCM ciphertext, optional
ALTER TABLE notes ADD COLUMN is_journal      INTEGER NOT NULL DEFAULT 0;
ALTER TABLE notes ADD COLUMN journal_date    TEXT;                       -- 'YYYY-MM-DD' (plaintext, used by Feature 4)
ALTER TABLE notes ADD COLUMN schema_version  INTEGER NOT NULL DEFAULT 0; -- 0 = legacy markdown, 1 = blocks
```

`title` is **encrypted** (zero-knowledge — server cannot resolve `[[Page]]`). Clients maintain a local title-to-note-id index, populated lazily on first decrypt and kept in sync on WebSocket events.

## 5. Encryption summary

```
note.DEK = HKDF(identity.notes_key, salt = note.id)       (existing)
note.title      = AES-256-GCM(plaintext_title, note.DEK)
block.content   = AES-256-GCM(plaintext_markdown, note.DEK)
block.metadata  = AES-256-GCM(json_string, note.DEK)
```

Sharing a note shares its DEK → recipient can decrypt all blocks. A cross-note block embed where the embedded block belongs to an unshared note renders as `🔒 inaccessible block`.

## 6. Migration strategy

Server adds the new tables empty. **No server-side data migration is possible** (E2E). Migration happens client-side and lazily:

1. Server migration `0008_blocks.sql` creates tables and adds columns.
2. When a client opens a note with `schema_version = 0`:
   - Decrypt `notes.content` → markdown string
   - Split on blank-line / heading boundaries → list of blocks
   - POST `/api/notes/:id/blocks` for each block in order
   - PATCH the note to set `schema_version = 1`
   - Legacy `content` is retained until next major release (rollback safety), then dropped in a follow-up migration.
3. Voice and image notes are not affected — they are atomic and have no blocks.

A `--migrate-blocks` CLI command is provided to bulk-migrate all legacy notes for a given identity.

## 7. API

All routes require the existing auth/share-permission middleware. Block routes inherit the share permission of the containing note (read/write/delete).

```
GET    /api/notes/:note_id/blocks                 → full tree (children inlined)
POST   /api/notes/:note_id/blocks                 → create block
       body: { parent_id?, position?, block_type, content_blob, metadata_blob? }
GET    /api/blocks/:id                            → single block (for embed resolution)
PATCH  /api/blocks/:id                            → update content/type/metadata
DELETE /api/blocks/:id                            → cascade delete subtree
POST   /api/blocks/:id/move                       → { new_parent_id?, new_position }
POST   /api/blocks/:id/indent                     → become child of preceding sibling
POST   /api/blocks/:id/outdent                    → become sibling of parent

GET    /api/blocks/:id/backlinks                  → block_links rows where target = this block
GET    /api/notes/:id/backlinks                   → block_links rows where target = this note
GET    /api/tags                                  → list user's tags
GET    /api/tags/:name/blocks                     → blocks tagged with :name

PUT    /api/blocks/:id/links                      → replace edge set for a block (idempotent batch)
```

### 7.1 WebSocket events

Existing WS channel emits per-note: `block.created`, `block.updated`, `block.moved`, `block.deleted`, with the affected block payload. Used by SPA and TUI for live refresh of shared boards.

## 8. CLI surface

```
jot block add <note-id> [--parent <id>] [--position N] [--type text|heading|todo|quote|code|divider] --text "..."
jot block list <note-id> [--tree]
jot block show <id>
jot block edit <id>                       # opens $EDITOR on decrypted content
jot block move <id> --to <parent-id> [--position N]
jot block indent <id>
jot block outdent <id>
jot block delete <id>
jot block ref <id>                        # prints ((id)) to stdout
jot block backlinks <id>
jot block migrate [--all | --note <id>]   # legacy → blocks migration
```

## 9. SPA surface

New component `BlockEditor.tsx` replaces `NoteEditor.tsx` for text notes.

- Tree rendering with indent guides (Logseq-style).
- Per-block bullet handle on the left: drag, context menu (copy ref, copy embed, delete, change type).
- Slash menu on `/`: insert typed block (heading, todo, code, quote, divider, embed).
- Markdown inline formatting inside each block (reuses existing toolbar logic, scoped per block).
- Keyboard:
  - `Enter` — new block
  - `Tab` / `Shift+Tab` — indent / outdent
  - `Backspace` at empty start — outdent or merge with previous
  - `Cmd+Shift+↑ / ↓` — move block
  - `Cmd+.` — toggle collapse
  - `Cmd+B / I / K` — bold / italic / link (existing toolbar shortcuts)
- Voice and image notes keep the existing atomic `NoteEditor`.

## 10. TUI surface

Hierarchical view inside the note panel of `crates/cli/src/tui`.

- `j` / `k` — navigate blocks
- `o` / `O` — new block below / above
- `>` / `<` — indent / outdent
- `dd` — delete (cascade)
- `yy` — yank `((id))` to clipboard
- `za` — toggle collapse
- `Enter` — open `$EDITOR` on the current block's content

## 11. Functional parity matrix

| Capability | API | CLI | SPA | TUI |
|---|---|---|---|---|
| List blocks of a note | ✅ | ✅ | ✅ | ✅ |
| Create block | ✅ | ✅ | ✅ | ✅ |
| Edit content | ✅ | ✅ | ✅ | ✅ |
| Indent / outdent | ✅ | ✅ | ✅ | ✅ |
| Move (reparent + reorder) | ✅ | ✅ | ✅ (drag) | ✅ (keys) |
| Delete (cascade) | ✅ | ✅ | ✅ | ✅ |
| Toggle collapse | local | local | ✅ | ✅ |
| Insert block reference / embed | ✅ | ✅ | ✅ (slash menu) | ✅ (yank) |
| List backlinks | ✅ | ✅ | (Feature 1) | (Feature 1) |
| Lazy migration of legacy notes | n/a | `migrate` | auto-on-open | auto-on-open |

## 12. Testing strategy

- **Unit (storage crate):** insert/get/list/move/indent/outdent round-trips; cascade delete; fractional-position monotonicity under shuffle.
- **Unit (core crate):** markdown → block split (migration); edge extraction from block content; encryption round-trip.
- **Integration (API):** all routes happy-path + permission-denied + missing parent.
- **CLI:** snapshot tests on every subcommand against a fixture DB.
- **SPA:** Vitest on BlockEditor reducer (keyboard ops), Playwright on drag-drop and slash menu.
- **TUI:** existing ratatui harness, snapshot of tree rendering.

## 13. Open questions deferred (do not block this spec)

- Block-level CRDT merge strategy (only matters when realtime collab spec lands).
- Title plaintext index encoding: per-device or cross-device sync? (Local for MVP; cross-device sync is a separate concern.)
- Tag deletion cascade vs orphaning blocks: orphan for MVP, user can re-tag.

## 14. Migration & rollout

1. Land migration `0008_blocks.sql`.
2. Ship API + CLI in one release behind no feature flag — additive only.
3. SPA `BlockEditor` ships alongside, defaulting to block view for `schema_version = 1` notes and triggering migration on open.
4. After two minor releases with no rollback, drop the legacy `notes.content` column for text notes in migration `0009_drop_legacy_content.sql`.

## 15. Risk register

| Risk | Mitigation |
|---|---|
| Migration corrupts a user's notes | Keep legacy `content` for ≥2 releases; provide `jot block migrate --dry-run` |
| Edge table grows unbounded with bad links | Cascade-delete on source block; recompute edges atomically on save |
| Title plaintext index drift across devices | Rebuild on demand from server tree; never authoritative |
| Block-level routes bypass share permissions | All routes resolve `note_id` and reuse existing `share_permission` middleware |
| Fractional position underflow over thousands of inserts at same spot | Renormalize positions on read when min-gap < 1e-6 |
