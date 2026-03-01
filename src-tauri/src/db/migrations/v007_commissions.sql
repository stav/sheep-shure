-- V006: Commission tracking tables

-- ============================================================================
-- Commission rate table: one row per carrier + plan_type + year
-- ============================================================================

CREATE TABLE IF NOT EXISTS commission_rates (
    id              TEXT PRIMARY KEY,
    carrier_id      TEXT NOT NULL REFERENCES carriers(id),
    plan_type_code  TEXT NOT NULL REFERENCES plan_types(code),
    plan_year       INTEGER NOT NULL,
    initial_rate    REAL NOT NULL DEFAULT 0,
    renewal_rate    REAL NOT NULL DEFAULT 0,
    notes           TEXT,
    created_at      TEXT DEFAULT (datetime('now')),
    updated_at      TEXT DEFAULT (datetime('now')),
    UNIQUE(carrier_id, plan_type_code, plan_year)
);

CREATE TRIGGER IF NOT EXISTS trg_commission_rates_updated_at
    AFTER UPDATE ON commission_rates
    FOR EACH ROW
BEGIN
    UPDATE commission_rates SET updated_at = datetime('now') WHERE id = OLD.id;
END;

-- ============================================================================
-- Commission entries: per-client per-month line items from carrier statements
-- ============================================================================

CREATE TABLE IF NOT EXISTS commission_entries (
    id                TEXT PRIMARY KEY,
    client_id         TEXT REFERENCES clients(id),
    enrollment_id     TEXT REFERENCES enrollments(id),
    carrier_id        TEXT NOT NULL REFERENCES carriers(id),
    plan_type_code    TEXT,
    commission_month  TEXT NOT NULL,
    statement_amount  REAL,
    paid_amount       REAL,
    member_name       TEXT,
    member_id         TEXT,
    is_initial        INTEGER,
    expected_rate     REAL,
    rate_difference   REAL,
    status            TEXT CHECK (status IN ('OK', 'UNDERPAID', 'OVERPAID', 'MISSING', 'ZERO_RATE', 'UNMATCHED', 'PENDING')),
    import_batch_id   TEXT,
    notes             TEXT,
    created_at        TEXT DEFAULT (datetime('now')),
    updated_at        TEXT DEFAULT (datetime('now'))
);

CREATE TRIGGER IF NOT EXISTS trg_commission_entries_updated_at
    AFTER UPDATE ON commission_entries
    FOR EACH ROW
BEGIN
    UPDATE commission_entries SET updated_at = datetime('now') WHERE id = OLD.id;
END;

CREATE UNIQUE INDEX IF NOT EXISTS idx_commission_entries_unique
    ON commission_entries(carrier_id, client_id, commission_month)
    WHERE client_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_commission_entries_client
    ON commission_entries(client_id);

CREATE INDEX IF NOT EXISTS idx_commission_entries_carrier
    ON commission_entries(carrier_id);

CREATE INDEX IF NOT EXISTS idx_commission_entries_month
    ON commission_entries(commission_month);

CREATE INDEX IF NOT EXISTS idx_commission_entries_batch
    ON commission_entries(import_batch_id);

CREATE INDEX IF NOT EXISTS idx_commission_entries_status
    ON commission_entries(status);

-- ============================================================================
-- Commission deposits: lump-sum bank deposits per carrier/month
-- ============================================================================

CREATE TABLE IF NOT EXISTS commission_deposits (
    id              TEXT PRIMARY KEY,
    carrier_id      TEXT NOT NULL REFERENCES carriers(id),
    deposit_month   TEXT NOT NULL,
    deposit_amount  REAL NOT NULL DEFAULT 0,
    deposit_date    TEXT,
    reference       TEXT,
    notes           TEXT,
    created_at      TEXT DEFAULT (datetime('now')),
    updated_at      TEXT DEFAULT (datetime('now')),
    UNIQUE(carrier_id, deposit_month)
);

CREATE TRIGGER IF NOT EXISTS trg_commission_deposits_updated_at
    AFTER UPDATE ON commission_deposits
    FOR EACH ROW
BEGIN
    UPDATE commission_deposits SET updated_at = datetime('now') WHERE id = OLD.id;
END;
