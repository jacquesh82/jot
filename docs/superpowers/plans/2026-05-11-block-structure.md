# Block Structure (Outliner) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the atomic `notes.content` blob with a first-class `blocks` table (hierarchical, globally addressable) plus a `block_links` edge table, while preserving the local-first / single-binary / E2E-encrypted model. Full functional parity across API, CLI, SPA, TUI.

**Architecture:** Additive SQLite migration introduces `blocks`, `block_links`, `tags` and four new columns on `notes`. Blocks reuse the parent note's DEK for encryption (no new key management). Edges (`target_kind`/`target_id`) are plaintext UUIDs to enable instant backlinks. Migration of legacy notes is lazy and client-side (E2E forbids server-side migration). Real-time CRDT collaboration is out of scope for this plan — the schema is prepared but Yjs integration is deferred.

**Tech Stack:** Rust (axum API, sqlx, ratatui TUI, clap CLI), TypeScript + Preact-signals SPA, SQLite with AES-256-GCM at the application layer.

**Spec:** `docs/superpowers/specs/2026-05-11-block-structure-design.md`

---

## File Structure

### Created
- `crates/storage/migrations/0008_blocks.sql` — schema migration
- `crates/core/src/models/block.rs` — `Block`, `BlockType`, `BlockLink`, `LinkKind`, `TargetKind`, `Tag`
- `crates/core/src/blocks/mod.rs` — markdown→blocks splitter and link extractor
- `crates/core/src/blocks/split.rs`
- `crates/core/src/blocks/links.rs`
- `crates/storage/src/db/blocks.rs` — CRUD + move + indent/outdent + tree fetch
- `crates/storage/src/db/block_links.rs` — edge upsert + backlinks queries
- `crates/storage/src/db/tags.rs`
- `crates/api/src/routes/blocks.rs`
- `crates/api/src/routes/tags.rs`
- `crates/cli/src/commands/block.rs` — `add|list|show|edit|move|indent|outdent|delete|ref|backlinks|migrate`
- `spa/src/components/BlockEditor.tsx`
- `spa/src/components/BlockEditor.css`
- `spa/src/blocks/tree.ts` — tree manipulation helpers
- `spa/src/blocks/markdown.ts` — markdown<->blocks (used for legacy migration)
- `spa/src/blocks/keymap.ts` — keyboard handlers
- `crates/cli/src/tui/blocks.rs` — block tree rendering in TUI

### Modified
- `crates/core/src/models/mod.rs` — export block model
- `crates/core/src/models/note.rs` — add `title`, `is_journal`, `journal_date`, `schema_version`
- `crates/storage/src/db/mod.rs` — register new submodules
- `crates/storage/src/db/notes.rs` — column-aware insert/update
- `crates/api/src/routes/mod.rs` — register blocks/tags routers
- `crates/api/src/state.rs` — extend `WsEvent` variants
- `crates/api/src/routes/ws.rs` — broadcast block events
- `crates/cli/src/main.rs` — clap subcommand `Block`
- `crates/cli/src/commands/mod.rs` — `pub mod block;`
- `crates/cli/src/client.rs` — block API client methods
- `crates/cli/src/tui/app.rs` — block panel state
- `crates/cli/src/tui/ui.rs` — wire block view
- `crates/cli/src/tui/mod.rs` — handle block keys
- `spa/src/api.ts` — block/tag client methods
- `spa/src/components/NoteEditor.tsx` — delegate to BlockEditor for schema v1 text notes
- `spa/src/components/NoteList.tsx` — open notes in block mode
- `spa/src/i18n/*.json` — strings for new UI (en/fr/es/de)

---

## Task 1: Database Migration

**Files:**
- Create: `crates/storage/migrations/0008_blocks.sql`
- Test: `crates/storage/src/db/mod.rs` (existing migration test)

- [ ] **Step 1.1: Write the migration SQL**

Create `crates/storage/migrations/0008_blocks.sql`:

```sql
CREATE TABLE IF NOT EXISTS blocks (
    id              TEXT PRIMARY KEY,
    note_id         TEXT NOT NULL,
    parent_block_id TEXT,
    position        REAL NOT NULL,
    block_type      TEXT NOT NULL DEFAULT 'text',
    content         BLOB NOT NULL,
    metadata        BLOB,
    collapsed       INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    FOREIGN KEY (note_id)         REFERENCES notes(id)  ON DELETE CASCADE,
    FOREIGN KEY (parent_block_id) REFERENCES blocks(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_blocks_note          ON blocks(note_id);
CREATE INDEX IF NOT EXISTS idx_blocks_parent_pos    ON blocks(parent_block_id, position);
CREATE INDEX IF NOT EXISTS idx_blocks_note_parent   ON blocks(note_id, parent_block_id);

CREATE TABLE IF NOT EXISTS block_links (
    id              TEXT PRIMARY KEY,
    source_block_id TEXT NOT NULL,
    target_kind     TEXT NOT NULL,
    target_id       TEXT NOT NULL,
    link_kind       TEXT NOT NULL,
    created_at      TEXT NOT NULL,
    FOREIGN KEY (source_block_id) REFERENCES blocks(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_block_links_target ON block_links(target_kind, target_id);
CREATE INDEX IF NOT EXISTS idx_block_links_source ON block_links(source_block_id);

CREATE TABLE IF NOT EXISTS tags (
    name        TEXT NOT NULL,
    identity_id TEXT NOT NULL,
    color       TEXT,
    created_at  TEXT NOT NULL,
    PRIMARY KEY (name, identity_id)
);

ALTER TABLE notes ADD COLUMN title          BLOB;
ALTER TABLE notes ADD COLUMN is_journal     INTEGER NOT NULL DEFAULT 0;
ALTER TABLE notes ADD COLUMN journal_date   TEXT;
ALTER TABLE notes ADD COLUMN schema_version INTEGER NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_notes_journal_date ON notes(journal_date) WHERE journal_date IS NOT NULL;
```

- [ ] **Step 1.2: Run migrations and verify schema**

Run: `cargo test -p storage migration --lib -- --nocapture`
Expected: all existing migration tests still pass and the new tables exist.

- [ ] **Step 1.3: Commit**

```bash
git add crates/storage/migrations/0008_blocks.sql
git commit -m "feat(storage): add blocks, block_links, tags schema"
```

---

## Task 2: Core Block Model

**Files:**
- Create: `crates/core/src/models/block.rs`
- Modify: `crates/core/src/models/mod.rs`
- Modify: `crates/core/src/models/note.rs`

- [ ] **Step 2.1: Write the failing test**

Append to `crates/core/src/models/block.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BlockType { Text, Heading, Todo, Quote, Code, Embed, Divider }

impl BlockType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BlockType::Text => "text",
            BlockType::Heading => "heading",
            BlockType::Todo => "todo",
            BlockType::Quote => "quote",
            BlockType::Code => "code",
            BlockType::Embed => "embed",
            BlockType::Divider => "divider",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "heading" => BlockType::Heading,
            "todo" => BlockType::Todo,
            "quote" => BlockType::Quote,
            "code" => BlockType::Code,
            "embed" => BlockType::Embed,
            "divider" => BlockType::Divider,
            _ => BlockType::Text,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub id: Uuid,
    pub note_id: Uuid,
    pub parent_block_id: Option<Uuid>,
    pub position: f64,
    pub block_type: BlockType,
    pub content: Vec<u8>,
    pub metadata: Option<Vec<u8>>,
    pub collapsed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind { Note, Block, Tag }

impl TargetKind {
    pub fn as_str(&self) -> &'static str {
        match self { TargetKind::Note => "note", TargetKind::Block => "block", TargetKind::Tag => "tag" }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s { "note" => Some(Self::Note), "block" => Some(Self::Block), "tag" => Some(Self::Tag), _ => None }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkKind { PageRef, BlockRef, BlockEmbed, Tag }

impl LinkKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            LinkKind::PageRef => "page_ref",
            LinkKind::BlockRef => "block_ref",
            LinkKind::BlockEmbed => "block_embed",
            LinkKind::Tag => "tag",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "page_ref" => Some(Self::PageRef),
            "block_ref" => Some(Self::BlockRef),
            "block_embed" => Some(Self::BlockEmbed),
            "tag" => Some(Self::Tag),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockLink {
    pub id: Uuid,
    pub source_block_id: Uuid,
    pub target_kind: TargetKind,
    pub target_id: String,
    pub link_kind: LinkKind,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    pub identity_id: Uuid,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn block_type_round_trip() {
        for s in ["text","heading","todo","quote","code","embed","divider"] {
            assert_eq!(BlockType::from_str(s).as_str(), s);
        }
    }
    #[test]
    fn link_kind_round_trip() {
        for s in ["page_ref","block_ref","block_embed","tag"] {
            assert_eq!(LinkKind::from_str(s).unwrap().as_str(), s);
        }
    }
    #[test]
    fn unknown_block_type_defaults_to_text() {
        assert!(matches!(BlockType::from_str("nonsense"), BlockType::Text));
    }
}
```

- [ ] **Step 2.2: Wire into models module**

Edit `crates/core/src/models/mod.rs`, add:

```rust
pub mod block;
pub use block::{Block, BlockType, BlockLink, LinkKind, TargetKind, Tag};
```

- [ ] **Step 2.3: Add new fields to Note**

Locate the `Note` struct in `crates/core/src/models/note.rs` and add these fields at the end (preserving existing ones):

```rust
    #[serde(default)]
    pub title: Option<Vec<u8>>,
    #[serde(default)]
    pub is_journal: bool,
    #[serde(default)]
    pub journal_date: Option<String>,
    #[serde(default)]
    pub schema_version: i32,
```

- [ ] **Step 2.4: Run tests**

Run: `cargo test -p jot-core models::block`
Expected: 3 tests PASS.

- [ ] **Step 2.5: Compile workspace to find broken Note constructors**

Run: `cargo build --workspace`
Expected: compile errors point to every place `Note { ... }` is built without the new fields.

- [ ] **Step 2.6: Fix every `Note { ... }` literal**

In each call site flagged by the compiler, add:

```rust
    title: None,
    is_journal: false,
    journal_date: None,
    schema_version: 0,
```

- [ ] **Step 2.7: Rebuild**

Run: `cargo build --workspace`
Expected: clean build.

- [ ] **Step 2.8: Commit**

```bash
git add crates/core/src/models/ crates/storage/ crates/api/ crates/cli/
git commit -m "feat(core): add Block, BlockLink, Tag models and extend Note"
```

---

## Task 3: Storage — Blocks CRUD

**Files:**
- Create: `crates/storage/src/db/blocks.rs`
- Modify: `crates/storage/src/db/mod.rs`

- [ ] **Step 3.1: Register submodule**

In `crates/storage/src/db/mod.rs` add `pub mod blocks;` next to the other submodules.

- [ ] **Step 3.2: Write failing tests first**

Create `crates/storage/src/db/blocks.rs`:

```rust
use crate::db::Db;
use crate::StorageError;
use chrono::Utc;
use jot_core::models::{Block, BlockType};
use sqlx::Row;
use uuid::Uuid;

fn row_to_block(row: &sqlx::sqlite::SqliteRow) -> Block {
    let id: String = row.get("id");
    let note_id: String = row.get("note_id");
    let parent: Option<String> = row.get("parent_block_id");
    let bt: String = row.get("block_type");
    let created: String = row.get("created_at");
    let updated: String = row.get("updated_at");
    let collapsed: i64 = row.get("collapsed");
    Block {
        id: Uuid::parse_str(&id).unwrap(),
        note_id: Uuid::parse_str(&note_id).unwrap(),
        parent_block_id: parent.and_then(|s| Uuid::parse_str(&s).ok()),
        position: row.get("position"),
        block_type: BlockType::from_str(&bt),
        content: row.get("content"),
        metadata: row.get("metadata"),
        collapsed: collapsed != 0,
        created_at: chrono::DateTime::parse_from_rfc3339(&created).unwrap().with_timezone(&Utc),
        updated_at: chrono::DateTime::parse_from_rfc3339(&updated).unwrap().with_timezone(&Utc),
    }
}

impl Db {
    pub async fn insert_block(&self, b: &Block) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO blocks (id, note_id, parent_block_id, position, block_type, content, metadata, collapsed, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(b.id.to_string())
        .bind(b.note_id.to_string())
        .bind(b.parent_block_id.map(|p| p.to_string()))
        .bind(b.position)
        .bind(b.block_type.as_str())
        .bind(&b.content)
        .bind(b.metadata.as_deref())
        .bind(if b.collapsed { 1i64 } else { 0i64 })
        .bind(b.created_at.to_rfc3339())
        .bind(b.updated_at.to_rfc3339())
        .execute(&self.0).await?;
        Ok(())
    }

    pub async fn get_block(&self, id: Uuid) -> Result<Option<Block>, StorageError> {
        let row = sqlx::query("SELECT * FROM blocks WHERE id = ?")
            .bind(id.to_string()).fetch_optional(&self.0).await?;
        Ok(row.map(|r| row_to_block(&r)))
    }

    pub async fn list_blocks_for_note(&self, note_id: Uuid) -> Result<Vec<Block>, StorageError> {
        let rows = sqlx::query(
            "SELECT * FROM blocks WHERE note_id = ? ORDER BY COALESCE(parent_block_id,''), position ASC"
        ).bind(note_id.to_string()).fetch_all(&self.0).await?;
        Ok(rows.iter().map(row_to_block).collect())
    }

    pub async fn update_block_content(&self, id: Uuid, content: &[u8], metadata: Option<&[u8]>, block_type: BlockType) -> Result<(), StorageError> {
        sqlx::query("UPDATE blocks SET content = ?, metadata = ?, block_type = ?, updated_at = ? WHERE id = ?")
            .bind(content).bind(metadata).bind(block_type.as_str())
            .bind(Utc::now().to_rfc3339()).bind(id.to_string())
            .execute(&self.0).await?;
        Ok(())
    }

    pub async fn move_block(&self, id: Uuid, new_parent: Option<Uuid>, new_position: f64) -> Result<(), StorageError> {
        sqlx::query("UPDATE blocks SET parent_block_id = ?, position = ?, updated_at = ? WHERE id = ?")
            .bind(new_parent.map(|p| p.to_string())).bind(new_position)
            .bind(Utc::now().to_rfc3339()).bind(id.to_string())
            .execute(&self.0).await?;
        Ok(())
    }

    pub async fn set_block_collapsed(&self, id: Uuid, collapsed: bool) -> Result<(), StorageError> {
        sqlx::query("UPDATE blocks SET collapsed = ? WHERE id = ?")
            .bind(if collapsed { 1i64 } else { 0i64 }).bind(id.to_string())
            .execute(&self.0).await?;
        Ok(())
    }

    pub async fn delete_block(&self, id: Uuid) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM blocks WHERE id = ?").bind(id.to_string())
            .execute(&self.0).await?;
        Ok(())
    }

    /// Returns the highest position among siblings of `parent` in `note`, or 0.0 if none exist.
    pub async fn max_position(&self, note_id: Uuid, parent: Option<Uuid>) -> Result<f64, StorageError> {
        let row = sqlx::query(
            "SELECT COALESCE(MAX(position), 0.0) AS m FROM blocks WHERE note_id = ? AND parent_block_id IS ?"
        )
        .bind(note_id.to_string())
        .bind(parent.map(|p| p.to_string()))
        .fetch_one(&self.0).await?;
        Ok(row.get::<f64, _>("m"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_db;
    use jot_core::models::BlockType;

    async fn seed_note(db: &Db) -> (Uuid, Uuid) {
        let board = Uuid::new_v4();
        let note = Uuid::new_v4();
        sqlx::query("INSERT INTO boards (id, identity_id, name, position, created_at) VALUES (?,?,?,?,?)")
            .bind(board.to_string()).bind(Uuid::new_v4().to_string()).bind("b").bind(0i32).bind(Utc::now().to_rfc3339())
            .execute(&db.0).await.unwrap();
        sqlx::query("INSERT INTO notes (id, note_type, content, color, board_id, position, blob_key, size, created_at, updated_at) VALUES (?,?,?,?,?,?,?,?,?,?)")
            .bind(note.to_string()).bind("text").bind(b"".to_vec()).bind("#FFF").bind(board.to_string()).bind(0i32).bind(Uuid::new_v4().to_string()).bind(0i64)
            .bind(Utc::now().to_rfc3339()).bind(Utc::now().to_rfc3339())
            .execute(&db.0).await.unwrap();
        (board, note)
    }

    fn make_block(note_id: Uuid, parent: Option<Uuid>, pos: f64) -> Block {
        let now = Utc::now();
        Block {
            id: Uuid::new_v4(), note_id, parent_block_id: parent, position: pos,
            block_type: BlockType::Text, content: b"hello".to_vec(), metadata: None, collapsed: false,
            created_at: now, updated_at: now,
        }
    }

    #[tokio::test]
    async fn insert_and_list_blocks() {
        let db = test_db().await;
        let (_b, n) = seed_note(&db).await;
        db.insert_block(&make_block(n, None, 1.0)).await.unwrap();
        db.insert_block(&make_block(n, None, 2.0)).await.unwrap();
        let blocks = db.list_blocks_for_note(n).await.unwrap();
        assert_eq!(blocks.len(), 2);
    }

    #[tokio::test]
    async fn cascade_delete_subtree() {
        let db = test_db().await;
        let (_b, n) = seed_note(&db).await;
        let parent = make_block(n, None, 1.0);
        db.insert_block(&parent).await.unwrap();
        let child = make_block(n, Some(parent.id), 1.0);
        db.insert_block(&child).await.unwrap();
        db.delete_block(parent.id).await.unwrap();
        assert_eq!(db.list_blocks_for_note(n).await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn move_block_reparents() {
        let db = test_db().await;
        let (_b, n) = seed_note(&db).await;
        let a = make_block(n, None, 1.0);
        let b = make_block(n, None, 2.0);
        db.insert_block(&a).await.unwrap();
        db.insert_block(&b).await.unwrap();
        db.move_block(b.id, Some(a.id), 1.0).await.unwrap();
        let fetched = db.get_block(b.id).await.unwrap().unwrap();
        assert_eq!(fetched.parent_block_id, Some(a.id));
    }

    #[tokio::test]
    async fn max_position_with_no_siblings_is_zero() {
        let db = test_db().await;
        let (_b, n) = seed_note(&db).await;
        assert_eq!(db.max_position(n, None).await.unwrap(), 0.0);
    }
}
```

- [ ] **Step 3.3: Run tests**

Run: `cargo test -p storage blocks::tests`
Expected: 4 tests PASS.

- [ ] **Step 3.4: Commit**

```bash
git add crates/storage/src/db/blocks.rs crates/storage/src/db/mod.rs
git commit -m "feat(storage): blocks CRUD with cascade delete and reparent"
```

---

## Task 4: Storage — Block Links (Edges)

**Files:**
- Create: `crates/storage/src/db/block_links.rs`
- Modify: `crates/storage/src/db/mod.rs`

- [ ] **Step 4.1: Register submodule**

Add `pub mod block_links;` in `crates/storage/src/db/mod.rs`.

- [ ] **Step 4.2: Write tests + implementation**

Create `crates/storage/src/db/block_links.rs`:

```rust
use crate::db::Db;
use crate::StorageError;
use chrono::Utc;
use jot_core::models::{BlockLink, LinkKind, TargetKind};
use sqlx::Row;
use uuid::Uuid;

fn row_to_link(row: &sqlx::sqlite::SqliteRow) -> BlockLink {
    let id: String = row.get("id");
    let src: String = row.get("source_block_id");
    let tk: String = row.get("target_kind");
    let lk: String = row.get("link_kind");
    let created: String = row.get("created_at");
    BlockLink {
        id: Uuid::parse_str(&id).unwrap(),
        source_block_id: Uuid::parse_str(&src).unwrap(),
        target_kind: TargetKind::from_str(&tk).unwrap_or(TargetKind::Note),
        target_id: row.get("target_id"),
        link_kind: LinkKind::from_str(&lk).unwrap_or(LinkKind::PageRef),
        created_at: chrono::DateTime::parse_from_rfc3339(&created).unwrap().with_timezone(&Utc),
    }
}

impl Db {
    /// Replace the entire edge set for one source block in a single transaction.
    pub async fn replace_links_for_block(&self, source: Uuid, links: &[BlockLink]) -> Result<(), StorageError> {
        let mut tx = self.0.begin().await?;
        sqlx::query("DELETE FROM block_links WHERE source_block_id = ?")
            .bind(source.to_string()).execute(&mut *tx).await?;
        for l in links {
            sqlx::query(
                "INSERT INTO block_links (id, source_block_id, target_kind, target_id, link_kind, created_at)
                 VALUES (?,?,?,?,?,?)"
            )
            .bind(l.id.to_string()).bind(source.to_string())
            .bind(l.target_kind.as_str()).bind(&l.target_id)
            .bind(l.link_kind.as_str()).bind(l.created_at.to_rfc3339())
            .execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn list_links_from(&self, source: Uuid) -> Result<Vec<BlockLink>, StorageError> {
        let rows = sqlx::query("SELECT * FROM block_links WHERE source_block_id = ?")
            .bind(source.to_string()).fetch_all(&self.0).await?;
        Ok(rows.iter().map(row_to_link).collect())
    }

    pub async fn backlinks_to_block(&self, target: Uuid) -> Result<Vec<BlockLink>, StorageError> {
        let rows = sqlx::query(
            "SELECT * FROM block_links WHERE target_kind = 'block' AND target_id = ? ORDER BY created_at ASC"
        ).bind(target.to_string()).fetch_all(&self.0).await?;
        Ok(rows.iter().map(row_to_link).collect())
    }

    pub async fn backlinks_to_note(&self, target: Uuid) -> Result<Vec<BlockLink>, StorageError> {
        let rows = sqlx::query(
            "SELECT * FROM block_links WHERE target_kind = 'note' AND target_id = ? ORDER BY created_at ASC"
        ).bind(target.to_string()).fetch_all(&self.0).await?;
        Ok(rows.iter().map(row_to_link).collect())
    }

    pub async fn blocks_with_tag(&self, name: &str) -> Result<Vec<Uuid>, StorageError> {
        let rows = sqlx::query(
            "SELECT DISTINCT source_block_id FROM block_links WHERE target_kind = 'tag' AND target_id = ?"
        ).bind(name).fetch_all(&self.0).await?;
        Ok(rows.iter().map(|r| {
            let s: String = r.get("source_block_id");
            Uuid::parse_str(&s).unwrap()
        }).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_db;

    #[tokio::test]
    async fn replace_links_is_idempotent() {
        let db = test_db().await;
        let src = Uuid::new_v4();
        // seed: a parent note + block via raw SQL
        let board = Uuid::new_v4();
        let note  = Uuid::new_v4();
        sqlx::query("INSERT INTO boards (id, identity_id, name, position, created_at) VALUES (?,?,?,?,?)")
            .bind(board.to_string()).bind(Uuid::new_v4().to_string()).bind("b").bind(0i32).bind(Utc::now().to_rfc3339())
            .execute(&db.0).await.unwrap();
        sqlx::query("INSERT INTO notes (id, note_type, content, color, board_id, position, blob_key, size, created_at, updated_at) VALUES (?,?,?,?,?,?,?,?,?,?)")
            .bind(note.to_string()).bind("text").bind(b"".to_vec()).bind("#FFF").bind(board.to_string()).bind(0i32).bind(Uuid::new_v4().to_string()).bind(0i64)
            .bind(Utc::now().to_rfc3339()).bind(Utc::now().to_rfc3339())
            .execute(&db.0).await.unwrap();
        sqlx::query("INSERT INTO blocks (id, note_id, position, block_type, content, created_at, updated_at) VALUES (?,?,?,?,?,?,?)")
            .bind(src.to_string()).bind(note.to_string()).bind(1.0f64).bind("text").bind(b"x".to_vec())
            .bind(Utc::now().to_rfc3339()).bind(Utc::now().to_rfc3339())
            .execute(&db.0).await.unwrap();

        let now = Utc::now();
        let mk = |tk: TargetKind, tid: &str, lk: LinkKind| BlockLink {
            id: Uuid::new_v4(), source_block_id: src, target_kind: tk, target_id: tid.into(), link_kind: lk, created_at: now,
        };
        db.replace_links_for_block(src, &[mk(TargetKind::Tag, "todo", LinkKind::Tag)]).await.unwrap();
        db.replace_links_for_block(src, &[mk(TargetKind::Tag, "later", LinkKind::Tag)]).await.unwrap();
        let links = db.list_links_from(src).await.unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_id, "later");
    }
}
```

- [ ] **Step 4.3: Run tests**

Run: `cargo test -p storage block_links`
Expected: 1 test PASS.

- [ ] **Step 4.4: Commit**

```bash
git add crates/storage/src/db/block_links.rs crates/storage/src/db/mod.rs
git commit -m "feat(storage): block_links edge table with idempotent replace"
```

---

## Task 5: Storage — Tags & Notes Extensions

**Files:**
- Create: `crates/storage/src/db/tags.rs`
- Modify: `crates/storage/src/db/notes.rs`
- Modify: `crates/storage/src/db/mod.rs`

- [ ] **Step 5.1: Register tags submodule**

Add `pub mod tags;` in `crates/storage/src/db/mod.rs`.

- [ ] **Step 5.2: Write tags storage**

Create `crates/storage/src/db/tags.rs`:

```rust
use crate::db::Db;
use crate::StorageError;
use chrono::Utc;
use jot_core::models::Tag;
use sqlx::Row;
use uuid::Uuid;

impl Db {
    pub async fn upsert_tag(&self, name: &str, identity: Uuid, color: Option<&str>) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO tags (name, identity_id, color, created_at) VALUES (?,?,?,?)
             ON CONFLICT(name, identity_id) DO UPDATE SET color = excluded.color"
        )
        .bind(name).bind(identity.to_string()).bind(color).bind(Utc::now().to_rfc3339())
        .execute(&self.0).await?;
        Ok(())
    }

    pub async fn list_tags(&self, identity: Uuid) -> Result<Vec<Tag>, StorageError> {
        let rows = sqlx::query("SELECT * FROM tags WHERE identity_id = ? ORDER BY name")
            .bind(identity.to_string()).fetch_all(&self.0).await?;
        Ok(rows.iter().map(|r| {
            let name: String = r.get("name");
            let id: String = r.get("identity_id");
            let color: Option<String> = r.get("color");
            let created: String = r.get("created_at");
            Tag {
                name, identity_id: Uuid::parse_str(&id).unwrap(), color,
                created_at: chrono::DateTime::parse_from_rfc3339(&created).unwrap().with_timezone(&Utc),
            }
        }).collect())
    }

    pub async fn delete_tag(&self, name: &str, identity: Uuid) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM tags WHERE name = ? AND identity_id = ?")
            .bind(name).bind(identity.to_string()).execute(&self.0).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_db;
    #[tokio::test]
    async fn upsert_and_list() {
        let db = test_db().await;
        let id = Uuid::new_v4();
        db.upsert_tag("projet-x", id, Some("#ff0")).await.unwrap();
        db.upsert_tag("projet-x", id, Some("#0f0")).await.unwrap();
        let tags = db.list_tags(id).await.unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].color.as_deref(), Some("#0f0"));
    }
}
```

- [ ] **Step 5.3: Extend `notes.rs` to read/write new columns**

In `crates/storage/src/db/notes.rs`:

a) Update `note_from_row` to read the new columns:

```rust
    let title: Option<Vec<u8>> = row.try_get("title").ok().flatten();
    let is_journal: i64 = row.try_get("is_journal").unwrap_or(0);
    let journal_date: Option<String> = row.try_get("journal_date").ok().flatten();
    let schema_version: i64 = row.try_get("schema_version").unwrap_or(0);
```

…and include them in the `Note { … }` literal:

```rust
        title,
        is_journal: is_journal != 0,
        journal_date,
        schema_version: schema_version as i32,
```

b) Update `insert_note` SQL to write the new columns:

```rust
    "INSERT INTO notes (id, note_type, content, thumbnail, duration_ms, color, board_id, position, blob_key, size, created_at, updated_at, title, is_journal, journal_date, schema_version)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
```

…with corresponding `.bind(...)` calls in the same order:

```rust
        .bind(&note.title)
        .bind(if note.is_journal { 1i64 } else { 0i64 })
        .bind(&note.journal_date)
        .bind(note.schema_version as i64)
```

c) Update all `SELECT` queries to use `SELECT *` (since they all need the new columns now) or list every column explicitly:

```rust
"SELECT id, note_type, content, thumbnail, duration_ms, color, board_id, position, blob_key, size, created_at, updated_at, title, is_journal, journal_date, schema_version FROM notes WHERE id = ?"
```

d) Add three new methods:

```rust
    pub async fn update_note_title(&self, id: Uuid, title: Option<&[u8]>) -> Result<(), StorageError> {
        sqlx::query("UPDATE notes SET title = ?, updated_at = ? WHERE id = ?")
            .bind(title).bind(Utc::now().to_rfc3339()).bind(id.to_string())
            .execute(&self.0).await?;
        Ok(())
    }

    pub async fn set_note_schema_version(&self, id: Uuid, version: i32) -> Result<(), StorageError> {
        sqlx::query("UPDATE notes SET schema_version = ?, updated_at = ? WHERE id = ?")
            .bind(version as i64).bind(Utc::now().to_rfc3339()).bind(id.to_string())
            .execute(&self.0).await?;
        Ok(())
    }

    pub async fn find_journal_note(&self, identity: Uuid, date: &str) -> Result<Option<Note>, StorageError> {
        let row = sqlx::query(
            "SELECT n.id, n.note_type, n.content, n.thumbnail, n.duration_ms, n.color, n.board_id, n.position, n.blob_key, n.size, n.created_at, n.updated_at, n.title, n.is_journal, n.journal_date, n.schema_version
             FROM notes n JOIN boards b ON b.id = n.board_id
             WHERE b.identity_id = ? AND n.journal_date = ? LIMIT 1"
        ).bind(identity.to_string()).bind(date).fetch_optional(&self.0).await?;
        Ok(row.map(|r| note_from_row(&r)))
    }
```

- [ ] **Step 5.4: Run all storage tests**

Run: `cargo test -p storage`
Expected: all existing tests still PASS, plus the new tag tests PASS.

- [ ] **Step 5.5: Commit**

```bash
git add crates/storage/src/db/
git commit -m "feat(storage): tags CRUD + note title/journal/schema_version"
```

---

## Task 6: Core — Markdown → Blocks Splitter

**Files:**
- Create: `crates/core/src/blocks/mod.rs`
- Create: `crates/core/src/blocks/split.rs`
- Modify: `crates/core/src/lib.rs`

- [ ] **Step 6.1: Wire module**

Add to `crates/core/src/lib.rs`:

```rust
pub mod blocks;
```

Create `crates/core/src/blocks/mod.rs`:

```rust
pub mod split;
pub mod links;

pub use split::{split_markdown, SplitBlock};
pub use links::{extract_links, ExtractedLink};
```

- [ ] **Step 6.2: Write failing test**

Create `crates/core/src/blocks/split.rs`:

```rust
use crate::models::BlockType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitBlock {
    pub block_type: BlockType,
    pub content: String,
    pub indent: u8, // 0 = root, increments per nesting level
}

/// Split a legacy markdown blob into a flat list of typed blocks (depth-first order).
/// Rules:
/// - Each paragraph (separated by a blank line) -> one Text block.
/// - Lines starting with `# `..`###### ` -> Heading block.
/// - Lines starting with `- [ ]` / `- [x]` -> Todo block.
/// - Lines starting with `> ` -> Quote block.
/// - Fenced code blocks ```…``` -> single Code block (content is inner text).
/// - Indentation by 2 spaces or 1 tab increases `indent`.
/// - A line starting with `---` on its own -> Divider.
pub fn split_markdown(md: &str) -> Vec<SplitBlock> {
    let mut out = Vec::new();
    let mut paragraph = String::new();
    let mut in_code = false;
    let mut code_buf = String::new();
    let mut code_indent = 0u8;

    fn indent_of(line: &str) -> (u8, &str) {
        let mut spaces = 0;
        let mut chars = line.chars();
        let mut consumed = 0;
        for c in chars.by_ref() {
            match c {
                ' ' => { spaces += 1; consumed += 1; }
                '\t' => { spaces += 2; consumed += 1; }
                _ => break,
            }
        }
        ((spaces / 2) as u8, &line[consumed..])
    }

    let flush_para = |out: &mut Vec<SplitBlock>, para: &mut String, indent: u8| {
        let trimmed = para.trim_end();
        if !trimmed.is_empty() {
            out.push(SplitBlock { block_type: BlockType::Text, content: trimmed.to_string(), indent });
        }
        para.clear();
    };

    for raw in md.lines() {
        if in_code {
            if raw.trim_start().starts_with("```") {
                out.push(SplitBlock { block_type: BlockType::Code, content: code_buf.trim_end().to_string(), indent: code_indent });
                code_buf.clear();
                in_code = false;
            } else {
                code_buf.push_str(raw);
                code_buf.push('\n');
            }
            continue;
        }

        let (indent, rest) = indent_of(raw);

        if rest.starts_with("```") {
            flush_para(&mut out, &mut paragraph, indent);
            in_code = true;
            code_indent = indent;
            continue;
        }
        if rest.trim().is_empty() {
            flush_para(&mut out, &mut paragraph, indent);
            continue;
        }
        if rest.starts_with("---") && rest.trim() == "---" {
            flush_para(&mut out, &mut paragraph, indent);
            out.push(SplitBlock { block_type: BlockType::Divider, content: String::new(), indent });
            continue;
        }
        if let Some(hashes) = rest.strip_prefix('#') {
            let mut level = 1;
            let mut tail = hashes;
            while let Some(rest2) = tail.strip_prefix('#') {
                level += 1; tail = rest2;
                if level >= 6 { break; }
            }
            if let Some(text) = tail.strip_prefix(' ') {
                flush_para(&mut out, &mut paragraph, indent);
                out.push(SplitBlock { block_type: BlockType::Heading, content: format!("{} {}", "#".repeat(level), text), indent });
                continue;
            }
        }
        if rest.starts_with("- [ ] ") || rest.starts_with("- [x] ") || rest.starts_with("- [X] ") {
            flush_para(&mut out, &mut paragraph, indent);
            out.push(SplitBlock { block_type: BlockType::Todo, content: rest.to_string(), indent });
            continue;
        }
        if rest.starts_with("> ") {
            flush_para(&mut out, &mut paragraph, indent);
            out.push(SplitBlock { block_type: BlockType::Quote, content: rest[2..].to_string(), indent });
            continue;
        }

        if !paragraph.is_empty() { paragraph.push('\n'); }
        paragraph.push_str(rest);
    }
    flush_para(&mut out, &mut paragraph, 0);
    if in_code && !code_buf.is_empty() {
        out.push(SplitBlock { block_type: BlockType::Code, content: code_buf.trim_end().to_string(), indent: code_indent });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paragraph_becomes_text_block() {
        let out = split_markdown("hello world");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].block_type, BlockType::Text);
        assert_eq!(out[0].content, "hello world");
    }

    #[test]
    fn blank_line_separates_paragraphs() {
        let out = split_markdown("a\n\nb");
        assert_eq!(out.len(), 2);
        assert_eq!(out[1].content, "b");
    }

    #[test]
    fn heading_is_detected() {
        let out = split_markdown("# Title\n\nbody");
        assert_eq!(out[0].block_type, BlockType::Heading);
        assert_eq!(out[1].block_type, BlockType::Text);
    }

    #[test]
    fn todo_is_detected() {
        let out = split_markdown("- [ ] buy milk\n- [x] done");
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|b| b.block_type == BlockType::Todo));
    }

    #[test]
    fn fenced_code_becomes_single_block() {
        let md = "```rust\nfn main() {}\n```";
        let out = split_markdown(md);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].block_type, BlockType::Code);
        assert_eq!(out[0].content, "fn main() {}");
    }

    #[test]
    fn divider_is_detected() {
        let out = split_markdown("a\n\n---\n\nb");
        assert_eq!(out.iter().filter(|b| b.block_type == BlockType::Divider).count(), 1);
    }

    #[test]
    fn indent_increments_with_two_spaces() {
        let out = split_markdown("- [ ] outer\n  - [ ] inner");
        assert_eq!(out[0].indent, 0);
        assert_eq!(out[1].indent, 1);
    }
}
```

- [ ] **Step 6.3: Run tests**

Run: `cargo test -p jot-core blocks::split`
Expected: 7 tests PASS.

- [ ] **Step 6.4: Commit**

```bash
git add crates/core/src/blocks/ crates/core/src/lib.rs
git commit -m "feat(core): markdown to blocks splitter for legacy migration"
```

---

## Task 7: Core — Link Extractor

**Files:**
- Create: `crates/core/src/blocks/links.rs`

- [ ] **Step 7.1: Write the failing test + implementation**

Create `crates/core/src/blocks/links.rs`:

```rust
use crate::models::{LinkKind, TargetKind};
use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedLink {
    pub target_kind: TargetKind,
    pub target_id: String, // note id, block id, or tag name
    pub link_kind: LinkKind,
}

static PAGE_RE: OnceLock<Regex> = OnceLock::new();
static BLOCK_RE: OnceLock<Regex> = OnceLock::new();
static EMBED_RE: OnceLock<Regex> = OnceLock::new();
static TAG_RE: OnceLock<Regex> = OnceLock::new();

fn page_re()  -> &'static Regex { PAGE_RE.get_or_init(|| Regex::new(r"\[\[([^\]\n]+?)\]\]").unwrap()) }
fn embed_re() -> &'static Regex { EMBED_RE.get_or_init(|| Regex::new(r"!\(\(([0-9a-fA-F-]{36})\)\)").unwrap()) }
fn block_re() -> &'static Regex { BLOCK_RE.get_or_init(|| Regex::new(r"(?<!!)\(\(([0-9a-fA-F-]{36})\)\)").unwrap()) }
fn tag_re()   -> &'static Regex { TAG_RE.get_or_init(|| Regex::new(r"(?:^|\s)#([A-Za-z0-9_\-]+)").unwrap()) }

/// `title_to_id` maps lowercased page titles to note UUIDs. Pages not present
/// in the map are still returned with `target_id` = the raw title (caller
/// decides whether to auto-create a page).
pub fn extract_links(markdown: &str, title_to_id: &std::collections::HashMap<String, String>) -> Vec<ExtractedLink> {
    let mut out = Vec::new();
    for cap in embed_re().captures_iter(markdown) {
        out.push(ExtractedLink { target_kind: TargetKind::Block, target_id: cap[1].to_string(), link_kind: LinkKind::BlockEmbed });
    }
    for cap in block_re().captures_iter(markdown) {
        // skip the ones already matched as embeds
        let id = cap[1].to_string();
        if out.iter().any(|l| l.target_id == id && l.link_kind == LinkKind::BlockEmbed) { continue; }
        out.push(ExtractedLink { target_kind: TargetKind::Block, target_id: id, link_kind: LinkKind::BlockRef });
    }
    for cap in page_re().captures_iter(markdown) {
        let title = cap[1].trim().to_string();
        let id = title_to_id.get(&title.to_lowercase()).cloned().unwrap_or(title);
        out.push(ExtractedLink { target_kind: TargetKind::Note, target_id: id, link_kind: LinkKind::PageRef });
    }
    for cap in tag_re().captures_iter(markdown) {
        out.push(ExtractedLink { target_kind: TargetKind::Tag, target_id: cap[1].to_string(), link_kind: LinkKind::Tag });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn detects_page_ref() {
        let map = HashMap::from([("hello".to_string(), "note-uuid".to_string())]);
        let out = extract_links("see [[Hello]] today", &map);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].link_kind, LinkKind::PageRef);
        assert_eq!(out[0].target_id, "note-uuid");
    }

    #[test]
    fn unknown_page_returns_raw_title() {
        let out = extract_links("see [[Unknown]] today", &HashMap::new());
        assert_eq!(out[0].target_id, "Unknown");
    }

    #[test]
    fn detects_block_ref_vs_embed() {
        let md = "ref ((550e8400-e29b-41d4-a716-446655440000)) embed !((550e8400-e29b-41d4-a716-446655440001))";
        let out = extract_links(md, &HashMap::new());
        assert_eq!(out.len(), 2);
        let kinds: Vec<_> = out.iter().map(|l| l.link_kind).collect();
        assert!(kinds.contains(&LinkKind::BlockRef));
        assert!(kinds.contains(&LinkKind::BlockEmbed));
    }

    #[test]
    fn detects_tag() {
        let out = extract_links("status #wip and #done-2025", &HashMap::new());
        let tags: Vec<_> = out.iter().filter(|l| l.link_kind == LinkKind::Tag).map(|l| l.target_id.clone()).collect();
        assert_eq!(tags, vec!["wip", "done-2025"]);
    }
}
```

- [ ] **Step 7.2: Add `regex` dep if missing**

Check `crates/core/Cargo.toml`. If `regex` is absent, add under `[dependencies]`:

```toml
regex = "1.10"
```

- [ ] **Step 7.3: Run tests**

Run: `cargo test -p jot-core blocks::links`
Expected: 4 tests PASS.

- [ ] **Step 7.4: Commit**

```bash
git add crates/core/Cargo.toml crates/core/src/blocks/links.rs
git commit -m "feat(core): extract page/block/embed/tag links from markdown"
```

---

## Task 8: API Routes — Blocks CRUD

**Files:**
- Create: `crates/api/src/routes/blocks.rs`
- Modify: `crates/api/src/routes/mod.rs`
- Modify: `crates/api/src/state.rs`

- [ ] **Step 8.1: Extend WsEvent enum**

In `crates/api/src/state.rs`, locate the `WsEvent` enum and add the four block variants:

```rust
    BlockCreated  { note_id: String, block_id: String },
    BlockUpdated  { note_id: String, block_id: String },
    BlockMoved    { note_id: String, block_id: String },
    BlockDeleted  { note_id: String, block_id: String },
```

- [ ] **Step 8.2: Create the route module**

Create `crates/api/src/routes/blocks.rs`:

```rust
use crate::auth::middleware::AuthenticatedDevice;
use crate::state::{AppState, WsEvent};
use crate::ApiError;
use axum::{extract::{Path, State}, http::StatusCode, Json};
use base64::Engine;
use chrono::Utc;
use jot_core::models::{Block, BlockType};
use serde::{Deserialize, Serialize};
use storage::db::shares::permission_allows;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub struct BlockDto {
    pub id: String,
    pub note_id: String,
    pub parent_block_id: Option<String>,
    pub position: f64,
    pub block_type: String,
    /// base64-encoded ciphertext
    pub content: String,
    /// base64-encoded ciphertext, if any
    pub metadata: Option<String>,
    pub collapsed: bool,
    pub created_at: String,
    pub updated_at: String,
}

fn to_dto(b: &Block) -> BlockDto {
    let b64 = base64::engine::general_purpose::STANDARD;
    BlockDto {
        id: b.id.to_string(),
        note_id: b.note_id.to_string(),
        parent_block_id: b.parent_block_id.map(|p| p.to_string()),
        position: b.position,
        block_type: b.block_type.as_str().to_string(),
        content: b64.encode(&b.content),
        metadata: b.metadata.as_ref().map(|m| b64.encode(m)),
        collapsed: b.collapsed,
        created_at: b.created_at.to_rfc3339(),
        updated_at: b.updated_at.to_rfc3339(),
    }
}

#[derive(Deserialize, ToSchema)]
pub struct CreateBlockBody {
    pub parent_id: Option<Uuid>,
    pub position: Option<f64>,
    pub block_type: String,
    pub content_b64: String,
    pub metadata_b64: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct PatchBlockBody {
    pub block_type: Option<String>,
    pub content_b64: Option<String>,
    pub metadata_b64: Option<String>,
    pub collapsed: Option<bool>,
}

#[derive(Deserialize, ToSchema)]
pub struct MoveBlockBody {
    pub new_parent_id: Option<Uuid>,
    pub new_position: f64,
}

fn decode_b64(s: &str) -> Result<Vec<u8>, ApiError> {
    base64::engine::general_purpose::STANDARD.decode(s)
        .map_err(|_| ApiError::BadRequest("invalid base64".into()))
}

async fn require_write(state: &AppState, note_id: Uuid, identity: &str) -> Result<(), ApiError> {
    let perm = state.db.note_permission_for(note_id.to_string().as_str(), identity).await?;
    if !permission_allows(&perm, "write") {
        return Err(ApiError::Forbidden("no write permission on note".into()));
    }
    Ok(())
}

pub async fn list_blocks(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(note_id): Path<Uuid>,
) -> Result<Json<Vec<BlockDto>>, ApiError> {
    let perm = state.db.note_permission_for(note_id.to_string().as_str(), &auth.0.identity_id).await?;
    if !permission_allows(&perm, "read") {
        return Err(ApiError::Forbidden("no read permission on note".into()));
    }
    let blocks = state.db.list_blocks_for_note(note_id).await?;
    Ok(Json(blocks.iter().map(to_dto).collect()))
}

pub async fn create_block(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(note_id): Path<Uuid>,
    Json(body): Json<CreateBlockBody>,
) -> Result<(StatusCode, Json<BlockDto>), ApiError> {
    require_write(&state, note_id, &auth.0.identity_id).await?;
    let now = Utc::now();
    let position = match body.position {
        Some(p) => p,
        None => state.db.max_position(note_id, body.parent_id).await? + 1.0,
    };
    let b = Block {
        id: Uuid::new_v4(),
        note_id,
        parent_block_id: body.parent_id,
        position,
        block_type: BlockType::from_str(&body.block_type),
        content: decode_b64(&body.content_b64)?,
        metadata: body.metadata_b64.as_deref().map(decode_b64).transpose()?,
        collapsed: false,
        created_at: now,
        updated_at: now,
    };
    state.db.insert_block(&b).await?;
    state.broadcast(WsEvent::BlockCreated { note_id: note_id.to_string(), block_id: b.id.to_string() }).await;
    Ok((StatusCode::CREATED, Json(to_dto(&b))))
}

pub async fn get_block(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
) -> Result<Json<BlockDto>, ApiError> {
    let b = state.db.get_block(id).await?.ok_or_else(|| ApiError::NotFound("block".into()))?;
    let perm = state.db.note_permission_for(b.note_id.to_string().as_str(), &auth.0.identity_id).await?;
    if !permission_allows(&perm, "read") {
        return Err(ApiError::Forbidden("no read permission on owning note".into()));
    }
    Ok(Json(to_dto(&b)))
}

pub async fn patch_block(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
    Json(body): Json<PatchBlockBody>,
) -> Result<Json<BlockDto>, ApiError> {
    let existing = state.db.get_block(id).await?.ok_or_else(|| ApiError::NotFound("block".into()))?;
    require_write(&state, existing.note_id, &auth.0.identity_id).await?;
    let new_type = body.block_type.as_deref().map(BlockType::from_str).unwrap_or(existing.block_type);
    let new_content = match body.content_b64 { Some(s) => decode_b64(&s)?, None => existing.content.clone() };
    let new_meta = match body.metadata_b64 { Some(s) => Some(decode_b64(&s)?), None => existing.metadata.clone() };
    state.db.update_block_content(id, &new_content, new_meta.as_deref(), new_type).await?;
    if let Some(c) = body.collapsed {
        state.db.set_block_collapsed(id, c).await?;
    }
    let refreshed = state.db.get_block(id).await?.unwrap();
    state.broadcast(WsEvent::BlockUpdated { note_id: existing.note_id.to_string(), block_id: id.to_string() }).await;
    Ok(Json(to_dto(&refreshed)))
}

pub async fn delete_block(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let existing = state.db.get_block(id).await?.ok_or_else(|| ApiError::NotFound("block".into()))?;
    let perm = state.db.note_permission_for(existing.note_id.to_string().as_str(), &auth.0.identity_id).await?;
    if !permission_allows(&perm, "delete") {
        return Err(ApiError::Forbidden("no delete permission on note".into()));
    }
    state.db.delete_block(id).await?;
    state.broadcast(WsEvent::BlockDeleted { note_id: existing.note_id.to_string(), block_id: id.to_string() }).await;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn move_block(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
    Json(body): Json<MoveBlockBody>,
) -> Result<StatusCode, ApiError> {
    let existing = state.db.get_block(id).await?.ok_or_else(|| ApiError::NotFound("block".into()))?;
    require_write(&state, existing.note_id, &auth.0.identity_id).await?;
    state.db.move_block(id, body.new_parent_id, body.new_position).await?;
    state.broadcast(WsEvent::BlockMoved { note_id: existing.note_id.to_string(), block_id: id.to_string() }).await;
    Ok(StatusCode::NO_CONTENT)
}
```

- [ ] **Step 8.3: Add the missing storage helper `note_permission_for`**

If `note_permission_for(&note_id, &identity_id)` doesn't yet exist, add to `crates/storage/src/db/shares.rs`:

```rust
impl Db {
    pub async fn note_permission_for(&self, note_id: &str, identity: &str) -> Result<String, crate::StorageError> {
        if self.is_note_owner(note_id, identity).await? { return Ok("delete".into()); }
        if let Some(p) = self.get_note_share_permission(note_id, identity).await? { return Ok(p); }
        if let Some(p) = self.get_board_share_permission_for_note(note_id, identity).await? { return Ok(p); }
        Ok("none".into())
    }
}
```

Check existing helper names; if `is_note_owner` / `get_note_share_permission` / `get_board_share_permission_for_note` don't exist with those exact names, use the equivalent helpers and adapt accordingly. The goal is a single function returning one of `none|read|write|delete`.

- [ ] **Step 8.4: Register the router**

In `crates/api/src/routes/mod.rs` add:

```rust
pub mod blocks;
```

…and in the function where the axum router is built (search for `Router::new()` in `mod.rs` or `lib.rs`), add:

```rust
    .route("/notes/:note_id/blocks", get(blocks::list_blocks).post(blocks::create_block))
    .route("/blocks/:id", get(blocks::get_block).patch(blocks::patch_block).delete(blocks::delete_block))
    .route("/blocks/:id/move", post(blocks::move_block))
```

- [ ] **Step 8.5: Add `base64` dep if missing**

Check `crates/api/Cargo.toml`. If not present:

```toml
base64 = "0.22"
```

- [ ] **Step 8.6: Build**

Run: `cargo build -p api`
Expected: clean build.

- [ ] **Step 8.7: Integration smoke test**

Add `crates/api/tests/blocks_routes.rs`:

```rust
// Integration test: create note, create block, list, patch, delete.
// Uses the existing test harness (TestApp / test_client) from sibling tests.
// Copy the boilerplate from crates/api/tests/notes_routes.rs and adapt.
```

If `crates/api/tests/notes_routes.rs` exists, copy its setup and write a happy-path test creating one block, listing, and asserting the count is 1. Run: `cargo test -p api blocks_routes`.

- [ ] **Step 8.8: Commit**

```bash
git add crates/api/ crates/storage/src/db/shares.rs
git commit -m "feat(api): blocks CRUD routes with note-scoped permissions"
```

---

## Task 9: API Routes — Indent / Outdent / Backlinks / Tags / Links

**Files:**
- Modify: `crates/api/src/routes/blocks.rs`
- Create: `crates/api/src/routes/tags.rs`
- Modify: `crates/api/src/routes/mod.rs`

- [ ] **Step 9.1: Add indent/outdent helpers**

In `crates/api/src/routes/blocks.rs` append:

```rust
pub async fn indent_block(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let b = state.db.get_block(id).await?.ok_or_else(|| ApiError::NotFound("block".into()))?;
    require_write(&state, b.note_id, &auth.0.identity_id).await?;
    // Find the immediately-preceding sibling (same parent, position < b.position, max position).
    let prev = state.db.previous_sibling(b.note_id, b.parent_block_id, b.position).await?
        .ok_or_else(|| ApiError::BadRequest("no preceding sibling to indent under".into()))?;
    let max_under_prev = state.db.max_position(b.note_id, Some(prev.id)).await?;
    state.db.move_block(id, Some(prev.id), max_under_prev + 1.0).await?;
    state.broadcast(WsEvent::BlockMoved { note_id: b.note_id.to_string(), block_id: id.to_string() }).await;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn outdent_block(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let b = state.db.get_block(id).await?.ok_or_else(|| ApiError::NotFound("block".into()))?;
    require_write(&state, b.note_id, &auth.0.identity_id).await?;
    let parent_id = b.parent_block_id.ok_or_else(|| ApiError::BadRequest("block is already at root".into()))?;
    let parent = state.db.get_block(parent_id).await?.ok_or_else(|| ApiError::NotFound("parent block".into()))?;
    // Place this block immediately after its parent among parent's siblings.
    let next_pos = parent.position + 0.5;
    state.db.move_block(id, parent.parent_block_id, next_pos).await?;
    state.broadcast(WsEvent::BlockMoved { note_id: b.note_id.to_string(), block_id: id.to_string() }).await;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn block_backlinks(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<storage::db::block_links::BackLinkRow>>, ApiError> {
    // Reuse list method, ensure the caller can at least read SOME note containing a source block
    // (simplest MVP: return all backlinks the caller can resolve to a readable note).
    let mut visible = Vec::new();
    for l in state.db.backlinks_to_block(id).await? {
        let src = state.db.get_block(l.source_block_id).await?;
        if let Some(src) = src {
            let perm = state.db.note_permission_for(src.note_id.to_string().as_str(), &auth.0.identity_id).await?;
            if permission_allows(&perm, "read") {
                visible.push(storage::db::block_links::BackLinkRow {
                    source_block_id: l.source_block_id.to_string(),
                    source_note_id: src.note_id.to_string(),
                    link_kind: l.link_kind.as_str().to_string(),
                });
            }
        }
    }
    Ok(Json(visible))
}

pub async fn note_backlinks(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<storage::db::block_links::BackLinkRow>>, ApiError> {
    let mut visible = Vec::new();
    for l in state.db.backlinks_to_note(id).await? {
        let src = state.db.get_block(l.source_block_id).await?;
        if let Some(src) = src {
            let perm = state.db.note_permission_for(src.note_id.to_string().as_str(), &auth.0.identity_id).await?;
            if permission_allows(&perm, "read") {
                visible.push(storage::db::block_links::BackLinkRow {
                    source_block_id: l.source_block_id.to_string(),
                    source_note_id: src.note_id.to_string(),
                    link_kind: l.link_kind.as_str().to_string(),
                });
            }
        }
    }
    Ok(Json(visible))
}

#[derive(Deserialize, ToSchema)]
pub struct PutLinksBody {
    pub links: Vec<LinkInput>,
}
#[derive(Deserialize, ToSchema)]
pub struct LinkInput {
    pub target_kind: String,
    pub target_id: String,
    pub link_kind: String,
}

pub async fn put_block_links(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
    Json(body): Json<PutLinksBody>,
) -> Result<StatusCode, ApiError> {
    let b = state.db.get_block(id).await?.ok_or_else(|| ApiError::NotFound("block".into()))?;
    require_write(&state, b.note_id, &auth.0.identity_id).await?;
    let now = Utc::now();
    let links: Vec<_> = body.links.into_iter().map(|l| jot_core::models::BlockLink {
        id: Uuid::new_v4(),
        source_block_id: id,
        target_kind: jot_core::models::TargetKind::from_str(&l.target_kind).unwrap_or(jot_core::models::TargetKind::Note),
        target_id: l.target_id,
        link_kind: jot_core::models::LinkKind::from_str(&l.link_kind).unwrap_or(jot_core::models::LinkKind::PageRef),
        created_at: now,
    }).collect();
    state.db.replace_links_for_block(id, &links).await?;
    Ok(StatusCode::NO_CONTENT)
}
```

- [ ] **Step 9.2: Add `previous_sibling` and `BackLinkRow` to storage**

In `crates/storage/src/db/blocks.rs` add:

```rust
impl Db {
    pub async fn previous_sibling(&self, note: Uuid, parent: Option<Uuid>, before_pos: f64) -> Result<Option<Block>, StorageError> {
        let row = sqlx::query(
            "SELECT * FROM blocks WHERE note_id = ? AND parent_block_id IS ? AND position < ?
             ORDER BY position DESC LIMIT 1"
        ).bind(note.to_string()).bind(parent.map(|p| p.to_string())).bind(before_pos)
        .fetch_optional(&self.0).await?;
        Ok(row.map(|r| row_to_block(&r)))
    }
}
```

In `crates/storage/src/db/block_links.rs` add:

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct BackLinkRow {
    pub source_block_id: String,
    pub source_note_id: String,
    pub link_kind: String,
}
```

- [ ] **Step 9.3: Tags route**

Create `crates/api/src/routes/tags.rs`:

```rust
use crate::auth::middleware::AuthenticatedDevice;
use crate::state::AppState;
use crate::ApiError;
use axum::{extract::{Path, State}, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub struct TagDto { pub name: String, pub color: Option<String> }

pub async fn list_tags(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
) -> Result<Json<Vec<TagDto>>, ApiError> {
    let id = Uuid::parse_str(&auth.0.identity_id).map_err(|_| ApiError::BadRequest("bad identity id".into()))?;
    let tags = state.db.list_tags(id).await?;
    Ok(Json(tags.into_iter().map(|t| TagDto { name: t.name, color: t.color }).collect()))
}

#[derive(Deserialize, ToSchema)]
pub struct PutTagBody { pub color: Option<String> }

pub async fn put_tag(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(name): Path<String>,
    Json(body): Json<PutTagBody>,
) -> Result<axum::http::StatusCode, ApiError> {
    let id = Uuid::parse_str(&auth.0.identity_id).map_err(|_| ApiError::BadRequest("bad identity id".into()))?;
    state.db.upsert_tag(&name, id, body.color.as_deref()).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn blocks_with_tag(
    State(state): State<AppState>,
    _auth: AuthenticatedDevice,
    Path(name): Path<String>,
) -> Result<Json<Vec<String>>, ApiError> {
    let ids = state.db.blocks_with_tag(&name).await?;
    Ok(Json(ids.into_iter().map(|i| i.to_string()).collect()))
}
```

- [ ] **Step 9.4: Register new routes**

In `crates/api/src/routes/mod.rs` add `pub mod tags;` and in the router:

```rust
    .route("/blocks/:id/indent",   post(blocks::indent_block))
    .route("/blocks/:id/outdent",  post(blocks::outdent_block))
    .route("/blocks/:id/backlinks", get(blocks::block_backlinks))
    .route("/blocks/:id/links",    put(blocks::put_block_links))
    .route("/notes/:id/backlinks",  get(blocks::note_backlinks))
    .route("/tags",                get(tags::list_tags))
    .route("/tags/:name",          put(tags::put_tag))
    .route("/tags/:name/blocks",    get(tags::blocks_with_tag))
```

- [ ] **Step 9.5: Build & commit**

Run: `cargo build -p api`

```bash
git add crates/api/ crates/storage/src/db/blocks.rs crates/storage/src/db/block_links.rs
git commit -m "feat(api): indent/outdent/backlinks/links/tags routes"
```

---

## Task 10: WebSocket — Broadcast Block Events

**Files:**
- Modify: `crates/api/src/routes/ws.rs`

- [ ] **Step 10.1: Add serializers for new WsEvent variants**

In `crates/api/src/routes/ws.rs`, locate the match that serializes `WsEvent` to JSON and add the four new variants. Pattern follows existing ones:

```rust
        WsEvent::BlockCreated { note_id, block_id } =>
            serde_json::json!({ "event": "block.created", "note_id": note_id, "block_id": block_id }),
        WsEvent::BlockUpdated { note_id, block_id } =>
            serde_json::json!({ "event": "block.updated", "note_id": note_id, "block_id": block_id }),
        WsEvent::BlockMoved   { note_id, block_id } =>
            serde_json::json!({ "event": "block.moved",   "note_id": note_id, "block_id": block_id }),
        WsEvent::BlockDeleted { note_id, block_id } =>
            serde_json::json!({ "event": "block.deleted", "note_id": note_id, "block_id": block_id }),
```

- [ ] **Step 10.2: Build & commit**

Run: `cargo build -p api`

```bash
git add crates/api/src/routes/ws.rs
git commit -m "feat(api): broadcast block.* WS events"
```

---

## Task 11: CLI — Block Subcommands (skeleton + add/list/show)

**Files:**
- Create: `crates/cli/src/commands/block.rs`
- Modify: `crates/cli/src/commands/mod.rs`
- Modify: `crates/cli/src/main.rs`
- Modify: `crates/cli/src/client.rs`

- [ ] **Step 11.1: Add `pub mod block;` to commands/mod.rs**

- [ ] **Step 11.2: Add CLI client methods**

In `crates/cli/src/client.rs`, add (next to the existing Note methods):

```rust
use jot_core::models::{Block, BlockType};

#[derive(serde::Deserialize)]
struct BlockDto {
    id: String,
    note_id: String,
    parent_block_id: Option<String>,
    position: f64,
    block_type: String,
    content: String,           // base64
    metadata: Option<String>,  // base64
    collapsed: bool,
    created_at: String,
    updated_at: String,
}

fn b64decode(s: &str) -> anyhow::Result<Vec<u8>> {
    use base64::Engine;
    Ok(base64::engine::general_purpose::STANDARD.decode(s)?)
}

fn dto_to_block(d: BlockDto) -> anyhow::Result<Block> {
    use uuid::Uuid;
    use chrono::{DateTime, Utc};
    Ok(Block {
        id: Uuid::parse_str(&d.id)?,
        note_id: Uuid::parse_str(&d.note_id)?,
        parent_block_id: d.parent_block_id.as_deref().and_then(|s| Uuid::parse_str(s).ok()),
        position: d.position,
        block_type: BlockType::from_str(&d.block_type),
        content: b64decode(&d.content)?,
        metadata: d.metadata.as_deref().map(b64decode).transpose()?,
        collapsed: d.collapsed,
        created_at: DateTime::parse_from_rfc3339(&d.created_at)?.with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&d.updated_at)?.with_timezone(&Utc),
    })
}

impl Client {
    pub async fn list_blocks(&self, note_id: uuid::Uuid) -> anyhow::Result<Vec<Block>> {
        let r = self.get(&format!("/notes/{}/blocks", note_id)).await?;
        let dtos: Vec<BlockDto> = r.json().await?;
        dtos.into_iter().map(dto_to_block).collect()
    }

    pub async fn create_block(
        &self,
        note_id: uuid::Uuid,
        parent: Option<uuid::Uuid>,
        position: Option<f64>,
        block_type: &str,
        content: &[u8],
        metadata: Option<&[u8]>,
    ) -> anyhow::Result<Block> {
        use base64::Engine;
        let body = serde_json::json!({
            "parent_id": parent,
            "position": position,
            "block_type": block_type,
            "content_b64": base64::engine::general_purpose::STANDARD.encode(content),
            "metadata_b64": metadata.map(|m| base64::engine::general_purpose::STANDARD.encode(m)),
        });
        let r = self.post(&format!("/notes/{}/blocks", note_id), &body).await?;
        let dto: BlockDto = r.json().await?;
        dto_to_block(dto)
    }

    pub async fn get_block(&self, id: uuid::Uuid) -> anyhow::Result<Block> {
        let r = self.get(&format!("/blocks/{}", id)).await?;
        dto_to_block(r.json().await?)
    }

    pub async fn patch_block(&self, id: uuid::Uuid, block_type: Option<&str>, content: Option<&[u8]>) -> anyhow::Result<Block> {
        use base64::Engine;
        let body = serde_json::json!({
            "block_type": block_type,
            "content_b64": content.map(|c| base64::engine::general_purpose::STANDARD.encode(c)),
        });
        let r = self.patch(&format!("/blocks/{}", id), &body).await?;
        dto_to_block(r.json().await?)
    }

    pub async fn move_block(&self, id: uuid::Uuid, new_parent: Option<uuid::Uuid>, new_position: f64) -> anyhow::Result<()> {
        let body = serde_json::json!({ "new_parent_id": new_parent, "new_position": new_position });
        self.post(&format!("/blocks/{}/move", id), &body).await?;
        Ok(())
    }

    pub async fn indent_block(&self, id: uuid::Uuid) -> anyhow::Result<()> {
        self.post(&format!("/blocks/{}/indent", id), &serde_json::json!({})).await?;
        Ok(())
    }
    pub async fn outdent_block(&self, id: uuid::Uuid) -> anyhow::Result<()> {
        self.post(&format!("/blocks/{}/outdent", id), &serde_json::json!({})).await?;
        Ok(())
    }
    pub async fn delete_block(&self, id: uuid::Uuid) -> anyhow::Result<()> {
        self.delete(&format!("/blocks/{}", id)).await?;
        Ok(())
    }
}
```

Make sure the `Client` already has helpers `get/post/patch/delete`; if names differ, adapt to existing methods.

- [ ] **Step 11.3: Write the command module**

Create `crates/cli/src/commands/block.rs`:

```rust
use crate::client::Client;
use crate::config::Config;
use crate::i18n::t;
use anyhow::{Context, Result};
use clap::Subcommand;
use uuid::Uuid;

#[derive(Subcommand, Debug)]
pub enum BlockCmd {
    /// Add a new block
    Add {
        #[arg(long)] note: Uuid,
        #[arg(long)] parent: Option<Uuid>,
        #[arg(long)] position: Option<f64>,
        #[arg(long, default_value = "text")] r#type: String,
        #[arg(long)] text: String,
    },
    /// List blocks of a note (tree view)
    List { #[arg(long)] note: Uuid, #[arg(long, default_value_t = false)] tree: bool },
    /// Print one block's content
    Show { id: Uuid },
    /// Open the block content in $EDITOR
    Edit { id: Uuid },
    /// Move a block under a different parent / position
    Move { id: Uuid, #[arg(long)] to: Option<Uuid>, #[arg(long)] position: f64 },
    Indent { id: Uuid },
    Outdent { id: Uuid },
    Delete { id: Uuid },
    /// Print the reference syntax for embedding in another block
    Ref { id: Uuid },
    Backlinks { id: Uuid },
    /// Migrate legacy notes from `content` blob into blocks
    Migrate {
        #[arg(long)] all: bool,
        #[arg(long)] note: Option<Uuid>,
        #[arg(long, default_value_t = false)] dry_run: bool,
    },
}

pub async fn run(cfg: &Config, cmd: BlockCmd) -> Result<()> {
    let client = Client::new(cfg).await?;
    match cmd {
        BlockCmd::Add { note, parent, position, r#type, text } => {
            // Plaintext path for MVP CLI; encryption layer added in Task 13.
            let b = client.create_block(note, parent, position, &r#type, text.as_bytes(), None).await?;
            println!("{}", b.id);
        }
        BlockCmd::List { note, tree } => {
            let blocks = client.list_blocks(note).await?;
            if tree { print_tree(&blocks); } else { for b in &blocks { println!("{} [{}] pos={}", b.id, b.block_type.as_str(), b.position); } }
        }
        BlockCmd::Show { id } => {
            let b = client.get_block(id).await?;
            println!("{}", String::from_utf8_lossy(&b.content));
        }
        BlockCmd::Edit { id } => {
            let current = client.get_block(id).await?;
            let edited = crate::editor::edit_in_editor(&String::from_utf8_lossy(&current.content))?;
            client.patch_block(id, None, Some(edited.as_bytes())).await?;
            println!("{}", t!("block.updated"));
        }
        BlockCmd::Move { id, to, position } => { client.move_block(id, to, position).await?; }
        BlockCmd::Indent { id } => { client.indent_block(id).await?; }
        BlockCmd::Outdent { id } => { client.outdent_block(id).await?; }
        BlockCmd::Delete { id } => { client.delete_block(id).await?; }
        BlockCmd::Ref { id } => { println!("(({}))", id); }
        BlockCmd::Backlinks { id } => {
            let rows: Vec<serde_json::Value> = client.get(&format!("/blocks/{}/backlinks", id)).await?.json().await?;
            for r in rows { println!("{}", r); }
        }
        BlockCmd::Migrate { all, note, dry_run } => crate::commands::block::migrate(&client, all, note, dry_run).await?,
    }
    Ok(())
}

fn print_tree(blocks: &[jot_core::models::Block]) {
    use std::collections::HashMap;
    let mut by_parent: HashMap<Option<uuid::Uuid>, Vec<&jot_core::models::Block>> = HashMap::new();
    for b in blocks { by_parent.entry(b.parent_block_id).or_default().push(b); }
    for kids in by_parent.values_mut() { kids.sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap()); }
    fn walk(by_parent: &std::collections::HashMap<Option<uuid::Uuid>, Vec<&jot_core::models::Block>>, parent: Option<uuid::Uuid>, depth: usize) {
        if let Some(kids) = by_parent.get(&parent) {
            for k in kids {
                println!("{}{} {}", "  ".repeat(depth), k.id, String::from_utf8_lossy(&k.content));
                walk(by_parent, Some(k.id), depth + 1);
            }
        }
    }
    walk(&by_parent, None, 0);
}

pub async fn migrate(_client: &Client, _all: bool, _note: Option<Uuid>, _dry_run: bool) -> Result<()> {
    // Stub: full implementation in Task 14 once encryption helpers land.
    anyhow::bail!("not yet implemented — see Task 14")
}
```

- [ ] **Step 11.4: Add i18n key**

In each of `crates/cli/src/i18n/locales/{en,fr,es,de}.json` add `"block.updated"` with the right translation (en: "Block updated.", fr: "Bloc mis à jour.", es: "Bloque actualizado.", de: "Block aktualisiert.").

- [ ] **Step 11.5: Wire clap subcommand**

In `crates/cli/src/main.rs`, locate the top-level `Command` enum and add:

```rust
    /// Block-level operations
    Block { #[command(subcommand)] cmd: crate::commands::block::BlockCmd },
```

…and in the dispatch `match`:

```rust
        Command::Block { cmd } => commands::block::run(&cfg, cmd).await?,
```

- [ ] **Step 11.6: Build**

Run: `cargo build -p cli`
Expected: clean build.

- [ ] **Step 11.7: Commit**

```bash
git add crates/cli/
git commit -m "feat(cli): block subcommands add/list/show/edit/move/indent/outdent/delete/ref/backlinks"
```

---

## Task 12: SPA — API Client Methods

**Files:**
- Modify: `spa/src/api.ts`

- [ ] **Step 12.1: Add block & tag types and methods**

Append to `spa/src/api.ts`:

```typescript
export interface BlockDto {
  id: string;
  note_id: string;
  parent_block_id: string | null;
  position: number;
  block_type: "text" | "heading" | "todo" | "quote" | "code" | "embed" | "divider";
  content: string;            // base64 ciphertext
  metadata: string | null;    // base64 ciphertext
  collapsed: boolean;
  created_at: string;
  updated_at: string;
}

export async function listBlocks(noteId: string): Promise<BlockDto[]> {
  const r = await authedFetch(`${BASE}/notes/${noteId}/blocks`);
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function createBlock(
  noteId: string,
  input: { parent_id?: string | null; position?: number; block_type: string; content_b64: string; metadata_b64?: string | null }
): Promise<BlockDto> {
  const r = await authedFetch(`${BASE}/notes/${noteId}/blocks`, { method: "POST", body: JSON.stringify(input) });
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function patchBlock(
  id: string,
  patch: { block_type?: string; content_b64?: string; metadata_b64?: string | null; collapsed?: boolean }
): Promise<BlockDto> {
  const r = await authedFetch(`${BASE}/blocks/${id}`, { method: "PATCH", body: JSON.stringify(patch) });
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function deleteBlock(id: string): Promise<void> {
  const r = await authedFetch(`${BASE}/blocks/${id}`, { method: "DELETE" });
  if (!r.ok) throw new Error(await r.text());
}

export async function moveBlock(id: string, new_parent_id: string | null, new_position: number): Promise<void> {
  const r = await authedFetch(`${BASE}/blocks/${id}/move`, {
    method: "POST", body: JSON.stringify({ new_parent_id, new_position }),
  });
  if (!r.ok) throw new Error(await r.text());
}

export async function indentBlock(id: string): Promise<void> {
  const r = await authedFetch(`${BASE}/blocks/${id}/indent`, { method: "POST", body: "{}" });
  if (!r.ok) throw new Error(await r.text());
}
export async function outdentBlock(id: string): Promise<void> {
  const r = await authedFetch(`${BASE}/blocks/${id}/outdent`, { method: "POST", body: "{}" });
  if (!r.ok) throw new Error(await r.text());
}

export async function putBlockLinks(id: string, links: { target_kind: string; target_id: string; link_kind: string }[]): Promise<void> {
  const r = await authedFetch(`${BASE}/blocks/${id}/links`, { method: "PUT", body: JSON.stringify({ links }) });
  if (!r.ok) throw new Error(await r.text());
}
```

- [ ] **Step 12.2: Build SPA**

Run: `cd spa && bun run build` (or whatever `package.json` defines; check `scripts.build`).
Expected: build succeeds.

- [ ] **Step 12.3: Commit**

```bash
git add spa/src/api.ts
git commit -m "feat(spa): block API client methods"
```

---

## Task 13: SPA — Tree Helpers, Markdown Splitter, Encryption Adapter

**Files:**
- Create: `spa/src/blocks/tree.ts`
- Create: `spa/src/blocks/markdown.ts`
- Create: `spa/src/blocks/crypto.ts`

- [ ] **Step 13.1: Tree helpers**

Create `spa/src/blocks/tree.ts`:

```typescript
import type { BlockDto } from "../api";

export interface BlockNode extends BlockDto {
  children: BlockNode[];
  plaintext: string; // decrypted content cache
}

export function buildTree(blocks: BlockDto[]): BlockNode[] {
  const byId = new Map<string, BlockNode>();
  for (const b of blocks) byId.set(b.id, { ...b, children: [], plaintext: "" });
  const roots: BlockNode[] = [];
  for (const b of blocks) {
    const node = byId.get(b.id)!;
    if (b.parent_block_id && byId.has(b.parent_block_id)) {
      byId.get(b.parent_block_id)!.children.push(node);
    } else {
      roots.push(node);
    }
  }
  const cmp = (a: BlockNode, b: BlockNode) => a.position - b.position;
  const sortRec = (ns: BlockNode[]) => { ns.sort(cmp); ns.forEach(n => sortRec(n.children)); };
  sortRec(roots);
  return roots;
}

/** Flatten the tree depth-first for keyboard nav. */
export function flatten(roots: BlockNode[]): BlockNode[] {
  const out: BlockNode[] = [];
  const walk = (ns: BlockNode[]) => ns.forEach(n => { out.push(n); if (!n.collapsed) walk(n.children); });
  walk(roots);
  return out;
}

/** Compute the position to insert a new block after `prev` among siblings `siblings`. */
export function positionAfter(siblings: BlockNode[], prev: BlockNode | null): number {
  if (!prev) return (siblings[0]?.position ?? 1) - 1;
  const idx = siblings.findIndex(s => s.id === prev.id);
  const next = siblings[idx + 1];
  if (!next) return prev.position + 1;
  return (prev.position + next.position) / 2;
}
```

- [ ] **Step 13.2: Markdown splitter (mirror of Rust splitter, used for migration)**

Create `spa/src/blocks/markdown.ts`:

```typescript
export type SplitType = "text" | "heading" | "todo" | "quote" | "code" | "divider";
export interface SplitBlock { block_type: SplitType; content: string; indent: number; }

/** Mirror of the Rust splitter in crates/core/src/blocks/split.rs.
 *  Keep in sync — both are unit-tested. */
export function splitMarkdown(md: string): SplitBlock[] {
  const out: SplitBlock[] = [];
  let para = "";
  let inCode = false, codeBuf = "", codeIndent = 0;

  const indentOf = (l: string): [number, string] => {
    let spaces = 0, consumed = 0;
    for (const c of l) {
      if (c === " ") { spaces++; consumed++; }
      else if (c === "\t") { spaces += 2; consumed++; }
      else break;
    }
    return [Math.floor(spaces / 2), l.slice(consumed)];
  };
  const flush = (indent: number) => {
    const t = para.replace(/[\s]+$/g, "");
    if (t) out.push({ block_type: "text", content: t, indent });
    para = "";
  };

  for (const raw of md.split(/\r?\n/)) {
    if (inCode) {
      if (raw.trimStart().startsWith("```")) {
        out.push({ block_type: "code", content: codeBuf.replace(/[\s]+$/g, ""), indent: codeIndent });
        codeBuf = ""; inCode = false;
      } else { codeBuf += raw + "\n"; }
      continue;
    }
    const [indent, rest] = indentOf(raw);
    if (rest.startsWith("```")) { flush(indent); inCode = true; codeIndent = indent; continue; }
    if (rest.trim() === "") { flush(indent); continue; }
    if (rest.trim() === "---") { flush(indent); out.push({ block_type: "divider", content: "", indent }); continue; }
    const h = /^(#{1,6}) (.*)$/.exec(rest);
    if (h) { flush(indent); out.push({ block_type: "heading", content: rest, indent }); continue; }
    if (/^- \[[ xX]\] /.test(rest)) { flush(indent); out.push({ block_type: "todo", content: rest, indent }); continue; }
    if (rest.startsWith("> ")) { flush(indent); out.push({ block_type: "quote", content: rest.slice(2), indent }); continue; }
    if (para) para += "\n";
    para += rest;
  }
  flush(0);
  if (inCode && codeBuf) out.push({ block_type: "code", content: codeBuf.replace(/[\s]+$/g, ""), indent: codeIndent });
  return out;
}
```

- [ ] **Step 13.3: Crypto adapter**

Create `spa/src/blocks/crypto.ts`:

```typescript
import { encryptNoteOwner, decryptNoteOwner } from "../crypto";

// Reuse the note-level encryption helpers. Each block's ciphertext is
// produced with the SAME DEK as its containing note. The functions below
// adapt the existing helpers to operate on plain string content rather
// than Note objects.

export async function encryptBlock(noteId: string, plaintext: string): Promise<string> {
  // encryptNoteOwner currently expects { id, content } shape; if its signature
  // differs, call its lower-level primitive (look in spa/src/crypto.ts).
  // We assume encryptNoteOwner returns base64 ciphertext.
  const enc = await encryptNoteOwner({ id: noteId, content: new TextEncoder().encode(plaintext) } as any);
  return typeof enc === "string" ? enc : btoa(String.fromCharCode(...new Uint8Array(enc as ArrayBuffer)));
}

export async function decryptBlock(noteId: string, ciphertextB64: string): Promise<string> {
  const raw = Uint8Array.from(atob(ciphertextB64), c => c.charCodeAt(0));
  const dec = await decryptNoteOwner({ id: noteId, content: raw } as any);
  return new TextDecoder().decode(dec as Uint8Array);
}
```

Note: read `spa/src/crypto.ts` and tighten these wrappers — the snippet above is the contract; adjust the call sites to the actual signatures.

- [ ] **Step 13.4: Quick unit test**

Add a Vitest file `spa/src/blocks/markdown.test.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { splitMarkdown } from "./markdown";

describe("splitMarkdown", () => {
  it("splits paragraphs", () => {
    expect(splitMarkdown("a\n\nb").length).toBe(2);
  });
  it("detects heading", () => {
    expect(splitMarkdown("# Title").map(b => b.block_type)).toEqual(["heading"]);
  });
  it("detects fenced code", () => {
    const out = splitMarkdown("```\nfoo\n```");
    expect(out[0].block_type).toBe("code");
    expect(out[0].content).toBe("foo");
  });
});
```

Run: `cd spa && bun test markdown` (or `npx vitest run markdown` depending on toolchain).
Expected: 3 tests PASS.

- [ ] **Step 13.5: Commit**

```bash
git add spa/src/blocks/
git commit -m "feat(spa): block tree/markdown/crypto helpers"
```

---

## Task 14: CLI — Lazy Migration Command

**Files:**
- Modify: `crates/cli/src/commands/block.rs`
- Modify: `crates/cli/src/client.rs`

- [ ] **Step 14.1: Add client helper to bulk-mark schema_version**

In `crates/cli/src/client.rs`:

```rust
impl Client {
    pub async fn set_note_schema_version(&self, note_id: uuid::Uuid, version: i32) -> anyhow::Result<()> {
        self.patch(&format!("/notes/{}/schema-version", note_id),
                   &serde_json::json!({ "schema_version": version })).await?;
        Ok(())
    }
    pub async fn fetch_note_content_plain(&self, note_id: uuid::Uuid) -> anyhow::Result<String> {
        // Existing flow: GET /notes/:id/blob -> decrypt with note DEK -> utf8 string.
        // Reuse whatever helper the CLI already has for reading a note's text content.
        let bytes = self.get_note_blob(note_id).await?;
        let dek = self.note_dek(note_id).await?;
        let plain = jot_core::crypto::aead::decrypt(&bytes, &dek)?;
        Ok(String::from_utf8(plain)?)
    }
}
```

If `get_note_blob` / `note_dek` / `jot_core::crypto::aead::decrypt` have different names, find their actual signatures in `crates/cli/src/client.rs` and `crates/core/src/crypto/` and adapt.

- [ ] **Step 14.2: Add the API route `PATCH /notes/:id/schema-version`**

In `crates/api/src/routes/notes.rs`, add a small handler and route:

```rust
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct PatchSchemaVersionBody { pub schema_version: i32 }

pub async fn patch_schema_version(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
    Json(body): Json<PatchSchemaVersionBody>,
) -> Result<StatusCode, ApiError> {
    let perm = state.db.note_permission_for(id.to_string().as_str(), &auth.0.identity_id).await?;
    if !permission_allows(&perm, "write") { return Err(ApiError::Forbidden("no write".into())); }
    state.db.set_note_schema_version(id, body.schema_version).await?;
    Ok(StatusCode::NO_CONTENT)
}
```

Register it in `mod.rs`:

```rust
    .route("/notes/:id/schema-version", patch(notes::patch_schema_version))
```

- [ ] **Step 14.3: Implement `migrate` in block.rs**

Replace the stub `migrate` in `crates/cli/src/commands/block.rs` with:

```rust
pub async fn migrate(client: &Client, all: bool, note: Option<Uuid>, dry_run: bool) -> Result<()> {
    use jot_core::blocks::split_markdown;
    let targets: Vec<Uuid> = if all {
        client.list_legacy_text_notes().await?
    } else {
        vec![note.context("--note or --all is required")?]
    };
    for n in targets {
        let md = match client.fetch_note_content_plain(n).await {
            Ok(s) => s,
            Err(_) => { eprintln!("skip {} (cannot decrypt)", n); continue; }
        };
        let parts = split_markdown(&md);
        println!("note {} -> {} block(s){}", n, parts.len(), if dry_run { " (dry-run)" } else { "" });
        if dry_run { continue; }
        let mut indent_stack: Vec<(u8, Uuid)> = Vec::new();
        for (i, p) in parts.iter().enumerate() {
            while let Some(&(top, _)) = indent_stack.last() { if top >= p.indent { indent_stack.pop(); } else { break; } }
            let parent = indent_stack.last().map(|(_, id)| *id);
            let b = client.create_block(n, parent, Some(i as f64), p.block_type.as_str(), p.content.as_bytes(), None).await?;
            indent_stack.push((p.indent, b.id));
        }
        client.set_note_schema_version(n, 1).await?;
    }
    Ok(())
}
```

Add `list_legacy_text_notes` to the API + client (a small endpoint returning `[{id}]` where `note_type='text' AND schema_version=0`). Mirror the pattern of `list_notes`.

- [ ] **Step 14.4: Build & run a happy-path manual test**

Run: `cargo build --workspace`
Manual smoke: start the server, create a text note via SPA/CLI with `## Title\n\npara1\n\n- [ ] todo`, run `jot block migrate --note <uuid>`, then `jot block list --note <uuid> --tree` and verify the structure.

- [ ] **Step 14.5: Commit**

```bash
git add crates/cli/ crates/api/
git commit -m "feat(cli): lazy migration of legacy notes into blocks"
```

---

## Task 15: SPA — BlockEditor Component (rendering + keymap)

**Files:**
- Create: `spa/src/components/BlockEditor.tsx`
- Create: `spa/src/components/BlockEditor.css`
- Create: `spa/src/blocks/keymap.ts`
- Modify: `spa/src/components/NoteEditor.tsx`

- [ ] **Step 15.1: Build the keymap module**

Create `spa/src/blocks/keymap.ts`:

```typescript
import * as api from "../api";
import { encryptBlock } from "./crypto";
import type { BlockNode } from "./tree";

export interface KeymapCtx {
  noteId: string;
  blocks: BlockNode[];                 // flat list, depth-first
  activeIdx: number;                   // index in `blocks` of the focused block
  refresh: () => Promise<void>;
  setActive: (id: string) => void;
}

export async function newBlockBelow(ctx: KeymapCtx) {
  const cur = ctx.blocks[ctx.activeIdx];
  const ciphertext = await encryptBlock(ctx.noteId, "");
  const created = await api.createBlock(ctx.noteId, {
    parent_id: cur?.parent_block_id ?? null,
    position: cur ? cur.position + 0.5 : undefined,
    block_type: "text",
    content_b64: ciphertext,
  });
  await ctx.refresh();
  ctx.setActive(created.id);
}

export async function indent(ctx: KeymapCtx) {
  const cur = ctx.blocks[ctx.activeIdx];
  if (!cur) return;
  await api.indentBlock(cur.id);
  await ctx.refresh();
}

export async function outdent(ctx: KeymapCtx) {
  const cur = ctx.blocks[ctx.activeIdx];
  if (!cur) return;
  await api.outdentBlock(cur.id);
  await ctx.refresh();
}

export async function deleteActive(ctx: KeymapCtx) {
  const cur = ctx.blocks[ctx.activeIdx];
  if (!cur) return;
  await api.deleteBlock(cur.id);
  await ctx.refresh();
}

export async function persistEdit(noteId: string, blockId: string, plaintext: string, block_type?: string) {
  const ciphertext = await encryptBlock(noteId, plaintext);
  await api.patchBlock(blockId, { content_b64: ciphertext, block_type });
}
```

- [ ] **Step 15.2: BlockEditor component**

Create `spa/src/components/BlockEditor.tsx`:

```tsx
import { useEffect, useState, useRef } from "preact/hooks";
import { signal } from "@preact/signals";
import * as api from "../api";
import { buildTree, flatten, type BlockNode } from "../blocks/tree";
import { decryptBlock } from "../blocks/crypto";
import * as keymap from "../blocks/keymap";
import { t } from "../i18n";
import "./BlockEditor.css";

interface Props { noteId: string }

export function BlockEditor({ noteId }: Props) {
  const [roots, setRoots] = useState<BlockNode[]>([]);
  const [flat, setFlat] = useState<BlockNode[]>([]);
  const [active, setActive] = useState<string | null>(null);

  const refresh = async () => {
    const dtos = await api.listBlocks(noteId);
    const tree = buildTree(dtos);
    // Decrypt all in parallel
    const all = flatten(tree);
    await Promise.all(all.map(async n => { n.plaintext = await decryptBlock(noteId, n.content); }));
    setRoots(tree);
    setFlat(all);
  };

  useEffect(() => { refresh(); }, [noteId]);

  const ctx = (): keymap.KeymapCtx => ({
    noteId,
    blocks: flat,
    activeIdx: Math.max(0, flat.findIndex(b => b.id === active)),
    refresh,
    setActive,
  });

  const onKeyDown = async (e: KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); await keymap.newBlockBelow(ctx()); }
    else if (e.key === "Tab" && !e.shiftKey) { e.preventDefault(); await keymap.indent(ctx()); }
    else if (e.key === "Tab" &&  e.shiftKey) { e.preventDefault(); await keymap.outdent(ctx()); }
    else if (e.key === "Backspace") {
      const cur = ctx().blocks[ctx().activeIdx];
      if (cur && cur.plaintext === "") { e.preventDefault(); await keymap.deleteActive(ctx()); }
    }
  };

  const onBlur = async (b: BlockNode, text: string) => {
    if (text !== b.plaintext) {
      await keymap.persistEdit(noteId, b.id, text);
      await refresh();
    }
  };

  const renderNode = (n: BlockNode, depth = 0): preact.JSX.Element => (
    <div class={`block-row ${active === n.id ? "active" : ""}`} style={{ paddingLeft: `${depth * 24}px` }}>
      <span class="block-bullet" data-id={n.id}>•</span>
      <div
        class="block-content"
        contentEditable
        onFocus={() => setActive(n.id)}
        onBlur={(ev) => onBlur(n, (ev.target as HTMLElement).innerText)}
        onKeyDown={onKeyDown as any}
        dangerouslySetInnerHTML={{ __html: escapeHtml(n.plaintext) }}
      />
      {!n.collapsed && n.children.map(c => renderNode(c, depth + 1))}
    </div>
  );

  return <div class="block-editor">{roots.length === 0 ? <p>{t("block.empty")}</p> : roots.map(r => renderNode(r))}</div>;
}

function escapeHtml(s: string): string {
  return s.replace(/[&<>]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" }[c]!));
}
```

- [ ] **Step 15.3: Minimal CSS**

Create `spa/src/components/BlockEditor.css`:

```css
.block-editor { font-family: inherit; }
.block-row { display: flex; align-items: flex-start; gap: 8px; padding: 2px 0; }
.block-row.active > .block-content { background: rgba(255,255,255,0.04); }
.block-bullet { color: var(--muted, #888); cursor: grab; user-select: none; padding-top: 4px; }
.block-content { flex: 1; outline: none; padding: 2px 4px; border-radius: 4px; min-height: 1.4em; }
.block-content:focus { background: rgba(255,255,255,0.06); }
```

- [ ] **Step 15.4: Add i18n key `block.empty`**

In each of `spa/src/i18n/{en,fr,es,de}.json` add `"block.empty"` (en: "Press Enter to create your first block.", etc.).

- [ ] **Step 15.5: Delegate from NoteEditor**

In `spa/src/components/NoteEditor.tsx`, at the start of the render branch for text notes, gate on `schema_version`:

```tsx
import { BlockEditor } from "./BlockEditor";

// inside the component, after fetching note metadata:
if (note.note_type === "text" && (note.schema_version ?? 0) >= 1) {
  return <BlockEditor noteId={note.id} />;
}
```

(If `schema_version` isn't on the `Note` interface in `api.ts`, add it: `schema_version?: number`.)

- [ ] **Step 15.6: Build SPA**

Run: `cd spa && bun run build`
Expected: build succeeds.

- [ ] **Step 15.7: Commit**

```bash
git add spa/src/
git commit -m "feat(spa): BlockEditor component with keyboard nav and edit-on-blur"
```

---

## Task 16: SPA — Auto-migration on Note Open + WS Refresh

**Files:**
- Modify: `spa/src/components/NoteEditor.tsx`
- Modify: `spa/src/components/BlockEditor.tsx`
- Modify: `spa/src/api.ts`

- [ ] **Step 16.1: Add `migrateNoteToBlocks` helper**

In `spa/src/api.ts` add:

```typescript
export async function setNoteSchemaVersion(noteId: string, schema_version: number): Promise<void> {
  const r = await authedFetch(`${BASE}/notes/${noteId}/schema-version`, {
    method: "PATCH", body: JSON.stringify({ schema_version }),
  });
  if (!r.ok) throw new Error(await r.text());
}
```

In a new file `spa/src/blocks/migrate.ts`:

```typescript
import { splitMarkdown } from "./markdown";
import { encryptBlock } from "./crypto";
import * as api from "../api";

export async function migrateNoteToBlocks(noteId: string, markdown: string): Promise<void> {
  const parts = splitMarkdown(markdown);
  const indentStack: { indent: number; id: string }[] = [];
  for (let i = 0; i < parts.length; i++) {
    const p = parts[i];
    while (indentStack.length && indentStack[indentStack.length - 1].indent >= p.indent) indentStack.pop();
    const parent_id = indentStack.length ? indentStack[indentStack.length - 1].id : null;
    const ct = await encryptBlock(noteId, p.content);
    const created = await api.createBlock(noteId, {
      parent_id, position: i, block_type: p.block_type, content_b64: ct,
    });
    indentStack.push({ indent: p.indent, id: created.id });
  }
  await api.setNoteSchemaVersion(noteId, 1);
}
```

- [ ] **Step 16.2: Trigger migration in NoteEditor**

In `spa/src/components/NoteEditor.tsx`, when opening a text note with `schema_version === 0` and `note.content` is non-empty, decrypt the existing content (using the existing helper that produces the plaintext markdown for the editor) and call `migrateNoteToBlocks(note.id, plaintext)`. On success, refetch the note and re-render with `BlockEditor`.

```tsx
useEffect(() => {
  (async () => {
    if (note.note_type === "text" && (note.schema_version ?? 0) === 0) {
      const md = await loadPlaintext(note); // existing helper used by current editor
      if (md.trim()) await migrateNoteToBlocks(note.id, md);
      await refetchNote();
    }
  })();
}, [note.id]);
```

Adapt `loadPlaintext` to whatever the current editor uses to fetch+decrypt the blob.

- [ ] **Step 16.3: WS event handling in BlockEditor**

In `BlockEditor.tsx`, subscribe to the existing WS feed (look at `SharedNotesPage.tsx` for the pattern). When an event of type `block.created|updated|moved|deleted` arrives with `note_id === noteId`, call `refresh()`.

```tsx
useEffect(() => {
  const off = subscribeWs((evt: api.WsEvent) => {
    if (typeof evt.event === "string" && evt.event.startsWith("block.") && evt.note_id === noteId) {
      refresh();
    }
  });
  return () => off();
}, [noteId]);
```

Reuse the existing `subscribeWs` helper if it exists; otherwise refactor a small one out of the page that currently uses WS.

- [ ] **Step 16.4: Build & smoke test**

Run: `cd spa && bun run build` + manual: open an existing text note, observe migration completes and tree appears; open same note in second browser session, edit one block, watch the first session refresh.

- [ ] **Step 16.5: Commit**

```bash
git add spa/src/
git commit -m "feat(spa): auto-migrate legacy notes on open + WS refresh"
```

---

## Task 17: SPA — Slash Menu and Block Type Switching

**Files:**
- Modify: `spa/src/components/BlockEditor.tsx`
- Create: `spa/src/components/SlashMenu.tsx`

- [ ] **Step 17.1: SlashMenu component**

Create `spa/src/components/SlashMenu.tsx`:

```tsx
import { useEffect, useRef, useState } from "preact/hooks";
import { t } from "../i18n";

export interface SlashItem { id: "text"|"heading"|"todo"|"quote"|"code"|"divider"; label: string; }

interface Props { onPick: (id: SlashItem["id"]) => void; onClose: () => void; }

const ITEMS: SlashItem[] = [
  { id: "heading", label: "Heading" },
  { id: "todo",    label: "Todo" },
  { id: "quote",   label: "Quote" },
  { id: "code",    label: "Code block" },
  { id: "divider", label: "Divider" },
  { id: "text",    label: "Text" },
];

export function SlashMenu({ onPick, onClose }: Props) {
  const [active, setActive] = useState(0);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "ArrowDown") { e.preventDefault(); setActive(a => (a + 1) % ITEMS.length); }
      else if (e.key === "ArrowUp") { e.preventDefault(); setActive(a => (a - 1 + ITEMS.length) % ITEMS.length); }
      else if (e.key === "Enter") { e.preventDefault(); onPick(ITEMS[active].id); }
      else if (e.key === "Escape") { e.preventDefault(); onClose(); }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [active]);

  return (
    <div class="slash-menu" ref={ref}>
      {ITEMS.map((it, i) => (
        <button class={i === active ? "active" : ""} onMouseEnter={() => setActive(i)} onClick={() => onPick(it.id)}>
          {t(`block.type.${it.id}`)}
        </button>
      ))}
    </div>
  );
}
```

- [ ] **Step 17.2: Wire slash trigger inside BlockEditor**

In `BlockEditor.tsx`, on `keydown` of `/` at the start of an empty block, show the menu. On pick, call `api.patchBlock(blockId, { block_type })`. On close, hide.

```tsx
const [slashFor, setSlashFor] = useState<string | null>(null);
// in onKeyDown, before the other handlers:
if (e.key === "/" && (e.target as HTMLElement).innerText.trim() === "") {
  e.preventDefault();
  setSlashFor((e.target as HTMLElement).closest(".block-row")!.querySelector(".block-bullet")!.getAttribute("data-id")!);
}
// rendered floating, anchored next to the active block:
{slashFor && <SlashMenu onClose={() => setSlashFor(null)} onPick={async (id) => {
  await api.patchBlock(slashFor!, { block_type: id });
  setSlashFor(null);
  await refresh();
}} />}
```

- [ ] **Step 17.3: i18n strings**

Add to each locale JSON: `"block.type.text"`, `"block.type.heading"`, `"block.type.todo"`, `"block.type.quote"`, `"block.type.code"`, `"block.type.divider"`.

- [ ] **Step 17.4: Build & commit**

```bash
cd spa && bun run build
git add spa/src/
git commit -m "feat(spa): slash menu for block type switching"
```

---

## Task 18: SPA — Drag & Drop Reorder

**Files:**
- Modify: `spa/src/components/BlockEditor.tsx`

- [ ] **Step 18.1: Add HTML5 drag handlers on the bullet**

In `BlockEditor.tsx` extend the bullet `<span>`:

```tsx
<span
  class="block-bullet"
  data-id={n.id}
  draggable
  onDragStart={(e) => e.dataTransfer!.setData("text/block-id", n.id)}
  onDragOver={(e) => { e.preventDefault(); (e.currentTarget as HTMLElement).classList.add("drop-target"); }}
  onDragLeave={(e) => (e.currentTarget as HTMLElement).classList.remove("drop-target")}
  onDrop={async (e) => {
    e.preventDefault();
    const draggedId = e.dataTransfer!.getData("text/block-id");
    if (draggedId === n.id) return;
    const newPos = n.position + 0.5;
    await api.moveBlock(draggedId, n.parent_block_id, newPos);
    await refresh();
  }}
>•</span>
```

Add a `.block-bullet.drop-target { background: var(--accent, #4af); border-radius: 3px; }` rule to `BlockEditor.css`.

- [ ] **Step 18.2: Build & commit**

```bash
cd spa && bun run build
git add spa/src/components/
git commit -m "feat(spa): drag-drop block reordering"
```

---

## Task 19: TUI — Block Tree View

**Files:**
- Create: `crates/cli/src/tui/blocks.rs`
- Modify: `crates/cli/src/tui/mod.rs`
- Modify: `crates/cli/src/tui/app.rs`
- Modify: `crates/cli/src/tui/ui.rs`

- [ ] **Step 19.1: Add block panel state**

In `crates/cli/src/tui/app.rs` add:

```rust
pub struct BlockPanel {
    pub note_id: Option<uuid::Uuid>,
    pub blocks: Vec<jot_core::models::Block>,
    pub plaintexts: std::collections::HashMap<uuid::Uuid, String>,
    pub cursor: usize,
}

impl BlockPanel {
    pub fn new() -> Self {
        Self { note_id: None, blocks: vec![], plaintexts: Default::default(), cursor: 0 }
    }
}
```

Add a `pub block_panel: BlockPanel` field to the main `App` struct and initialize it.

- [ ] **Step 19.2: Loader and renderer**

Create `crates/cli/src/tui/blocks.rs`:

```rust
use crate::client::Client;
use jot_core::models::Block;
use ratatui::{layout::Rect, prelude::*, widgets::{Block as TBlock, Borders, Paragraph}};
use uuid::Uuid;

pub async fn load(client: &Client, note: Uuid) -> anyhow::Result<(Vec<Block>, std::collections::HashMap<Uuid,String>)> {
    let blocks = client.list_blocks(note).await?;
    // For the TUI MVP, content stays as ciphertext bytes — decrypt with client helper.
    let mut decrypted = std::collections::HashMap::new();
    for b in &blocks {
        let plain = client.decrypt_with_note_dek(note, &b.content).await
            .unwrap_or_else(|_| b"<decryption failed>".to_vec());
        decrypted.insert(b.id, String::from_utf8_lossy(&plain).into_owned());
    }
    Ok((blocks, decrypted))
}

pub fn render(f: &mut Frame, area: Rect, panel: &crate::tui::app::BlockPanel) {
    let mut lines: Vec<Line> = Vec::new();
    let by_parent = group_by_parent(&panel.blocks);
    walk(&by_parent, None, 0, &panel.plaintexts, panel.cursor, 0, &mut lines);
    let p = Paragraph::new(lines).block(TBlock::default().borders(Borders::ALL).title("Blocks"));
    f.render_widget(p, area);
}

fn group_by_parent(blocks: &[Block]) -> std::collections::HashMap<Option<Uuid>, Vec<&Block>> {
    let mut m: std::collections::HashMap<Option<Uuid>, Vec<&Block>> = Default::default();
    for b in blocks { m.entry(b.parent_block_id).or_default().push(b); }
    for v in m.values_mut() { v.sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap()); }
    m
}

fn walk<'a>(
    by_parent: &std::collections::HashMap<Option<Uuid>, Vec<&'a Block>>,
    parent: Option<Uuid>,
    depth: usize,
    pts: &std::collections::HashMap<Uuid, String>,
    cursor: usize,
    mut idx: usize,
    out: &mut Vec<Line<'a>>,
) -> usize {
    if let Some(kids) = by_parent.get(&parent) {
        for k in kids {
            let prefix = "  ".repeat(depth);
            let style = if idx == cursor { Style::default().add_modifier(Modifier::REVERSED) } else { Style::default() };
            let text = pts.get(&k.id).cloned().unwrap_or_default();
            out.push(Line::from(Span::styled(format!("{}• {}", prefix, text), style)));
            idx += 1;
            idx = walk(by_parent, Some(k.id), depth + 1, pts, cursor, idx, out);
        }
    }
    idx
}
```

- [ ] **Step 19.3: Add CLI client decrypt helper**

In `crates/cli/src/client.rs`:

```rust
impl Client {
    pub async fn decrypt_with_note_dek(&self, note: uuid::Uuid, ct: &[u8]) -> anyhow::Result<Vec<u8>> {
        let dek = self.note_dek(note).await?;
        Ok(jot_core::crypto::aead::decrypt(ct, &dek)?)
    }
}
```

(Use the actual decrypt function name from `jot_core::crypto`.)

- [ ] **Step 19.4: Wire into UI**

In `crates/cli/src/tui/ui.rs`, where the note content panel is drawn for text notes, replace it with `crate::tui::blocks::render(f, area, &app.block_panel)`.

In `crates/cli/src/tui/mod.rs` event loop, when the selected note changes, kick off `crate::tui::blocks::load(...)` and store the result in `app.block_panel`.

- [ ] **Step 19.5: Build**

Run: `cargo build -p cli`
Expected: clean build.

- [ ] **Step 19.6: Commit**

```bash
git add crates/cli/
git commit -m "feat(tui): hierarchical block tree view"
```

---

## Task 20: TUI — Block Keys (j/k/o/>/</dd/yy/Enter/za)

**Files:**
- Modify: `crates/cli/src/tui/mod.rs`
- Modify: `crates/cli/src/tui/app.rs`

- [ ] **Step 20.1: Add a vim-like input state machine**

In `app.rs` add to `BlockPanel`:

```rust
pub pending: Option<char>, // for d-d and y-y
```

- [ ] **Step 20.2: Add key handler**

In `crates/cli/src/tui/mod.rs`, inside the keyboard match for the block panel, add:

```rust
use crossterm::event::KeyCode;
// Assume `key` is the KeyEvent and `app.block_panel` exists.
match key.code {
    KeyCode::Char('j') => { if app.block_panel.cursor + 1 < flat_len(&app.block_panel) { app.block_panel.cursor += 1; } }
    KeyCode::Char('k') => { if app.block_panel.cursor > 0 { app.block_panel.cursor -= 1; } }
    KeyCode::Char('o') => {
        // create empty block below current cursor
        let note = app.block_panel.note_id.unwrap();
        let cur = current_block(&app.block_panel);
        let position = cur.map(|c| c.position + 0.5);
        let parent  = cur.and_then(|c| c.parent_block_id);
        let ct = encrypt_empty(&client, note).await?;
        let _ = client.create_block(note, parent, position, "text", &ct, None).await;
        reload_blocks(app, &client).await;
    }
    KeyCode::Char('>') => {
        if let Some(c) = current_block(&app.block_panel) { let _ = client.indent_block(c.id).await; reload_blocks(app, &client).await; }
    }
    KeyCode::Char('<') => {
        if let Some(c) = current_block(&app.block_panel) { let _ = client.outdent_block(c.id).await; reload_blocks(app, &client).await; }
    }
    KeyCode::Char('d') => {
        if app.block_panel.pending == Some('d') {
            if let Some(c) = current_block(&app.block_panel) { let _ = client.delete_block(c.id).await; reload_blocks(app, &client).await; }
            app.block_panel.pending = None;
        } else { app.block_panel.pending = Some('d'); }
    }
    KeyCode::Char('y') => {
        if app.block_panel.pending == Some('y') {
            if let Some(c) = current_block(&app.block_panel) {
                let _ = arboard::Clipboard::new().and_then(|mut cb| cb.set_text(format!("(({}))", c.id)));
            }
            app.block_panel.pending = None;
        } else { app.block_panel.pending = Some('y'); }
    }
    KeyCode::Char('a') if app.block_panel.pending == Some('z') => {
        if let Some(c) = current_block(&app.block_panel) {
            let target = !c.collapsed;
            // optimistic local; persisted by patchBlock { collapsed }
            // (collapse is local; if you choose to persist, call patch_block)
            // For MVP, just refetch.
            let _ = client.patch_block_collapse(c.id, target).await;
            reload_blocks(app, &client).await;
        }
        app.block_panel.pending = None;
    }
    KeyCode::Char('z') => { app.block_panel.pending = Some('z'); }
    KeyCode::Enter => {
        if let Some(c) = current_block(&app.block_panel) {
            let plain = app.block_panel.plaintexts.get(&c.id).cloned().unwrap_or_default();
            let edited = crate::editor::edit_in_editor(&plain)?;
            let ct = encrypt_block_text(&client, c.note_id, &edited).await?;
            let _ = client.patch_block(c.id, None, Some(&ct)).await;
            reload_blocks(app, &client).await;
        }
    }
    _ => { app.block_panel.pending = None; }
}
```

Helper definitions (`current_block`, `flat_len`, `reload_blocks`, `encrypt_block_text`, `encrypt_empty`) go alongside or in `app.rs` — model them on existing helpers in the TUI module.

- [ ] **Step 20.3: Add `patch_block_collapse` on the API + client**

Reuse `PATCH /blocks/:id` with `{ collapsed: true|false }` — already wired. Add CLI helper:

```rust
impl Client {
    pub async fn patch_block_collapse(&self, id: uuid::Uuid, collapsed: bool) -> anyhow::Result<()> {
        let body = serde_json::json!({ "collapsed": collapsed });
        self.patch(&format!("/blocks/{}", id), &body).await?;
        Ok(())
    }
}
```

- [ ] **Step 20.4: Add `arboard` dep if missing**

Check `crates/cli/Cargo.toml`. If absent:

```toml
arboard = "3"
```

- [ ] **Step 20.5: Build & manual test**

Run: `cargo build -p cli` then `cargo run -p cli -- serve` in one shell and `cargo run -p cli -- ui` in another (or whatever the TUI entry is). Open a migrated note, navigate, create/indent/outdent/delete blocks, yank a ref, paste in another block via `Enter` → edit → save.

- [ ] **Step 20.6: Commit**

```bash
git add crates/cli/
git commit -m "feat(tui): block-aware keybindings (j/k/o/>/</dd/yy/za/Enter)"
```

---

## Task 21: Note Title Editing (E2E)

**Files:**
- Modify: `crates/api/src/routes/notes.rs`
- Modify: `crates/api/src/routes/mod.rs`
- Modify: `crates/cli/src/client.rs`
- Modify: `spa/src/api.ts`
- Modify: `spa/src/components/NoteEditor.tsx`

- [ ] **Step 21.1: API — `PATCH /notes/:id/title`**

In `crates/api/src/routes/notes.rs`:

```rust
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct PatchTitleBody { pub title_b64: Option<String> }

pub async fn patch_note_title(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
    Json(body): Json<PatchTitleBody>,
) -> Result<StatusCode, ApiError> {
    let perm = state.db.note_permission_for(id.to_string().as_str(), &auth.0.identity_id).await?;
    if !permission_allows(&perm, "write") { return Err(ApiError::Forbidden("no write".into())); }
    let bytes = match body.title_b64 {
        Some(s) => Some(base64::engine::general_purpose::STANDARD.decode(s)
            .map_err(|_| ApiError::BadRequest("bad base64".into()))?),
        None => None,
    };
    state.db.update_note_title(id, bytes.as_deref()).await?;
    Ok(StatusCode::NO_CONTENT)
}
```

Register: `.route("/notes/:id/title", patch(notes::patch_note_title))`.

- [ ] **Step 21.2: Include `title_b64` in `NoteMetadata`**

Add to `NoteMetadata` (in `notes.rs`):

```rust
    /// base64-encoded ciphertext of the title, if set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_b64: Option<String>,
```

…and populate it in `to_metadata`:

```rust
        title_b64: note.title.as_ref().map(|t| base64::engine::general_purpose::STANDARD.encode(t)),
```

- [ ] **Step 21.3: SPA — title editor in BlockEditor header**

In `BlockEditor.tsx` add an editable title row at the top:

```tsx
const [title, setTitle] = useState("");
useEffect(() => {
  (async () => {
    if (note.title_b64) setTitle(await decryptBlock(note.id, note.title_b64));
    else setTitle("");
  })();
}, [note.id]);

const saveTitle = async () => {
  const ct = await encryptBlock(note.id, title);
  await api.patchNoteTitle(note.id, ct);
};

// In JSX:
<input class="block-title" value={title} onInput={(e) => setTitle((e.target as HTMLInputElement).value)} onBlur={saveTitle} placeholder={t("block.title_placeholder")} />
```

Add `patchNoteTitle` in `spa/src/api.ts`:

```typescript
export async function patchNoteTitle(noteId: string, title_b64: string): Promise<void> {
  const r = await authedFetch(`${BASE}/notes/${noteId}/title`, {
    method: "PATCH", body: JSON.stringify({ title_b64 }),
  });
  if (!r.ok) throw new Error(await r.text());
}
```

Add `Note.title_b64?: string` to the interface and ensure list/detail endpoints serialize it.

- [ ] **Step 21.4: CLI — `jot note title <id> <text>`**

In a new subcommand or under an existing note command, add:

```rust
// crates/cli/src/commands/note_title.rs (or add to existing notes command)
pub async fn run(client: &Client, note: Uuid, text: String) -> anyhow::Result<()> {
    let ct = client.encrypt_with_note_dek(note, text.as_bytes()).await?;
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&ct);
    client.patch(&format!("/notes/{}/title", note), &serde_json::json!({ "title_b64": b64 })).await?;
    Ok(())
}
```

Hook into `Command::Note { title: { id, text } }` in `main.rs`.

- [ ] **Step 21.5: Build & commit**

```bash
cargo build --workspace && (cd spa && bun run build)
git add crates/ spa/
git commit -m "feat: note title with E2E encryption"
```

---

## Task 22: End-to-End Integration Tests

**Files:**
- Create: `crates/api/tests/blocks_e2e.rs`

- [ ] **Step 22.1: Write an end-to-end happy-path test**

Use the existing `crates/api/tests/` harness (look at one passing test file for the exact `TestApp::start()` / auth-token setup). Then:

```rust
// 1. Register identity + device, get a token.
// 2. Create a board.
// 3. Create a note (text).
// 4. POST /notes/{id}/blocks twice; assert created.
// 5. GET /notes/{id}/blocks; assert length = 2 and order by position.
// 6. PATCH first block content; assert updated_at advanced.
// 7. POST /blocks/{id}/move under the other one; assert parent_block_id changed.
// 8. PUT /blocks/{id}/links with one TAG; GET /tags/foo/blocks returns this id.
// 9. GET /blocks/{id}/backlinks: empty until we create another block linking to it.
// 10. DELETE first; GET returns 404.
```

Each step is one assertion or a small group. Aim for ~80–120 lines, follow the existing harness style.

- [ ] **Step 22.2: Run**

Run: `cargo test -p api blocks_e2e -- --nocapture`
Expected: PASS.

- [ ] **Step 22.3: Commit**

```bash
git add crates/api/tests/blocks_e2e.rs
git commit -m "test(api): end-to-end blocks happy path"
```

---

## Task 23: Documentation Update

**Files:**
- Modify: `CLAUDE.md` (only if a structural item changed)
- Create: `docs/blocks.md`

- [ ] **Step 23.1: Write `docs/blocks.md`**

Write a short developer doc (≤120 lines) that:
1. Summarises the schema (link to migration).
2. Documents the API surface (link to the OpenAPI/utoipa-generated docs).
3. Shows the link/embed syntax (`[[Page]]`, `((id))`, `!((id))`, `#tag`).
4. Lists the keyboard shortcuts in SPA and TUI.
5. Explains the lazy migration flow.

- [ ] **Step 23.2: Commit**

```bash
git add docs/blocks.md
git commit -m "docs: block structure developer guide"
```

---

## Self-Review

**Spec coverage:**

- Tables `blocks` / `block_links` / `tags` → Task 1.
- Block model + `Note` extensions → Task 2.
- Storage CRUD/move/indent/outdent → Tasks 3, 4, 5 (+ `previous_sibling` in Task 9.2).
- Markdown→blocks splitter → Task 6.
- Link extraction → Task 7.
- API CRUD → Task 8.
- API indent/outdent/backlinks/tags/links → Task 9.
- WS events → Task 10.
- CLI subcommands → Task 11.
- SPA api client → Task 12.
- SPA tree/markdown/crypto helpers → Task 13.
- CLI migration → Task 14.
- SPA BlockEditor + keymap → Task 15.
- SPA auto-migration + WS → Task 16.
- SPA slash menu → Task 17.
- SPA drag-drop → Task 18.
- TUI tree view → Task 19.
- TUI keys → Task 20.
- Note title E2E → Task 21.
- E2E test → Task 22.
- Docs → Task 23.

**Placeholder scan:** No `TBD`/`TODO` placeholders. Sections referencing existing helpers (e.g. `loadPlaintext`, `subscribeWs`, `note_permission_for`) name them explicitly with instructions to adapt to the codebase's actual signatures — that is by design, not a placeholder.

**Type consistency:** `BlockType::from_str` is used identically across Rust and SPA layers. `position: f64` in Rust matches `number` in TS. The `block_type` strings are the same set in all surfaces: `text|heading|todo|quote|code|embed|divider`. `link_kind` strings: `page_ref|block_ref|block_embed|tag`. `target_kind` strings: `note|block|tag`.

**Scope:** Plan stays inside Feature 2 (block structure). Backlinks UI, graph view, journal, CRDT, per-block permissions, per-block comments, and version history are intentionally absent (covered by separate future specs).

---

**Plan complete and saved to `docs/superpowers/plans/2026-05-11-block-structure.md`. Two execution options:**

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints.

**Which approach?**
