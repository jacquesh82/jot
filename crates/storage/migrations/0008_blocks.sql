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
    FOREIGN KEY (source_block_id) REFERENCES blocks(id) ON DELETE CASCADE,
    UNIQUE (source_block_id, target_kind, target_id, link_kind)
);

CREATE INDEX IF NOT EXISTS idx_block_links_target ON block_links(target_kind, target_id);
CREATE INDEX IF NOT EXISTS idx_block_links_source ON block_links(source_block_id);

CREATE TABLE IF NOT EXISTS tags (
    name        TEXT NOT NULL,
    identity_id TEXT NOT NULL,
    color       TEXT,
    created_at  TEXT NOT NULL,
    PRIMARY KEY (name, identity_id),
    FOREIGN KEY (identity_id) REFERENCES identities(id) ON DELETE CASCADE
);

ALTER TABLE notes ADD COLUMN title          BLOB;
ALTER TABLE notes ADD COLUMN is_journal     INTEGER NOT NULL DEFAULT 0;
ALTER TABLE notes ADD COLUMN journal_date   TEXT;
ALTER TABLE notes ADD COLUMN schema_version INTEGER NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_notes_journal_date ON notes(journal_date) WHERE journal_date IS NOT NULL;
