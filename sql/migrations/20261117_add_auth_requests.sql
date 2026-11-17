CREATE TABLE IF NOT EXISTS auth_requests (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    request_device_identifier TEXT NOT NULL,
    device_type INTEGER NOT NULL DEFAULT 14,
    request_ip TEXT NOT NULL,
    response_device_id TEXT,
    access_code_hash TEXT NOT NULL,
    public_key TEXT NOT NULL,
    enc_key TEXT,
    master_password_hash TEXT,
    approved INTEGER,
    created_at TEXT NOT NULL,
    created_at_epoch INTEGER NOT NULL,
    response_at TEXT,
    authentication_at TEXT,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_auth_requests_user
    ON auth_requests(user_id);
CREATE INDEX IF NOT EXISTS idx_auth_requests_created
    ON auth_requests(created_at_epoch);
