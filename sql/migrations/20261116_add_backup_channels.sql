CREATE TABLE IF NOT EXISTS backup_channels (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    type TEXT NOT NULL DEFAULT 'webdav',
    url TEXT NOT NULL,
    username TEXT,
    password_enc TEXT,
    enabled BOOLEAN NOT NULL DEFAULT 1,
    cadence TEXT NOT NULL DEFAULT 'daily',
    time TEXT NOT NULL DEFAULT '03:00',
    weekday INTEGER NOT NULL DEFAULT 1,
    utc_offset_minutes INTEGER NOT NULL DEFAULT 480,
    keep_count INTEGER NOT NULL DEFAULT 7,
    next_run_at INTEGER,
    last_run_at INTEGER,
    last_status TEXT,
    last_error TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS backup_runs (
    id TEXT PRIMARY KEY NOT NULL,
    channel_id TEXT NOT NULL,
    trigger TEXT NOT NULL,
    status TEXT NOT NULL,
    file_name TEXT,
    size INTEGER,
    error TEXT,
    started_at INTEGER NOT NULL,
    finished_at INTEGER
);

CREATE INDEX IF NOT EXISTS idx_backup_runs_channel
    ON backup_runs(channel_id, started_at);

CREATE TABLE IF NOT EXISTS backup_locks (
    channel_id TEXT PRIMARY KEY NOT NULL,
    locked_until INTEGER NOT NULL
);
