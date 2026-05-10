CREATE TABLE IF NOT EXISTS board_shares (
    board_id          TEXT NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    owner_identity_id TEXT NOT NULL,
    shared_with_id    TEXT NOT NULL,
    created_at        TEXT NOT NULL,
    PRIMARY KEY (board_id, shared_with_id)
);

CREATE INDEX IF NOT EXISTS idx_board_shares_shared_with ON board_shares(shared_with_id);
