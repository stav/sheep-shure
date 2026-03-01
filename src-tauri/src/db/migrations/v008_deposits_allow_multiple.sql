-- Allow multiple deposits per carrier per month.
-- SQLite cannot DROP constraints, so recreate the table without the UNIQUE.

CREATE TABLE IF NOT EXISTS commission_deposits_new (
    id              TEXT PRIMARY KEY,
    carrier_id      TEXT NOT NULL REFERENCES carriers(id),
    deposit_month   TEXT NOT NULL,
    deposit_amount  REAL NOT NULL DEFAULT 0,
    deposit_date    TEXT,
    reference       TEXT,
    notes           TEXT,
    created_at      TEXT DEFAULT (datetime('now')),
    updated_at      TEXT DEFAULT (datetime('now'))
);

INSERT INTO commission_deposits_new
    SELECT id, carrier_id, deposit_month, deposit_amount, deposit_date, reference, notes, created_at, updated_at
    FROM commission_deposits;

DROP TABLE commission_deposits;

ALTER TABLE commission_deposits_new RENAME TO commission_deposits;

-- Recreate the update trigger
CREATE TRIGGER IF NOT EXISTS trg_commission_deposits_updated_at
    AFTER UPDATE ON commission_deposits
    FOR EACH ROW
BEGIN
    UPDATE commission_deposits SET updated_at = datetime('now') WHERE id = OLD.id;
END;
