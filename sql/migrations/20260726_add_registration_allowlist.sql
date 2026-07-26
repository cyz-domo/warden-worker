CREATE TABLE IF NOT EXISTS registration_allowlist (
    email TEXT PRIMARY KEY NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_registration_allowlist_enabled
    ON registration_allowlist(enabled);
