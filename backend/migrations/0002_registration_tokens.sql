ALTER TABLE teachers ADD COLUMN is_admin INTEGER NOT NULL DEFAULT 0;
CREATE TABLE registration_tokens (
 id TEXT PRIMARY KEY, label TEXT NOT NULL, token_hash TEXT NOT NULL UNIQUE,
 created_by TEXT NOT NULL REFERENCES teachers(id), created_at INTEGER NOT NULL DEFAULT (unixepoch()),
 expires_at INTEGER NOT NULL, used_by TEXT REFERENCES teachers(id), used_at INTEGER,
 revoked INTEGER NOT NULL DEFAULT 0
);
