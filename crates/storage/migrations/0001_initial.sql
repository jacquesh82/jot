CREATE TABLE IF NOT EXISTS boards (
    id          TEXT PRIMARY KEY,
    identity_id TEXT NOT NULL,
    name        TEXT NOT NULL,
    position    INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS notes (
    id          TEXT PRIMARY KEY,
    note_type   TEXT NOT NULL,
    content     BLOB NOT NULL,
    thumbnail   BLOB,
    duration_ms INTEGER,
    color       TEXT NOT NULL DEFAULT '#FFFFFF',
    board_id    TEXT NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    position    INTEGER NOT NULL DEFAULT 0,
    blob_key    TEXT NOT NULL,
    size        INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS devices (
    id              TEXT PRIMARY KEY,
    identity_id     TEXT NOT NULL,
    pub_key_x25519  TEXT NOT NULL,
    pub_key_ed25519 TEXT NOT NULL,
    name            TEXT NOT NULL,
    last_seen       TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS link_sessions (
    token               TEXT PRIMARY KEY,
    code                TEXT NOT NULL,
    status              TEXT NOT NULL DEFAULT 'pending',
    pub_key_initiator   TEXT NOT NULL,
    encrypted_symkey    BLOB,
    expires_at          TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_notes_board_id ON notes(board_id);
CREATE INDEX IF NOT EXISTS idx_notes_position ON notes(board_id, position);
