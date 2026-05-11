-- Hierarchical key derivation: owner derives BEK locally (never stored).
-- Board members receive the BEK encrypted for them via ECDH wrap key.
CREATE TABLE IF NOT EXISTS board_keys (
    board_id      TEXT NOT NULL,
    identity_id   TEXT NOT NULL,
    encrypted_bek BLOB NOT NULL,
    created_at    TEXT NOT NULL,
    PRIMARY KEY (board_id, identity_id)
);

-- Remove owner self-share rows; with deterministic derivation owners never need
-- their DEK stored in the database.
DELETE FROM note_shares WHERE owner_identity_id = shared_with_id;
