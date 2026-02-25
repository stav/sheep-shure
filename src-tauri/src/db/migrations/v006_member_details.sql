ALTER TABLE clients ADD COLUMN member_record_locator TEXT;

CREATE TABLE IF NOT EXISTS client_providers (
    id          TEXT PRIMARY KEY,
    client_id   TEXT NOT NULL REFERENCES clients(id),
    first_name  TEXT,
    last_name   TEXT,
    npi         TEXT,
    specialty   TEXT,
    phone       TEXT,
    is_pcp      INTEGER DEFAULT 0,
    source      TEXT,
    is_active   INTEGER DEFAULT 1,
    created_at  TEXT DEFAULT (datetime('now')),
    updated_at  TEXT DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_client_providers_client ON client_providers(client_id);
