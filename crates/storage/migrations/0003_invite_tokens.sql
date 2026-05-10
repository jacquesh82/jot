CREATE TABLE IF NOT EXISTS invite_tokens (
    token       TEXT PRIMARY KEY,
    created_by  TEXT NOT NULL,
    label       TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL,
    revoked_at  TEXT
);

CREATE INDEX IF NOT EXISTS idx_invite_tokens_created_by ON invite_tokens(created_by);
