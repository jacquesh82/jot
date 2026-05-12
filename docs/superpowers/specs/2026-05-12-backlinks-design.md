# Backlinks & Knowledge Graph — Design

**Status:** Approved — ready for implementation plan
**Date:** 2026-05-12
**Scope:** Feature 1 of the knowledge-graph roadmap. Builds on Feature 2 (block structure). Adds full `[[Page]]` / `((id))` / `!((id))` / `#tag` semantics, backlinks UI, virtual tag pages, and semantic suggestions via ruvector.

---

## 1. Goals

Turn `jot` into a Logseq-style knowledge graph while preserving the local-first / single-binary deployment model:

- Every link syntax becomes interactive (click navigates / opens / inserts).
- The link graph populates automatically on block save — no manual API call.
- A "Linked references" section under each note shows incoming links with context.
- Clicking a non-existent `[[Page]]` auto-creates a note in a dedicated `Pages` board.
- `#tag` opens a virtual page listing all tagged blocks.
- `((block-id))` jumps to the target block in its note.
- `!((block-id))` renders the target block inline (read-only embed).
- `[[` and `#` typing triggers an autocomplete dropdown.
- A "Suggestions" section under each note proposes semantically related blocks via ruvector.

## 2. Non-goals (deferred)

- Block-level CRDT collaboration (still pending from Feature 2 deferral).
- Graph view / interactive node-link diagram (Feature 3).
- Daily journal pages (Feature 4).
- Title-rename cascading: renaming a note's title does **not** auto-rewrite `[[OldTitle]]` references in other blocks (manual operation, with reconcile endpoint as a workaround).
- Tag aliases, hierarchies, or namespaces.
- Authority decay / relevance ranking of backlinks beyond chronological order.

## 3. Resolution model

### 3.1 `[[Page]]` is identity-global

`[[Title]]` resolves to a note titled "Title" anywhere in the user's identity (across all boards owned by the identity). Conflicts (two notes with the same title) are resolved as **most-recently-updated wins**, with a `conflicts` array exposing the other ids for an "⚠ Ambiguous" hover hint.

Because titles are E2E-encrypted, the **client** owns the title→note_id index. The server cannot resolve `[[]]` directly.

### 3.2 Client-side title index

A `Map<lowercase_title, note_id>` maintained in memory by the SPA:

- Populated on session start by fetching all `notes` for the identity and decrypting each `title_b64`.
- Mutated on every WebSocket `note_*` event and on local `patchNoteTitle` calls.
- Persisted optionally to `localStorage` for fast startup (best-effort cache, always rebuildable).

For large identities (>10K notes), each title is ~50 bytes ciphertext + 12-byte nonce → ~620 KB to fetch + decrypt at startup. Acceptable.

### 3.3 Future-friendly optimisation (deferred)

If startup cost grows unacceptable: switch to `title_hash = HMAC-SHA256(lowercase_title, identity.derived_key)`, store as `notes.title_hash` plaintext, server-side O(1) lookup. Not in this spec — flagged for future.

## 4. Data model

### 4.1 New migration `0009_backlinks.sql`

```sql
ALTER TABLE boards ADD COLUMN board_kind TEXT NOT NULL DEFAULT 'regular';
-- 'regular' | 'pages' | 'journal' (Feature 4 reserves 'journal')

ALTER TABLE identities ADD COLUMN embeddings_enabled INTEGER NOT NULL DEFAULT 0;
-- Per-identity opt-in for the semantic-suggestion pipeline (sends plaintexts to server)

CREATE TABLE block_embeddings (
    block_id      TEXT PRIMARY KEY,
    embedding_id  TEXT NOT NULL,        -- id assigned by the ruvector index
    text_hash     TEXT NOT NULL,        -- sha256(plaintext) — skip re-embed if unchanged
    updated_at    TEXT NOT NULL,
    FOREIGN KEY (block_id) REFERENCES blocks(id) ON DELETE CASCADE
);
CREATE INDEX idx_block_embeddings_hash ON block_embeddings(text_hash);
```

### 4.2 New link_kind value: `page_ref_unresolved`

`block_links.link_kind` gains the value `'page_ref_unresolved'`. Semantics: a `[[Title]]` that did not match any existing note at extraction time. `target_kind='note'`, `target_id=lowercase_title`. When a note is later created with that title, a server-side reconcile flips all matching rows to `link_kind='page_ref'` with `target_id=note.id`.

### 4.3 `Pages` board lifecycle

- Created lazily on first `POST /pages` (auto-create-by-title endpoint).
- One per identity, marked by `boards.board_kind = 'pages'`.
- Server enforces uniqueness: a second attempt to create a second `pages` board for the same identity returns the existing one.
- Surfaced in the sidebar with a distinct icon (📄) and pinned to the top, above regular boards.

## 5. Auto-extraction pipeline

### 5.1 On block save

Both SPA and CLI, every time a block is created or its content patched:

1. Parse plaintext → list of `ExtractedLink { target_kind, target_id, link_kind }`.
2. Resolve each `[[Title]]` against the local title index:
   - Match → `target_id = note.id`, `link_kind = 'page_ref'`.
   - No match → `target_id = lowercase_title`, `link_kind = 'page_ref_unresolved'`.
3. `PUT /blocks/:id/links` with the full resolved set (idempotent replace, already implemented).

Rust parser already exists (`jot_core::blocks::extract_links`). SPA gains `spa/src/blocks/links.ts` mirroring its logic and tested for parity.

### 5.2 Reconciling unresolved links when a page appears

When a note is created (any path), the server immediately runs:

```sql
UPDATE block_links
   SET target_id = ?new_note_id,
       link_kind = 'page_ref'
 WHERE link_kind = 'page_ref_unresolved'
   AND target_kind = 'note'
   AND target_id = ?lowercase_title;
```

Exposed as `POST /links/reconcile-title { title, note_id }` so the CLI can also trigger it.

### 5.3 Embedding pipeline

For every text block with ≥30 characters of plaintext, after save:

1. Client computes `sha256(plaintext)`.
2. Client calls `POST /embeddings/upsert { block_id, plaintext }`.
3. Server: if existing `text_hash == sha256(plaintext)` → no-op. Otherwise → `ruvector.embed(plaintext)` → upsert `block_embeddings(block_id, embedding_id, text_hash, updated_at=now)`.

**Privacy compromise:** the server sees the plaintext on embed. Embedding is gated by a per-identity opt-in toggle (Settings → "Enable semantic suggestions"), off by default. The toggle surfaces a clear warning before enabling. When off:
- Client skips the `POST /embeddings/upsert` call.
- Server `GET /suggestions/related` returns 200 with `enabled: false` and an empty list.

Re-index endpoint `POST /embeddings/reindex` walks all blocks of the calling identity and embeds those without an entry; used after migration or fresh install.

## 6. API additions

```
GET    /resolve/page?title=<t>          → { matches: [{ note_id, board_id }], conflicts_count }
POST   /pages                           → body: { title }
                                           creates or returns the Pages board for the caller,
                                           creates a note in it with the given (encrypted) title,
                                           returns { note_id, board_id, created: bool }
POST   /links/reconcile-title           → body: { title, note_id }
                                           server-side bulk update of unresolved page refs

POST   /embeddings/upsert               → body: { block_id, plaintext }
                                           204 on success
GET    /embeddings/status               → { enabled: bool, indexed: number, total: number }
POST   /embeddings/reindex              → 202 Accepted (kicks off a background job)
GET    /suggestions/related             → ?note_id=X or ?tag=Y; ?top_k=5 (default)
                                           returns [{ block_id, source_note_id, score, snippet_b64 }]
```

WS events: `page_resolved` (after reconcile-title, so clients can refresh edges), `embedding_indexed` (per block, for progress bar in Settings).

## 7. Rendering

### 7.1 Two-mode block rendering

`spa/src/blocks/render.ts` exports `renderBlockContent(plaintext, mode)`:

- `mode === "edit"` (block has focus) → emits raw plaintext into `contentEditable` so the user can type.
- `mode === "view"` (block does not have focus) → emits HTML where every link pattern is wrapped:
  - `[[Title]]` → `<a class="page-link" data-note-id="…" data-title="Title">Title</a>` (resolved)
  - `[[Title]]` unresolved → `<a class="page-link unresolved" data-title="Title">Title</a>` (italic, accent border)
  - `((uuid))` → `<a class="block-link" data-block-id="…">⛓ <span class="block-snippet">…</span></a>` (snippet loaded lazily)
  - `!((uuid))` → `<EmbedBlock block_id="…" />` mount point — rendered as a separate component in a second pass
  - `#tag` → `<a class="tag-link" data-tag="…">#tag</a>`

The switch is driven by `editingBlockId: string | null` state and a per-block `onFocus`/`onBlur`. Caret position is preserved on focus by saving range before the swap and restoring after.

### 7.2 Click delegation

Single `onClick` on the `.block-list` container, dispatches via `event.target.closest("[data-...]")`:

- `.page-link[data-note-id]` → `selectedNoteId.value = note_id`; if note is in another board, also `selectedBoardId.value = board_id`.
- `.page-link.unresolved` → `POST /pages { title }` → wait for response → navigate to the new note id.
- `.block-link[data-block-id]` → resolve block → load its note → scroll to block id, flash highlight 1.5s.
- `.tag-link[data-tag]` → navigate to `#/tag/:name`.

### 7.3 EmbedBlock component

`spa/src/components/EmbedBlock.tsx` fetches `GET /blocks/:id`, decrypts via the embedded block's note DEK (requires knowing its `note.board_id`, fetched alongside), renders read-only. Cycle protection: an embed of an embed renders only one level deep; deeper embeds show a 🔁 placeholder.

## 8. Autocomplete

`spa/src/components/LinkAutocomplete.tsx` — generic dropdown. Triggered by `BlockEditor`'s input listener detecting:

- `[[` followed by 0+ chars and no closing `]]` between trigger and caret → trigger="page", source=titleIndex
- `#` at word boundary followed by chars → trigger="tag", source=`GET /tags` cache

Positioned absolutely under the caret using `getBoundingClientRect()` of a temporary marker. Max 5 results, plus an "➕ Create …" row when there's a non-empty query and no exact match. Keyboard: ArrowUp/Down navigation, Enter selects, Escape closes.

Selection behavior:
- Page existing → inserts `[[Title]]` (closing brackets added).
- Page creating → inserts `[[Title]]` and a `POST /pages` is queued; the link resolves once `page_resolved` WS event arrives.
- Tag → inserts `#tagname`.

## 9. Backlinks section

`spa/src/components/BacklinksSection.tsx` placed under `.block-list` in `BlockEditor`. Two collapsible subsections:

1. **Linked references (N)** — open by default.
   - Source: `GET /notes/:id/backlinks`.
   - Group by `source_note_id`. Each group header shows the source note title (decrypted). Each entry: the source block, with the `[[Title]]` of the current page highlighted.
   - Click an entry → navigate to source note + scroll to source block.

2. **Unlinked references (N)** — collapsed by default, computed lazily.
   - Activation: user clicks to expand.
   - Client-side scan: walk every block in the current title index's notes, decrypt content, find substring matches of the current title that are NOT inside a `[[…]]`.
   - Capped at the 200 most-recently-updated notes of the identity (configurable).
   - One-click action per entry: "Convert to link" → replaces the substring with `[[Title]]` and persists.

## 10. Tag page

Route: `#/tag/:name`. Component: `spa/src/components/TagPage.tsx`.

Sections:
1. **Header**: `#name`, color swatch (editable, `PUT /tags/:name { color }`), tag block count.
2. **Tagged blocks**: `GET /tags/:name/blocks` → fetch each block → decrypt → group by note → render with click-to-jump.
3. **Suggestions**: `GET /suggestions/related?tag=name` → top-5 semantically close blocks not yet tagged. Each shows a "+ Add #name to this block" button → patches the block's content to append `#name` and re-extracts.

Page is read-only (no editable body). If the user wants a "topic page" with text, they create a regular note titled `#name` — a future enhancement can auto-merge if both exist.

## 11. Suggestions section

`spa/src/components/SuggestionsSection.tsx`, below `BacklinksSection`. Collapsed by default. When opened or when the note is opened with the toggle on, calls `GET /suggestions/related?note_id=…`.

Each entry renders: source-note title + block snippet + score. Actions:
- **+ Link**: insert `((block-id))` at the end of the most-recently-focused block of the current note. If no block has been focused since the note opened, append a new block at the end with just that ref.
- **Open**: navigate to source.

Throttle: only refreshed when the user explicitly hits "↻", or on note open. Not refreshed on every keystroke.

## 12. Settings: opt-in toggle

New section in `WhoamiPage.tsx`:

```
🧠 Semantic suggestions

[ ] Enable semantic suggestions
    Warning: enabling sends your block plaintexts to the server so it can
    compute embeddings via ruvector. This breaks the end-to-end encryption
    property for the suggestion pipeline only — your block bodies, titles,
    and metadata remain E2E-encrypted at rest. Disabling this option does
    not delete already-computed embeddings; use [Delete all embeddings] for that.

[Delete all embeddings] [Re-index now]
```

Toggle state stored in `identities.embeddings_enabled` (new column, NOT NULL DEFAULT 0).

## 13. Functional parity matrix

| Capability | API | CLI | SPA | TUI |
|---|---|---|---|---|
| Auto-extract links on save | ✅ via existing `PUT /blocks/:id/links` | ✅ in `create_block_encrypted` / `patch_block` | ✅ in `persistEdit` | ⏸ defer (no autocomplete in TUI; manual ((id)) yank still works) |
| Resolve `[[Page]]` to note_id | ✅ `/resolve/page` | ✅ `jot page resolve <title>` | ✅ via local index | ⏸ |
| Create page on demand | ✅ `POST /pages` | ✅ `jot page create <title>` | ✅ on click + on autocomplete pick | ⏸ |
| List backlinks of a note | ✅ existing | ✅ existing | ✅ in BacklinksSection | ✅ existing (yy / view) |
| Render clickable links | n/a | n/a | ✅ render.ts | ❌ TUI keeps plaintext |
| Render inline embeds | n/a | n/a | ✅ EmbedBlock | ❌ |
| Tag page | ✅ existing routes | ✅ `jot tag list/show` | ✅ TagPage component | ❌ |
| Autocomplete `[[` `#` | n/a | n/a | ✅ LinkAutocomplete | ❌ |
| Semantic suggestions | ✅ `/suggestions/related` | ✅ `jot suggestions --note <id>` | ✅ SuggestionsSection | ❌ |

TUI parity for Feature 1 is intentionally minimal — the surface targets vim users who write raw and grep results elsewhere. A follow-up spec can add `:Backlinks`-style commands if demand emerges.

## 14. Migration & rollout

1. Land `0009_backlinks.sql`.
2. Land API endpoints (additive; no breaking change).
3. Land SPA `extractLinks` + title index + auto-extraction on save (no UI changes yet — graph starts populating silently).
4. Land BlockEditor render.ts (clickable view-mode rendering).
5. Land BacklinksSection + delegation click handlers.
6. Land EmbedBlock, TagPage, LinkAutocomplete.
7. Land SuggestionsSection + Settings toggle. Embedding pipeline ships disabled-by-default.
8. After two minor releases with no rollback, enable a one-time client-side reconcile job: for every block in identity, call `PUT /blocks/:id/links` to backfill the graph from existing block contents.

## 15. Testing

- **Unit (TS):** `extractLinks` parity with Rust (same 4 fixtures); title index update on WS events.
- **Unit (Rust):** `reconcile-title` SQL idempotency; `Pages` board auto-creation uniqueness.
- **Integration (API):** create note → `[[NewPage]]` → reconcile → backlink visible from source.
- **Integration (E2E):** Playwright on the SPA — type `[[`, pick from dropdown, navigate via click, see backlinks update.
- **Manual:** semantic suggestions return non-trivial results on a seeded dataset.

## 16. Open questions deferred

- Title rename behavior when other blocks reference old title — for MVP, refs stay unchanged (still resolved by note_id if they were `page_ref`, become "stale" labels). User can manually rewrite or invoke a future "rename with refresh" tool.
- Tag rename — currently destructive (no migration). Out of scope.
- Embedding model selection — ruvector default. Tunable later.

## 17. Risk register

| Risk | Mitigation |
|---|---|
| Title index over large identities (>50K notes) | Profile at 10K; if pain, ship HMAC-titles option (section 3.3). |
| Embedding pipeline leaks plaintexts unexpectedly | Toggle off by default; clear warning; client never calls `embeddings/upsert` when off. |
| Unresolved page refs accumulating | Reconcile runs synchronously on every page create; one-time backfill after 2 releases. |
| Embed cycle (`!((A))` inside A) | Render depth-limited to 1, then placeholder. |
| Page in Pages board duplicates an existing note title in a regular board | Resolution is global → `[[X]]` opens whichever was updated most recently. User-visible conflict hint. |
