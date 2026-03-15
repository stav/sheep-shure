CREATE TABLE IF NOT EXISTS convex_sync_decisions (
    cloud_record_id TEXT PRIMARY KEY,
    decision        TEXT NOT NULL,
    diff            TEXT,
    decided_at      DATETIME NOT NULL DEFAULT (datetime('now')),
    expires_at      DATETIME
);
