CREATE TABLE IF NOT EXISTS identities (
    id            TEXT PRIMARY KEY,
    friendly_name TEXT UNIQUE NOT NULL,
    created_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS note_shares (
    note_id           TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    owner_identity_id TEXT NOT NULL,
    shared_with_id    TEXT NOT NULL,
    encrypted_dek     BLOB,
    created_at        TEXT NOT NULL,
    PRIMARY KEY (note_id, shared_with_id)
);

CREATE INDEX IF NOT EXISTS idx_shares_shared_with ON note_shares(shared_with_id);
