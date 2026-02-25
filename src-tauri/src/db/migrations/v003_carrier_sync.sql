-- Carrier portal sync log table
CREATE TABLE IF NOT EXISTS carrier_sync_logs (
    id           TEXT PRIMARY KEY,
    carrier_id   TEXT NOT NULL REFERENCES carriers(id),
    synced_at    TEXT DEFAULT (datetime('now')),
    portal_count INTEGER DEFAULT 0,
    matched      INTEGER DEFAULT 0,
    disenrolled  INTEGER DEFAULT 0,
    new_found    INTEGER DEFAULT 0,
    status       TEXT DEFAULT 'COMPLETED'
);
