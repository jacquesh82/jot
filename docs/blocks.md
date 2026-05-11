# Block Structure — Developer Guide

Block-structured notes (Outliner / Logseq-style). For the full architecture and
rationale, see the design spec:
[`docs/superpowers/specs/2026-05-11-block-structure-design.md`](superpowers/specs/2026-05-11-block-structure-design.md).

## Schema

Migration: [`crates/storage/migrations/0008_blocks.sql`](../crates/storage/migrations/0008_blocks.sql).

New tables:

- **`blocks`** — `id`, `note_id`, `parent_block_id?`, `position` (float, fractional
  reordering), `block_type` (`text|heading|todo|quote|code|embed|divider`),
  `content` (encrypted BLOB), `metadata?`, `collapsed`, timestamps. FKs cascade
  from `notes` and self-cascade from `parent_block_id`.
- **`block_links`** — edges from a source block to a target. Columns:
  `source_block_id`, `target_kind` (`note|block|tag`), `target_id`, `link_kind`
  (`page_ref|block_ref|block_embed|tag`). `UNIQUE(source, target_kind, target_id, link_kind)`.
- **`tags`** — `(name, identity_id)` primary key, optional `color`. FK on
  `identity_id` cascades from `identities`.

New columns on `notes`:

- `title BLOB` — encrypted title (separate from body).
- `is_journal INTEGER` + `journal_date TEXT` — reserved for journal entries.
- `schema_version INTEGER` — `0` = legacy text note, `1` = block-structured.

## API

Browse the live spec at **`/swagger-ui`** (OpenAPI JSON at `/api-docs/openapi.json`).
Wired in [`crates/api/src/routes/mod.rs`](../crates/api/src/routes/mod.rs).

Blocks CRUD:

- `GET    /notes/:note_id/blocks` — flat list.
- `POST   /notes/:note_id/blocks` — create.
- `GET    /blocks/:id`            — fetch one.
- `PATCH  /blocks/:id`            — update content/type/metadata/collapsed.
- `DELETE /blocks/:id`            — cascade-deletes descendants.

Tree ops:

- `POST   /blocks/:id/move`       — `{ to_parent?, position }`.
- `POST   /blocks/:id/indent`     — slot under previous sibling.
- `POST   /blocks/:id/outdent`    — move under grandparent.

Links & tags:

- `PUT    /blocks/:id/links`      — replace link edges for a block.
- `GET    /blocks/:id/backlinks`  — blocks linking here.
- `GET    /notes/:id/backlinks`   — blocks linking to any block in this note.
- `GET    /tags`, `PUT /tags/:name`, `GET /tags/:name/blocks`.

Notes extensions:

- `PATCH  /notes/:id/title`            — encrypted title.
- `PATCH  /notes/:id/schema-version`   — flip 0 → 1 after migration.
- `GET    /notes/legacy-text`          — list notes still on schema 0.

WebSocket events (see [`routes/ws.rs`](../crates/api/src/routes/ws.rs)):
`block.created | block.updated | block.deleted | block.moved`.

## Link syntax

Parsed by [`crates/core/src/blocks/links.rs`](../crates/core/src/blocks/links.rs)
and produces `block_links` rows on save.

| Syntax                                  | Meaning           | `link_kind`   | `target_kind` |
|-----------------------------------------|-------------------|---------------|---------------|
| `[[Page Name]]`                         | Page reference    | `page_ref`    | `note`        |
| `((550e8400-e29b-41d4-a716-...))`       | Block reference   | `block_ref`   | `block`       |
| `!((550e8400-e29b-41d4-a716-...))`      | Inline embed      | `block_embed` | `block`       |
| `#tag`                                  | Tag               | `tag`         | `tag`         |

Example block content:

```
See [[Roadmap]] and ((550e8400-e29b-41d4-a716-446655440000)) — #urgent
!((550e8400-e29b-41d4-a716-446655440001))
```

Yields four `block_links` rows: one `page_ref` → note "Roadmap", one
`block_ref`, one `tag` (`urgent`), one `block_embed`.

## SPA shortcuts

Implemented in [`spa/src/blocks/BlockEditor.tsx`](../spa/src/blocks/BlockEditor.tsx).

| Key            | Action                                   |
|----------------|------------------------------------------|
| `Enter`        | Split at cursor / new sibling block      |
| `Tab`          | Indent under previous sibling            |
| `Shift+Tab`    | Outdent under grandparent                |
| `Backspace`    | At col 0 → merge into previous block     |
| `Cmd/Ctrl + .` | Toggle collapse on a block               |
| `/`            | Open slash menu (block-type switch)      |
| Drag bullet    | Reorder/reparent (HTML5 drag and drop)   |

## TUI shortcuts

Implemented in [`crates/tui/src/blocks/`](../crates/tui/src/blocks/).

| Key        | Action                                   |
|------------|------------------------------------------|
| `j` / `k`  | Move cursor down / up                    |
| `o`        | New block below cursor                   |
| `>` / `<`  | Indent / outdent block                   |
| `dd`       | Delete block (cascades to children)      |
| `yy`       | Yank block id (for `((id))` paste)       |
| `za`       | Toggle collapse                          |
| `Enter`    | Edit block content (in `$EDITOR`)        |

## Lazy migration

Notes created before the block feature have `schema_version = 0` and store
their markdown in the legacy `notes.blob` column. They are migrated on demand,
**client-side**, because content is end-to-end encrypted — the server has no
access to plaintext.

Two entry points:

- **SPA** — on first open, the editor detects `schema_version = 0`, decrypts the
  blob, splits via `splitMarkdown()`, creates blocks via the API (re-encrypted
  with the same DEK), then PATCHes `schema_version = 1`. See
  [`spa/src/blocks/migrate.ts`](../spa/src/blocks/migrate.ts).
- **CLI** — `jot block migrate --all` (or `--note <id>`, `--dry-run`). Runs the
  same logic over `GET /notes/legacy-text`. See
  [`crates/cli/src/commands/block.rs`](../crates/cli/src/commands/block.rs).

Migration is idempotent: once `schema_version = 1`, the note is skipped.
