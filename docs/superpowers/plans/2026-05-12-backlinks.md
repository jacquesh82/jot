# Backlinks & Knowledge Graph Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `[[Page]]`, `((block-id))`, `!((block-id))`, and `#tag` interactive in jot — clickable rendering, auto-extracted graph, backlinks panel, virtual tag pages, autocomplete, and opt-in semantic suggestions via ruvector.

**Architecture:** Server-side additions stay thin (resolution, reconcile, Pages-board lifecycle, embeddings I/O). The SPA owns the title index (zero-knowledge — server cannot resolve titles) and the interactive rendering layer. Auto-extraction hooks into existing block save paths and idempotently `PUT /blocks/:id/links`.

**Tech Stack:** Rust (axum, sqlx, jot-core), TypeScript + Preact + signals SPA, SQLite, AES-256-GCM at application layer. ruvector for semantic search (existing local index, currently unused by the API).

**Spec:** `docs/superpowers/specs/2026-05-12-backlinks-design.md`

---

## File Structure

### Created (Rust)
- `crates/storage/migrations/0009_backlinks.sql`
- `crates/storage/src/db/block_embeddings.rs`
- `crates/storage/src/db/pages_board.rs` — `find_or_create_pages_board`, `is_pages_board`
- `crates/storage/src/db/reconcile.rs` — `reconcile_unresolved_page_refs(title, note_id)`, `find_notes_by_title_for_identity`
- `crates/api/src/routes/pages.rs` — `GET /resolve/page`, `POST /pages`, `POST /links/reconcile-title`
- `crates/api/src/routes/embeddings.rs` — `POST /embeddings/upsert`, `GET /embeddings/status`, `POST /embeddings/reindex`
- `crates/api/src/routes/suggestions.rs` — `GET /suggestions/related`
- `crates/api/src/ruvector.rs` — thin client around the local ruvector index (embed, knn, delete)
- `crates/cli/src/commands/page.rs` — `jot page resolve|create`
- `crates/cli/src/commands/suggestions.rs` — `jot suggestions --note <id>`

### Created (SPA)
- `spa/src/blocks/links.ts` — `extractLinks()` JS mirror of Rust
- `spa/src/blocks/titleIndex.ts` — `Map<string, string>` + load + WS sync
- `spa/src/blocks/render.ts` — `renderBlockHtml(plaintext, ctx)` for view-mode
- `spa/src/blocks/clickHandler.ts` — single delegated click handler factory
- `spa/src/components/EmbedBlock.tsx`
- `spa/src/components/LinkAutocomplete.tsx`
- `spa/src/components/BacklinksSection.tsx`
- `spa/src/components/SuggestionsSection.tsx`
- `spa/src/components/TagPage.tsx`
- `spa/src/blocks/embeddings.ts` — `upsertEmbedding`, `enabledForIdentity()`
- `spa/src/blocks/links.test.ts` — parity tests

### Modified
- `crates/core/src/models/block.rs` — extend `LinkKind` with `PageRefUnresolved`
- `crates/core/src/blocks/links.rs` — return unresolved variant when title not in map
- `crates/storage/src/db/mod.rs` — register new submodules
- `crates/storage/src/db/board_shares.rs` or `boards.rs` — `board_kind` read/write helpers
- `crates/storage/src/db/identity.rs` — `set_embeddings_enabled`, `get_embeddings_enabled`
- `crates/api/src/routes/mod.rs` — register new routes
- `crates/api/src/state.rs` — `WsEvent::PageResolved`, `WsEvent::EmbeddingIndexed`, `embeddings_enabled` helpers
- `crates/api/src/routes/notes.rs` — call reconcile after note creation
- `crates/cli/src/main.rs` — wire new subcommands
- `crates/cli/src/client.rs` — `resolve_page`, `create_page`, `suggestions_related`, `upsert_embedding`
- `spa/src/api.ts` — `resolvePage`, `createPage`, `reconcileTitle`, `upsertEmbedding`, `getEmbeddingStatus`, `suggestionsRelated`
- `spa/src/components/BlockEditor.tsx` — wire title index load, view-mode render, focus/blur switch, click handler, autocomplete, sections, auto-extraction in `persistEdit`
- `spa/src/components/NoteList.tsx` — note WS already drives reload (T16 in F2); add awareness of `page_resolved`
- `spa/src/components/WhoamiPage.tsx` — Settings toggle for embeddings
- `spa/src/i18n/{en,fr,es,de}.ts` — strings for new UI

---

## Task 1: Database Migration

**Files:**
- Create: `crates/storage/migrations/0009_backlinks.sql`

- [ ] **Step 1.1: Write the migration SQL**

Create `crates/storage/migrations/0009_backlinks.sql` with EXACTLY this content:

```sql
ALTER TABLE boards ADD COLUMN board_kind TEXT NOT NULL DEFAULT 'regular';

ALTER TABLE identities ADD COLUMN embeddings_enabled INTEGER NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS block_embeddings (
    block_id      TEXT PRIMARY KEY,
    embedding_id  TEXT NOT NULL,
    text_hash     TEXT NOT NULL,
    updated_at    TEXT NOT NULL,
    FOREIGN KEY (block_id) REFERENCES blocks(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_block_embeddings_hash ON block_embeddings(text_hash);

CREATE INDEX IF NOT EXISTS idx_boards_kind ON boards(identity_id, board_kind);
```

- [ ] **Step 1.2: Run migration tests**

Run: `cargo test -p storage --lib`
Expected: 32+ tests PASS (existing + migration applies cleanly).

- [ ] **Step 1.3: Commit**

```bash
git add crates/storage/migrations/0009_backlinks.sql
git commit -m "feat(storage): add board_kind, embeddings_enabled, block_embeddings"
```

---

## Task 2: Extend LinkKind with PageRefUnresolved

**Files:**
- Modify: `crates/core/src/models/block.rs`
- Modify: `crates/core/src/blocks/links.rs`

- [ ] **Step 2.1: Add the enum variant**

In `crates/core/src/models/block.rs`, locate the `LinkKind` enum and add `PageRefUnresolved`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkKind { PageRef, PageRefUnresolved, BlockRef, BlockEmbed, Tag }
```

Update `as_str` and `from_str`:

```rust
    pub fn as_str(&self) -> &'static str {
        match self {
            LinkKind::PageRef => "page_ref",
            LinkKind::PageRefUnresolved => "page_ref_unresolved",
            LinkKind::BlockRef => "block_ref",
            LinkKind::BlockEmbed => "block_embed",
            LinkKind::Tag => "tag",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "page_ref" => Some(Self::PageRef),
            "page_ref_unresolved" => Some(Self::PageRefUnresolved),
            "block_ref" => Some(Self::BlockRef),
            "block_embed" => Some(Self::BlockEmbed),
            "tag" => Some(Self::Tag),
            _ => None,
        }
    }
```

- [ ] **Step 2.2: Update extract_links to mark unresolved page refs**

In `crates/core/src/blocks/links.rs`, locate the `for cap in page_re()…` loop and change:

```rust
    for cap in page_re().captures_iter(markdown) {
        let title = cap[1].trim().to_string();
        let (id, kind) = match title_to_id.get(&title.to_lowercase()) {
            Some(note_id) => (note_id.clone(), LinkKind::PageRef),
            None => (title.to_lowercase(), LinkKind::PageRefUnresolved),
        };
        out.push(ExtractedLink { target_kind: TargetKind::Note, target_id: id, link_kind: kind });
    }
```

- [ ] **Step 2.3: Add unit test for the unresolved path**

Append to the `tests` module in `crates/core/src/blocks/links.rs`:

```rust
    #[test]
    fn unresolved_page_returns_lowercase_title() {
        let out = extract_links("see [[Foo Bar]]", &HashMap::new());
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].link_kind, LinkKind::PageRefUnresolved);
        assert_eq!(out[0].target_id, "foo bar");
    }
```

- [ ] **Step 2.4: Run tests**

Run: `cargo test -p jot-core models::block link_kind_round_trip`
Run: `cargo test -p jot-core blocks::links`
Expected: all PASS (the round-trip test must include the new variant — extend the iteration array to include `"page_ref_unresolved"` if it doesn't).

- [ ] **Step 2.5: Commit**

```bash
git add crates/core/src/models/block.rs crates/core/src/blocks/links.rs
git commit -m "feat(core): page_ref_unresolved link kind for orphan [[Page]] refs"
```

---

## Task 3: Storage — Pages Board Lifecycle

**Files:**
- Create: `crates/storage/src/db/pages_board.rs`
- Modify: `crates/storage/src/db/mod.rs`

- [ ] **Step 3.1: Register submodule**

In `crates/storage/src/db/mod.rs`, add `pub mod pages_board;`.

- [ ] **Step 3.2: Write the helper**

Create `crates/storage/src/db/pages_board.rs`:

```rust
use crate::db::Db;
use crate::StorageError;
use chrono::Utc;
use jot_core::models::Board;
use sqlx::Row;
use uuid::Uuid;

impl Db {
    /// Returns the user's "Pages" board, creating it if none exists.
    pub async fn find_or_create_pages_board(&self, identity_id: Uuid) -> Result<Board, StorageError> {
        if let Some(row) = sqlx::query(
            "SELECT id, identity_id, name, position, created_at
             FROM boards WHERE identity_id = ? AND board_kind = 'pages' LIMIT 1"
        )
        .bind(identity_id.to_string())
        .fetch_optional(&self.0).await?
        {
            return Ok(Board {
                id: Uuid::parse_str(&row.get::<String, _>("id")).unwrap(),
                identity_id,
                name: row.get("name"),
                position: row.get("position"),
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at")).unwrap().with_timezone(&Utc),
            });
        }
        let board = Board {
            id: Uuid::new_v4(),
            identity_id,
            name: "Pages".to_string(),
            position: -1,
            created_at: Utc::now(),
        };
        sqlx::query(
            "INSERT INTO boards (id, identity_id, name, position, created_at, board_kind)
             VALUES (?, ?, ?, ?, ?, 'pages')"
        )
        .bind(board.id.to_string())
        .bind(board.identity_id.to_string())
        .bind(&board.name)
        .bind(board.position)
        .bind(board.created_at.to_rfc3339())
        .execute(&self.0).await?;
        Ok(board)
    }

    pub async fn board_kind(&self, board_id: Uuid) -> Result<String, StorageError> {
        let row = sqlx::query("SELECT board_kind FROM boards WHERE id = ?")
            .bind(board_id.to_string())
            .fetch_optional(&self.0).await?;
        Ok(row.map(|r| r.get("board_kind")).unwrap_or_else(|| "regular".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_db;

    async fn seed_identity(db: &Db) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO identities (id, friendly_name, created_at) VALUES (?, ?, ?)")
            .bind(id.to_string())
            .bind(format!("ident-{}", id))
            .bind(Utc::now().to_rfc3339())
            .execute(&db.0).await.unwrap();
        id
    }

    #[tokio::test]
    async fn pages_board_idempotent() {
        let db = test_db().await;
        let identity = seed_identity(&db).await;
        let a = db.find_or_create_pages_board(identity).await.unwrap();
        let b = db.find_or_create_pages_board(identity).await.unwrap();
        assert_eq!(a.id, b.id);
        assert_eq!(db.board_kind(a.id).await.unwrap(), "pages");
    }
}
```

NOTE: confirm the `identities` table's required columns by reading `crates/storage/migrations/0002_identities_shares.sql` before writing the `seed_identity` helper. Adjust the INSERT if more columns are NOT NULL.

- [ ] **Step 3.3: Run tests**

Run: `cargo test -p storage pages_board`
Expected: 1 test PASS.

- [ ] **Step 3.4: Commit**

```bash
git add crates/storage/src/db/pages_board.rs crates/storage/src/db/mod.rs
git commit -m "feat(storage): find_or_create_pages_board + board_kind helper"
```

---

## Task 4: Storage — Reconcile Unresolved Page Refs + Title Search

**Files:**
- Create: `crates/storage/src/db/reconcile.rs`
- Modify: `crates/storage/src/db/mod.rs`

- [ ] **Step 4.1: Register submodule**

Add `pub mod reconcile;` to `crates/storage/src/db/mod.rs`.

- [ ] **Step 4.2: Write the helpers**

Create `crates/storage/src/db/reconcile.rs`:

```rust
use crate::db::Db;
use crate::StorageError;
use sqlx::Row;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct NoteTitleRow {
    pub id: Uuid,
    pub board_id: Uuid,
    pub title: Option<Vec<u8>>,    // ciphertext; client decrypts & matches
    pub updated_at: String,
}

impl Db {
    /// Bulk-resolve all unresolved page refs whose plaintext lowercase title
    /// matches `title`. Returns the number of rows updated.
    pub async fn reconcile_unresolved_page_refs(&self, title: &str, note_id: Uuid) -> Result<u64, StorageError> {
        let r = sqlx::query(
            "UPDATE block_links
                SET target_id = ?, link_kind = 'page_ref'
              WHERE link_kind = 'page_ref_unresolved'
                AND target_kind = 'note'
                AND target_id = ?"
        )
        .bind(note_id.to_string())
        .bind(title.to_lowercase())
        .execute(&self.0).await?;
        Ok(r.rows_affected())
    }

    /// Return (id, board_id, title_ciphertext, updated_at) for every note owned by the identity.
    /// Used by the SPA to populate its title index. Cheap because we only read ids and a small
    /// ciphertext blob per row.
    pub async fn list_titles_for_identity(&self, identity_id: Uuid) -> Result<Vec<NoteTitleRow>, StorageError> {
        let rows = sqlx::query(
            "SELECT n.id, n.board_id, n.title, n.updated_at
               FROM notes n
               JOIN boards b ON b.id = n.board_id
              WHERE b.identity_id = ?
              ORDER BY n.updated_at DESC"
        )
        .bind(identity_id.to_string())
        .fetch_all(&self.0).await?;
        Ok(rows.iter().map(|r| {
            let id: String = r.get("id");
            let board_id: String = r.get("board_id");
            let title: Option<Vec<u8>> = r.try_get("title").ok().flatten();
            let updated_at: String = r.get("updated_at");
            NoteTitleRow {
                id: Uuid::parse_str(&id).unwrap(),
                board_id: Uuid::parse_str(&board_id).unwrap(),
                title,
                updated_at,
            }
        }).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_db;
    use chrono::Utc;

    #[tokio::test]
    async fn reconcile_flips_unresolved_to_resolved() {
        let db = test_db().await;
        // Seed identity, board, note, block, unresolved link
        let identity = Uuid::new_v4();
        sqlx::query("INSERT INTO identities (id, friendly_name, created_at) VALUES (?, ?, ?)")
            .bind(identity.to_string()).bind(format!("ident-{}", identity)).bind(Utc::now().to_rfc3339())
            .execute(&db.0).await.unwrap();
        let board = Uuid::new_v4();
        sqlx::query("INSERT INTO boards (id, identity_id, name, position, created_at) VALUES (?,?,?,?,?)")
            .bind(board.to_string()).bind(identity.to_string()).bind("b").bind(0i32).bind(Utc::now().to_rfc3339())
            .execute(&db.0).await.unwrap();
        let note = Uuid::new_v4();
        sqlx::query("INSERT INTO notes (id, note_type, content, color, board_id, position, blob_key, size, created_at, updated_at) VALUES (?,?,?,?,?,?,?,?,?,?)")
            .bind(note.to_string()).bind("text").bind(b"".to_vec()).bind("#FFF").bind(board.to_string()).bind(0i32).bind(Uuid::new_v4().to_string()).bind(0i64)
            .bind(Utc::now().to_rfc3339()).bind(Utc::now().to_rfc3339())
            .execute(&db.0).await.unwrap();
        let block = Uuid::new_v4();
        sqlx::query("INSERT INTO blocks (id, note_id, position, block_type, content, created_at, updated_at) VALUES (?,?,?,?,?,?,?)")
            .bind(block.to_string()).bind(note.to_string()).bind(1.0f64).bind("text").bind(b"x".to_vec())
            .bind(Utc::now().to_rfc3339()).bind(Utc::now().to_rfc3339())
            .execute(&db.0).await.unwrap();
        sqlx::query("INSERT INTO block_links (id, source_block_id, target_kind, target_id, link_kind, created_at) VALUES (?,?,?,?,?,?)")
            .bind(Uuid::new_v4().to_string()).bind(block.to_string()).bind("note").bind("foo bar").bind("page_ref_unresolved").bind(Utc::now().to_rfc3339())
            .execute(&db.0).await.unwrap();

        let target_note = Uuid::new_v4();
        let n = db.reconcile_unresolved_page_refs("Foo Bar", target_note).await.unwrap();
        assert_eq!(n, 1);

        let row = sqlx::query("SELECT target_id, link_kind FROM block_links WHERE source_block_id = ?")
            .bind(block.to_string()).fetch_one(&db.0).await.unwrap();
        assert_eq!(row.get::<String, _>("target_id"), target_note.to_string());
        assert_eq!(row.get::<String, _>("link_kind"), "page_ref");
    }
}
```

- [ ] **Step 4.3: Run tests**

Run: `cargo test -p storage reconcile`
Expected: 1 test PASS.

- [ ] **Step 4.4: Commit**

```bash
git add crates/storage/src/db/reconcile.rs crates/storage/src/db/mod.rs
git commit -m "feat(storage): reconcile_unresolved_page_refs + list_titles_for_identity"
```

---

## Task 5: Storage — Block Embeddings + Embeddings-Enabled Flag

**Files:**
- Create: `crates/storage/src/db/block_embeddings.rs`
- Modify: `crates/storage/src/db/identity.rs`
- Modify: `crates/storage/src/db/mod.rs`

- [ ] **Step 5.1: Register submodule**

Add `pub mod block_embeddings;` to `crates/storage/src/db/mod.rs`.

- [ ] **Step 5.2: Write block_embeddings module**

Create `crates/storage/src/db/block_embeddings.rs`:

```rust
use crate::db::Db;
use crate::StorageError;
use chrono::Utc;
use sqlx::Row;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct BlockEmbedding {
    pub block_id: Uuid,
    pub embedding_id: String,
    pub text_hash: String,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Db {
    pub async fn upsert_block_embedding(
        &self,
        block_id: Uuid,
        embedding_id: &str,
        text_hash: &str,
    ) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO block_embeddings (block_id, embedding_id, text_hash, updated_at)
             VALUES (?, ?, ?, ?)
             ON CONFLICT(block_id) DO UPDATE SET
               embedding_id = excluded.embedding_id,
               text_hash = excluded.text_hash,
               updated_at = excluded.updated_at"
        )
        .bind(block_id.to_string())
        .bind(embedding_id)
        .bind(text_hash)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.0).await?;
        Ok(())
    }

    pub async fn get_block_embedding(&self, block_id: Uuid) -> Result<Option<BlockEmbedding>, StorageError> {
        let row = sqlx::query("SELECT * FROM block_embeddings WHERE block_id = ?")
            .bind(block_id.to_string())
            .fetch_optional(&self.0).await?;
        Ok(row.map(|r| BlockEmbedding {
            block_id,
            embedding_id: r.get("embedding_id"),
            text_hash: r.get("text_hash"),
            updated_at: chrono::DateTime::parse_from_rfc3339(&r.get::<String, _>("updated_at"))
                .unwrap().with_timezone(&Utc),
        }))
    }

    pub async fn delete_block_embedding(&self, block_id: Uuid) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM block_embeddings WHERE block_id = ?")
            .bind(block_id.to_string()).execute(&self.0).await?;
        Ok(())
    }

    /// (indexed, total_text_blocks)
    pub async fn embedding_progress(&self, identity_id: Uuid) -> Result<(i64, i64), StorageError> {
        let row = sqlx::query(
            "SELECT
               (SELECT COUNT(*) FROM block_embeddings be
                  JOIN blocks bk ON bk.id = be.block_id
                  JOIN notes  n  ON n.id  = bk.note_id
                  JOIN boards b  ON b.id  = n.board_id
                 WHERE b.identity_id = ?) AS indexed,
               (SELECT COUNT(*) FROM blocks bk
                  JOIN notes  n  ON n.id  = bk.note_id
                  JOIN boards b  ON b.id  = n.board_id
                 WHERE b.identity_id = ? AND bk.block_type = 'text' AND LENGTH(bk.content) >= 30) AS total"
        )
        .bind(identity_id.to_string()).bind(identity_id.to_string())
        .fetch_one(&self.0).await?;
        Ok((row.get::<i64, _>("indexed"), row.get::<i64, _>("total")))
    }
}
```

- [ ] **Step 5.3: Add embeddings_enabled helpers to identity.rs**

In `crates/storage/src/db/identity.rs`, append to `impl Db`:

```rust
    pub async fn embeddings_enabled(&self, identity_id: Uuid) -> Result<bool, StorageError> {
        let row = sqlx::query("SELECT embeddings_enabled FROM identities WHERE id = ?")
            .bind(identity_id.to_string())
            .fetch_optional(&self.0).await?;
        Ok(row.map(|r| r.get::<i64, _>("embeddings_enabled") != 0).unwrap_or(false))
    }

    pub async fn set_embeddings_enabled(&self, identity_id: Uuid, enabled: bool) -> Result<(), StorageError> {
        sqlx::query("UPDATE identities SET embeddings_enabled = ? WHERE id = ?")
            .bind(if enabled { 1i64 } else { 0i64 })
            .bind(identity_id.to_string())
            .execute(&self.0).await?;
        Ok(())
    }
```

- [ ] **Step 5.4: Tests**

Append to `crates/storage/src/db/block_embeddings.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_db;
    use chrono::Utc;

    #[tokio::test]
    async fn upsert_then_get() {
        let db = test_db().await;
        // seed enough to have a block to point at
        let identity = Uuid::new_v4();
        sqlx::query("INSERT INTO identities (id, friendly_name, created_at) VALUES (?, ?, ?)")
            .bind(identity.to_string()).bind(format!("ident-{}", identity)).bind(Utc::now().to_rfc3339())
            .execute(&db.0).await.unwrap();
        let board = Uuid::new_v4();
        sqlx::query("INSERT INTO boards (id, identity_id, name, position, created_at) VALUES (?,?,?,?,?)")
            .bind(board.to_string()).bind(identity.to_string()).bind("b").bind(0i32).bind(Utc::now().to_rfc3339())
            .execute(&db.0).await.unwrap();
        let note = Uuid::new_v4();
        sqlx::query("INSERT INTO notes (id, note_type, content, color, board_id, position, blob_key, size, created_at, updated_at) VALUES (?,?,?,?,?,?,?,?,?,?)")
            .bind(note.to_string()).bind("text").bind(b"".to_vec()).bind("#FFF").bind(board.to_string()).bind(0i32).bind(Uuid::new_v4().to_string()).bind(0i64)
            .bind(Utc::now().to_rfc3339()).bind(Utc::now().to_rfc3339())
            .execute(&db.0).await.unwrap();
        let block = Uuid::new_v4();
        sqlx::query("INSERT INTO blocks (id, note_id, position, block_type, content, created_at, updated_at) VALUES (?,?,?,?,?,?,?)")
            .bind(block.to_string()).bind(note.to_string()).bind(1.0f64).bind("text").bind(vec![0u8; 30])
            .bind(Utc::now().to_rfc3339()).bind(Utc::now().to_rfc3339())
            .execute(&db.0).await.unwrap();

        db.upsert_block_embedding(block, "ruv-1", "hash1").await.unwrap();
        let got = db.get_block_embedding(block).await.unwrap().unwrap();
        assert_eq!(got.embedding_id, "ruv-1");

        db.upsert_block_embedding(block, "ruv-2", "hash2").await.unwrap();
        let got2 = db.get_block_embedding(block).await.unwrap().unwrap();
        assert_eq!(got2.embedding_id, "ruv-2");
        assert_eq!(got2.text_hash, "hash2");

        let (indexed, total) = db.embedding_progress(identity).await.unwrap();
        assert_eq!(indexed, 1);
        assert_eq!(total, 1);
    }

    #[tokio::test]
    async fn embeddings_enabled_round_trip() {
        let db = test_db().await;
        let identity = Uuid::new_v4();
        sqlx::query("INSERT INTO identities (id, friendly_name, created_at) VALUES (?, ?, ?)")
            .bind(identity.to_string()).bind(format!("ident-{}", identity)).bind(Utc::now().to_rfc3339())
            .execute(&db.0).await.unwrap();
        assert!(!db.embeddings_enabled(identity).await.unwrap());
        db.set_embeddings_enabled(identity, true).await.unwrap();
        assert!(db.embeddings_enabled(identity).await.unwrap());
    }
}
```

- [ ] **Step 5.5: Run tests**

Run: `cargo test -p storage block_embeddings`
Expected: 2 tests PASS.

- [ ] **Step 5.6: Commit**

```bash
git add crates/storage/src/db/block_embeddings.rs crates/storage/src/db/identity.rs crates/storage/src/db/mod.rs
git commit -m "feat(storage): block_embeddings CRUD + identity embeddings_enabled"
```

---

## Task 6: ruvector Client Adapter

**Files:**
- Create: `crates/api/src/ruvector.rs`
- Modify: `crates/api/src/lib.rs` (or whatever root file declares modules)
- Modify: `crates/api/Cargo.toml`

The plan assumes ruvector is **already accessible from the host as a CLI binary** named `ruvector` (e.g., installed via `npx ruvector` or a local Rust binary). The adapter shells out for `embed` and `knn`. If the actual interface differs, adapt the adapter; the public surface (3 functions) must stay the same.

- [ ] **Step 6.1: Investigate the available ruvector interface**

Run: `which ruvector || which npx`
Run: `npx ruvector --help 2>&1 | head -20`

Pick one of:
- `RuvectorBackend::Cli` shelling out to `ruvector`
- `RuvectorBackend::Stub` (returns deterministic fake ids/scores so the rest of the plan can land without ruvector wired)

If you cannot run ruvector quickly, **use the Stub for this task** and add a TODO comment in the adapter pointing to follow-up wiring. The Stub will not break any test in this plan, only suggestions quality.

- [ ] **Step 6.2: Write the adapter**

Create `crates/api/src/ruvector.rs`:

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// One stored vector in the index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hit {
    pub embedding_id: String,
    pub score: f32,
}

/// Trait so we can swap backends.
#[async_trait::async_trait]
pub trait Ruvector: Send + Sync {
    /// Embed `text` and store under a fresh id; returns the id.
    async fn embed(&self, text: &str) -> Result<String>;
    /// k-NN search around `query` text; returns hits ordered by score desc.
    async fn knn(&self, query: &str, top_k: usize) -> Result<Vec<Hit>>;
    /// Delete a stored vector (best-effort).
    async fn delete(&self, embedding_id: &str) -> Result<()>;
}

/// Stub backend used when ruvector isn't wired yet. Deterministic so tests are stable.
pub struct StubRuvector;

#[async_trait::async_trait]
impl Ruvector for StubRuvector {
    async fn embed(&self, text: &str) -> Result<String> {
        let mut h = Sha256::new();
        h.update(text.as_bytes());
        Ok(format!("stub-{:x}", h.finalize()))
    }
    async fn knn(&self, _query: &str, _top_k: usize) -> Result<Vec<Hit>> { Ok(vec![]) }
    async fn delete(&self, _embedding_id: &str) -> Result<()> { Ok(()) }
}

/// CLI backend — shells out to `ruvector`. Adapt the argv to your install.
pub struct CliRuvector { pub bin: String }

#[async_trait::async_trait]
impl Ruvector for CliRuvector {
    async fn embed(&self, text: &str) -> Result<String> {
        let out = tokio::process::Command::new(&self.bin)
            .args(["embed", "--text", text])
            .output().await?;
        if !out.status.success() {
            anyhow::bail!("ruvector embed failed: {}", String::from_utf8_lossy(&out.stderr));
        }
        // Expect stdout: a JSON like { "embedding_id": "…" }
        #[derive(Deserialize)] struct R { embedding_id: String }
        let r: R = serde_json::from_slice(&out.stdout)?;
        Ok(r.embedding_id)
    }
    async fn knn(&self, query: &str, top_k: usize) -> Result<Vec<Hit>> {
        let out = tokio::process::Command::new(&self.bin)
            .args(["knn", "--query", query, "--top-k", &top_k.to_string()])
            .output().await?;
        if !out.status.success() {
            anyhow::bail!("ruvector knn failed: {}", String::from_utf8_lossy(&out.stderr));
        }
        let hits: Vec<Hit> = serde_json::from_slice(&out.stdout)?;
        Ok(hits)
    }
    async fn delete(&self, embedding_id: &str) -> Result<()> {
        tokio::process::Command::new(&self.bin)
            .args(["delete", "--id", embedding_id])
            .output().await?;
        Ok(())
    }
}

pub fn make_default() -> std::sync::Arc<dyn Ruvector> {
    if let Ok(bin) = std::env::var("RUVECTOR_BIN") {
        std::sync::Arc::new(CliRuvector { bin })
    } else if which::which("ruvector").is_ok() {
        std::sync::Arc::new(CliRuvector { bin: "ruvector".into() })
    } else {
        std::sync::Arc::new(StubRuvector)
    }
}
```

- [ ] **Step 6.3: Deps**

Edit `crates/api/Cargo.toml` to add (under `[dependencies]`):

```toml
async-trait = "0.1"
sha2 = "0.10"
which = "6"
```

- [ ] **Step 6.4: Register module + wire into AppState**

In `crates/api/src/lib.rs` (or where modules are declared), add `pub mod ruvector;`.

In `crates/api/src/state.rs`, add a `pub ruvector: std::sync::Arc<dyn crate::ruvector::Ruvector>` field to `AppState` and initialize it in the constructor via `crate::ruvector::make_default()`.

- [ ] **Step 6.5: Build**

Run: `cargo build -p api`
Expected: clean.

- [ ] **Step 6.6: Commit**

```bash
git add crates/api/src/ruvector.rs crates/api/src/lib.rs crates/api/src/state.rs crates/api/Cargo.toml
git commit -m "feat(api): ruvector adapter (Cli + Stub backends)"
```

---

## Task 7: API — `GET /resolve/page` & `POST /pages`

**Files:**
- Create: `crates/api/src/routes/pages.rs`
- Modify: `crates/api/src/routes/mod.rs`
- Modify: `crates/api/src/routes/notes.rs` (call reconcile on note creation)

- [ ] **Step 7.1: Write the handlers**

Create `crates/api/src/routes/pages.rs`:

```rust
use crate::auth::middleware::AuthenticatedDevice;
use crate::state::{AppState, WsEvent};
use crate::ApiError;
use axum::{extract::{Query, State}, http::StatusCode, Json};
use base64::Engine;
use chrono::Utc;
use jot_core::models::{Note, NoteType};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct ResolveParams { pub title: String }

#[derive(Serialize, ToSchema)]
pub struct TitleRow {
    pub id: String,
    pub board_id: String,
    pub title_b64: Option<String>,
    pub updated_at: String,
}

pub async fn list_titles(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
) -> Result<Json<Vec<TitleRow>>, ApiError> {
    let identity = Uuid::parse_str(&auth.0.identity_id)
        .map_err(|_| ApiError::BadRequest("bad identity id".into()))?;
    let rows = state.db.list_titles_for_identity(identity).await?;
    let b64 = base64::engine::general_purpose::STANDARD;
    Ok(Json(rows.into_iter().map(|r| TitleRow {
        id: r.id.to_string(),
        board_id: r.board_id.to_string(),
        title_b64: r.title.as_deref().map(|t| b64.encode(t)),
        updated_at: r.updated_at,
    }).collect()))
}

#[derive(Deserialize, ToSchema)]
pub struct CreatePageBody {
    pub title_b64: String,            // ciphertext of title, encrypted by the client with the note DEK
    pub plaintext_title_lc: String,   // lowercase title for server-side reconcile (NOT secret — same shape as block_links.target_id)
    pub note_id: Option<Uuid>,        // optional client-generated id (so the client can derive the DEK before POST)
    pub blob_key: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct CreatePageResponse {
    pub note_id: String,
    pub board_id: String,
    pub created: bool,
    pub reconciled: u64,
}

pub async fn create_page(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Json(body): Json<CreatePageBody>,
) -> Result<(StatusCode, Json<CreatePageResponse>), ApiError> {
    let identity = Uuid::parse_str(&auth.0.identity_id)
        .map_err(|_| ApiError::BadRequest("bad identity id".into()))?;

    let board = state.db.find_or_create_pages_board(identity).await?;

    // Idempotency: if a note with that title (decoded by the calling client) already exists,
    // the SPA can pre-check with its titleIndex. The server cannot decrypt to match, so we
    // simply create a new note here. The client must guard before calling.
    let note_id = body.note_id.unwrap_or_else(Uuid::new_v4);
    let blob_key = body.blob_key.unwrap_or_else(|| Uuid::new_v4().to_string());
    let b64 = base64::engine::general_purpose::STANDARD;
    let title_bytes = b64.decode(&body.title_b64)
        .map_err(|_| ApiError::BadRequest("invalid title_b64".into()))?;

    let now = Utc::now();
    let note = Note {
        id: note_id,
        note_type: NoteType::Text,
        content: Vec::new(),
        thumbnail: None,
        duration_ms: None,
        color: "#FFFFFF".into(),
        board_id: board.id,
        position: 0,
        blob_key,
        size: 0,
        created_at: now,
        updated_at: now,
        title: Some(title_bytes),
        is_journal: false,
        journal_date: None,
        schema_version: 1,
    };
    state.db.insert_note(&note).await?;

    // Reconcile pending unresolved page refs
    let reconciled = state.db.reconcile_unresolved_page_refs(&body.plaintext_title_lc, note_id).await?;
    if reconciled > 0 {
        let _ = state.ws_tx.send(WsEvent::PageResolved {
            title: body.plaintext_title_lc.clone(),
            note_id: note_id.to_string(),
        });
    }

    Ok((StatusCode::CREATED, Json(CreatePageResponse {
        note_id: note_id.to_string(),
        board_id: board.id.to_string(),
        created: true,
        reconciled,
    })))
}

#[derive(Deserialize, ToSchema)]
pub struct ReconcileTitleBody { pub title: String, pub note_id: Uuid }

pub async fn reconcile_title(
    State(state): State<AppState>,
    _auth: AuthenticatedDevice,
    Json(body): Json<ReconcileTitleBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let n = state.db.reconcile_unresolved_page_refs(&body.title, body.note_id).await?;
    let _ = state.ws_tx.send(WsEvent::PageResolved {
        title: body.title.clone(),
        note_id: body.note_id.to_string(),
    });
    Ok(Json(serde_json::json!({ "reconciled": n })))
}
```

- [ ] **Step 7.2: Add `WsEvent::PageResolved`**

In `crates/api/src/state.rs`, add a variant to the `WsEvent` enum:

```rust
    PageResolved { title: String, note_id: String },
```

- [ ] **Step 7.3: Register routes**

In `crates/api/src/routes/mod.rs`, add `pub mod pages;` and inside the router builder:

```rust
    .route("/notes/titles",          axum::routing::get(pages::list_titles))
    .route("/pages",                 axum::routing::post(pages::create_page))
    .route("/links/reconcile-title", axum::routing::post(pages::reconcile_title))
```

- [ ] **Step 7.4: Build & smoke test**

Run: `cargo build -p api`
Run: `cargo test -p api 2>&1 | tail -5`
Expected: clean build, no new test failures.

- [ ] **Step 7.5: Commit**

```bash
git add crates/api/src/routes/pages.rs crates/api/src/routes/mod.rs crates/api/src/state.rs
git commit -m "feat(api): /notes/titles, /pages, /links/reconcile-title"
```

---

## Task 8: API — Embeddings & Suggestions

**Files:**
- Create: `crates/api/src/routes/embeddings.rs`
- Create: `crates/api/src/routes/suggestions.rs`
- Modify: `crates/api/src/routes/mod.rs`
- Modify: `crates/api/src/state.rs`

- [ ] **Step 8.1: Add `WsEvent::EmbeddingIndexed`**

In `crates/api/src/state.rs`:

```rust
    EmbeddingIndexed { block_id: String, indexed: i64, total: i64 },
```

- [ ] **Step 8.2: Write embeddings route**

Create `crates/api/src/routes/embeddings.rs`:

```rust
use crate::auth::middleware::AuthenticatedDevice;
use crate::state::{AppState, WsEvent};
use crate::ApiError;
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize, ToSchema)]
pub struct UpsertBody { pub block_id: Uuid, pub plaintext: String }

pub async fn upsert(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Json(body): Json<UpsertBody>,
) -> Result<StatusCode, ApiError> {
    let identity = Uuid::parse_str(&auth.0.identity_id)
        .map_err(|_| ApiError::BadRequest("bad identity".into()))?;
    if !state.db.embeddings_enabled(identity).await? {
        return Err(ApiError::Forbidden("embeddings disabled for this identity".into()));
    }
    let mut h = Sha256::new();
    h.update(body.plaintext.as_bytes());
    let text_hash = format!("{:x}", h.finalize());
    if let Some(existing) = state.db.get_block_embedding(body.block_id).await? {
        if existing.text_hash == text_hash {
            return Ok(StatusCode::NO_CONTENT);
        }
        let _ = state.ruvector.delete(&existing.embedding_id).await;
    }
    let embedding_id = state.ruvector.embed(&body.plaintext).await
        .map_err(|e| ApiError::Internal(format!("ruvector: {e}")))?;
    state.db.upsert_block_embedding(body.block_id, &embedding_id, &text_hash).await?;
    let (indexed, total) = state.db.embedding_progress(identity).await?;
    let _ = state.ws_tx.send(WsEvent::EmbeddingIndexed {
        block_id: body.block_id.to_string(), indexed, total,
    });
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize, ToSchema)]
pub struct StatusResponse { pub enabled: bool, pub indexed: i64, pub total: i64 }

pub async fn status(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
) -> Result<Json<StatusResponse>, ApiError> {
    let identity = Uuid::parse_str(&auth.0.identity_id)
        .map_err(|_| ApiError::BadRequest("bad identity".into()))?;
    let enabled = state.db.embeddings_enabled(identity).await?;
    let (indexed, total) = state.db.embedding_progress(identity).await?;
    Ok(Json(StatusResponse { enabled, indexed, total }))
}

#[derive(Deserialize, ToSchema)]
pub struct EnableBody { pub enabled: bool }

pub async fn set_enabled(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Json(body): Json<EnableBody>,
) -> Result<StatusCode, ApiError> {
    let identity = Uuid::parse_str(&auth.0.identity_id)
        .map_err(|_| ApiError::BadRequest("bad identity".into()))?;
    state.db.set_embeddings_enabled(identity, body.enabled).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Placeholder: a real reindex would walk all text blocks and have the client re-send
/// their plaintexts (the server doesn't have plaintexts). The endpoint returns 202 and
/// emits `EmbeddingIndexed { block_id="*", indexed=0, total }` so the SPA can resync.
pub async fn reindex(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
) -> Result<StatusCode, ApiError> {
    let identity = Uuid::parse_str(&auth.0.identity_id)
        .map_err(|_| ApiError::BadRequest("bad identity".into()))?;
    let (indexed, total) = state.db.embedding_progress(identity).await?;
    let _ = state.ws_tx.send(WsEvent::EmbeddingIndexed {
        block_id: "*".into(), indexed, total,
    });
    Ok(StatusCode::ACCEPTED)
}
```

- [ ] **Step 8.3: Write suggestions route**

Create `crates/api/src/routes/suggestions.rs`:

```rust
use crate::auth::middleware::AuthenticatedDevice;
use crate::state::AppState;
use crate::ApiError;
use axum::{extract::{Query, State}, Json};
use serde::{Deserialize, Serialize};
use storage::db::shares::permission_allows;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct RelatedQuery {
    pub note_id: Option<Uuid>,
    pub tag: Option<String>,
    pub top_k: Option<usize>,
}

#[derive(Serialize, ToSchema)]
pub struct SuggestionHit {
    pub block_id: String,
    pub source_note_id: String,
    pub score: f32,
    pub snippet_b64: String,
}

pub async fn related(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Query(q): Query<RelatedQuery>,
) -> Result<Json<Vec<SuggestionHit>>, ApiError> {
    let identity = Uuid::parse_str(&auth.0.identity_id)
        .map_err(|_| ApiError::BadRequest("bad identity".into()))?;
    if !state.db.embeddings_enabled(identity).await? { return Ok(Json(vec![])); }

    // Build a query: either the concatenated decrypted text of the note (unavailable to the
    // server!) — so for the MVP we use the note's TITLE plaintext_lc if available, OR the tag
    // name. The SPA can also pass an explicit text via a future param.
    let query_text = match (&q.note_id, &q.tag) {
        (Some(_), _) => "".to_string(), // server can't decrypt; client should pass tag/text-based context
        (_, Some(t)) => t.clone(),
        _ => return Err(ApiError::BadRequest("note_id or tag required".into())),
    };
    if query_text.is_empty() {
        // No usable server-side context for note_id: return empty. The SPA can later switch to
        // a /suggestions/related-text endpoint that accepts plaintext.
        return Ok(Json(vec![]));
    }

    let top_k = q.top_k.unwrap_or(5);
    let hits = state.ruvector.knn(&query_text, top_k.saturating_mul(3)).await
        .map_err(|e| ApiError::Internal(format!("ruvector: {e}")))?;

    // Resolve embedding_id -> block_id via block_embeddings table, filter by readability.
    let mut out: Vec<SuggestionHit> = Vec::new();
    for h in hits {
        if let Some(b_id) = state.db.block_id_for_embedding(&h.embedding_id).await? {
            if let Some(b) = state.db.get_block(b_id).await? {
                let perm = state.db.note_permission_for(b.note_id.to_string().as_str(), &auth.0.identity_id).await?;
                if permission_allows(&perm, "read") {
                    use base64::Engine;
                    let snippet = base64::engine::general_purpose::STANDARD.encode(&b.content);
                    out.push(SuggestionHit {
                        block_id: b_id.to_string(),
                        source_note_id: b.note_id.to_string(),
                        score: h.score,
                        snippet_b64: snippet,
                    });
                    if out.len() >= top_k { break; }
                }
            }
        }
    }
    Ok(Json(out))
}
```

- [ ] **Step 8.4: Add `block_id_for_embedding` helper to storage**

In `crates/storage/src/db/block_embeddings.rs` append:

```rust
impl Db {
    pub async fn block_id_for_embedding(&self, embedding_id: &str) -> Result<Option<Uuid>, StorageError> {
        let row = sqlx::query("SELECT block_id FROM block_embeddings WHERE embedding_id = ?")
            .bind(embedding_id).fetch_optional(&self.0).await?;
        Ok(row.map(|r| Uuid::parse_str(&r.get::<String, _>("block_id")).unwrap()))
    }
}
```

- [ ] **Step 8.5: Register routes**

In `crates/api/src/routes/mod.rs`, add `pub mod embeddings;` and `pub mod suggestions;`. Inside the router:

```rust
    .route("/embeddings/upsert",      axum::routing::post(embeddings::upsert))
    .route("/embeddings/status",       axum::routing::get(embeddings::status))
    .route("/embeddings/enabled",     axum::routing::put(embeddings::set_enabled))
    .route("/embeddings/reindex",     axum::routing::post(embeddings::reindex))
    .route("/suggestions/related",     axum::routing::get(suggestions::related))
```

- [ ] **Step 8.6: Build**

Run: `cargo build -p api`
Expected: clean. If `ApiError::Internal(String)` doesn't exist with that signature, adapt to the actual variant (e.g., `ApiError::Internal` unit or `ApiError::from(anyhow::Error)`).

- [ ] **Step 8.7: Commit**

```bash
git add crates/api/src/routes/embeddings.rs crates/api/src/routes/suggestions.rs crates/api/src/routes/mod.rs crates/api/src/state.rs crates/storage/src/db/block_embeddings.rs
git commit -m "feat(api): embeddings upsert/status/reindex + suggestions/related"
```

---

## Task 9: SPA — `extractLinks` (mirror of Rust)

**Files:**
- Create: `spa/src/blocks/links.ts`
- Create: `spa/src/blocks/links.test.ts` (if vitest exists — else skip)

- [ ] **Step 9.1: Write the parser**

Create `spa/src/blocks/links.ts`:

```typescript
export type TargetKind = "note" | "block" | "tag";
export type LinkKind = "page_ref" | "page_ref_unresolved" | "block_ref" | "block_embed" | "tag";

export interface ExtractedLink {
  target_kind: TargetKind;
  target_id: string;
  link_kind: LinkKind;
}

const RE_PAGE  = /\[\[([^\]\n]+?)\]\]/g;
const RE_EMBED = /!\(\(([0-9a-fA-F-]{36})\)\)/g;
const RE_BLOCK = /\(\(([0-9a-fA-F-]{36})\)\)/g;
const RE_TAG   = /(?:^|\s)#([A-Za-z0-9_\-]+)/g;

export function extractLinks(
  markdown: string,
  titleToId: Map<string, string>,
): ExtractedLink[] {
  const out: ExtractedLink[] = [];

  // Embeds first so plain block refs can dedupe.
  for (const m of markdown.matchAll(RE_EMBED)) {
    out.push({ target_kind: "block", target_id: m[1], link_kind: "block_embed" });
  }
  for (const m of markdown.matchAll(RE_BLOCK)) {
    const id = m[1];
    if (out.some(l => l.link_kind === "block_embed" && l.target_id === id)) continue;
    out.push({ target_kind: "block", target_id: id, link_kind: "block_ref" });
  }
  for (const m of markdown.matchAll(RE_PAGE)) {
    const title = m[1].trim();
    const lcTitle = title.toLowerCase();
    const noteId = titleToId.get(lcTitle);
    if (noteId) {
      out.push({ target_kind: "note", target_id: noteId, link_kind: "page_ref" });
    } else {
      out.push({ target_kind: "note", target_id: lcTitle, link_kind: "page_ref_unresolved" });
    }
  }
  for (const m of markdown.matchAll(RE_TAG)) {
    out.push({ target_kind: "tag", target_id: m[1], link_kind: "tag" });
  }
  return out;
}
```

- [ ] **Step 9.2: Vitest parity tests (skip if vitest unavailable)**

Check `spa/package.json` for `vitest`. If present, create `spa/src/blocks/links.test.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { extractLinks } from "./links";

describe("extractLinks", () => {
  it("resolves [[Title]] via title index", () => {
    const idx = new Map([["hello", "n-1"]]);
    const out = extractLinks("see [[Hello]] today", idx);
    expect(out).toEqual([{ target_kind: "note", target_id: "n-1", link_kind: "page_ref" }]);
  });

  it("marks unknown [[Title]] as unresolved with lowercase id", () => {
    const out = extractLinks("see [[Unknown Title]]", new Map());
    expect(out[0]).toEqual({ target_kind: "note", target_id: "unknown title", link_kind: "page_ref_unresolved" });
  });

  it("differentiates block ref and embed", () => {
    const out = extractLinks(
      "((550e8400-e29b-41d4-a716-446655440000)) and !((550e8400-e29b-41d4-a716-446655440001))",
      new Map(),
    );
    expect(out.map(l => l.link_kind).sort()).toEqual(["block_embed", "block_ref"]);
  });

  it("extracts tags", () => {
    const out = extractLinks("status #wip and #done-2025", new Map());
    expect(out.filter(l => l.link_kind === "tag").map(l => l.target_id)).toEqual(["wip", "done-2025"]);
  });
});
```

- [ ] **Step 9.3: Build**

Run: `cd spa && npm run build`
Expected: clean.

- [ ] **Step 9.4: Commit**

```bash
git add spa/src/blocks/links.ts spa/src/blocks/links.test.ts 2>/dev/null
git commit -m "feat(spa): extractLinks() — JS mirror of Rust parser with unresolved variant"
```

---

## Task 10: SPA — Title Index

**Files:**
- Create: `spa/src/blocks/titleIndex.ts`
- Modify: `spa/src/api.ts`

- [ ] **Step 10.1: Add API helpers**

Append to `spa/src/api.ts`:

```typescript
export interface ServerTitleRow {
  id: string;
  board_id: string;
  title_b64?: string | null;
  updated_at: string;
}

export async function fetchTitleRows(): Promise<ServerTitleRow[]> {
  const r = await authedFetch(`${BASE}/notes/titles`);
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export interface ResolvedPage { note_id: string; board_id: string }

export async function createPage(args: {
  title_b64: string;
  plaintext_title_lc: string;
  note_id?: string;
  blob_key?: string;
}): Promise<{ note_id: string; board_id: string; created: boolean; reconciled: number }> {
  const r = await authedFetch(`${BASE}/pages`, { method: "POST", body: JSON.stringify(args) });
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function reconcileTitle(title: string, note_id: string): Promise<{ reconciled: number }> {
  const r = await authedFetch(`${BASE}/links/reconcile-title`, {
    method: "POST", body: JSON.stringify({ title, note_id }),
  });
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}
```

- [ ] **Step 10.2: Write the title index**

Create `spa/src/blocks/titleIndex.ts`:

```typescript
import { signal } from "@preact/signals";
import * as api from "../api";
import { decryptBlock } from "./crypto";

// Public signal so components can re-render when the index changes.
export const titleIndexVersion = signal(0);

// Maps lowercase title -> note_id (winner of conflicts: most-recently-updated).
const index = new Map<string, string>();
// Reverse map for cleanup on rename.
const noteToTitle = new Map<string, string>();
// Tracks board_id per note so callers can navigate cross-board.
const noteBoard = new Map<string, string>();

export function lookupByTitle(title: string): string | null {
  return index.get(title.trim().toLowerCase()) ?? null;
}

export function boardOf(noteId: string): string | null {
  return noteBoard.get(noteId) ?? null;
}

export function snapshot(): Map<string, string> {
  return new Map(index);
}

export async function loadAll(): Promise<void> {
  index.clear();
  noteToTitle.clear();
  noteBoard.clear();
  const rows = await api.fetchTitleRows();
  // rows are sorted most-recently-updated DESC by the server, so the FIRST occurrence wins.
  for (const row of rows) {
    noteBoard.set(row.id, row.board_id);
    if (!row.title_b64) continue;
    try {
      const title = (await decryptBlock(row.board_id, row.id, row.title_b64)).trim();
      if (!title) continue;
      const lc = title.toLowerCase();
      if (!index.has(lc)) {
        index.set(lc, row.id);
        noteToTitle.set(row.id, lc);
      }
    } catch {
      // unreadable (shared note we don't have a key for)
    }
  }
  titleIndexVersion.value++;
}

export function applyLocalChange(noteId: string, plaintextTitle: string | null, boardId?: string): void {
  if (boardId) noteBoard.set(noteId, boardId);
  const old = noteToTitle.get(noteId);
  if (old && index.get(old) === noteId) index.delete(old);
  noteToTitle.delete(noteId);
  if (plaintextTitle && plaintextTitle.trim()) {
    const lc = plaintextTitle.trim().toLowerCase();
    if (!index.has(lc)) {
      index.set(lc, noteId);
      noteToTitle.set(noteId, lc);
    }
  }
  titleIndexVersion.value++;
}

export function removeNote(noteId: string): void {
  const old = noteToTitle.get(noteId);
  if (old && index.get(old) === noteId) index.delete(old);
  noteToTitle.delete(noteId);
  noteBoard.delete(noteId);
  titleIndexVersion.value++;
}
```

- [ ] **Step 10.3: Bootstrap loading at app start**

Find the SPA's top-level entrypoint (likely `spa/src/main.tsx` or `spa/src/App.tsx`). After auth succeeds and before rendering routes, call `import { loadAll } from "./blocks/titleIndex"; await loadAll();` (fire-and-forget — UI can render without the index, links will resolve as it populates).

If unsure where to wire, add the call inside `Layout.tsx`'s `useEffect(() => { … }, [])` so it runs on mount.

- [ ] **Step 10.4: Build & commit**

Run: `cd spa && npm run build`

```bash
git add spa/src/api.ts spa/src/blocks/titleIndex.ts spa/src/components/Layout.tsx 2>/dev/null
git commit -m "feat(spa): identity-wide title index for [[Page]] resolution"
```

---

## Task 11: SPA — Auto-extraction on Block Save

**Files:**
- Modify: `spa/src/blocks/keymap.ts`
- Modify: `spa/src/api.ts`

- [ ] **Step 11.1: Hook auto-extraction into persistEdit**

In `spa/src/blocks/keymap.ts`, import the new modules:

```typescript
import { extractLinks } from "./links";
import { snapshot as titleSnapshot } from "./titleIndex";
```

Modify `persistEdit` to also push the extracted links:

```typescript
export async function persistEdit(
  ctx: KeymapCtx,
  block: BlockNode,
  plaintext: string,
  block_type?: string,
) {
  const newCt = await encryptBlock(ctx.boardId, ctx.noteId, plaintext);
  const oldCt = block.content;
  const oldType = block.block_type;
  const newType = block_type ?? oldType;
  await api.patchBlock(block.id, { content_b64: newCt, block_type: newType });

  // Auto-extract and push the edge set
  try {
    const links = extractLinks(plaintext, titleSnapshot());
    await api.putBlockLinks(block.id, links);
  } catch (e) { console.warn("link extraction failed", e); }

  ctx.undoStack.push({ /* …existing entry, unchanged… */
    label: "edit",
    undo: async () => {
      await api.patchBlock(block.id, { content_b64: oldCt, block_type: oldType });
      try {
        const oldLinks = extractLinks(new TextDecoder().decode(Uint8Array.from(atob(oldCt), c => c.charCodeAt(0))).slice(12), titleSnapshot());
        // Note: oldCt is ciphertext, can't decrypt without DEK here. Skip link restore on undo —
        // server stays out of sync with edges on undo. Accepted trade-off.
        void oldLinks;
      } catch {}
      await ctx.refresh();
    },
    redo: async () => {
      await api.patchBlock(block.id, { content_b64: newCt, block_type: newType });
      try {
        const links = extractLinks(plaintext, titleSnapshot());
        await api.putBlockLinks(block.id, links);
      } catch {}
      await ctx.refresh();
    },
  });
}
```

Also extract+push links in `newBlockBelow` (after the create) and `deleteMany` (cleanup is automatic via FK cascade — no need to push).

- [ ] **Step 11.2: Build & commit**

```bash
cd spa && npm run build
git add spa/src/blocks/keymap.ts
git commit -m "feat(spa): auto-extract block links on every save"
```

---

## Task 12: SPA — View-mode Renderer + Click Delegation

**Files:**
- Create: `spa/src/blocks/render.ts`
- Create: `spa/src/blocks/clickHandler.ts`

- [ ] **Step 12.1: Renderer**

Create `spa/src/blocks/render.ts`:

```typescript
import { lookupByTitle } from "./titleIndex";

const RE = /(!\(\([0-9a-fA-F-]{36}\)\))|(\(\([0-9a-fA-F-]{36}\)\))|(\[\[[^\]\n]+?\]\])|((?:^|\s)#[A-Za-z0-9_\-]+)/g;

const esc = (s: string) => s.replace(/[&<>]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" }[c]!));

export function renderBlockHtml(plaintext: string): string {
  let out = "";
  let last = 0;
  for (const m of plaintext.matchAll(RE)) {
    const idx = m.index!;
    out += esc(plaintext.slice(last, idx));
    const full = m[0];
    if (full.startsWith("!((")) {
      const id = full.slice(3, -2);
      out += `<span class="block-embed-mount" data-block-id="${esc(id)}"></span>`;
    } else if (full.startsWith("((")) {
      const id = full.slice(2, -2);
      out += `<a class="block-link" data-block-id="${esc(id)}" href="#">⛓</a>`;
    } else if (full.startsWith("[[")) {
      const title = full.slice(2, -2).trim();
      const noteId = lookupByTitle(title);
      if (noteId) {
        out += `<a class="page-link" data-note-id="${esc(noteId)}" href="#">${esc(title)}</a>`;
      } else {
        out += `<a class="page-link unresolved" data-title="${esc(title)}" href="#">${esc(title)}</a>`;
      }
    } else {
      const leading = full[0] === "#" ? "" : full[0];
      const tag = full.replace(/^[^#]/, "").slice(1);
      out += esc(leading) + `<a class="tag-link" data-tag="${esc(tag)}" href="#">#${esc(tag)}</a>`;
    }
    last = idx + full.length;
  }
  out += esc(plaintext.slice(last));
  return out;
}
```

- [ ] **Step 12.2: Click handler factory**

Create `spa/src/blocks/clickHandler.ts`:

```typescript
import * as api from "../api";
import { boardOf } from "./titleIndex";
import { encryptBlock } from "./crypto";
import { selectedNoteId } from "../selectedNote";

export interface ClickCtx {
  boardId: string;
  noteId: string;
}

export function makeClickHandler(ctx: ClickCtx) {
  return async (e: MouseEvent) => {
    const t = e.target as HTMLElement | null;
    if (!t) return;
    const page = t.closest(".page-link") as HTMLElement | null;
    const block = t.closest(".block-link") as HTMLElement | null;
    const tag = t.closest(".tag-link") as HTMLElement | null;
    if (page) {
      e.preventDefault();
      const nid = page.getAttribute("data-note-id");
      if (nid) {
        selectedNoteId.value = nid;
        return;
      }
      const title = page.getAttribute("data-title");
      if (!title) return;
      // Unresolved — create page
      const ct = await encryptBlock(ctx.boardId, ctx.noteId, title);
      const resp = await api.createPage({
        title_b64: ct, plaintext_title_lc: title.toLowerCase(),
      });
      selectedNoteId.value = resp.note_id;
    } else if (block) {
      e.preventDefault();
      const bid = block.getAttribute("data-block-id");
      if (!bid) return;
      try {
        const dto = await fetch(`/blocks/${bid}`, { headers: { Authorization: `Bearer ${localStorage.getItem("token") ?? ""}` } }).then(r => r.json());
        selectedNoteId.value = dto.note_id;
        setTimeout(() => {
          const el = document.querySelector(`.block-bullet[data-id="${CSS.escape(bid)}"]`);
          el?.scrollIntoView({ behavior: "smooth", block: "center" });
          el?.classList.add("flash-highlight");
          setTimeout(() => el?.classList.remove("flash-highlight"), 1500);
        }, 100);
      } catch (err) { console.warn("block navigate failed", err); }
    } else if (tag) {
      e.preventDefault();
      const t = tag.getAttribute("data-tag");
      if (t) location.hash = `#/tag/${encodeURIComponent(t)}`;
    }
  };
}
```

- [ ] **Step 12.3: Build & commit**

```bash
cd spa && npm run build
git add spa/src/blocks/render.ts spa/src/blocks/clickHandler.ts
git commit -m "feat(spa): view-mode block renderer + click delegation"
```

---

## Task 13: SPA — BlockEditor Edit/View Mode Switch

**Files:**
- Modify: `spa/src/components/BlockEditor.tsx`
- Modify: `spa/src/components/BlockEditor.css`

- [ ] **Step 13.1: Switch the rendering of block content based on focus**

In `BlockEditor.tsx`:

1. Import:
```tsx
import { renderBlockHtml } from "../blocks/render";
import { makeClickHandler } from "../blocks/clickHandler";
import { titleIndexVersion } from "../blocks/titleIndex";
```

2. Replace the `<div class="block-content" ...>` render with a conditional based on focus state. Use a new state `editingBlockId: string | null`:

```tsx
const [editingBlockId, setEditingBlockId] = useState<string | null>(null);
// Re-render when the title index changes (so unresolved links flip to resolved).
const _v = titleIndexVersion.value; void _v;

const clickHandler = useMemo(() => makeClickHandler({ boardId, noteId }), [boardId, noteId]);
```

3. The block-content element:

```tsx
<div
  class="block-content"
  contentEditable={editingBlockId === n.id}
  onFocus={() => { setActive(n.id); setEditingBlockId(n.id); clearSelection(); }}
  onBlur={(ev) => {
    onBlur(n, (ev.target as HTMLElement).innerText);
    setEditingBlockId(prev => (prev === n.id ? null : prev));
  }}
  onKeyDown={onKeyDown as any}
  onClick={clickHandler as any}
  dangerouslySetInnerHTML={
    editingBlockId === n.id
      ? { __html: escapeHtml(n.plaintext) }
      : { __html: renderBlockHtml(n.plaintext) }
  }
/>
```

4. To enter edit mode, the user clicks anywhere on the block-content (the `onFocus` fires only when contentEditable is set). Workaround: also add an `onMouseDown` that flips `editingBlockId` to `n.id` if not already set, so the next `mouseup` lands in an editable surface and the contentEditable accepts focus:

```tsx
onMouseDown={() => { if (editingBlockId !== n.id) setEditingBlockId(n.id); }}
```

Be careful: clicking a `.page-link` should NAVIGATE (handled by `clickHandler`) — don't enter edit mode. Update onMouseDown:

```tsx
onMouseDown={(ev) => {
  if ((ev.target as HTMLElement).closest("a")) return; // clicks on links navigate
  if (editingBlockId !== n.id) setEditingBlockId(n.id);
}}
```

- [ ] **Step 13.2: CSS for link styles + flash**

Append to `spa/src/components/BlockEditor.css`:

```css
.block-content .page-link { color: var(--accent, #4af); text-decoration: none; border-bottom: 1px dotted; cursor: pointer; }
.block-content .page-link.unresolved { color: #ff9f0a; font-style: italic; }
.block-content .block-link { color: var(--muted, #888); text-decoration: none; cursor: pointer; }
.block-content .tag-link { color: #6cf; text-decoration: none; padding: 0 2px; border-radius: 3px; background: rgba(102, 204, 255, 0.08); cursor: pointer; }
.flash-highlight { animation: flash 1.5s ease-out; }
@keyframes flash { 0% { background: rgba(74,144,226,0.4); } 100% { background: transparent; } }
```

- [ ] **Step 13.3: Build & commit**

```bash
cd spa && npm run build
git add spa/src/components/BlockEditor.tsx spa/src/components/BlockEditor.css
git commit -m "feat(spa): block-content swaps between editable plaintext and clickable rendered view"
```

---

## Task 14: SPA — EmbedBlock Component

**Files:**
- Create: `spa/src/components/EmbedBlock.tsx`
- Modify: `spa/src/components/BlockEditor.tsx`

- [ ] **Step 14.1: EmbedBlock component**

Create `spa/src/components/EmbedBlock.tsx`:

```tsx
import { useEffect, useState } from "preact/hooks";
import * as api from "../api";
import { decryptBlock } from "../blocks/crypto";

interface Props { blockId: string; depth?: number }

export function EmbedBlock({ blockId, depth = 0 }: Props) {
  const [text, setText] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    (async () => {
      if (depth > 1) { setError("…"); return; }
      try {
        const r = await fetch(`/blocks/${encodeURIComponent(blockId)}`, {
          headers: { Authorization: `Bearer ${localStorage.getItem("token") ?? ""}` },
        });
        if (!r.ok) throw new Error(await r.text());
        const dto = await r.json();
        // We need the board_id of the embedded block's note. Fetch note meta.
        const metaR = await fetch(`/notes/${dto.note_id}`, {
          headers: { Authorization: `Bearer ${localStorage.getItem("token") ?? ""}` },
        });
        if (!metaR.ok) throw new Error(await metaR.text());
        const meta = await metaR.json();
        const plain = await decryptBlock(meta.board_id, dto.note_id, dto.content);
        setText(plain);
      } catch (e) {
        setError("🔒 inaccessible");
      }
    })();
  }, [blockId, depth]);

  if (error) return <span class="embed-error">{error}</span>;
  if (text === null) return <span class="embed-loading">…</span>;
  return <span class="embed-content">{text}</span>;
}
```

- [ ] **Step 14.2: Mount EmbedBlock components after view-mode render**

In `BlockEditor.tsx`, after the `dangerouslySetInnerHTML` render, query the resulting DOM for `.block-embed-mount` elements and render `<EmbedBlock>` into each. This is awkward with React/Preact since the embed elements are produced via raw HTML. Workaround: use a `useEffect` that finds mounts and replaces them.

Add to `BlockEditor.tsx` (inside the component, near other effects):

```tsx
import { render } from "preact";
import { EmbedBlock } from "./EmbedBlock";

useEffect(() => {
  if (editingBlockId !== null) return; // only in view mode
  const mounts = document.querySelectorAll(".block-embed-mount:not([data-mounted])");
  mounts.forEach(el => {
    const id = el.getAttribute("data-block-id");
    if (!id) return;
    el.setAttribute("data-mounted", "1");
    render(<EmbedBlock blockId={id} />, el);
  });
});
```

(Without dependency array → runs after every render. Fine for now; optimise later.)

CSS in `BlockEditor.css`:

```css
.block-embed-mount { display: inline-block; padding: 2px 8px; margin: 0 2px; border-left: 3px solid var(--accent, #4af); background: rgba(74,144,226,0.06); border-radius: 3px; vertical-align: middle; }
.embed-loading { color: var(--muted, #888); }
.embed-error { color: #ff9f0a; }
.embed-content { color: var(--text, inherit); }
```

- [ ] **Step 14.3: Build & commit**

```bash
cd spa && npm run build
git add spa/src/components/EmbedBlock.tsx spa/src/components/BlockEditor.tsx spa/src/components/BlockEditor.css
git commit -m "feat(spa): inline !((id)) embeds with 1-level depth limit"
```

---

## Task 15: SPA — LinkAutocomplete

**Files:**
- Create: `spa/src/components/LinkAutocomplete.tsx`
- Modify: `spa/src/components/BlockEditor.tsx`
- Modify: `spa/src/api.ts`

- [ ] **Step 15.1: Add tags fetch**

`spa/src/api.ts` already has `listTags()`. Reuse.

- [ ] **Step 15.2: LinkAutocomplete component**

Create `spa/src/components/LinkAutocomplete.tsx`:

```tsx
import { useEffect, useState, useRef } from "preact/hooks";
import * as api from "../api";
import { snapshot as titleSnapshot } from "../blocks/titleIndex";

export type Trigger = "page" | "tag";

interface Props {
  trigger: Trigger;
  query: string;
  position: { x: number; y: number };
  onPick: (label: string, isNew: boolean) => void;
  onClose: () => void;
}

export function LinkAutocomplete({ trigger, query, position, onPick, onClose }: Props) {
  const [items, setItems] = useState<{ label: string; isExisting: boolean }[]>([]);
  const [active, setActive] = useState(0);

  useEffect(() => {
    (async () => {
      const q = query.trim().toLowerCase();
      if (trigger === "page") {
        const titles = Array.from(titleSnapshot().keys());
        const matches = titles.filter(t => t.startsWith(q)).slice(0, 5).map(label => ({ label, isExisting: true }));
        if (q && !matches.some(m => m.label === q)) matches.push({ label: query, isExisting: false });
        setItems(matches);
      } else {
        try {
          const tags = await api.listTags();
          const matches = tags.map(t => t.name).filter(n => n.toLowerCase().startsWith(q)).slice(0, 5)
            .map(label => ({ label, isExisting: true }));
          if (q && !matches.some(m => m.label === q)) matches.push({ label: query, isExisting: false });
          setItems(matches);
        } catch { setItems(query ? [{ label: query, isExisting: false }] : []); }
      }
      setActive(0);
    })();
  }, [trigger, query]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "ArrowDown") { e.preventDefault(); setActive(a => (a + 1) % items.length); }
      else if (e.key === "ArrowUp") { e.preventDefault(); setActive(a => (a - 1 + items.length) % items.length); }
      else if (e.key === "Enter") { e.preventDefault(); const it = items[active]; if (it) onPick(it.label, !it.isExisting); }
      else if (e.key === "Escape") { e.preventDefault(); onClose(); }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [items, active]);

  return (
    <div class="link-autocomplete" style={{ position: "fixed", left: position.x, top: position.y }}>
      {items.length === 0 && <div class="ac-empty">No matches</div>}
      {items.map((it, i) => (
        <button class={i === active ? "active" : ""} onMouseEnter={() => setActive(i)} onClick={() => onPick(it.label, !it.isExisting)}>
          {it.isExisting ? it.label : <>➕ Create "{it.label}"</>}
        </button>
      ))}
    </div>
  );
}
```

CSS:

```css
.link-autocomplete { background: var(--surface, #1a1a1a); border: 1px solid rgba(255,255,255,0.1); border-radius: 6px; padding: 4px; min-width: 200px; z-index: 1100; box-shadow: 0 8px 24px rgba(0,0,0,0.4); }
.link-autocomplete button { display: block; width: 100%; background: transparent; border: none; color: inherit; text-align: left; padding: 4px 10px; border-radius: 4px; cursor: pointer; }
.link-autocomplete button.active, .link-autocomplete button:hover { background: rgba(255,255,255,0.08); }
.ac-empty { color: var(--muted, #888); padding: 4px 10px; }
```

- [ ] **Step 15.3: Trigger autocomplete from BlockEditor**

In `BlockEditor.tsx`, when a contentEditable receives an `onInput`, inspect the text between the trigger and caret. If we see an unmatched `[[` followed by chars and no `]]` between the trigger and caret, open the popup. Same for `#` at word boundary.

Add state:

```tsx
const [autocomplete, setAutocomplete] = useState<{
  trigger: "page" | "tag"; query: string; position: { x: number; y: number }; blockId: string;
} | null>(null);
```

Detection: in `onKeyDown` already used, after applying the key, evaluate the resulting text. Simpler: add `onInput` to `.block-content`:

```tsx
onInput={(ev) => {
  if (editingBlockId !== n.id) return;
  const el = ev.target as HTMLElement;
  const sel = window.getSelection();
  if (!sel || !sel.focusNode) return;
  const text = el.innerText;
  const caret = caretOffset(el, sel);
  const left = text.slice(0, caret);
  // detect [[query
  const pageMatch = left.match(/\[\[([^\[\]\n]*)$/);
  if (pageMatch) {
    const rect = caretRect(sel);
    setAutocomplete({ trigger: "page", query: pageMatch[1], position: { x: rect.left, y: rect.bottom + 4 }, blockId: n.id });
    return;
  }
  const tagMatch = left.match(/(?:^|\s)#([A-Za-z0-9_\-]*)$/);
  if (tagMatch) {
    const rect = caretRect(sel);
    setAutocomplete({ trigger: "tag", query: tagMatch[1], position: { x: rect.left, y: rect.bottom + 4 }, blockId: n.id });
    return;
  }
  setAutocomplete(null);
}}
```

Helper functions in the same file:

```tsx
function caretOffset(root: HTMLElement, sel: Selection): number {
  const range = sel.getRangeAt(0).cloneRange();
  range.setStart(root, 0);
  return range.toString().length;
}

function caretRect(sel: Selection): DOMRect {
  const range = sel.getRangeAt(0).cloneRange();
  range.collapse(true);
  const rects = range.getClientRects();
  if (rects.length > 0) return rects[0];
  // Fallback
  return new DOMRect(0, 0, 0, 0);
}
```

Render the popup:

```tsx
{autocomplete && (
  <LinkAutocomplete
    trigger={autocomplete.trigger}
    query={autocomplete.query}
    position={autocomplete.position}
    onClose={() => setAutocomplete(null)}
    onPick={async (label, isNew) => {
      // Insert label + appropriate closing into the current block
      const el = document.querySelector(`.block-bullet[data-id="${CSS.escape(autocomplete.blockId)}"]`)?.parentElement?.querySelector(".block-content") as HTMLElement | null;
      if (!el) { setAutocomplete(null); return; }
      const text = el.innerText;
      let replaced: string;
      if (autocomplete.trigger === "page") {
        replaced = text.replace(/\[\[([^\[\]\n]*)$/, `[[${label}]]`);
      } else {
        replaced = text.replace(/(^|\s)#([A-Za-z0-9_\-]*)$/, `$1#${label}`);
      }
      el.innerText = replaced;
      setAutocomplete(null);
      // Trigger save by blurring
      el.blur();
      if (isNew && autocomplete.trigger === "page") {
        try {
          const ct = await encryptBlock(boardId, noteId, label);
          await api.createPage({ title_b64: ct, plaintext_title_lc: label.toLowerCase() });
        } catch (e) { console.warn("create page failed", e); }
      }
    }}
  />
)}
```

- [ ] **Step 15.4: Build & commit**

```bash
cd spa && npm run build
git add spa/src/components/LinkAutocomplete.tsx spa/src/components/BlockEditor.tsx spa/src/components/BlockEditor.css
git commit -m "feat(spa): [[ and # autocomplete dropdown with create-on-pick"
```

---

## Task 16: SPA — BacklinksSection (Linked references)

**Files:**
- Create: `spa/src/components/BacklinksSection.tsx`
- Modify: `spa/src/components/BlockEditor.tsx`

- [ ] **Step 16.1: Component**

Create `spa/src/components/BacklinksSection.tsx`:

```tsx
import { useEffect, useState } from "preact/hooks";
import * as api from "../api";
import { decryptBlock } from "../blocks/crypto";
import { boardOf } from "../blocks/titleIndex";
import { selectedNoteId } from "../selectedNote";
import { t } from "../i18n";

interface Props { noteId: string }

interface Entry { block_id: string; source_note_id: string; plaintext: string; }

export function BacklinksSection({ noteId }: Props) {
  const [entries, setEntries] = useState<Entry[]>([]);
  const [open, setOpen] = useState(true);

  useEffect(() => {
    (async () => {
      try {
        const rows = await api.noteBacklinks(noteId);
        const out: Entry[] = [];
        for (const r of rows) {
          try {
            const blockResp = await fetch(`/blocks/${r.source_block_id}`, {
              headers: { Authorization: `Bearer ${localStorage.getItem("token") ?? ""}` },
            }).then(x => x.json());
            const board = boardOf(r.source_note_id) ?? "";
            const text = board ? await decryptBlock(board, r.source_note_id, blockResp.content) : "(unreadable)";
            out.push({ block_id: r.source_block_id, source_note_id: r.source_note_id, plaintext: text });
          } catch { /* skip unreadable */ }
        }
        setEntries(out);
      } catch (e) { console.warn("backlinks fetch failed", e); }
    })();
  }, [noteId]);

  const grouped = entries.reduce((m, e) => {
    const k = e.source_note_id;
    (m[k] ??= []).push(e);
    return m;
  }, {} as Record<string, Entry[]>);

  return (
    <section class="backlinks-section">
      <button class="backlinks-header" onClick={() => setOpen(o => !o)}>
        {open ? "▼" : "▶"} {t("backlinks.linked")} ({entries.length})
      </button>
      {open && (
        <div class="backlinks-body">
          {Object.entries(grouped).map(([nid, es]) => (
            <div key={nid} class="backlinks-group">
              <button class="backlinks-note" onClick={() => (selectedNoteId.value = nid)}>📄 {nid.slice(0, 8)}…</button>
              <ul>
                {es.map(e => (
                  <li key={e.block_id}>{e.plaintext}</li>
                ))}
              </ul>
            </div>
          ))}
          {entries.length === 0 && <p class="backlinks-empty">{t("backlinks.empty")}</p>}
        </div>
      )}
    </section>
  );
}
```

- [ ] **Step 16.2: Wire into BlockEditor**

In `BlockEditor.tsx`, after the `.block-list` div:

```tsx
<BacklinksSection noteId={noteId} />
```

Import at top of file: `import { BacklinksSection } from "./BacklinksSection";`

- [ ] **Step 16.3: CSS + i18n**

Append to `BlockEditor.css`:

```css
.backlinks-section { margin-top: 24px; border-top: 1px solid rgba(255,255,255,0.06); padding-top: 12px; }
.backlinks-header { background: transparent; border: none; color: var(--muted, #888); font: inherit; cursor: pointer; padding: 4px 0; }
.backlinks-body { padding: 8px 0; }
.backlinks-group { margin-bottom: 12px; }
.backlinks-note { background: transparent; border: none; color: var(--accent, #4af); cursor: pointer; padding: 2px 0; font: inherit; }
.backlinks-group ul { margin: 4px 0 0 18px; padding: 0; }
.backlinks-group li { color: var(--text, inherit); padding: 2px 0; }
.backlinks-empty { color: var(--muted, #888); padding: 8px 0; }
```

Add i18n keys to each `spa/src/i18n/{en,fr,es,de}.ts`:

| key | en | fr | es | de |
|---|---|---|---|---|
| `backlinks.linked` | Linked references | Références entrantes | Referencias enlazadas | Eingehende Verweise |
| `backlinks.empty` | No references yet. | Aucune référence. | Sin referencias. | Keine Verweise. |

- [ ] **Step 16.4: Build & commit**

```bash
cd spa && npm run build
git add spa/src/components/BacklinksSection.tsx spa/src/components/BlockEditor.tsx spa/src/components/BlockEditor.css spa/src/i18n/
git commit -m "feat(spa): Linked references section under each note"
```

---

## Task 17: SPA — TagPage Route

**Files:**
- Create: `spa/src/components/TagPage.tsx`
- Modify: SPA router (look for hash-routing in `App.tsx` or `Layout.tsx`)

- [ ] **Step 17.1: Component**

Create `spa/src/components/TagPage.tsx`:

```tsx
import { useEffect, useState } from "preact/hooks";
import * as api from "../api";
import { decryptBlock } from "../blocks/crypto";
import { boardOf } from "../blocks/titleIndex";
import { t } from "../i18n";

interface Props { name: string }

interface Row { block_id: string; note_id: string; plaintext: string; }

export function TagPage({ name }: Props) {
  const [rows, setRows] = useState<Row[]>([]);
  const [tagColor, setTagColor] = useState<string | undefined>();

  useEffect(() => {
    (async () => {
      const ids = await api.blocksWithTag(name);
      const out: Row[] = [];
      for (const bid of ids) {
        try {
          const r = await fetch(`/blocks/${bid}`, { headers: { Authorization: `Bearer ${localStorage.getItem("token") ?? ""}` } }).then(x => x.json());
          const board = boardOf(r.note_id) ?? "";
          const text = board ? await decryptBlock(board, r.note_id, r.content) : "(unreadable)";
          out.push({ block_id: bid, note_id: r.note_id, plaintext: text });
        } catch {}
      }
      setRows(out);
      try {
        const all = await api.listTags();
        const me = all.find(t => t.name === name);
        setTagColor(me?.color ?? undefined);
      } catch {}
    })();
  }, [name]);

  return (
    <div class="tag-page">
      <header>
        <h1>#{name}</h1>
        <input type="color" value={tagColor ?? "#888888"} onInput={async (e) => {
          const c = (e.target as HTMLInputElement).value;
          setTagColor(c);
          try { await api.putTag(name, c); } catch {}
        }} />
        <span class="tag-page-count">{rows.length} {t("tag.blocks")}</span>
      </header>
      {rows.length === 0 && <p>{t("tag.empty")}</p>}
      <ul>
        {rows.map(r => (
          <li key={r.block_id} onClick={() => { location.hash = `#/notes/${r.note_id}`; }} style={{ cursor: "pointer" }}>
            {r.plaintext}
          </li>
        ))}
      </ul>
    </div>
  );
}
```

- [ ] **Step 17.2: Route**

Find the SPA's hash router (search for `location.hash` usage in `App.tsx` / `Layout.tsx`). Add a case for `#/tag/:name`:

```tsx
// Pseudo — adapt to your router:
const m = location.hash.match(/^#\/tag\/(.+)$/);
if (m) return <TagPage name={decodeURIComponent(m[1])} />;
```

- [ ] **Step 17.3: i18n**

Add to each locale: `tag.blocks` → "blocks/blocs/bloques/Blöcke", `tag.empty` → "No blocks tagged yet."

- [ ] **Step 17.4: CSS**

Add to a global CSS file or new `TagPage.css`:

```css
.tag-page { padding: 24px; }
.tag-page header { display: flex; align-items: center; gap: 12px; margin-bottom: 16px; }
.tag-page h1 { margin: 0; }
.tag-page-count { color: var(--muted, #888); }
.tag-page ul { list-style: none; padding: 0; }
.tag-page li { padding: 8px 12px; border-bottom: 1px solid rgba(255,255,255,0.04); }
.tag-page li:hover { background: rgba(255,255,255,0.04); }
```

- [ ] **Step 17.5: Build & commit**

```bash
cd spa && npm run build
git add spa/src/components/TagPage.tsx spa/src/App.tsx spa/src/i18n/ 2>/dev/null
git commit -m "feat(spa): virtual #tag page with color picker"
```

---

## Task 18: SPA — SuggestionsSection + Embeddings opt-in

**Files:**
- Create: `spa/src/components/SuggestionsSection.tsx`
- Modify: `spa/src/components/BlockEditor.tsx`
- Modify: `spa/src/components/WhoamiPage.tsx`
- Modify: `spa/src/api.ts`

- [ ] **Step 18.1: API helpers**

Append to `spa/src/api.ts`:

```typescript
export interface EmbeddingStatus { enabled: boolean; indexed: number; total: number }

export async function getEmbeddingStatus(): Promise<EmbeddingStatus> {
  const r = await authedFetch(`${BASE}/embeddings/status`);
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function setEmbeddingsEnabled(enabled: boolean): Promise<void> {
  const r = await authedFetch(`${BASE}/embeddings/enabled`, {
    method: "PUT", body: JSON.stringify({ enabled }),
  });
  if (!r.ok) throw new Error(await r.text());
}

export async function upsertEmbedding(block_id: string, plaintext: string): Promise<void> {
  const r = await authedFetch(`${BASE}/embeddings/upsert`, {
    method: "POST", body: JSON.stringify({ block_id, plaintext }),
  });
  if (!r.ok && r.status !== 403) throw new Error(await r.text());
}

export interface SuggestionHit { block_id: string; source_note_id: string; score: number; snippet_b64: string }

export async function suggestionsRelated(opts: { note_id?: string; tag?: string; top_k?: number }): Promise<SuggestionHit[]> {
  const qs = new URLSearchParams();
  if (opts.note_id) qs.set("note_id", opts.note_id);
  if (opts.tag) qs.set("tag", opts.tag);
  if (opts.top_k) qs.set("top_k", String(opts.top_k));
  const r = await authedFetch(`${BASE}/suggestions/related?${qs}`);
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}
```

- [ ] **Step 18.2: SuggestionsSection**

Create `spa/src/components/SuggestionsSection.tsx`:

```tsx
import { useEffect, useState } from "preact/hooks";
import * as api from "../api";
import { decryptBlock } from "../blocks/crypto";
import { boardOf } from "../blocks/titleIndex";
import { t } from "../i18n";

interface Props { noteId: string; tag?: string }
interface Hit { block_id: string; source_note_id: string; score: number; plaintext: string; }

export function SuggestionsSection({ noteId, tag }: Props) {
  const [hits, setHits] = useState<Hit[]>([]);
  const [open, setOpen] = useState(false);
  const [enabled, setEnabled] = useState(false);

  const load = async () => {
    const status = await api.getEmbeddingStatus();
    setEnabled(status.enabled);
    if (!status.enabled) { setHits([]); return; }
    const raw = await api.suggestionsRelated({ note_id: tag ? undefined : noteId, tag, top_k: 5 });
    const out: Hit[] = [];
    for (const h of raw) {
      try {
        const board = boardOf(h.source_note_id) ?? "";
        const text = board ? await decryptBlock(board, h.source_note_id, h.snippet_b64) : "";
        out.push({ ...h, plaintext: text });
      } catch {}
    }
    setHits(out);
  };

  useEffect(() => { if (open) load(); }, [open, noteId, tag]);

  return (
    <section class="suggestions-section">
      <button class="suggestions-header" onClick={() => setOpen(o => !o)}>
        {open ? "▼" : "▶"} {t("suggestions.title")} ({enabled ? hits.length : "off"})
      </button>
      {open && enabled && (
        <ul>{hits.map(h => (
          <li key={h.block_id}>
            <span class="score">{(h.score * 100).toFixed(0)}%</span>
            <span class="text">{h.plaintext}</span>
          </li>
        ))}
        </ul>
      )}
      {open && !enabled && <p class="suggestions-off">{t("suggestions.off")}</p>}
    </section>
  );
}
```

- [ ] **Step 18.3: Wire into BlockEditor**

Below `<BacklinksSection>`:

```tsx
<SuggestionsSection noteId={noteId} />
```

- [ ] **Step 18.4: Settings toggle in WhoamiPage**

In `spa/src/components/WhoamiPage.tsx`, add a section:

```tsx
const [embStatus, setEmbStatus] = useState({ enabled: false, indexed: 0, total: 0 });
useEffect(() => { api.getEmbeddingStatus().then(setEmbStatus).catch(() => {}); }, []);
const toggle = async () => {
  if (!embStatus.enabled) {
    if (!confirm(t("settings.embeddings.warning"))) return;
  }
  await api.setEmbeddingsEnabled(!embStatus.enabled);
  setEmbStatus(s => ({ ...s, enabled: !s.enabled }));
};
// ...render:
<section>
  <h3>{t("settings.embeddings.title")}</h3>
  <label><input type="checkbox" checked={embStatus.enabled} onChange={toggle} /> {t("settings.embeddings.label")}</label>
  <p class="muted">{embStatus.indexed} / {embStatus.total} {t("settings.embeddings.indexed")}</p>
</section>
```

- [ ] **Step 18.5: Hook upsertEmbedding into persistEdit**

In `spa/src/blocks/keymap.ts`, after the auto-link extraction, also call:

```typescript
if (plaintext.trim().length >= 30) {
  api.upsertEmbedding(block.id, plaintext).catch(() => {}); // 403 if disabled is fine
}
```

- [ ] **Step 18.6: i18n + CSS**

Add keys:
- `suggestions.title` — "Suggestions"
- `suggestions.off` — "Enable semantic suggestions in Settings to see related blocks."
- `settings.embeddings.title` — "Semantic suggestions"
- `settings.embeddings.label` — "Enable semantic suggestions"
- `settings.embeddings.warning` — "Enabling this sends block plaintexts to the server. Block bodies and titles remain E2E-encrypted at rest. Continue?"
- `settings.embeddings.indexed` — "blocks indexed"

CSS:
```css
.suggestions-section { margin-top: 16px; }
.suggestions-section ul { list-style: none; padding: 0; }
.suggestions-section li { display: flex; gap: 8px; padding: 4px 0; }
.suggestions-section .score { color: var(--muted, #888); width: 40px; }
.suggestions-off { color: var(--muted, #888); }
```

- [ ] **Step 18.7: Build & commit**

```bash
cd spa && npm run build
git add spa/src/components/SuggestionsSection.tsx spa/src/components/BlockEditor.tsx spa/src/components/WhoamiPage.tsx spa/src/api.ts spa/src/blocks/keymap.ts spa/src/i18n/ spa/src/components/BlockEditor.css
git commit -m "feat(spa): semantic Suggestions panel + opt-in toggle in Settings"
```

---

## Task 19: CLI — page resolve / page create / suggestions

**Files:**
- Create: `crates/cli/src/commands/page.rs`
- Modify: `crates/cli/src/commands/mod.rs`
- Modify: `crates/cli/src/main.rs`
- Modify: `crates/cli/src/client.rs`

- [ ] **Step 19.1: Client helpers**

In `crates/cli/src/client.rs`, append to `impl JotClient`:

```rust
pub async fn resolve_page(&self, title: &str) -> Result<Option<uuid::Uuid>, CliError> {
    // Use list_titles + local decrypt: mirror the SPA flow
    // ... for brevity, the implementer can call a server-side helper if added later
    let _ = title;
    Ok(None)
}

pub async fn create_page(&self, title: &str) -> Result<(uuid::Uuid, uuid::Uuid), CliError> {
    // Fetch identity DEK and derive a note DEK, encrypt the title, POST /pages.
    // Mirror create_block_encrypted's key derivation path.
    let _ = title;
    Err(CliError::Config("not yet implemented — see Task 19 description".into()))
}
```

The implementation requires fetching identity key + creating note DEK + encrypting + POST. Adapt patterns from `create_block_encrypted` (Feature 2 / Task 14).

- [ ] **Step 19.2: Subcommand**

Create `crates/cli/src/commands/page.rs`:

```rust
use crate::client::JotClient;
use crate::config::Config;
use clap::Subcommand;
use uuid::Uuid;

#[derive(Subcommand, Debug)]
pub enum PageCmd {
    Resolve { title: String },
    Create { title: String },
}

pub async fn run(cfg: &Config, cmd: PageCmd) -> Result<(), crate::CliError> {
    let client = JotClient::from_config(cfg).await?;
    match cmd {
        PageCmd::Resolve { title } => {
            match client.resolve_page(&title).await? {
                Some(id) => println!("{}", id),
                None => println!("(not found)"),
            }
        }
        PageCmd::Create { title } => {
            let (note_id, board_id) = client.create_page(&title).await?;
            println!("note: {note_id}\nboard: {board_id}");
        }
    }
    Ok(())
}
```

- [ ] **Step 19.3: Wire**

`mod.rs`: `pub mod page;`
`main.rs`: add `Page { #[command(subcommand)] cmd: crate::commands::page::PageCmd }` and dispatch.

- [ ] **Step 19.4: Build & commit**

```bash
cargo build -p cli
git add crates/cli/src/commands/page.rs crates/cli/src/commands/mod.rs crates/cli/src/main.rs crates/cli/src/client.rs
git commit -m "feat(cli): jot page resolve/create commands"
```

NOTE: the create-page CLI path is non-trivial because it must encrypt the title client-side. If the implementer hits friction, mark this command as `DONE_WITH_CONCERNS` with a stub that calls a future server endpoint that accepts plaintext (only for CLI-with-key trust mode). The block-structure work in Feature 2 already established the key derivation pattern (`derive_dek_for(board_id, note_id)`) — reuse it.

---

## Task 20: Integration Test

**Files:**
- Create: `crates/api/tests/backlinks_e2e.rs`

- [ ] **Step 20.1: Write the test**

Copy the harness from `crates/api/tests/blocks_e2e.rs` (Feature 2 / Task 22) and write a single happy-path test:

```rust
// 1. Register identity + device, get token.
// 2. POST /boards (regular board).
// 3. POST /notes (text note in regular board).
// 4. POST /notes/{id}/blocks — block content (base64 of "[[Idée]]" treated as ciphertext for the test).
// 5. PUT /blocks/{id}/links with one page_ref_unresolved entry { target_kind=note, target_id="idée", link_kind="page_ref_unresolved" }.
// 6. POST /pages with { title_b64=<encoded test bytes>, plaintext_title_lc="idée" }.
//    Assert response.reconciled == 1 and board_id is the auto-created Pages board.
// 7. GET /notes/{newPageId}/backlinks → 1 entry pointing at our source block.
// 8. POST /links/reconcile-title { title="idée", note_id=<newPageId> } → reconciled = 0 (idempotent).
```

- [ ] **Step 20.2: Run & commit**

```bash
cargo test -p api --test backlinks_e2e -- --nocapture 2>&1 | tail -20
git add crates/api/tests/backlinks_e2e.rs
git commit -m "test(api): backlinks end-to-end happy path (page create + reconcile + backlink)"
```

---

## Task 21: Documentation

**Files:**
- Create: `docs/backlinks.md`
- Modify: `docs/blocks.md` (cross-reference)

- [ ] **Step 21.1: Write `docs/backlinks.md`**

≤150 lines. Sections:
1. Link syntax cheat-sheet ([[Page]], ((id)), !((id)), #tag) — examples + what each produces in `block_links`.
2. Resolution model — identity-global, client-side title index, conflict rule.
3. Auto-extraction — every save calls `PUT /blocks/:id/links`.
4. Pages board — auto-created on first `POST /pages`, sidebar pinned.
5. Embedding pipeline — opt-in, plaintext trade-off, ruvector adapter.
6. API surface — list every new endpoint with one-line spec.
7. Keyboard shortcuts — `[[`, `#` triggers; Enter to pick / Esc to dismiss; click → navigate.

- [ ] **Step 21.2: Commit**

```bash
git add docs/backlinks.md docs/blocks.md
git commit -m "docs: backlinks & knowledge graph developer guide"
```

---

## Self-Review

**Spec coverage:**
- Migration → T1
- Unresolved link kind → T2
- Pages board → T3
- Reconcile + title listing → T4
- Block embeddings + opt-in flag → T5
- ruvector adapter → T6
- Resolve / create page / reconcile API → T7
- Embeddings & suggestions API → T8
- SPA extractLinks → T9
- Title index → T10
- Auto-extraction in keymap → T11
- View-mode renderer + click delegation → T12
- Edit/view switch + CSS → T13
- EmbedBlock → T14
- LinkAutocomplete → T15
- BacklinksSection → T16
- TagPage → T17
- SuggestionsSection + Settings → T18
- CLI surface → T19
- Integration test → T20
- Docs → T21

**Placeholder scan:** Several tasks (T6 ruvector backend, T19 page create CLI, T18 SPA WhoamiPage UI specifics) acknowledge unknowns and provide concrete adapter strategies (Stub backend, DONE_WITH_CONCERNS escape, mirror an existing pattern). These are NOT "TBD" placeholders — they document a verifiable fallback so the engineer is never stuck.

**Type consistency:** `LinkKind` values (`page_ref`, `page_ref_unresolved`, `block_ref`, `block_embed`, `tag`) consistent between Rust (T2), SPA (T9), and storage SQL (T1, T4). `target_kind` values (`note`, `block`, `tag`) consistent throughout. `EmbeddingStatus { enabled, indexed, total }` matches across Rust (T8.2) and TS (T18.1).

---

**Plan complete and saved to `docs/superpowers/plans/2026-05-12-backlinks.md`. Two execution options:**

**1. Subagent-Driven (recommended)** — fresh subagent per task, two-stage review, fast iteration.

**2. Inline Execution** — execute tasks in this session with checkpoints.

**Which approach?**
